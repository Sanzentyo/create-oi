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
//! # Polyphony reduction (multi-voice → few-voice → mono)
//!
//! For dense orchestral files with 10+ simultaneous voices, set
//! [`MidiConfig::max_voices`] to an intermediate count (e.g. 3) before the
//! final monophonization. This preserves more musical content than going
//! directly from N voices to 1.
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

mod config;
mod events;
mod parse;
mod voice_reduce;

pub use config::{MidiConfig, MidiError, VoiceSelection};
pub use parse::{midi_initial_tempo, midi_to_notes, notes_to_chunks};

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use core::num::NonZeroUsize;
    use create_oi_protocol::MAX_SONG_NOTES;

    use super::events::NoteEvent;
    use super::voice_reduce::limit_voices;
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

    /// Build a [`NoteEvent`] for a NoteOn.
    fn note_on(tick: u64, ch: u8, pitch: u8, vel: u8) -> NoteEvent {
        NoteEvent {
            abs_tick: tick,
            is_on: true,
            pitch,
            channel: ch,
            velocity: vel,
        }
    }

    /// Build a [`NoteEvent`] for a NoteOff.
    fn note_off(tick: u64, ch: u8, pitch: u8) -> NoteEvent {
        NoteEvent {
            abs_tick: tick,
            is_on: false,
            pitch,
            channel: ch,
            velocity: 0,
        }
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

    fn make_notes(count: usize) -> Vec<crate::types::SongNote> {
        (0..count)
            .map(|_| crate::types::SongNote::new(60, 32).unwrap())
            .collect()
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

    // ── VoiceSelection::NearestPitch ─────────────────────────────────────────

    #[test]
    fn test_nearest_pitch_prefers_closer_note() {
        // Single track, two channels.
        // ch0: C4(60) tick 0–120, then A3(57) tick 120–240.
        // ch1: E4(64) tick 120–240.
        //
        // With NearestPitch: after C4 plays, reference = 60.
        // At tick 120: |57-60|=3 vs |64-60|=4 → A3 wins.
        // Expected output: [C4, A3].
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], // Tempo
            0x00, 0x90, 60, 64, // tick   0: ch0 NoteOn C4 vel=64
            0x78, 0x80, 60, 0, // tick 120: ch0 NoteOff C4
            0x00, 0x90, 57, 64, // tick 120: ch0 NoteOn A3 vel=64
            0x00, 0x91, 64, 64, // tick 120: ch1 NoteOn E4 vel=64
            0x78, 0x80, 57, 0, // tick 240: ch0 NoteOff A3
            0x00, 0x81, 64, 0, // tick 240: ch1 NoteOff E4
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        let config = MidiConfig {
            merge_all_tracks: true,
            voice_selection: VoiceSelection::NearestPitch,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].midi_note(), 60); // C4
        assert_eq!(notes[1].midi_note(), 57); // A3 (nearest to C4)
    }

    #[test]
    fn test_nearest_pitch_equal_distance_tiebreak_higher_wins() {
        // C4(60) then F#3(54) and F#4(66) simultaneously at tick 120.
        // |54-60| = 6, |66-60| = 6: equal distance → higher pitch (F#4=66) wins.
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 60,
            64, // tick   0: NoteOn C4 ch0
            0x78, 0x80, 60, 0, // tick 120: NoteOff C4 ch0
            0x00, 0x90, 54, 64, // tick 120: NoteOn F#3 ch0
            0x00, 0x91, 66, 64, // tick 120: NoteOn F#4 ch1
            0x78, 0x80, 54, 0, // tick 240: NoteOff F#3
            0x00, 0x81, 66, 0, // tick 240: NoteOff F#4
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        let config = MidiConfig {
            merge_all_tracks: true,
            voice_selection: VoiceSelection::NearestPitch,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].midi_note(), 60); // C4
        assert_eq!(notes[1].midi_note(), 66); // F#4 (equal dist, higher pitch wins)
    }

    #[test]
    fn test_nearest_pitch_no_prior_context_falls_back_to_highest() {
        // No prior note: NearestPitch falls back to HighestPitch.
        // C4(60) and E4(64) from tick 0 simultaneously — E4 should win.
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 60,
            64, // tick 0: NoteOn C4 ch0
            0x00, 0x91, 64, 64, // tick 0: NoteOn E4 ch1
            0x78, 0x80, 60, 0, // tick 120: NoteOff C4
            0x00, 0x81, 64, 0, // tick 120: NoteOff E4
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        let config = MidiConfig {
            merge_all_tracks: true,
            voice_selection: VoiceSelection::NearestPitch,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 64); // E4 (HighestPitch fallback)
    }

    // ── VoiceSelection::HighestVelocity ──────────────────────────────────────

    #[test]
    fn test_highest_velocity_louder_note_wins() {
        // C4 vel=40 and E4 vel=80 simultaneously: E4 wins.
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 60,
            40, // tick 0: ch0 NoteOn C4 vel=40
            0x00, 0x91, 64, 80, // tick 0: ch1 NoteOn E4 vel=80
            0x78, 0x80, 60, 0, // tick 120: NoteOff C4
            0x00, 0x81, 64, 0, // tick 120: NoteOff E4
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        let config = MidiConfig {
            merge_all_tracks: true,
            voice_selection: VoiceSelection::HighestVelocity,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 64); // E4 (louder)
    }

    #[test]
    fn test_highest_velocity_equal_velocity_higher_pitch_wins() {
        // C4 vel=64 and E4 vel=64 simultaneously: equal velocity → higher pitch wins.
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 60,
            64, // tick 0: ch0 NoteOn C4 vel=64
            0x00, 0x91, 64, 64, // tick 0: ch1 NoteOn E4 vel=64
            0x78, 0x80, 60, 0, // tick 120: NoteOff C4
            0x00, 0x81, 64, 0, // tick 120: NoteOff E4
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        let config = MidiConfig {
            merge_all_tracks: true,
            voice_selection: VoiceSelection::HighestVelocity,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].midi_note(), 64); // E4 (equal vel, higher pitch wins)
    }

    #[test]
    fn test_highest_velocity_softer_note_still_visible_when_loud_ends() {
        // C4 vel=80 from tick 0–120 (louder, wins while both active).
        // E4 vel=40 from tick 0–180 (continues after C4 ends).
        // Expected: C4 wins during 0–120; E4 plays alone from 120–180.
        let tb = tempo_bytes(500_000);
        let track: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 60,
            80, // tick   0: ch0 NoteOn C4 vel=80
            0x00, 0x91, 64, 40, // tick   0: ch1 NoteOn E4 vel=40
            0x78, 0x80, 60, 0, // tick 120: ch0 NoteOff C4
            0x3C, 0x81, 64, 0, // tick 180: ch1 NoteOff E4 (delta=60=0x3C)
            0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf0(120, track);
        let config = MidiConfig {
            merge_all_tracks: true,
            voice_selection: VoiceSelection::HighestVelocity,
            ..MidiConfig::default()
        };
        let notes = midi_to_notes(&midi, &config).unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].midi_note(), 60); // C4 wins while both active
        assert_eq!(notes[1].midi_note(), 64); // E4 plays alone after C4 ends
    }

    // ── limit_voices: HighestPitch ───────────────────────────────────────────

    #[test]
    fn test_limit_voices_highest_pitch_drops_lowest() {
        // 3 NoteOns at tick 0, max=2, HighestPitch → C4(60) dropped.
        let events = vec![
            note_on(0, 0, 60, 64), // C4 — should be dropped
            note_on(0, 0, 64, 64), // E4 — kept
            note_on(0, 0, 67, 64), // G4 — kept
            note_off(120, 0, 60),
            note_off(120, 0, 64),
            note_off(120, 0, 67),
        ];
        let result = limit_voices(
            &events,
            NonZeroUsize::new(2).unwrap(),
            VoiceSelection::HighestPitch,
        );
        let on_pitches: Vec<u8> = result.iter().filter(|e| e.is_on).map(|e| e.pitch).collect();
        assert_eq!(on_pitches.len(), 2);
        assert!(on_pitches.contains(&64)); // E4 kept
        assert!(on_pitches.contains(&67)); // G4 kept
        assert!(!on_pitches.contains(&60)); // C4 dropped
    }

    #[test]
    fn test_limit_voices_pass_through_when_under_limit() {
        // 2 NoteOns, max=3 → all pass through unchanged.
        let events = vec![
            note_on(0, 0, 60, 64),
            note_on(0, 0, 64, 64),
            note_off(120, 0, 60),
            note_off(120, 0, 64),
        ];
        let result = limit_voices(
            &events,
            NonZeroUsize::new(3).unwrap(),
            VoiceSelection::HighestPitch,
        );
        let on_pitches: Vec<u8> = result.iter().filter(|e| e.is_on).map(|e| e.pitch).collect();
        assert_eq!(on_pitches.len(), 2); // both kept
        assert!(on_pitches.contains(&60));
        assert!(on_pitches.contains(&64));
    }

    // ── limit_voices: sustained note evicted → synthetic NoteOff ─────────────

    #[test]
    fn test_limit_voices_sustained_note_evicted_gets_synthetic_noteoff() {
        // C4 starts at tick 0. At tick 120, E4 and G4 join → 3 > max=2.
        // HighestPitch keeps G4 and E4, evicts C4 (which is sustained).
        // Expected: synthetic NoteOff for C4 at tick 120; original off at 240 dropped.
        let events = vec![
            note_on(0, 0, 60, 64),   // C4 starts
            note_on(120, 0, 64, 64), // E4 joins at tick 120
            note_on(120, 0, 67, 64), // G4 joins at tick 120
            note_off(240, 0, 60),    // original C4 off (should be dropped)
            note_off(240, 0, 64),
            note_off(240, 0, 67),
        ];
        let result = limit_voices(
            &events,
            NonZeroUsize::new(2).unwrap(),
            VoiceSelection::HighestPitch,
        );

        // Synthetic NoteOff for C4 at tick 120.
        let synthetic_off = result
            .iter()
            .find(|e| !e.is_on && e.pitch == 60 && e.abs_tick == 120);
        assert!(
            synthetic_off.is_some(),
            "synthetic NoteOff for C4 must appear at tick 120"
        );

        // Original NoteOff for C4 at tick 240 must NOT appear (note already closed).
        let late_off = result
            .iter()
            .find(|e| !e.is_on && e.pitch == 60 && e.abs_tick == 240);
        assert!(
            late_off.is_none(),
            "original NoteOff for evicted C4 must be dropped"
        );

        // E4 and G4 NoteOns are emitted.
        let on_pitches: Vec<u8> = result.iter().filter(|e| e.is_on).map(|e| e.pitch).collect();
        assert!(on_pitches.contains(&64));
        assert!(on_pitches.contains(&67));
    }

    // ── limit_voices: new NoteOn suppressed when slots full ──────────────────

    #[test]
    fn test_limit_voices_new_noteon_suppressed_when_slots_full() {
        // C4 and E4 active from tick 0. At tick 120, G4 tries to join → 3 > max=2.
        // With LowestPitch, G4 (highest) is the least-important → evicted.
        // G4 is a NEW NoteOn (not sustained) → suppressed, no synthetic NoteOff.
        let events = vec![
            note_on(0, 0, 60, 64),
            note_on(0, 0, 64, 64),
            note_on(120, 0, 67, 64), // new arrival, highest pitch — evicted by LowestPitch
            note_off(240, 0, 60),
            note_off(240, 0, 64),
            note_off(240, 0, 67),
        ];
        let result = limit_voices(
            &events,
            NonZeroUsize::new(2).unwrap(),
            VoiceSelection::LowestPitch,
        );

        // G4 NoteOn must not appear (was never admitted).
        let g4_on = result.iter().find(|e| e.is_on && e.pitch == 67);
        assert!(g4_on.is_none(), "G4 NoteOn must be suppressed");

        // No NoteOff for G4 either (was never opened).
        let g4_off = result.iter().find(|e| !e.is_on && e.pitch == 67);
        assert!(g4_off.is_none(), "G4 NoteOff must not appear");

        // C4 and E4 NoteOns are kept (they're already active).
        let on_pitches: Vec<u8> = result.iter().filter(|e| e.is_on).map(|e| e.pitch).collect();
        assert!(on_pitches.contains(&60));
        assert!(on_pitches.contains(&64));
    }

    // ── limit_voices: LowestPitch ────────────────────────────────────────────

    #[test]
    fn test_limit_voices_lowest_pitch_keeps_lowest() {
        // 3 notes, max=2, LowestPitch → G4(67) dropped.
        let events = vec![
            note_on(0, 0, 60, 64),
            note_on(0, 0, 64, 64),
            note_on(0, 0, 67, 64), // highest — should be dropped
            note_off(120, 0, 60),
            note_off(120, 0, 64),
            note_off(120, 0, 67),
        ];
        let result = limit_voices(
            &events,
            NonZeroUsize::new(2).unwrap(),
            VoiceSelection::LowestPitch,
        );
        let on_pitches: Vec<u8> = result.iter().filter(|e| e.is_on).map(|e| e.pitch).collect();
        assert_eq!(on_pitches.len(), 2);
        assert!(on_pitches.contains(&60)); // C4 kept
        assert!(on_pitches.contains(&64)); // E4 kept
        assert!(!on_pitches.contains(&67)); // G4 dropped
    }

    // ── limit_voices: HighestVelocity ────────────────────────────────────────

    #[test]
    fn test_limit_voices_highest_velocity_keeps_loudest() {
        // 3 notes with different velocities, max=2 → quietest dropped.
        let events = vec![
            note_on(0, 0, 60, 30), // C4 vel=30 — quietest, dropped
            note_on(0, 0, 64, 80), // E4 vel=80 — kept
            note_on(0, 0, 67, 60), // G4 vel=60 — kept
            note_off(120, 0, 60),
            note_off(120, 0, 64),
            note_off(120, 0, 67),
        ];
        let result = limit_voices(
            &events,
            NonZeroUsize::new(2).unwrap(),
            VoiceSelection::HighestVelocity,
        );
        let on_pitches: Vec<u8> = result.iter().filter(|e| e.is_on).map(|e| e.pitch).collect();
        assert_eq!(on_pitches.len(), 2);
        assert!(on_pitches.contains(&64)); // E4 (loudest) kept
        assert!(on_pitches.contains(&67)); // G4 (middle) kept
        assert!(!on_pitches.contains(&60)); // C4 (quietest) dropped
    }

    // ── limit_voices: per-tick batching invariant ────────────────────────────

    #[test]
    fn test_limit_voices_same_tick_batch_is_atomic() {
        // 3 NoteOns at the same tick but in different order should produce the
        // same result — all are in the same tick group, processed atomically.
        let events_a = vec![
            note_on(0, 0, 60, 64),
            note_on(0, 0, 64, 64),
            note_on(0, 0, 67, 64),
        ];
        let mut events_b = events_a.clone();
        // Reverse the order within the tick.
        events_b.reverse();

        let result_a = limit_voices(
            &events_a,
            NonZeroUsize::new(2).unwrap(),
            VoiceSelection::HighestPitch,
        );
        let result_b = limit_voices(
            &events_b,
            NonZeroUsize::new(2).unwrap(),
            VoiceSelection::HighestPitch,
        );

        let pitches_a: Vec<u8> = result_a
            .iter()
            .filter(|e| e.is_on)
            .map(|e| e.pitch)
            .collect();
        let pitches_b: Vec<u8> = result_b
            .iter()
            .filter(|e| e.is_on)
            .map(|e| e.pitch)
            .collect();

        // Both orderings must keep the same set of pitches.
        let mut sorted_a = pitches_a.clone();
        let mut sorted_b = pitches_b.clone();
        sorted_a.sort_unstable();
        sorted_b.sort_unstable();
        assert_eq!(sorted_a, sorted_b);

        // The fix: both sorted results should be [64, 67] (E4 and G4).
        assert_eq!(sorted_a, vec![64, 67]);

        // Suppress unused warnings for the sorted input slices.
        let _ = events_a;
        let _ = events_b;
    }

    // ── limit_voices: via midi_to_notes end-to-end ───────────────────────────

    #[test]
    fn test_max_voices_integration_reduces_polyphony() {
        // Two tracks: C4 and E4 and G4 all simultaneously.
        // Without max_voices: highest pitch (G4) wins → 1 note.
        // With max_voices=2: first reduce to {E4, G4}, then monophonize → G4.
        // The result is the same here (G4 wins), but we verify it runs without error.
        let tb = tempo_bytes(500_000);
        let track0: &[u8] = &[
            0x00, 0xFF, 0x51, 0x03, tb[0], tb[1], tb[2], 0x00, 0x90, 60, 64, // C4
            0x78, 0x80, 60, 0, 0x00, 0xFF, 0x2F, 0x00,
        ];
        let track1: &[u8] = &[
            0x00, 0x91, 64, 64, // E4 ch1
            0x78, 0x81, 64, 0, 0x00, 0xFF, 0x2F, 0x00,
        ];
        let midi = smf1(120, track0, track1);

        // Without max_voices — baseline.
        let config_base = MidiConfig {
            merge_all_tracks: true,
            ..MidiConfig::default()
        };
        let notes_base = midi_to_notes(&midi, &config_base).unwrap();
        assert_eq!(notes_base.len(), 1);
        assert_eq!(notes_base[0].midi_note(), 64); // E4 (highest in track1 ch1)

        // With max_voices=1 — same result since already monophonic after limit.
        let config_limited = MidiConfig {
            merge_all_tracks: true,
            max_voices: NonZeroUsize::new(1),
            ..MidiConfig::default()
        };
        let notes_limited = midi_to_notes(&midi, &config_limited).unwrap();
        assert_eq!(notes_limited.len(), 1);
        assert_eq!(notes_limited[0].midi_note(), 64); // still E4 (highest overall)
    }
}
