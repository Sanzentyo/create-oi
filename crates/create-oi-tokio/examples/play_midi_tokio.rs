//! Play a MIDI file on the robot using `TokioTransport`.
//!
//! Reads a Standard MIDI File (SMF), converts it to robot song notes, and
//! plays them sequentially through the robot's built-in speaker.
//!
//! Songs are uploaded in chunks of up to 16 notes (the OI limit per song
//! slot).  Slot 0 is reused for every chunk; the task sleeps between chunks
//! to let the robot finish playing each one.
//!
//! # Usage
//!
//! ```text
//! cargo run --example play_midi_tokio --features midi -- /dev/ttyUSB0 song.mid
//! ```

use std::env;
use std::time::Duration;

use create_oi::midi::{MidiConfig, midi_to_notes, notes_to_chunks};
use create_oi::prelude::*;
use create_oi_tokio::TokioTransport;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env::args().nth(1).unwrap_or_else(|| "/dev/ttyUSB0".into());
    let path = env::args().nth(2).unwrap_or_else(|| "song.mid".into());

    println!("Reading {path}…");
    let bytes = tokio::fs::read(&path).await?;

    let notes = midi_to_notes(&bytes, &MidiConfig::default())?;
    println!("{} notes parsed from MIDI file", notes.len());
    let chunks = notes_to_chunks(notes);
    println!("{} song chunk(s) to play", chunks.len());

    println!("Opening {port}…");
    let transport = TokioTransport::open(&port, RobotModel::Create2)?;
    let create = AsyncCreate::new(transport, RobotModel::Create2);
    let create = create.start().await.map_err(|e| e.source)?;
    let mut create = create.to_safe().await.map_err(|e| e.source)?;

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

        create.define_song(slot, chunk).await?;
        create.play_song(slot).await?;
        tokio::time::sleep(Duration::from_millis(duration_ms + 100)).await;
    }

    let _create = create.to_passive().await.map_err(|e| e.source)?;
    println!("Done!");
    Ok(())
}
