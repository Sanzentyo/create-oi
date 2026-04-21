//! Play a MIDI file on the robot using `SerialTransport`.
//!
//! Reads a Standard MIDI File (SMF), converts it to robot song notes, and
//! plays the notes sequentially through the robot's built-in speaker.
//!
//! ## Playback strategy
//!
//! Songs are uploaded in chunks of up to 16 notes (the OI limit per song
//! slot).  Two slots are alternated in a **double-buffer** pattern combined
//! with **time-based** (non-polling) chunk transitions:
//!
//! * While slot A is playing the current chunk, slot B is already pre-loaded
//!   with the next chunk.
//! * Transition: at the expected end of the current chunk, issue
//!   `PLAY_SONG(slot B)` immediately (2-byte write, ~0.2 ms).
//! * The pre-load of the next-next chunk into the now-free slot happens
//!   during the new chunk's playback (no gap contribution).
//!
//! **Why time-based instead of polling?**
//! On macOS (and Linux), USB-to-serial adapters buffer read data for up to
//! 15–20 ms in the kernel/driver before delivering it to userspace.  Each
//! `query_sensor` round-trip therefore has ~10–20 ms jitter *independent* of
//! the poll interval — the poll interval only sets the lower bound, not the
//! actual latency.  Since each chunk duration (OI unit = 1/64 s = 15.625 ms)
//! is exactly known from the MIDI data, sleeping for that duration is more
//! accurate than polling.
//!
//! Expected gap per chunk transition:
//! * Polling approach: 10–30 ms (USB driver latency)
//! * Time-based approach: ~1–5 ms (OS sleep jitter only)
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
use std::mem;
use std::thread::sleep;
use std::time::{Duration, Instant};

use create_oi::midi::{MidiConfig, midi_to_notes, notes_to_chunks};
use create_oi::prelude::*;
use create_oi_serial::SerialTransport;

/// Small buffer added to each chunk's sleep duration so that the
/// `PLAY_SONG` command never arrives before the current chunk has actually
/// finished playing.  Needs to exceed OS sleep jitter (~2 ms on most
/// systems) plus serial write latency (~1 ms at 115200 baud).
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

    // Double-buffer: two slots alternate between "playing" and "pre-loaded".
    let mut playing_slot = SongNumber::new(0)?;
    let mut preloaded_slot = SongNumber::new(1)?;

    create.define_song(playing_slot, &chunks[0])?;
    if n > 1 {
        create.define_song(preloaded_slot, &chunks[1])?;
    }

    // Start the clock the moment we issue play_song so the sleep target
    // below is measured from the right reference point.
    create.play_song(playing_slot)?;
    let mut play_start = Instant::now();

    for i in 0..n {
        let dur = chunk_duration(&chunks[i]);
        println!(
            "Chunk {}/{}: {} notes, {:.3}s",
            i + 1,
            n,
            chunks[i].len(),
            dur.as_secs_f64()
        );

        // Sleep until (dur + SONG_TIMING_BUFFER) has elapsed since the last
        // play_song.  Subtract time already spent printing / computing above.
        let target = dur + SONG_TIMING_BUFFER;
        let elapsed = play_start.elapsed();
        if target > elapsed {
            sleep(target - elapsed);
        }

        if i + 1 < n {
            // Transition: switch to the pre-loaded slot immediately.
            mem::swap(&mut playing_slot, &mut preloaded_slot);
            create.play_song(playing_slot)?;
            play_start = Instant::now();

            // Pre-load the next-next chunk into the now-free slot.
            // This write happens during the new chunk's playback — no gap.
            if i + 2 < n {
                create.define_song(preloaded_slot, &chunks[i + 2])?;
            }
        }
    }

    let _create = create.to_passive().map_err(|e| e.source)?;
    println!("Done!");
    Ok(())
}
