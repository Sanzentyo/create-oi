//! Play a MIDI file on the robot using `SerialTransport`.
//!
//! Reads a Standard MIDI File (SMF), converts it to robot song notes, and
//! plays the notes sequentially through the robot's built-in speaker.
//!
//! ## Playback strategy
//!
//! Songs are uploaded in chunks of up to 16 notes (the OI limit per song slot).
//! One slot (slot 0) is reused for every chunk:
//!
//! 1. `define_song(0, chunk[i])` — upload the chunk (~2 ms serial write).
//! 2. `play_song(0)` — start playback; record `play_start`.
//! 3. Sleep until `chunk_duration + SONG_TIMING_BUFFER` has elapsed since
//!    `play_start`.
//! 4. Repeat for chunk `i+1`.
//!
//! **Why not double-buffering?**
//! The OI spec states that `play_song` is *ignored* while a song is already
//! playing.  A double-buffer therefore requires issuing `play_song` for the
//! next slot at exactly the right instant; any timing jitter makes it fail
//! silently, causing every other chunk to be skipped.  A simple sequential
//! loop sidesteps this by never calling `play_song` while the robot is busy.
//!
//! Expected inter-chunk gap:
//! * `define_song` serial write ≈ 2 ms
//! * `SONG_TIMING_BUFFER` (extra sleep beyond the chunk duration) ≈ 3 ms
//! * Total ≈ **5 ms** per chunk transition.
//!
//! # Usage
//!
//! ```text
//! cargo run --example play_midi_sync --features midi -- /dev/ttyUSB0 [song.mid]
//! ```
//!
//! When no MIDI path is given the bundled CC0 demo file
//! (`assets/midi/game-over.mid`) is used.

use std::env;
use std::thread::sleep;
use std::time::{Duration, Instant};

use create_oi::midi::{MidiConfig, midi_to_notes, notes_to_chunks};
use create_oi::prelude::*;
use create_oi_serial::SerialTransport;

/// Extra sleep time added after each chunk's calculated duration.
/// Accounts for OS sleep jitter (±2 ms) so that `play_song` is never
/// issued while the robot is still finishing the current chunk.
const SONG_TIMING_BUFFER: Duration = Duration::from_millis(3);

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

    println!("Reading {path}…");
    let bytes = std::fs::read(&path)?;

    let notes = midi_to_notes(
        &bytes,
        &MidiConfig {
            merge_all_tracks: true,
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

    let slot = SongNumber::new(0)?;

    for (i, chunk) in chunks.iter().enumerate() {
        let dur = chunk_duration(chunk);
        println!(
            "Chunk {}/{}: {} notes, {:.3}s",
            i + 1,
            n,
            chunk.len(),
            dur.as_secs_f64()
        );

        // Upload chunk, then start playback immediately.
        create.define_song(slot, chunk)?;
        create.play_song(slot)?;
        let play_start = Instant::now();

        // Sleep until the chunk finishes plus a small buffer to ensure
        // play_song is never issued while the robot is still playing.
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
