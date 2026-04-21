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
//! - **Rests are dropped**: gaps between notes are lost because the robot's
//!   song format has no silence representation. All emitted notes play
//!   back-to-back.
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
    /// Only used when [`merge_all_tracks`](Self::merge_all_tracks) is `true`.
    pub filter_percussion: bool,
}

impl Default for MidiConfig {
    fn default() -> Self {
        Self {
            track: None,
            tempo_micros_per_beat: None,
            merge_all_tracks: false,
            voice_selection: VoiceSelection::default(),
            filter_percussion: true,
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
/// MIDI channel 10 (0-indexed: 9, percussion).
fn collect_note_events(smf: &Smf<'_>, filter_percussion: bool) -> Vec<NoteEvent> {
    let mut events = Vec::new();
    for track in &smf.tracks {
        let mut abs_tick: u64 = 0;
        for event in track {
            abs_tick += u64::from(event.delta.as_int());
            if let TrackEventKind::Midi { channel, message } = event.kind {
                let ch = channel.as_int();
                if filter_percussion && ch == 9 {
                    continue;
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
fn monophonize_events(
    events: &[NoteEvent],
    voice_selection: VoiceSelection,
    tempo_map: &[TempoChange],
    ticks_per_beat: u32,
) -> Vec<SongNote> {
    let mut active: BTreeMap<u8, u32> = BTreeMap::new(); // pitch → active count
    let mut current_winner: Option<u8> = None;
    let mut segment_start: u64 = 0;
    let mut last_tick: u64 = 0;
    let mut notes: Vec<SongNote> = Vec::new();

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
            // Flush previous segment only if it has nonzero duration.
            if let Some(pitch) = current_winner {
                let dur = event.abs_tick.saturating_sub(segment_start);
                if dur > 0 {
                    if let Some(note) =
                        make_note(pitch, segment_start, dur, tempo_map, ticks_per_beat)
                    {
                        notes.push(note);
                    }
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
/// Silence between notes is **dropped** in both modes. The robot's song format
/// has no rest representation; all emitted notes play back-to-back.
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

    // Build a global tempo map from all tracks, then apply any override.
    let mut tempo_map = build_tempo_map(&smf);
    if let Some(override_tempo) = config.tempo_micros_per_beat {
        tempo_map.clear();
        tempo_map.push(TempoChange {
            abs_tick: 0,
            micros_per_beat: override_tempo,
        });
    }

    let notes = if config.merge_all_tracks {
        let mut events = collect_note_events(&smf, config.filter_percussion);
        events.sort_unstable();
        monophonize_events(&events, config.voice_selection, &tempo_map, ticks_per_beat)
    } else {
        single_track_notes(&smf, config.track, &tempo_map, ticks_per_beat)?
    };

    if notes.is_empty() {
        Err(MidiError::NoNotes)
    } else {
        Ok(notes)
    }
}

/// Extract notes from a single track using "latest NoteOn wins" monophony.
fn single_track_notes(
    smf: &Smf<'_>,
    track_selection: Option<usize>,
    tempo_map: &[TempoChange],
    ticks_per_beat: u32,
) -> Result<Vec<SongNote>, MidiError> {
    let track_idx = match track_selection {
        Some(idx) => idx,
        None => smf
            .tracks
            .iter()
            .position(|track| {
                track.iter().any(|e| {
                    if let TrackEventKind::Midi { message, .. } = e.kind {
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

    for event in selected_track {
        abs_tick += u64::from(event.delta.as_int());

        match event.kind {
            TrackEventKind::Midi {
                message: MidiMessage::NoteOn { key, vel },
                ..
            } => {
                if vel.as_int() > 0 {
                    // New note: cut any active note (monophonic extraction).
                    if let Some((pitch, start)) = active.take() {
                        if let Some(note) =
                            make_note(pitch, start, abs_tick - start, tempo_map, ticks_per_beat)
                        {
                            notes.push(note);
                        }
                    }
                    active = Some((key.as_int(), abs_tick));
                } else {
                    // NoteOn with vel == 0 is equivalent to NoteOff.
                    if let Some((pitch, start)) = active {
                        if pitch == key.as_int() {
                            active = None;
                            if let Some(note) =
                                make_note(pitch, start, abs_tick - start, tempo_map, ticks_per_beat)
                            {
                                notes.push(note);
                            }
                        }
                    }
                }
            }
            TrackEventKind::Midi {
                message: MidiMessage::NoteOff { key, .. },
                ..
            } => {
                if let Some((pitch, start)) = active {
                    if pitch == key.as_int() {
                        active = None;
                        if let Some(note) =
                            make_note(pitch, start, abs_tick - start, tempo_map, ticks_per_beat)
                        {
                            notes.push(note);
                        }
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

/// Split a flat [`Vec<SongNote>`] into chunks of at most [`MAX_SONG_NOTES`]
/// (16) notes each.
///
/// Each chunk can be uploaded to a single song slot with
/// [`define_song`](crate::create::Create::define_song).
pub fn notes_to_chunks(notes: Vec<SongNote>) -> Vec<Vec<SongNote>> {
    notes.chunks(MAX_SONG_NOTES).map(|c| c.to_vec()).collect()
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
}
