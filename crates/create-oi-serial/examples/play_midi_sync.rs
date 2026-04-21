//! Play a MIDI file on the robot using `SerialTransport`.
//!
//! Reads a Standard MIDI File (SMF), converts it to robot song notes, and
//! plays the notes sequentially through the robot's built-in speaker.
//!
//! Songs are uploaded in chunks of up to 16 notes (the OI limit per song
//! slot).  The example reuses slot 0 for every chunk and waits for each chunk
//! to finish before uploading the next one.
//!
//! # Usage
//!
//! ```text
//! cargo run --example play_midi_sync --features midi -- /dev/ttyUSB0 song.mid
//! ```
//!
//! The MIDI file path defaults to `song.mid` in the current directory.

use std::env;
use std::thread::sleep;
use std::time::Duration;

use create_oi::midi::{MidiConfig, midi_to_notes, notes_to_chunks};
use create_oi::prelude::*;
use create_oi_serial::SerialTransport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env::args().nth(1).unwrap_or_else(|| "/dev/ttyUSB0".into());
    let path = env::args().nth(2).unwrap_or_else(|| "song.mid".into());

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
        let duration_ms: u64 = chunk
            .iter()
            .map(|n| u64::from(n.duration_64ths()) * 1000 / 64)
            .sum();

        println!(
            "Playing chunk {}/{} ({} notes, ~{}ms)…",
            i + 1,
            chunks.len(),
            chunk.len(),
            duration_ms
        );

        create.define_song(slot, chunk)?;
        create.play_song(slot)?;
        // Wait for the song to finish, plus a small margin.
        sleep(Duration::from_millis(duration_ms + 100));
    }

    let _create = create.to_passive().map_err(|e| e.source)?;
    println!("Done!");
    Ok(())
}
