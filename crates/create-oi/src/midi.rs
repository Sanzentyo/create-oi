//! MIDI-to-robot-song conversion utilities.
//!
//! Parses a Standard MIDI File (SMF) and converts notes to [`SongNote`] values
//! suitable for [`define_song`](crate::create::Create::define_song).
//!
//! # Polyphony and multi-track files
//!
//! By default, only the first track that contains note events is used, and
//! polyphony within that track is resolved by "latest `NoteOn` wins" (previous
//! note cut). For complex multi-track or polyphonic MIDI files, enable
//! [`MidiConfig::merge_all_tracks`] to merge all tracks into a single voice
//! using [`MidiConfig::voice_selection`].
//!
//! # Limitations
//!
//! - **Rests are optional**: by default, gaps between notes are dropped because
//!   the robot's song format has no silence representation. Enable
//!   [`MidiConfig::include_rests`] to encode silence as rest notes (pitch 0).
//!   Use [`MidiConfig::trim_start`] and [`MidiConfig::trim_end`] to control
//!   whether leading/trailing silence is included.
//! - **MIDI Format 2** (sequential multi-song) is not supported.
//! - **SMPTE timecode** is not supported; use metrical timing.
//!
//! # Example
//!
//! ```no_run
//! use create_oi::midi::{midi_to_notes, notes_to_chunks, MidiConfig};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let bytes = std::fs::read("assets/midi/game-over.mid")?;
//! // Multi-track MIDI: merge all tracks, highest pitch wins.
//! let config = MidiConfig { merge_all_tracks: true, ..MidiConfig::default() };
//! let notes = midi_to_notes(&bytes, &config)?;
//! println!("{} notes parsed", notes.len());
//! let chunks = notes_to_chunks(notes);
//! println!("{} song chunks (≤16 notes each)", chunks.len());
//! # Ok(())
//! # }
//! ```

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use midly::{Format, MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

use create_oi_protocol::MAX_SONG_NOTES;

use crate::types::SongNote;

/// Configuration for MIDI parsing.
#[derive(Debug, Clone)]
pub struct MidiConfig {
    /// Track index to use (0-based). `None` = auto-detect the first track
    /// that contains at least one `NoteOn` event with nonzero velocity.
    ///
    /// Ignored when [`merge_all_tracks`](Self::merge_all_tracks) is `true`.
    pub track: Option<usize>,
    /// Override the tempo (µs per beat). `None` = read from the MIDI file;
    /// defaults to 500 000 (120 BPM) if no tempo event is present.
    pub tempo_micros_per_beat: Option<u32>,
    /// When `true`, notes from **all** tracks are merged into a single
    /// monophonic voice using the sweep-line algorithm.
    ///
    /// [`voice_selection`](Self::voice_selection) determines which note wins
    /// when multiple are active simultaneously. Useful for complex multi-track
    /// MIDI files where a simple single-track extraction produces many very
    /// short notes.
    ///
    /// Default: `false` (single-track extraction, same behaviour as before).
    pub merge_all_tracks: bool,
    /// Voice selection policy used when [`merge_all_tracks`](Self::merge_all_tracks)
    /// is `true`. Default: [`VoiceSelection::HighestPitch`].
    pub voice_selection: VoiceSelection,
    /// When `true` (default), MIDI channel 10 (0-indexed: 9) is excluded from
    /// the multi-track merge. Channel 10 is conventionally reserved for
    /// percussion in General MIDI; including drums in a melodic monophonization
    /// usually produces poor results.
    ///
    /// Ignored when [`channel`](Self::channel) is explicitly set (the explicit
    /// channel selection takes precedence).
    ///
    /// Only used when [`merge_all_tracks`](Self::merge_all_tracks) is `true`.
    pub filter_percussion: bool,
    /// Restrict note extraction to a single MIDI channel (0-indexed, 0–15).
    /// `None` (default) includes all channels.
    ///
    /// When set, this overrides [`filter_percussion`](Self::filter_percussion):
    /// even `channel = Some(9)` is allowed so that percussion can be extracted
    /// deliberately. Valid values are `0..=15`; [`MidiError::InvalidChannel`]
    /// is returned for values outside this range.
    ///
    /// Works in both single-track and multi-track merge modes.
    pub channel: Option<u8>,
    /// When `true` (default), silence gaps between notes are encoded as rest
    /// notes (MIDI pitch 0) in the output. Set to `false` to drop gaps so that
    /// notes play back-to-back.
    ///
    /// Note spans with out-of-range pitches (those that would be dropped by
    /// the converter) are treated as silence for this purpose.
    pub include_rests: bool,
    /// Trim the leading silence before the first audible note.
    ///
    /// Only effective when [`include_rests`](Self::include_rests) is `true`.
    /// Default: `true`.
    pub trim_start: bool,
    /// Trim the trailing silence after the last audible note.
    ///
    /// Only effective when [`include_rests`](Self::include_rests) is `true`.
    /// Default: `true`.
    pub trim_end: bool,
}

impl Default for MidiConfig {
    fn default() -> Self {
        Self {
            track: None,
            tempo_micros_per_beat: None,
            merge_all_tracks: false,
            voice_selection: VoiceSelection::default(),
            filter_percussion: true,
            channel: None,
            include_rests: true,
            trim_start: true,
            trim_end: true,
        }
    }
}

/// Policy for selecting the active note when multiple are sounding
/// simultaneously during multi-track monophonization.
///
/// Only used when [`MidiConfig::merge_all_tracks`] is `true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VoiceSelection {
    /// The sounding note with the highest MIDI pitch wins (soprano voice).
    ///
    /// This matches the melody in most pop and game music, where the melody
    /// sits above the harmony and bass.
    #[default]
    HighestPitch,
    /// The sounding note with the lowest MIDI pitch wins (bass voice).
    LowestPitch,
}

/// Error type for MIDI parsing.
#[derive(Debug)]
pub enum MidiError {
    /// The MIDI byte stream could not be parsed.
    Parse(midly::Error),
    /// The parsed MIDI file contains no usable notes (all pitches were out of
    /// the robot's range 31–127, or the file is empty).
    NoNotes,
    /// The file uses SMPTE timecode, which cannot be converted to robot song
    /// units. Use metrical (tempo-based) timing instead.
    UnsupportedTiming,
    /// The file is MIDI Format 2 (sequential multi-song), which this crate
    /// does not support.
    UnsupportedFormat,
    /// The timing header has `ticks_per_beat == 0`, which would cause a
    /// division by zero in duration conversion.
    InvalidTiming,
    /// The requested MIDI channel number is out of range. Valid channels are
    /// 0–15 (0-indexed); the robot's OI has no concept of MIDI channels.
    InvalidChannel(u8),
}

impl core::fmt::Display for MidiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MidiError::Parse(e) => write!(f, "MIDI parse error: {e}"),
            MidiError::NoNotes => write!(f, "no playable notes in MIDI file"),
            MidiError::UnsupportedTiming => {
                write!(f, "SMPTE timecode is not supported; use metrical timing")
            }
            MidiError::UnsupportedFormat => {
                write!(f, "MIDI Format 2 (sequential) is not supported")
            }
            MidiError::InvalidTiming => {
                write!(f, "ticks_per_beat is zero; invalid MIDI file")
            }
            MidiError::InvalidChannel(ch) => {
                write!(f, "channel {ch} is out of range; valid channels are 0–15")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MidiError {}

/// A tempo change: the tempo becomes `micros_per_beat` at `abs_tick`.
#[derive(Clone, Copy, Debug)]
struct TempoChange {
    abs_tick: u64,
    micros_per_beat: u32,
}

/// Collect all tempo change events from every track in the file.
///
/// Events at the same tick are de-duplicated: only the last one (in source
/// order) is kept, matching the "last writer wins" semantics of MIDI.
fn build_tempo_map(smf: &Smf<'_>) -> Vec<TempoChange> {
    let mut changes: Vec<TempoChange> = Vec::new();

    for track in &smf.tracks {
        let mut abs_tick: u64 = 0;
        for event in track {
            abs_tick += u64::from(event.delta.as_int());
            if let TrackEventKind::Meta(MetaMessage::Tempo(t)) = event.kind {
                // Remove any existing entry at exactly the same tick so the
                // last-seen value wins (stable sort order is preserved because
                // we scan tracks sequentially).
                changes.retain(|c| c.abs_tick != abs_tick);
                changes.push(TempoChange {
                    abs_tick,
                    micros_per_beat: t.as_int(),
                });
            }
        }
    }

    changes.sort_by_key(|c| c.abs_tick);
    changes
}

/// Return the tempo (µs/beat) that is in effect at `abs_tick`.
fn tempo_at(tempo_map: &[TempoChange], abs_tick: u64) -> u32 {
    tempo_map
        .iter()
        .rev()
        .find(|c| c.abs_tick <= abs_tick)
        .map(|c| c.micros_per_beat)
        .unwrap_or(500_000) // 120 BPM default
}

/// Convert a note's span `[start_tick, start_tick + dur_ticks)` to robot song
/// duration units (1/64 s = 15 625 µs).
///
/// Uses u128 arithmetic throughout to handle very long notes without overflow.
/// Piecewise-integrates across tempo-change boundaries so mid-note tempo
/// changes are handled correctly.
///
/// Returns a value clamped to `1..=255`.
fn ticks_to_robot_units(
    start_tick: u64,
    dur_ticks: u64,
    tempo_map: &[TempoChange],
    ticks_per_beat: u32,
) -> u8 {
    debug_assert!(ticks_per_beat > 0);

    if dur_ticks == 0 {
        return 1;
    }

    let tpb = u128::from(ticks_per_beat);
    let end_tick = start_tick + dur_ticks;
    let mut total_micros: u128 = 0;
    let mut cursor = start_tick;

    for change in tempo_map {
        if change.abs_tick <= cursor {
            continue;
        }
        if change.abs_tick >= end_tick {
            break;
        }
        let segment_ticks = u128::from(change.abs_tick - cursor);
        let current_tempo = u128::from(tempo_at(tempo_map, cursor));
        total_micros =
            total_micros.saturating_add(segment_ticks.saturating_mul(current_tempo) / tpb);
        cursor = change.abs_tick;
    }

    // Final segment from cursor to end_tick.
    let remaining = u128::from(end_tick - cursor);
    let final_tempo = u128::from(tempo_at(tempo_map, cursor));
    total_micros = total_micros.saturating_add(remaining.saturating_mul(final_tempo) / tpb);

    // 1 robot unit = 1/64 s = 15 625 µs.
    let units = total_micros / 15_625;
    units.clamp(1, 255) as u8
}

/// An internal note event produced during multi-track sweep-line collection.
///
/// Derived `Ord` sorts by `(abs_tick, is_on, pitch, channel)`. Because `false
/// < true`, `NoteOff` events (is_on = false) sort before `NoteOn` events at
/// the same tick, which ensures a clean handoff when one note ends exactly as
/// another begins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct NoteEvent {
    abs_tick: u64,
    /// `false` = NoteOff (sorts first at same tick); `true` = NoteOn.
    is_on: bool,
    pitch: u8,
    channel: u8,
}

/// Collect all NoteOn/NoteOff events from every track, optionally skipping
/// MIDI channel 10 (0-indexed: 9, percussion) or filtering to a single channel.
fn collect_note_events(
    smf: &Smf<'_>,
    filter_percussion: bool,
    channel_filter: Option<u8>,
) -> Vec<NoteEvent> {
    let mut events = Vec::new();
    for track in &smf.tracks {
        let mut abs_tick: u64 = 0;
        for event in track {
            abs_tick += u64::from(event.delta.as_int());
            if let TrackEventKind::Midi { channel, message } = event.kind {
                let ch = channel.as_int();
                match channel_filter {
                    Some(only_ch) => {
                        if ch != only_ch {
                            continue;
                        }
                    }
                    None => {
                        if filter_percussion && ch == 9 {
                            continue;
                        }
                    }
                }
                let (is_on, pitch) = match message {
                    MidiMessage::NoteOn { key, vel } => (vel.as_int() > 0, key.as_int()),
                    MidiMessage::NoteOff { key, .. } => (false, key.as_int()),
                    _ => continue,
                };
                events.push(NoteEvent {
                    abs_tick,
                    is_on,
                    pitch,
                    channel: ch,
                });
            }
        }
    }
    events
}

/// Sweep-line monophonization over a sorted slice of note events.
///
/// Maintains a count-map of currently-sounding pitches. At each tick where the
/// active set changes, the current winner (highest or lowest pitch) is checked;
/// if it changed, the previous segment is emitted as a [`SongNote`].
///
/// Zero-duration segments (two NoteOn events at the exact same tick) are
/// discarded. Dangling notes with no NoteOff are emitted at the end with their
/// accumulated duration (or clamped to 1 robot unit if duration is zero).
///
/// When `include_rests` is `true`, silence gaps (including spans where the
/// winner is out of the 31–127 range) are emitted as rest notes (pitch 0).
#[allow(clippy::too_many_arguments)]
fn monophonize_events(
    events: &[NoteEvent],
    voice_selection: VoiceSelection,
    tempo_map: &[TempoChange],
    ticks_per_beat: u32,
    include_rests: bool,
    trim_start: bool,
    trim_end: bool,
    track_end_tick: u64,
) -> Vec<SongNote> {
    let mut active: BTreeMap<u8, u32> = BTreeMap::new(); // pitch → active count
    let mut current_winner: Option<u8> = None;
    let mut segment_start: u64 = 0;
    let mut last_tick: u64 = 0;
    let mut notes: Vec<SongNote> = Vec::new();

    // Start of the current silence gap; `Some(0)` tracks leading silence.
    let mut rest_start: Option<u64> = if include_rests { Some(0) } else { None };
    // Whether any audible note has been started yet.
    let mut first_note_started = false;

    for event in events {
        last_tick = event.abs_tick;

        if event.is_on {
            *active.entry(event.pitch).or_insert(0) += 1;
        } else {
            match active.get_mut(&event.pitch) {
                Some(count) if *count > 1 => *count -= 1,
                Some(_) => {
                    active.remove(&event.pitch);
                }
                None => {} // Spurious NoteOff — ignore.
            }
        }

        let new_winner = match voice_selection {
            VoiceSelection::HighestPitch => active.keys().next_back().copied(),
            VoiceSelection::LowestPitch => active.keys().next().copied(),
        };

        if new_winner != current_winner {
            // Flush the previous segment.
            if let Some(pitch) = current_winner {
                let dur = event.abs_tick.saturating_sub(segment_start);
                if dur > 0 {
                    if let Some(note) =
                        make_note(pitch, segment_start, dur, tempo_map, ticks_per_beat)
                    {
                        notes.push(note);
                        // Audible note ended; silence starts here.
                        if include_rests {
                            rest_start = Some(event.abs_tick);
                        }
                    }
                    // Out-of-range segment: rest_start unchanged (silence continues).
                }
            }
            // previous was None (silence) — rest_start already set; no flush needed.

            // Start the new segment.
            match new_winner {
                Some(new_pitch) if (31u8..=127).contains(&new_pitch) => {
                    // Audible note starting: emit any pending rest.
                    if include_rests {
                        if let Some(rs) = rest_start.take() {
                            let is_leading = !first_note_started;
                            if !(trim_start && is_leading) {
                                if let Some(rest) =
                                    make_rest(rs, event.abs_tick - rs, tempo_map, ticks_per_beat)
                                {
                                    notes.push(rest);
                                }
                            }
                        }
                    }
                    first_note_started = true;
                }
                Some(_) => {
                    // Out-of-range winner: treat as silence.
                    // Open rest_start if not already tracking silence.
                    if include_rests && rest_start.is_none() {
                        rest_start = Some(event.abs_tick);
                    }
                }
                None => {
                    // No active notes: silence.
                    // rest_start was already set when the last note ended (above).
                }
            }

            segment_start = event.abs_tick;
            current_winner = new_winner;
        }
    }

    // Flush final dangling segment (always emit, even if duration is zero).
    if let Some(pitch) = current_winner {
        let dur = last_tick.saturating_sub(segment_start);
        if let Some(note) = make_note(pitch, segment_start, dur, tempo_map, ticks_per_beat) {
            notes.push(note);
            if include_rests {
                rest_start = Some(last_tick);
            }
        }
        // Out-of-range dangling note: rest_start unchanged.
    }

    // Trailing rest (after last audible note, up to track end).
    if include_rests && !trim_end {
        let effective_end = track_end_tick.max(last_tick);
        if let Some(rs) = rest_start {
            if let Some(rest) = make_rest(rs, effective_end - rs, tempo_map, ticks_per_beat) {
                notes.push(rest);
            }
        }
    }

    notes
}

/// Parse a Standard MIDI File and extract a sequence of [`SongNote`]s.
///
/// # Single-track mode (default)
///
/// Only the selected track (or the first track with note events) is used for
/// notes. Tempo events from **all** tracks are collected into a global tempo
/// map, so that a conductor track in a Format 1 file is handled correctly.
///
/// Polyphony within the selected track is resolved with "latest `NoteOn` wins":
/// a new `NoteOn` cuts the previous active note. Chords are reduced to the most
/// recently started note.
///
/// # Multi-track merge mode
///
/// When [`MidiConfig::merge_all_tracks`] is `true`, note events from all tracks
/// are merged into a single timeline and reduced to one voice using the
/// sweep-line algorithm. The active note is selected according to
/// [`MidiConfig::voice_selection`]. This dramatically reduces the chunk count
/// for complex MIDI files with many short overlapping notes.
///
/// # Rests
///
/// By default, silence between notes is dropped. Enable
/// [`MidiConfig::include_rests`] to encode gaps as rest notes (pitch 0).
/// Use [`MidiConfig::trim_start`] / [`MidiConfig::trim_end`] to control
/// whether leading/trailing silence is included.
///
/// # Errors
///
/// Returns [`MidiError`] if the file cannot be parsed, contains no usable
/// notes, uses unsupported timing, or is MIDI Format 2.
pub fn midi_to_notes(midi_bytes: &[u8], config: &MidiConfig) -> Result<Vec<SongNote>, MidiError> {
    let smf = Smf::parse(midi_bytes).map_err(MidiError::Parse)?;

    // Reject Format 2 (sequential multi-song).
    if smf.header.format == Format::Sequential {
        return Err(MidiError::UnsupportedFormat);
    }

    let ticks_per_beat = match smf.header.timing {
        Timing::Metrical(t) => {
            let v = u32::from(t.as_int());
            if v == 0 {
                return Err(MidiError::InvalidTiming);
            }
            v
        }
        Timing::Timecode(..) => return Err(MidiError::UnsupportedTiming),
    };

    // Validate channel if provided.
    if let Some(ch) = config.channel {
        if ch > 15 {
            return Err(MidiError::InvalidChannel(ch));
        }
    }

    // Build a global tempo map from all tracks, then apply any override.
    let mut tempo_map = build_tempo_map(&smf);
    if let Some(override_tempo) = config.tempo_micros_per_beat {
        tempo_map.clear();
        tempo_map.push(TempoChange {
            abs_tick: 0,
            micros_per_beat: override_tempo,
        });
    }

    // Compute the track end tick only when needed (trailing rest support).
    let track_end_tick = if config.include_rests && !config.trim_end {
        find_max_track_end_tick(&smf)
    } else {
        0
    };

    let notes = if config.merge_all_tracks {
        let mut events = collect_note_events(&smf, config.filter_percussion, config.channel);
        events.sort_unstable();
        monophonize_events(
            &events,
            config.voice_selection,
            &tempo_map,
            ticks_per_beat,
            config.include_rests,
            config.trim_start,
            config.trim_end,
            track_end_tick,
        )
    } else {
        single_track_notes(
            &smf,
            config.track,
            config.channel,
            &tempo_map,
            ticks_per_beat,
            config.include_rests,
            config.trim_start,
            config.trim_end,
        )?
    };

    if notes.is_empty() {
        Err(MidiError::NoNotes)
    } else {
        Ok(notes)
    }
}

/// Extract notes from a single track using "latest NoteOn wins" monophony.
#[allow(clippy::too_many_arguments)]
fn single_track_notes(
    smf: &Smf<'_>,
    track_selection: Option<usize>,
    channel_filter: Option<u8>,
    tempo_map: &[TempoChange],
    ticks_per_beat: u32,
    include_rests: bool,
    trim_start: bool,
    trim_end: bool,
) -> Result<Vec<SongNote>, MidiError> {
    let track_idx = match track_selection {
        Some(idx) => idx,
        None => smf
            .tracks
            .iter()
            .position(|track| {
                track.iter().any(|e| {
                    if let TrackEventKind::Midi { channel, message } = e.kind {
                        if channel_filter.is_some_and(|ch| channel.as_int() != ch) {
                            return false;
                        }
                        matches!(message, MidiMessage::NoteOn { vel, .. } if vel.as_int() > 0)
                    } else {
                        false
                    }
                })
            })
            .ok_or(MidiError::NoNotes)?,
    };

    let selected_track = smf.tracks.get(track_idx).ok_or(MidiError::NoNotes)?;

    let mut notes: Vec<SongNote> = Vec::new();
    let mut abs_tick: u64 = 0;
    // Currently sounding note: (pitch, start_tick).
    let mut active: Option<(u8, u64)> = None;
    // Start of the current silence gap (tick); `Some(0)` at the beginning so
    // that leading silence before the first note is tracked.
    let mut gap_start: Option<u64> = if include_rests { Some(0) } else { None };
    // Whether any audible note (pitch 31–127) has started yet.
    let mut first_audible_started = false;
    // Track end tick (from EndOfTrack meta or last event tick).
    let mut track_end_tick: u64 = 0;

    for event in selected_track {
        abs_tick += u64::from(event.delta.as_int());

        match event.kind {
            TrackEventKind::Meta(MetaMessage::EndOfTrack) => {
                track_end_tick = abs_tick;
            }
            TrackEventKind::Midi {
                channel,
                message: MidiMessage::NoteOn { key, vel },
            } => {
                if channel_filter.is_some_and(|ch| channel.as_int() != ch) {
                    continue;
                }
                if vel.as_int() > 0 {
                    // New note: cut any active note (monophonic extraction, no gap).
                    if let Some((pitch, start)) = active.take() {
                        if let Some(note) =
                            make_note(pitch, start, abs_tick - start, tempo_map, ticks_per_beat)
                        {
                            notes.push(note);
                            // In-range note was cut; new note immediately follows —
                            // no gap is created, so don't open gap_start here.
                        }
                        // If out-of-range: gap_start remains unchanged (silence continues).
                    }
                    let key_u8 = key.as_int();
                    let in_range = (31u8..=127).contains(&key_u8);
                    if in_range {
                        // Audible note starting: emit any pending rest.
                        if include_rests {
                            if let Some(gs) = gap_start.take() {
                                let is_leading = !first_audible_started;
                                if !(trim_start && is_leading) {
                                    if let Some(rest) =
                                        make_rest(gs, abs_tick - gs, tempo_map, ticks_per_beat)
                                    {
                                        notes.push(rest);
                                    }
                                }
                            }
                        }
                        first_audible_started = true;
                    }
                    // Out-of-range NoteOn: treat as silence; don't update gap_start.
                    active = Some((key_u8, abs_tick));
                } else {
                    // NoteOn with vel == 0 is equivalent to NoteOff.
                    if let Some((pitch, start)) = active {
                        if pitch == key.as_int() {
                            active = None;
                            if let Some(note) =
                                make_note(pitch, start, abs_tick - start, tempo_map, ticks_per_beat)
                            {
                                notes.push(note);
                                // Audible note ended: silence starts here.
                                if include_rests {
                                    gap_start = Some(abs_tick);
                                }
                            }
                            // Out-of-range: gap_start unchanged (silence was already tracked).
                        }
                    }
                }
            }
            TrackEventKind::Midi {
                channel,
                message: MidiMessage::NoteOff { key, .. },
            } => {
                if channel_filter.is_some_and(|ch| channel.as_int() != ch) {
                    continue;
                }
                if let Some((pitch, start)) = active {
                    if pitch == key.as_int() {
                        active = None;
                        if let Some(note) =
                            make_note(pitch, start, abs_tick - start, tempo_map, ticks_per_beat)
                        {
                            notes.push(note);
                            if include_rests {
                                gap_start = Some(abs_tick);
                            }
                        }
                        // Out-of-range: gap_start unchanged.
                    }
                }
            }
            _ => {}
        }
    }

    // Dangling note still active at end-of-track.
    if let Some((pitch, start)) = active {
        if let Some(note) = make_note(pitch, start, abs_tick - start, tempo_map, ticks_per_beat) {
            notes.push(note);
            if include_rests {
                gap_start = Some(abs_tick);
            }
        }
        // Out-of-range dangling note: gap_start unchanged.
    }

    // Trailing rest (after last audible note, up to EndOfTrack).
    if include_rests && !trim_end {
        let effective_end = track_end_tick.max(abs_tick);
        if let Some(gs) = gap_start {
            if let Some(rest) = make_rest(gs, effective_end - gs, tempo_map, ticks_per_beat) {
                notes.push(rest);
            }
        }
    }

    Ok(notes)
}

/// Attempt to build a [`SongNote`] from a MIDI pitch and timing data.
///
/// Returns `None` if the pitch is outside the robot's range (31–127).
fn make_note(
    pitch: u8,
    start_tick: u64,
    dur_ticks: u64,
    tempo_map: &[TempoChange],
    ticks_per_beat: u32,
) -> Option<SongNote> {
    let duration = ticks_to_robot_units(start_tick, dur_ticks, tempo_map, ticks_per_beat);
    SongNote::new(pitch, duration).ok()
}

/// Build a rest [`SongNote`] (pitch = 0) from a silence span.
///
/// Returns `None` if `dur_ticks` is zero (explicit check before calling
/// `ticks_to_robot_units`, which clamps zero to 1).
fn make_rest(
    start_tick: u64,
    dur_ticks: u64,
    tempo_map: &[TempoChange],
    ticks_per_beat: u32,
) -> Option<SongNote> {
    if dur_ticks == 0 {
        return None;
    }
    let duration = ticks_to_robot_units(start_tick, dur_ticks, tempo_map, ticks_per_beat);
    Some(SongNote::rest(duration))
}

/// Find the maximum track end tick across all tracks.
///
/// Uses the `EndOfTrack` meta event if present; falls back to the tick of the
/// last event in the track if there is no explicit `EndOfTrack`.
fn find_max_track_end_tick(smf: &Smf<'_>) -> u64 {
    smf.tracks
        .iter()
        .map(|track| {
            let mut abs_tick: u64 = 0;
            let mut end_tick: u64 = 0;
            for event in track {
                abs_tick += u64::from(event.delta.as_int());
                if let TrackEventKind::Meta(MetaMessage::EndOfTrack) = event.kind {
                    end_tick = abs_tick;
                }
            }
            end_tick.max(abs_tick)
        })
        .max()
        .unwrap_or(0)
}

/// Split a flat [`Vec<SongNote>`] into chunks of at most [`MAX_SONG_NOTES`]
/// (16) notes each.
///
/// Each chunk can be uploaded to a single song slot with
/// [`define_song`](crate::create::Create::define_song).
pub fn notes_to_chunks(notes: Vec<SongNote>) -> Vec<Vec<SongNote>> {
    notes.chunks(MAX_SONG_NOTES).map(|c| c.to_vec()).collect()
}

/// Read the initial tempo from a Standard MIDI File, in **microseconds per
/// beat** (µs/beat).
///
/// Returns the tempo that is in effect at tick 0 according to the file's
/// tempo map.  If no `Set Tempo` meta-event is present the MIDI default of
/// **500 000 µs/beat** (120 BPM) is returned.
///
/// To convert to BPM: `60_000_000 / tempo_micros_per_beat`.
///
/// # Errors
///
/// Returns [`MidiError`] if the file cannot be parsed, uses SMPTE timing,
/// is MIDI Format 2, or has an invalid ticks-per-beat of zero.
///
/// # Example
///
/// ```no_run
/// use create_oi::midi::{midi_initial_tempo, MidiError};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let bytes = std::fs::read("assets/midi/game-over.mid")?;
/// let tempo = midi_initial_tempo(&bytes)?;
/// println!("Initial tempo: {} BPM", 60_000_000 / tempo);
/// # Ok(())
/// # }
/// ```
pub fn midi_initial_tempo(midi_bytes: &[u8]) -> Result<u32, MidiError> {
    let smf = Smf::parse(midi_bytes).map_err(MidiError::Parse)?;

    if smf.header.format == Format::Sequential {
        return Err(MidiError::UnsupportedFormat);
    }
    match smf.header.timing {
        Timing::Timecode(..) => return Err(MidiError::UnsupportedTiming),
        Timing::Metrical(t) => {
            if t.as_int() == 0 {
                return Err(MidiError::InvalidTiming);
            }
        }
    }

    let tempo_map = build_tempo_map(&smf);
    Ok(tempo_at(&tempo_map, 0))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal SMF Format 0 byte stream with one track.
    fn smf0(ticks_per_beat: u16, track_bytes: &[u8]) -> Vec<u8> {
        let len = track_bytes.len() as u32;
        let mut out = Vec::new();
        // MThd
        out.extend_from_slice(b"MThd");
        out.extend_from_slice(&6u32.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes()); // format 0
        out.extend_from_slice(&1u16.to_be_bytes()); // 1 track
        out.extend_from_slice(&ticks_per_beat.to_be_bytes());
        // MTrk
        out.extend_from_slice(b"MTrk");
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(track_bytes);
        out
    }

    /// Build a minimal SMF Format 1 byte stream with two tracks.
    fn smf1(ticks_per_beat: u16, track0: &[u8], track1: &[u8]) -> Vec<u8> {
        let len0 = track0.len() as u32;
        let len1 = track1.len() as u32;
        let mut out = Vec::new();
        // MThd
        out.extend_from_slice(b"MThd");
        out.extend_from_slice(&6u32.to_be_bytes());
        out.extend_from_slice(&1u16.to_be_bytes()); // format 1
        out.extend_from_slice(&2u16.to_be_bytes()); // 2 tracks
        out.extend_from_slice(&ticks_per_beat.to_be_bytes());
        // Track 0
        out.extend_from_slice(b"MTrk");
        out.extend_from_slice(&len0.to_be_bytes());
        out.extend_from_slice(track0);
        // Track 1
        out.extend_from_slice(b"MTrk");
        out.extend_from_slice(&len1.to_be_bytes());
        out.extend_from_slice(track1);
        out
    }

    /// Encode a 3-byte (24-bit) big-endian tempo value.
    fn tempo_bytes(micros_per_beat: u32) -> [u8; 3] {
        let b = micros_per_beat.to_be_bytes();
        [b[1], b[2], b[3]]
    }

    // ── Basic note ──────────────────────────────────────────────────────────

    #[test]
    fn test_single_quarter_note_120bpm() {
        // 120 ticks/beat, 500 000 µs/beat (120 BPM), C4 (MIDI 60) quarter note
        // Expected: robot_units = (120 × 500 000) / (120 × 15 625) = 32
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], // Tempo = 500 000
            0x00, 0x90, 0x3C, 0x40, // delta=0, NoteOn ch0, C4 (60), vel 64
            0x78, 0x80, 0x3C, 0x00, // delta=120, NoteOff ch0, C4, vel 0
            0x00, 0xFF, 0x2F, 0x00, // EndOfTrack
        ];
        let midi = smf0(120, track);
        let notes = midi_to_notes(&midi, &MidiConfig::default()).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 60);
        assert_eq!(notes[0].duration_64ths(), 32);
    }

    // ── NoteOn vel=0 == NoteOff ─────────────────────────────────────────────

    #[test]
    fn test_noteon_vel0_acts_as_noteoff() {
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C,
            0x40, // NoteOn C4 vel=64
            0x78, 0x90, 0x3C, 0x00, // NoteOn C4 vel=0  (= NoteOff)
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        let notes = midi_to_notes(&midi, &MidiConfig::default()).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].duration_64ths(), 32);
    }

    // ── Tempo from track 0 applied to track 1 (Format 1) ───────────────────

    #[test]
    fn test_format1_tempo_from_conductor_track() {
        // Track 0: tempo only (120 BPM)
        let tb = tempo_bytes(500_000);
        let track0: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0xFF, 0x2F, 0x00,
        ];
        // Track 1: C4 quarter note at 120 ticks/beat
        let track1: &[u8] = &[
            0x00, 0x90, 0x3C, 0x40, 0x78, 0x80, 0x3C, 0x00, 0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf1(120, track0, track1);
        let notes = midi_to_notes(&midi, &MidiConfig::default()).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 60);
        assert_eq!(notes[0].duration_64ths(), 32); // 500 000 µs / 15 625 = 32
    }

    // ── Tempo override ──────────────────────────────────────────────────────

    #[test]
    fn test_tempo_override() {
        // MIDI has 120 BPM, but we override to 250 BPM (240 000 µs/beat).
        // quarter note = 120 ticks; expected = 240 000 / 15 625 ≈ 15 (≈0.235 s)
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C, 0x40, 0x78, 0x80, 0x3C,
            0x00, 0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        let config = MidiConfig {
            tempo_micros_per_beat: Some(240_000),
            ..Default::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].duration_64ths(), 15);
    }

    // ── Out-of-range pitches filtered ───────────────────────────────────────

    #[test]
    fn test_out_of_range_pitches_return_no_notes() {
        let tb = tempo_bytes(500_000);
        // Pitch 20 < 31 (too low), pitch 128 > 127 (too high — actually MIDI
        // max is 127, but SongNote rejects < 31)
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x14,
            0x40, // NoteOn pitch 20 (below 31)
            0x78, 0x80, 0x14, 0x00, 0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        let err = midi_to_notes(&midi, &MidiConfig::default()).unwrap_err();
        assert!(matches!(err, MidiError::NoNotes));
    }

    #[test]
    fn test_out_of_range_pitches_skipped_valid_remain() {
        let tb = tempo_bytes(500_000);
        // pitch 20 skipped, pitch 60 kept
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x14,
            0x40, // NoteOn pitch 20
            0x78, 0x80, 0x14, 0x00, // NoteOff pitch 20
            0x00, 0x90, 0x3C, 0x40, // NoteOn pitch 60 (C4)
            0x78, 0x80, 0x3C, 0x00, // NoteOff pitch 60
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        let notes = midi_to_notes(&midi, &MidiConfig::default()).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 60);
    }

    // ── SMPTE timing rejected ────────────────────────────────────────────────

    #[test]
    fn test_smpte_timing_rejected() {
        // Header with SMPTE timing: high byte has MSB set (0xE7 = -25 fps),
        // low byte = subframe (0x28 = 40). ticks_per_beat field encodes SMPTE.
        let track: &[u8] = &[0x00, 0xFF, 0x2F, 0x00];
        // Override the ticks_per_beat bytes manually.
        let mut midi = smf0(0x1928, track); // 0x1928 — actually use proper SMPTE
        // The SMPTE indicator is: bit 15 of ticks_per_beat = 1.
        // Replace bytes 12-13 (ticks_per_beat in the header) with 0xE728.
        midi[12] = 0xE7;
        midi[13] = 0x28;
        let err = midi_to_notes(&midi, &MidiConfig::default()).unwrap_err();
        assert!(matches!(err, MidiError::UnsupportedTiming));
    }

    // ── Format 2 rejected ───────────────────────────────────────────────────

    #[test]
    fn test_format2_rejected() {
        // Build a Format 2 header manually.
        let track: &[u8] = &[0x00, 0xFF, 0x2F, 0x00];
        let len = track.len() as u32;
        let mut midi = Vec::new();
        midi.extend_from_slice(b"MThd");
        midi.extend_from_slice(&6u32.to_be_bytes());
        midi.extend_from_slice(&2u16.to_be_bytes()); // format 2 = Sequential
        midi.extend_from_slice(&1u16.to_be_bytes());
        midi.extend_from_slice(&120u16.to_be_bytes());
        midi.extend_from_slice(b"MTrk");
        midi.extend_from_slice(&len.to_be_bytes());
        midi.extend_from_slice(track);
        let err = midi_to_notes(&midi, &MidiConfig::default()).unwrap_err();
        assert!(matches!(err, MidiError::UnsupportedFormat));
    }

    // ── Dangling note at end of track ────────────────────────────────────────

    #[test]
    fn test_dangling_active_note_at_end_of_track() {
        // NoteOn never followed by NoteOff — note runs to end-of-track.
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C, 0x40, // NoteOn C4
            // No NoteOff — but EndOfTrack is at abs_tick=0 from NoteOn tick
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        // The note has zero ticks before EndOfTrack, so duration clamps to 1.
        let notes = midi_to_notes(&midi, &MidiConfig::default()).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 60);
        // duration clamped to 1 (zero ticks → 1 robot unit)
        assert_eq!(notes[0].duration_64ths(), 1);
    }

    // ── Monophonic cut (new NoteOn before NoteOff) ───────────────────────────

    #[test]
    fn test_monophonic_cut_on_new_noteon() {
        // C4 starts, then G4 starts before C4 ends → C4 is cut at G4 start.
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C, 0x40, // NoteOn C4
            0x3C, 0x90, 0x43, 0x40, // delta=60, NoteOn G4 → cuts C4 at tick 60
            0x3C, 0x80, 0x43, 0x00, // delta=60, NoteOff G4
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        let notes = midi_to_notes(&midi, &MidiConfig::default()).unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].midi_note(), 60); // C4, 60 ticks
        assert_eq!(notes[1].midi_note(), 67); // G4, 60 ticks
        // Both are 60 ticks = half quarter note = 16 robot units
        assert_eq!(notes[0].duration_64ths(), 16);
        assert_eq!(notes[1].duration_64ths(), 16);
    }

    // ── Explicit track selection ─────────────────────────────────────────────

    #[test]
    fn test_explicit_track_selection() {
        // Format 1: track 0 has tempo only, track 1 has C4, track 1 selected.
        let tb = tempo_bytes(500_000);
        let track0: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0xFF, 0x2F, 0x00,
        ];
        let track1: &[u8] = &[
            0x00, 0x90, 0x3C, 0x40, 0x78, 0x80, 0x3C, 0x00, 0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf1(120, track0, track1);
        let config = MidiConfig {
            track: Some(1),
            ..Default::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 60);
    }

    // ── notes_to_chunks ──────────────────────────────────────────────────────

    fn make_notes(count: usize) -> Vec<SongNote> {
        (0..count).map(|_| SongNote::new(60, 32).unwrap()).collect()
    }

    #[test]
    fn test_chunks_single() {
        let chunks = notes_to_chunks(make_notes(1));
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), 1);
    }

    #[test]
    fn test_chunks_exactly_max() {
        let chunks = notes_to_chunks(make_notes(MAX_SONG_NOTES));
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), MAX_SONG_NOTES);
    }

    #[test]
    fn test_chunks_over_max() {
        let chunks = notes_to_chunks(make_notes(MAX_SONG_NOTES + 1));
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), MAX_SONG_NOTES);
        assert_eq!(chunks[1].len(), 1);
    }

    #[test]
    fn test_chunks_empty() {
        let chunks = notes_to_chunks(Vec::new());
        assert!(chunks.is_empty());
    }

    // ── Duration clamping / rounding ─────────────────────────────────────────

    #[test]
    fn test_duration_very_long_clamped_to_255() {
        // 480 ticks/beat, very slow tempo (20 BPM = 3 000 000 µs/beat).
        // 1 beat = 3 000 000 µs / 15 625 = 192 robot units.
        // Use 2 beats (960 ticks) → 384 → clamped to 255.
        let tb = tempo_bytes(3_000_000);
        // VLQ encode 960 = 0x87 0x40
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C, 0x40, // NoteOn C4
            0x87, 0x40, 0x80, 0x3C, 0x00, // delta=960 (VLQ), NoteOff C4
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(480, track);
        let notes = midi_to_notes(&midi, &MidiConfig::default()).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].duration_64ths(), 255);
    }

    // ── Multi-track merge: highest pitch wins ────────────────────────────────

    #[test]
    fn test_merge_highest_pitch_wins() {
        // Format 1, 120 ticks/beat, 120 BPM.
        // Track 0: C4 (pitch 60) quarter note.
        // Track 1: E4 (pitch 64) quarter note, both sounding at the same time.
        // merge_all_tracks=true → only E4 should be emitted (highest pitch).
        let tb = tempo_bytes(500_000);
        let track0: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], // Tempo
            0x00, 0x90, 0x3C, 0x40, // NoteOn C4 at tick 0
            0x78, 0x80, 0x3C, 0x00, // NoteOff C4 at tick 120
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let track1: &[u8] = &[
            0x00, 0x90, 0x40, 0x40, // NoteOn E4 at tick 0
            0x78, 0x80, 0x40, 0x00, // NoteOff E4 at tick 120
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf1(120, track0, track1);
        let config = MidiConfig {
            merge_all_tracks: true,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 64); // E4, not C4
        assert_eq!(notes[0].duration_64ths(), 32);
    }

    // ── Multi-track merge: lowest pitch wins ────────────────────────────────

    #[test]
    fn test_merge_lowest_pitch_wins() {
        let tb = tempo_bytes(500_000);
        let track0: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C,
            0x40, // NoteOn C4 at tick 0
            0x78, 0x80, 0x3C, 0x00, // NoteOff C4 at tick 120
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let track1: &[u8] = &[
            0x00, 0x90, 0x40, 0x40, // NoteOn E4 at tick 0
            0x78, 0x80, 0x40, 0x00, // NoteOff E4 at tick 120
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf1(120, track0, track1);
        let config = MidiConfig {
            merge_all_tracks: true,
            voice_selection: VoiceSelection::LowestPitch,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 60); // C4, not E4
        assert_eq!(notes[0].duration_64ths(), 32);
    }

    // ── Multi-track merge: sequential notes from different tracks ────────────

    #[test]
    fn test_merge_sequential_different_tracks() {
        // Track 0: C4 for first 60 ticks.
        // Track 1: E4 for second 60 ticks (starts at tick 60).
        // Expected: C4 (16 units) then E4 (16 units).
        let tb = tempo_bytes(500_000);
        let track0: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C,
            0x40, // NoteOn C4 at tick 0
            0x3C, 0x80, 0x3C, 0x00, // NoteOff C4 at tick 60 (delta=60=0x3C)
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let track1: &[u8] = &[
            0x3C, 0x90, 0x40, 0x40, // NoteOn E4 at tick 60 (delta=60=0x3C)
            0x3C, 0x80, 0x40, 0x00, // NoteOff E4 at tick 120
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf1(120, track0, track1);
        let config = MidiConfig {
            merge_all_tracks: true,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].midi_note(), 60); // C4
        assert_eq!(notes[0].duration_64ths(), 16);
        assert_eq!(notes[1].midi_note(), 64); // E4
        assert_eq!(notes[1].duration_64ths(), 16);
    }

    // ── Multi-track merge: handoff at same tick ──────────────────────────────

    #[test]
    fn test_merge_same_tick_handoff() {
        // Track 0: C4 from tick 0–120 (full quarter note).
        // Track 1: E4 from tick 60–120 (overlaps second half).
        // Expected: C4 for ticks 0–60 (16 units), E4 for ticks 60–120 (16 units).
        let tb = tempo_bytes(500_000);
        let track0: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C,
            0x40, // NoteOn C4 at tick 0
            0x78, 0x80, 0x3C, 0x00, // NoteOff C4 at tick 120
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let track1: &[u8] = &[
            0x3C, 0x90, 0x40, 0x40, // NoteOn E4 at tick 60
            0x3C, 0x80, 0x40, 0x00, // NoteOff E4 at tick 120
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf1(120, track0, track1);
        let config = MidiConfig {
            merge_all_tracks: true,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].midi_note(), 60); // C4 before E4 enters
        assert_eq!(notes[0].duration_64ths(), 16);
        assert_eq!(notes[1].midi_note(), 64); // E4 wins for the overlap
        assert_eq!(notes[1].duration_64ths(), 16);
    }

    // ── Multi-track merge: overlapping same pitch merged ─────────────────────

    #[test]
    fn test_merge_overlapping_same_pitch_merged() {
        // Track 0: C4 ticks 0–120.
        // Track 1: C4 ticks 60–180.
        // Both are the same pitch; the merged output should be one long C4
        // from tick 0 to 180 (= 48 robot units at 120 BPM / 120 ticks/beat).
        let tb = tempo_bytes(500_000);
        let track0: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C,
            0x40, // NoteOn C4 at tick 0
            0x78, 0x80, 0x3C, 0x00, // NoteOff C4 at tick 120
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let track1: &[u8] = &[
            0x3C, 0x90, 0x3C, 0x40, // NoteOn C4 at tick 60
            0x78, 0x80, 0x3C, 0x00, // NoteOff C4 at tick 180
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf1(120, track0, track1);
        let config = MidiConfig {
            merge_all_tracks: true,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        // Winner is always C4; no change in winner → one segment 0–180 ticks.
        // 180 ticks * 500_000 µs/beat / (120 ticks/beat * 15_625 µs) = 48.
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 60);
        assert_eq!(notes[0].duration_64ths(), 48);
    }

    // ── Multi-track merge: percussion filtered ───────────────────────────────

    #[test]
    fn test_merge_percussion_filtered_out() {
        // Track 0: C4 on channel 0 (melodic).
        // Track 1 (same byte stream but on ch9 = percussion): A4 on channel 9.
        // With filter_percussion=true, A4 is excluded → C4 wins.
        // With filter_percussion=false, A4 (pitch 69) > C4 (pitch 60) → A4 wins.
        let tb = tempo_bytes(500_000);
        let track0: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C,
            0x40, // NoteOn ch0 C4 at tick 0
            0x78, 0x80, 0x3C, 0x00, // NoteOff ch0 C4 at tick 120
            0x00, 0xFF, 0x2F, 0x00,
        ];
        // 0x99 = NoteOn channel 9; 0x89 = NoteOff channel 9.
        let track1: &[u8] = &[
            0x00, 0x99, 0x45, 0x40, // NoteOn ch9 A4 (pitch 69) at tick 0
            0x78, 0x89, 0x45, 0x00, // NoteOff ch9 A4 at tick 120
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf1(120, track0, track1);

        // filter_percussion=true (default): percussion excluded → C4 emitted.
        let config_filtered = MidiConfig {
            merge_all_tracks: true,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config_filtered).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 60); // C4

        // filter_percussion=false: A4 (69) > C4 (60) → A4 wins.
        let config_all = MidiConfig {
            merge_all_tracks: true,
            filter_percussion: false,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config_all).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 69); // A4
    }

    // ── Multi-track merge: empty after filtering → NoNotes ──────────────────

    #[test]
    fn test_merge_all_percussion_no_notes() {
        // Only percussion track; after filtering, no notes remain.
        let tb = tempo_bytes(500_000);
        let track0: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0xFF, 0x2F, 0x00,
        ];
        let track1: &[u8] = &[
            0x00, 0x99, 0x24, 0x7F, // NoteOn ch9 pitch 36 (kick drum)
            0x78, 0x89, 0x24, 0x00, // NoteOff
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf1(120, track0, track1);
        let config = MidiConfig {
            merge_all_tracks: true,
            ..MidiConfig::default()
        };
        let err = midi_to_notes(&midi, &config).unwrap_err();
        assert!(matches!(err, MidiError::NoNotes));
    }

    // ── Channel filter: invalid channel rejected ─────────────────────────────

    #[test]
    fn test_invalid_channel_rejected() {
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C, 0x40, 0x78, 0x80, 0x3C,
            0x00, 0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        let config = MidiConfig {
            channel: Some(16),
            ..MidiConfig::default()
        };
        let err = midi_to_notes(&midi, &config).unwrap_err();
        assert!(matches!(err, MidiError::InvalidChannel(16)));
    }

    // ── Channel filter: single-track mode ───────────────────────────────────

    /// A single track contains events on two channels; only the requested
    /// channel's notes should be returned.
    fn make_two_channel_track_smf0() -> Vec<u8> {
        // 120 ticks/beat, 120 BPM
        // tick 0:   NoteOn ch0 C4 (0x90, 0x3C, 0x40)
        // tick 120: NoteOff ch0 C4 (0x80, 0x3C, 0x00)
        // tick 120: NoteOn ch1 E4 (0x91, 0x40, 0x40)
        // tick 240: NoteOff ch1 E4 (0x81, 0x40, 0x00)
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], // tempo
            0x00, 0x90, 0x3C, 0x40, // tick 0: NoteOn ch0 C4
            0x78, 0x80, 0x3C, 0x00, // tick 120: NoteOff ch0 C4 (delta=120)
            0x00, 0x91, 0x40, 0x40, // tick 120: NoteOn ch1 E4 (delta=0)
            0x78, 0x81, 0x40, 0x00, // tick 240: NoteOff ch1 E4 (delta=120)
            0x00, 0xFF, 0x2F, 0x00,
        ];
        smf0(120, track)
    }

    #[test]
    fn test_channel_filter_single_track_ch0_only() {
        let midi = make_two_channel_track_smf0();
        let config = MidiConfig {
            channel: Some(0),
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 60); // C4 on ch0
    }

    #[test]
    fn test_channel_filter_single_track_ch1_only() {
        let midi = make_two_channel_track_smf0();
        let config = MidiConfig {
            channel: Some(1),
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 64); // E4 on ch1
    }

    #[test]
    fn test_channel_filter_single_track_no_match_returns_no_notes() {
        let midi = make_two_channel_track_smf0();
        let config = MidiConfig {
            channel: Some(7),
            ..MidiConfig::default()
        };
        let err = midi_to_notes(&midi, &config).unwrap_err();
        assert!(matches!(err, MidiError::NoNotes));
    }

    // ── Channel filter: multi-track merge mode ───────────────────────────────

    #[test]
    fn test_channel_filter_merge_mode_selects_channel() {
        // track0: ch0 C4; track1: ch1 E4 — both quarter notes simultaneously.
        // channel=Some(0) → only C4.
        let tb = tempo_bytes(500_000);
        let track0: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C,
            0x40, // NoteOn ch0 C4 at tick 0
            0x78, 0x80, 0x3C, 0x00, // NoteOff ch0 C4 at tick 120
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let track1: &[u8] = &[
            0x00, 0x91, 0x40, 0x40, // NoteOn ch1 E4 at tick 0
            0x78, 0x81, 0x40, 0x00, // NoteOff ch1 E4 at tick 120
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf1(120, track0, track1);
        let config = MidiConfig {
            merge_all_tracks: true,
            channel: Some(0),
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 60); // C4 only
    }

    // ── Channel filter: channel=9 overrides filter_percussion=true ──────────

    #[test]
    fn test_channel9_overrides_filter_percussion() {
        // filter_percussion=true (default) normally skips ch9.
        // But channel=Some(9) should explicitly include ch9.
        let tb = tempo_bytes(500_000);
        // 0x99 = NoteOn ch9; 0x89 = NoteOff ch9.
        let track0: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0xFF, 0x2F, 0x00,
        ];
        let track1: &[u8] = &[
            0x00, 0x99, 0x24, 0x7F, // NoteOn ch9 pitch 36 (kick drum, in robot range)
            0x78, 0x89, 0x24, 0x00, // NoteOff
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf1(120, track0, track1);
        let config = MidiConfig {
            merge_all_tracks: true,
            filter_percussion: true, // default; should be overridden by channel=Some(9)
            channel: Some(9),
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 36); // kick drum pitch
    }

    // ── Channel filter: auto-detect skips track with no notes on channel ─────

    #[test]
    fn test_channel_filter_autodetect_skips_wrong_channel_track() {
        // Format 1: track 0 has tempo; track 1 has only ch0 notes;
        // track 2 has ch2 notes. channel=Some(2) should auto-detect track 2.
        let tb = tempo_bytes(500_000);
        let track0: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0xFF, 0x2F, 0x00,
        ];
        let track1: &[u8] = &[
            0x00, 0x90, 0x3C, 0x40, // NoteOn ch0 C4
            0x78, 0x80, 0x3C, 0x00, // NoteOff ch0 C4
            0x00, 0xFF, 0x2F, 0x00,
        ];
        // 0x92 = NoteOn ch2; 0x82 = NoteOff ch2. G4 = pitch 67.
        let track2: &[u8] = &[
            0x00, 0x92, 0x43, 0x40, // NoteOn ch2 G4
            0x78, 0x82, 0x43, 0x00, // NoteOff ch2 G4
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let len0 = track0.len() as u32;
        let len1 = track1.len() as u32;
        let len2 = track2.len() as u32;
        let mut midi = Vec::new();
        midi.extend_from_slice(b"MThd");
        midi.extend_from_slice(&6u32.to_be_bytes());
        midi.extend_from_slice(&1u16.to_be_bytes()); // format 1
        midi.extend_from_slice(&3u16.to_be_bytes()); // 3 tracks
        midi.extend_from_slice(&120u16.to_be_bytes()); // ticks_per_beat
        for (chunk, len) in [(track0, len0), (track1, len1), (track2, len2)] {
            midi.extend_from_slice(b"MTrk");
            midi.extend_from_slice(&len.to_be_bytes());
            midi.extend_from_slice(chunk);
        }
        let config = MidiConfig {
            channel: Some(2),
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 67); // G4 on ch2
    }

    // ── Rest / silence support ───────────────────────────────────────────────

    /// Build a track with [C4(0-120) | gap(120-240) | E4(240-360)] at 120 BPM.
    fn make_rest_gap_track() -> Vec<u8> {
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], // tempo 500_000
            0x00, 0x90, 0x3C, 0x40, // tick   0: NoteOn C4
            0x78, 0x80, 0x3C, 0x00, // tick 120: NoteOff C4
            0x78, 0x90, 0x40, 0x40, // tick 240: NoteOn E4
            0x78, 0x80, 0x40, 0x00, // tick 360: NoteOff E4
            0x00, 0xFF, 0x2F, 0x00, // EndOfTrack
        ];
        smf0(120, track)
    }

    #[test]
    fn test_include_rests_disabled() {
        // Explicit include_rests=false: gaps are dropped; notes play back-to-back.
        let midi = make_rest_gap_track();
        let config = MidiConfig {
            include_rests: false,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].midi_note(), 60); // C4
        assert_eq!(notes[1].midi_note(), 64); // E4
    }

    #[test]
    fn test_include_rests_basic() {
        // Default config (include_rests=true): gap between C4 and E4 becomes a rest note.
        let midi = make_rest_gap_track();
        let notes = midi_to_notes(&midi, &MidiConfig::default()).unwrap();
        assert_eq!(notes.len(), 3);
        assert_eq!(notes[0].midi_note(), 60); // C4
        assert!(notes[1].is_rest());
        assert_eq!(notes[1].duration_64ths(), 32); // 120 ticks = 32 units
        assert_eq!(notes[2].midi_note(), 64); // E4
    }

    #[test]
    fn test_trim_start_suppresses_leading_rest() {
        // Silence at the start (tick 0-120), then C4 (120-240).
        // trim_start=true (default) → leading rest not emitted.
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x78, 0x90, 0x3C,
            0x40, // tick 120: NoteOn C4 (delta=120)
            0x78, 0x80, 0x3C, 0x00, // tick 240: NoteOff C4
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        let config = MidiConfig {
            include_rests: true,
            trim_start: true,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        // Only the audible note; leading rest trimmed.
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 60);
    }

    #[test]
    fn test_no_trim_start_emits_leading_rest() {
        // Same track, but trim_start=false → leading rest is emitted.
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x78, 0x90, 0x3C,
            0x40, // tick 120: NoteOn C4
            0x78, 0x80, 0x3C, 0x00, // tick 240: NoteOff C4
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        let config = MidiConfig {
            include_rests: true,
            trim_start: false,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 2);
        assert!(notes[0].is_rest());
        assert_eq!(notes[0].duration_64ths(), 32); // 120 ticks leading silence
        assert_eq!(notes[1].midi_note(), 60);
    }

    #[test]
    fn test_trim_end_suppresses_trailing_rest() {
        // C4 (0-120), then silence to EOT at tick 240.
        // trim_end=true (default) → trailing rest not emitted.
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C,
            0x40, // tick   0: NoteOn C4
            0x78, 0x80, 0x3C, 0x00, // tick 120: NoteOff C4
            0x78, 0xFF, 0x2F, 0x00, // tick 240: EndOfTrack
        ];
        let midi = smf0(120, track);
        let config = MidiConfig {
            include_rests: true,
            trim_end: true,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 60);
    }

    #[test]
    fn test_no_trim_end_emits_trailing_rest() {
        // Same track, trim_end=false → trailing rest is emitted.
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C,
            0x40, // tick   0: NoteOn C4
            0x78, 0x80, 0x3C, 0x00, // tick 120: NoteOff C4
            0x78, 0xFF, 0x2F, 0x00, // tick 240: EndOfTrack
        ];
        let midi = smf0(120, track);
        let config = MidiConfig {
            include_rests: true,
            trim_end: false,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].midi_note(), 60);
        assert!(notes[1].is_rest());
        assert_eq!(notes[1].duration_64ths(), 32); // 120 ticks trailing silence
    }

    #[test]
    fn test_note_at_tick_zero_no_spurious_rest() {
        // Note starts at tick 0 → gap_start(0) should be suppressed (zero-duration).
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C,
            0x40, // tick   0: NoteOn C4 (delta=0)
            0x78, 0x80, 0x3C, 0x00, // tick 120: NoteOff C4
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        let config = MidiConfig {
            include_rests: true,
            trim_start: false, // even without trim, zero-duration rest must not appear
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        // No spurious rest; only the audible note.
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 60);
    }

    #[test]
    fn test_out_of_range_span_treated_as_rest() {
        // C4 (0-120) | pitch 20 out-of-range (120-240) | C4 (240-360).
        // Out-of-range span should appear as a rest between the two C4 notes.
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C,
            0x40, // tick   0: NoteOn C4
            0x78, 0x80, 0x3C, 0x00, // tick 120: NoteOff C4
            0x00, 0x90, 0x14, 0x40, // tick 120: NoteOn pitch 20 (out-of-range)
            0x78, 0x80, 0x14, 0x00, // tick 240: NoteOff pitch 20
            0x00, 0x90, 0x3C, 0x40, // tick 240: NoteOn C4
            0x78, 0x80, 0x3C, 0x00, // tick 360: NoteOff C4
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        let config = MidiConfig {
            include_rests: true,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 3);
        assert_eq!(notes[0].midi_note(), 60); // first C4
        assert!(notes[1].is_rest());
        assert_eq!(notes[1].duration_64ths(), 32); // 120-tick gap filled by out-of-range span
        assert_eq!(notes[2].midi_note(), 60); // second C4
    }

    #[test]
    fn test_merge_include_rests_basic() {
        // Merge mode: C4 (0-120) | gap (120-240) | E4 (240-360).
        let tb = tempo_bytes(500_000);
        let track0: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 0x3C,
            0x40, // tick   0: NoteOn C4
            0x78, 0x80, 0x3C, 0x00, // tick 120: NoteOff C4
            0x78, 0x90, 0x40, 0x40, // tick 240: NoteOn E4
            0x78, 0x80, 0x40, 0x00, // tick 360: NoteOff E4
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf1(120, track0, &[0x00, 0xFF, 0x2F, 0x00]);
        let config = MidiConfig {
            merge_all_tracks: true,
            include_rests: true,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 3);
        assert_eq!(notes[0].midi_note(), 60); // C4
        assert!(notes[1].is_rest());
        assert_eq!(notes[1].duration_64ths(), 32);
        assert_eq!(notes[2].midi_note(), 64); // E4
    }
}
