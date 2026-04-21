//! Play a MIDI file on the robot using `SmolTransport`.
//!
//! See `play_midi_sync` for a full explanation of the sequential playback
//! strategy.  This variant uses smol async I/O and `smol::Timer::after`
//! for the chunk-duration wait.

use std::env;
use std::time::{Duration, Instant};

use create_oi::midi::{MidiConfig, midi_to_notes, notes_to_chunks};
use create_oi::prelude::*;
use create_oi_smol::SmolTransport;

/// See `play_midi_sync` for rationale.  50 ms is conservative; reduce once
/// playback is confirmed clean.
const SONG_TIMING_BUFFER: Duration = Duration::from_millis(50);

fn chunk_duration(chunk: &[SongNote]) -> Duration {
    Duration::from_micros(
        chunk
            .iter()
            .map(|note: &SongNote| u64::from(note.duration_64ths()) * 15_625)
            .sum::<u64>(),
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    smol::block_on(async {
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
        let transport = SmolTransport::open(&port, RobotModel::Create2)?;
        let create = AsyncCreate::new(transport, RobotModel::Create2);
        let create = create.start().await.map_err(|e| e.source)?;
        let mut create = create.to_safe().await.map_err(|e| e.source)?;

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

            create.define_song(slot, chunk).await?;
            create.play_song(slot).await?;
            let play_start = Instant::now();

            let target = dur + SONG_TIMING_BUFFER;
            let elapsed = play_start.elapsed();
            if target > elapsed {
                smol::Timer::after(target - elapsed).await;
            }
        }

        let _create = create.to_passive().await.map_err(|e| e.source)?;
        println!("Done!");
        Ok(())
    })
}
