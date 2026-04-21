//! Play a MIDI file on the robot using `SerialTransport`.
//!
//! Reads a Standard MIDI File (SMF), converts it to robot song notes, and
//! plays the notes sequentially through the robot's built-in speaker.
//!
//! ## Playback strategy
//!
//! Songs are uploaded in chunks of up to 16 notes (the OI limit per song slot).
//! Two slots (0 and 1) are **alternated** to eliminate any same-slot reuse
//! artefacts.  Each chunk is uploaded and played sequentially:
//!
//! 1. `define_song(slot, chunk[i])` — upload the chunk (~2 ms serial write).
//! 2. `play_song(slot)` — start playback; record `play_start`.
//! 3. Sleep until `chunk_duration + SONG_TIMING_BUFFER` elapses.
//! 4. Repeat with the other slot for chunk `i+1`.
//!
//! ## Timing note (macOS USB-to-serial)
//!
//! On macOS, USB-to-serial write latency is typically ≤10 ms. `SONG_TIMING_BUFFER`
//! (20 ms) provides a 2× margin so `play_song` always arrives at the robot after
//! the previous chunk has finished playing.
//!
//! # Usage
//!
//! ```text
//! cargo run --example play_midi_sync --features midi -- /dev/ttyUSB0 [song.mid] [bpm]
//! ```
//!
//! - `song.mid` defaults to the bundled CC0 demo file.
//! - `bpm` overrides the tempo read from the MIDI file (e.g. `120`).

use std::env;
use std::thread::sleep;
use std::time::{Duration, Instant};

use create_oi::midi::{MidiConfig, midi_initial_tempo, midi_to_notes, notes_to_chunks};
use create_oi::prelude::*;
use create_oi_serial::SerialTransport;

/// Extra sleep added beyond each chunk's calculated duration.
///
/// This must exceed the USB-to-serial write latency so that `play_song`
/// always arrives at the robot after the previous chunk has finished.
/// macOS USB-serial latency is typically ≤10 ms; 20 ms provides a 2× margin.
const SONG_TIMING_BUFFER: Duration = Duration::from_millis(20);

fn chunk_duration(chunk: &[SongNote]) -> Duration {
    Duration::from_micros(
        chunk
            .iter()
            .map(|note: &SongNote| u64::from(note.duration_64ths()) * 15_625)
            .sum::<u64>(),
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env::args().nth(1).unwrap_or_else(|| "/dev/ttyUSB0".into());
    let path = env::args().nth(2).unwrap_or_else(|| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/midi/game-over.mid"
        )
        .into()
    });
    let bpm_override: Option<u32> = env::args()
        .nth(3)
        .as_deref()
        .map(str::parse)
        .transpose()
        .map_err(|_| "BPM must be a positive integer")?;

    println!("Reading {path}…");
    let bytes = std::fs::read(&path)?;

    let file_tempo = midi_initial_tempo(&bytes)?;
    let file_bpm = 60_000_000 / file_tempo;
    println!("File tempo: {file_bpm} BPM ({file_tempo} µs/beat)");

    let tempo_override = bpm_override.map(|bpm| {
        let micros = 60_000_000 / bpm;
        println!("BPM override: {bpm} BPM ({micros} µs/beat)");
        micros
    });

    let notes = midi_to_notes(
        &bytes,
        &MidiConfig {
            merge_all_tracks: true,
            tempo_micros_per_beat: tempo_override,
            ..MidiConfig::default()
        },
    )?;
    println!("{} notes parsed from MIDI file", notes.len());
    let chunks = notes_to_chunks(notes);
    let n = chunks.len();
    println!("{n} song chunk(s) to play");

    if n == 0 {
        return Ok(());
    }

    println!("Opening {port}…");
    let transport = SerialTransport::open(&port, RobotModel::Create2)?;
    let create = Create::new(transport, RobotModel::Create2);
    let create = create.start().map_err(|e| e.source)?;
    let mut create = create.to_safe().map_err(|e| e.source)?;

    // Alternate between slots 0 and 1 to avoid any same-slot reuse artefacts.
    let slots = [SongNumber::new(0)?, SongNumber::new(1)?];

    for (i, chunk) in chunks.iter().enumerate() {
        let slot = slots[i % 2];
        let dur = chunk_duration(chunk);
        let pitch_min = chunk.iter().map(|note| note.midi_note()).min().unwrap_or(0);
        let pitch_max = chunk.iter().map(|note| note.midi_note()).max().unwrap_or(0);
        println!(
            "Chunk {}/{}: slot={} notes={} dur={:.3}s pitches={}..{}",
            i + 1,
            n,
            i % 2,
            chunk.len(),
            dur.as_secs_f64(),
            pitch_min,
            pitch_max,
        );

        create.define_song(slot, chunk)?;
        create.play_song(slot)?;
        let play_start = Instant::now();

        let target = dur + SONG_TIMING_BUFFER;
        let elapsed = play_start.elapsed();
        if target > elapsed {
            sleep(target - elapsed);
        }
    }

    let _create = create.to_passive().map_err(|e| e.source)?;
    println!("Done!");
    Ok(())
}
