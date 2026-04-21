//! Play a MIDI file on the robot using `TokioTransport`.
//!
//! See `play_midi_sync` for a full explanation of the sequential playback
//! strategy.  This variant uses Tokio async I/O and `tokio::time::sleep`
//! for the chunk-duration wait.

use std::env;
use std::time::{Duration, Instant};

use create_oi::midi::{MidiConfig, midi_to_notes, notes_to_chunks};
use create_oi::prelude::*;
use create_oi_tokio::TokioTransport;

/// See `play_midi_sync` for rationale.
const SONG_TIMING_BUFFER: Duration = Duration::from_millis(3);

fn chunk_duration(chunk: &[SongNote]) -> Duration {
    Duration::from_micros(
        chunk
            .iter()
            .map(|note: &SongNote| u64::from(note.duration_64ths()) * 15_625)
            .sum::<u64>(),
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let transport = TokioTransport::open(&port, RobotModel::Create2)?;
    let create = AsyncCreate::new(transport, RobotModel::Create2);
    let create = create.start().await.map_err(|e| e.source)?;
    let mut create = create.to_safe().await.map_err(|e| e.source)?;

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

        create.define_song(slot, chunk).await?;
        create.play_song(slot).await?;
        let play_start = Instant::now();

        let target = dur + SONG_TIMING_BUFFER;
        let elapsed = play_start.elapsed();
        if target > elapsed {
            tokio::time::sleep(target - elapsed).await;
        }
    }

    let _create = create.to_passive().await.map_err(|e| e.source)?;
    println!("Done!");
    Ok(())
}
