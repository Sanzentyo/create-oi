use alloc::vec::Vec;

use midly::{Format, MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

use create_oi_protocol::MAX_SONG_NOTES;

use super::config::{MidiConfig, MidiError};
use super::events::{
    TempoChange, build_tempo_map, collect_note_events, find_max_track_end_tick, make_note,
    make_rest, tempo_at,
};
use super::voice_reduce::{limit_voices, monophonize_events};
use crate::types::SongNote;

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
/// If [`MidiConfig::max_voices`] is set, polyphony is first reduced to that
/// limit before the final monophonization, which can produce better melodic
/// output for dense orchestral files.
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
        let events = if let Some(max_v) = config.max_voices {
            limit_voices(&events, max_v, config.voice_selection)
        } else {
            events
        };
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
                        }
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
                                if include_rests {
                                    gap_start = Some(abs_tick);
                                }
                            }
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

/// Split a flat [`Vec<SongNote>`] into chunks of at most 16 notes each.
///
/// Each chunk can be uploaded to a single song slot with
/// [`define_song`](crate::create::Create::define_song).
pub fn notes_to_chunks(notes: Vec<SongNote>) -> Vec<Vec<SongNote>> {
    notes
        .chunks(MAX_SONG_NOTES)
        .map(|c: &[SongNote]| c.to_vec())
        .collect()
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
