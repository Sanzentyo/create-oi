//! MIDI-to-robot-song conversion utilities.
//!
//! Parses a Standard MIDI File (SMF) and converts notes to [`SongNote`] values
//! suitable for [`define_song`](crate::create::Create::define_song).
//!
//! # Limitations
//!
//! - **Monophonic extraction only**: when multiple notes are playing
//!   simultaneously (chords, polyphony), a new `NoteOn` cuts the previous
//!   active note. Only one voice is emitted per time instant. For melody
//!   tracks use [`MidiConfig::track`] to select the right track.
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
//! let notes = midi_to_notes(&bytes, &MidiConfig::default())?;
//! println!("{} notes parsed", notes.len());
//! let chunks = notes_to_chunks(notes);
//! println!("{} song chunks (≤16 notes each)", chunks.len());
//! # Ok(())
//! # }
//! ```

extern crate alloc;

use alloc::vec::Vec;

use midly::{Format, MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

use create_oi_protocol::MAX_SONG_NOTES;

use crate::types::SongNote;

/// Configuration for MIDI parsing.
#[derive(Debug, Clone, Default)]
pub struct MidiConfig {
    /// Track index to use (0-based). `None` = auto-detect the first track
    /// that contains at least one `NoteOn` event with nonzero velocity.
    pub track: Option<usize>,
    /// Override the tempo (µs per beat). `None` = read from the MIDI file;
    /// defaults to 500 000 (120 BPM) if no tempo event is present.
    pub tempo_micros_per_beat: Option<u32>,
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

/// Parse a Standard MIDI File and extract a sequence of [`SongNote`]s.
///
/// Only the selected track (or the first track with note events) is used for
/// notes. Tempo events from **all** tracks are collected into a global tempo
/// map, so that a conductor track in a Format 1 file is handled correctly.
///
/// # Polyphony
///
/// Monophonic extraction is used: when a new `NoteOn` arrives before the
/// previous note has ended, the previous note is cut at that tick. Chords are
/// reduced to a single voice (most recently started note wins). For polyphonic
/// files, pass an explicit melody track via [`MidiConfig::track`].
///
/// # Rests
///
/// Silence between notes is **dropped**. The robot's song format has no rest
/// representation; all emitted notes play back-to-back.
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

    // Determine which track to use for notes.
    let track_idx = match config.track {
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
                            make_note(pitch, start, abs_tick - start, &tempo_map, ticks_per_beat)
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
                            if let Some(note) = make_note(
                                pitch,
                                start,
                                abs_tick - start,
                                &tempo_map,
                                ticks_per_beat,
                            ) {
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
                            make_note(pitch, start, abs_tick - start, &tempo_map, ticks_per_beat)
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
        if let Some(note) = make_note(pitch, start, abs_tick - start, &tempo_map, ticks_per_beat) {
            notes.push(note);
        }
    }

    if notes.is_empty() {
        Err(MidiError::NoNotes)
    } else {
        Ok(notes)
    }
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
}
