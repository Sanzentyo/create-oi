//! Play a MIDI file on the robot using `SerialTransport`.
//!
//! Reads a Standard MIDI File (SMF), converts it to robot song notes, and
//! plays the notes sequentially through the robot's built-in speaker.
//!
//! Songs are uploaded in chunks of up to 16 notes (the OI limit per song
//! slot).  The example reuses slot 0 for every chunk and polls the
//! `SONG_PLAYING` sensor (packet 37) to know when each chunk has finished,
//! rather than using a fixed timer.
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

/// How often to poll `SONG_PLAYING` while waiting for a chunk to finish.
const SONG_POLL_INTERVAL: Duration = Duration::from_millis(30);
/// Extra timeout headroom added on top of the expected chunk duration.
const SONG_TIMEOUT_EXTRA: Duration = Duration::from_secs(2);

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

    let notes = midi_to_notes(&bytes, &MidiConfig::default())?;
    println!("{} notes parsed from MIDI file", notes.len());
    let chunks = notes_to_chunks(notes);
    println!("{} song chunk(s) to play", chunks.len());

    println!("Opening {port}…");
    let transport = SerialTransport::open(&port, RobotModel::Create2)?;
    let create = Create::new(transport, RobotModel::Create2);
    let create = create.start().map_err(|e| e.source)?;
    let mut create = create.to_safe().map_err(|e| e.source)?;

    let slot = SongNumber::new(0)?;

    for (i, chunk) in chunks.iter().enumerate() {
        // Exact duration: each robot unit = 1/64 s = 15 625 µs.
        let chunk_duration = Duration::from_micros(
            chunk
                .iter()
                .map(|n| u64::from(n.duration_64ths()) * 15_625)
                .sum::<u64>(),
        );

        println!(
            "Chunk {}/{}: {} notes, {:.2}s",
            i + 1,
            chunks.len(),
            chunk.len(),
            chunk_duration.as_secs_f64()
        );

        create.define_song(slot, chunk)?;
        create.play_song(slot)?;

        // Poll SONG_PLAYING (packet 37) until the robot signals it has finished.
        // `saw_playing` guards against a false early exit if we poll before the
        // robot's firmware has transitioned to the playing state.
        let started = Instant::now();
        let mut saw_playing = false;
        loop {
            let sensor = create.query_sensor(37)?;
            match sensor.song_playing {
                Some(true) => saw_playing = true,
                Some(false) if saw_playing || started.elapsed() >= chunk_duration => break,
                _ => {}
            }
            if started.elapsed() >= chunk_duration + SONG_TIMEOUT_EXTRA {
                eprintln!(
                    "Warning: timed out waiting for chunk {}/{}",
                    i + 1,
                    chunks.len()
                );
                break;
            }
            sleep(SONG_POLL_INTERVAL);
        }
    }

    let _create = create.to_passive().map_err(|e| e.source)?;
    println!("Done!");
    Ok(())
}
