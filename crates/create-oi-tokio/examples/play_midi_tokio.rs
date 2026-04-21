//! Play a MIDI file on the robot using `TokioTransport`.
//!
//! Reads a Standard MIDI File (SMF), converts it to robot song notes, and
//! plays them sequentially through the robot's built-in speaker.
//!
//! Songs are uploaded in chunks of up to 16 notes (the OI limit per song
//! slot).  Two slots are alternated in a **double-buffer** pattern: while
//! slot A is playing, slot B is already pre-loaded with the next chunk so
//! each transition fires only the 2-byte `PLAY_SONG` command with no
//! upload latency, reducing inter-chunk gaps from ~33 ms to ~5 ms.
//!
//! # Usage
//!
//! ```text
//! cargo run --example play_midi_tokio --features midi -- /dev/ttyUSB0 [song.mid]
//! ```
//!
//! When no MIDI path is given the bundled CC0 demo file
//! (`assets/midi/game-over.mid`) is used.

use std::env;
use std::mem;
use std::time::{Duration, Instant};

use create_oi::midi::{MidiConfig, midi_to_notes, notes_to_chunks};
use create_oi::prelude::*;
use create_oi_tokio::TokioTransport;

/// How often to poll `SONG_PLAYING` while waiting for a chunk to finish.
const SONG_POLL_INTERVAL: Duration = Duration::from_millis(5);
/// Extra timeout headroom added on top of the expected chunk duration.
const SONG_TIMEOUT_EXTRA: Duration = Duration::from_secs(2);

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
    let bytes = tokio::fs::read(&path).await?;

    let notes = midi_to_notes(&bytes, &MidiConfig::default())?;
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

    let mut playing_slot = SongNumber::new(0)?;
    let mut preloaded_slot = SongNumber::new(1)?;

    create.define_song(playing_slot, &chunks[0]).await?;
    if n > 1 {
        create.define_song(preloaded_slot, &chunks[1]).await?;
    }
    create.play_song(playing_slot).await?;

    for i in 0..n {
        // Exact duration: each robot unit = 1/64 s = 15 625 µs.
        let chunk_duration = Duration::from_micros(
            chunks[i]
                .iter()
                .map(|note| u64::from(note.duration_64ths()) * 15_625)
                .sum::<u64>(),
        );

        println!(
            "Chunk {}/{}: {} notes, {:.2}s",
            i + 1,
            n,
            chunks[i].len(),
            chunk_duration.as_secs_f64()
        );

        // Poll SONG_PLAYING (packet 37) until the robot signals it has finished.
        let started = Instant::now();
        let mut saw_playing = false;
        loop {
            let sensor = create.query_sensor(37).await?;
            match sensor.song_playing {
                Some(true) => saw_playing = true,
                Some(false) if saw_playing || started.elapsed() >= chunk_duration => break,
                _ => {}
            }
            if started.elapsed() >= chunk_duration + SONG_TIMEOUT_EXTRA {
                eprintln!("Warning: timed out on chunk {}/{}", i + 1, n);
                break;
            }
            tokio::time::sleep(SONG_POLL_INTERVAL).await;
        }

        if i + 1 < n {
            mem::swap(&mut playing_slot, &mut preloaded_slot);
            create.play_song(playing_slot).await?;

            if i + 2 < n {
                create.define_song(preloaded_slot, &chunks[i + 2]).await?;
            }
        }
    }

    let _create = create.to_passive().await.map_err(|e| e.source)?;
    println!("Done!");
    Ok(())
}
