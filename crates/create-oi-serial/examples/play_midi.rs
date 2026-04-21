//! Play a MIDI file on the robot using `SerialTransport`.
//!
//! Reads a Standard MIDI File (SMF), converts it to robot song notes, and
//! plays the notes sequentially through the robot's built-in speaker.
//!
//! ## Playback strategy (double-buffer)
//!
//! Songs are split into chunks of up to 16 notes (the OI limit per song slot).
//! Two slots (0 and 1) are used as a **double-buffer** to eliminate the ~22 ms
//! inter-chunk gap of the naive sequential approach:
//!
//! **Setup**
//! 1. `define_song(slot 0, chunks[0])` — load first chunk.
//! 2. `define_song(slot 1, chunks[1])` — pre-load second chunk (if present).
//! 3. `play_song(slot 0)` — start playback.
//!
//! **Per-chunk loop** (for chunk *i* that is currently playing)
//! 1. Sleep until ~50 ms before the chunk's expected end (to reduce serial traffic).
//! 2. Poll `SONG_PLAYING` (packet 37) every 5 ms until it goes `false`.
//! 3. Immediately call `play_song(next_slot)` — the slot was pre-loaded, so only
//!    2 bytes are sent (≈ 0.2 ms latency).
//! 4. Pre-load `chunks[i+2]` into the now-free slot while `i+1` plays.
//!
//! The inter-chunk gap is reduced to the OI sensor update period (≈ 15.6 ms)
//! plus the query round-trip (~2 ms), compared with the fixed 22 ms in the
//! naive approach.
//!
//! # Usage
//!
//! ```text
//! cargo run -p create-oi-serial --example play_midi --features midi -- <PORT> [OPTIONS]
//! ```

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::{Duration, Instant};

use clap::{Parser, ValueEnum};
use create_oi::midi::{
    MidiConfig, VoiceSelection, midi_initial_tempo, midi_to_notes, notes_to_chunks,
};
use create_oi::prelude::*;
use create_oi_serial::SerialTransport;

/// Poll interval when waiting for `SONG_PLAYING` (packet 37) to go false.
const SONG_POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Switch from sleeping to polling this long before the expected chunk end.
const SONG_POLL_EARLY: Duration = Duration::from_millis(50);

/// How long past the expected chunk end before we give up waiting and advance.
const SONG_POLL_TIMEOUT: Duration = Duration::from_millis(500);

/// CLI voice selection policy (maps to [`VoiceSelection`]).
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum VoiceArg {
    #[default]
    Highest,
    Lowest,
    Nearest,
    Velocity,
}

impl From<VoiceArg> for VoiceSelection {
    fn from(v: VoiceArg) -> Self {
        match v {
            VoiceArg::Highest => VoiceSelection::HighestPitch,
            VoiceArg::Lowest => VoiceSelection::LowestPitch,
            VoiceArg::Nearest => VoiceSelection::NearestPitch,
            VoiceArg::Velocity => VoiceSelection::HighestVelocity,
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about = "Play a MIDI file on the iRobot Create 2")]
struct Args {
    /// Serial port (e.g. /dev/ttyUSB0 or /dev/cu.usbserial-*)
    port: String,

    /// MIDI file to play (defaults to the bundled CC0 game-over.mid)
    file: Option<PathBuf>,

    /// Override the MIDI file tempo (beats per minute, 1–)
    #[arg(short, long, value_parser = clap::value_parser!(u32).range(1..))]
    bpm: Option<u32>,

    /// Only play notes from this MIDI channel (0-indexed, 0–15)
    #[arg(short = 'C', long, value_parser = clap::value_parser!(u8).range(0..=15))]
    channel: Option<u8>,

    /// Merge all tracks into one monophonic voice (highest pitch wins by default)
    #[arg(short = 'm', long)]
    merge_tracks: bool,

    /// Voice selection strategy when merging tracks (requires --merge-tracks)
    #[arg(short = 'v', long, value_enum, default_value = "highest")]
    voice: VoiceArg,

    /// Limit polyphony to at most N simultaneous voices before monophonization
    #[arg(short = 'p', long)]
    max_voices: Option<NonZeroUsize>,

    /// Omit silence gaps between notes (suppress pitch-0 rest notes)
    #[arg(long)]
    no_rests: bool,

    /// Keep the leading silence before the first note (only with --include-rests)
    #[arg(long)]
    keep_start_silence: bool,

    /// Keep the trailing silence after the last note (only with --include-rests)
    #[arg(long)]
    keep_end_silence: bool,
}

fn chunk_duration(chunk: &[SongNote]) -> Duration {
    Duration::from_micros(
        chunk
            .iter()
            .map(|note: &SongNote| u64::from(note.duration_64ths()) * 15_625)
            .sum::<u64>(),
    )
}

fn print_chunk_info(i: usize, n: usize, chunk: &[SongNote]) {
    let dur = chunk_duration(chunk);
    let pitch_min = chunk
        .iter()
        .filter(|note| !note.is_rest())
        .map(|note| note.midi_note())
        .min()
        .unwrap_or(0);
    let pitch_max = chunk
        .iter()
        .filter(|note| !note.is_rest())
        .map(|note| note.midi_note())
        .max()
        .unwrap_or(0);
    let rest_count = chunk.iter().filter(|note| note.is_rest()).count();
    println!(
        "Chunk {}/{}: slot={} notes={} rests={} dur={:.3}s pitches={}..{}",
        i + 1,
        n,
        i % 2,
        chunk.len() - rest_count,
        rest_count,
        dur.as_secs_f64(),
        pitch_min,
        pitch_max,
    );
}

/// Wait until the robot finishes playing the current chunk, then return.
///
/// Sleeps until near the expected end of the chunk, then polls `SONG_PLAYING`
/// (packet 37) at [`SONG_POLL_INTERVAL`] intervals.  Returns when the robot
/// reports playback stopped, or when the fallback timeout fires.
fn wait_for_chunk_end(
    create: &mut Create<Safe, SerialTransport>,
    play_start: Instant,
    dur: Duration,
    chunk_idx: usize,
) {
    // Sleep until SONG_POLL_EARLY before expected end to reduce serial traffic.
    let approach_at = play_start + dur.saturating_sub(SONG_POLL_EARLY);
    let now = Instant::now();
    if now < approach_at {
        sleep(approach_at - now);
    }

    // Poll SONG_PLAYING until the robot reports the song has stopped.
    let timeout_at = play_start + dur + SONG_POLL_TIMEOUT;
    let mut saw_playing = false;
    loop {
        match create.query_sensor(37) {
            Ok(sd) => {
                let playing = sd.song_playing.unwrap_or(false);
                saw_playing |= playing;
                // Break on true→false transition, or when elapsed time covers
                // very short chunks that may finish before the first poll.
                if !playing && (saw_playing || play_start.elapsed() >= dur) {
                    return;
                }
            }
            Err(e) => eprintln!(
                "Warning: sensor query failed during chunk {}: {e}",
                chunk_idx + 1
            ),
        }
        if Instant::now() >= timeout_at {
            eprintln!(
                "Warning: chunk {} timed out waiting for song end; advancing",
                chunk_idx + 1
            );
            return;
        }
        sleep(SONG_POLL_INTERVAL);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let path = args.file.unwrap_or_else(|| {
        PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/midi/game-over.mid"
        ))
    });

    println!("Reading {}…", path.display());
    let bytes = std::fs::read(&path)?;

    let file_tempo = midi_initial_tempo(&bytes)?;
    let file_bpm = 60_000_000 / file_tempo;
    println!("File tempo: {file_bpm} BPM ({file_tempo} µs/beat)");

    let tempo_override = args.bpm.map(|bpm| {
        let micros = 60_000_000 / bpm;
        println!("BPM override: {bpm} BPM ({micros} µs/beat)");
        micros
    });

    let config = MidiConfig {
        merge_all_tracks: args.merge_tracks,
        tempo_micros_per_beat: tempo_override,
        voice_selection: args.voice.into(),
        channel: args.channel,
        include_rests: !args.no_rests,
        trim_start: !args.keep_start_silence,
        trim_end: !args.keep_end_silence,
        max_voices: args.max_voices,
        ..MidiConfig::default()
    };

    let notes = midi_to_notes(&bytes, &config)?;
    println!("{} notes parsed from MIDI file", notes.len());
    let chunks = notes_to_chunks(notes);
    let n = chunks.len();
    println!("{n} song chunk(s) to play");

    if n == 0 {
        return Ok(());
    }

    println!("Opening {}…", args.port);
    let transport = SerialTransport::open(&args.port, RobotModel::Create2)?;
    let create = Create::new(transport, RobotModel::Create2);
    let create = create.start().map_err(|e| e.source)?;
    let mut create = create.to_safe().map_err(|e| e.source)?;

    let slots = [SongNumber::new(0)?, SongNumber::new(1)?];

    // Double-buffer setup: pre-load the first two chunks before starting playback
    // so that play_song for chunk 1 can fire immediately when chunk 0 ends.
    create.define_song(slots[0], &chunks[0])?;
    if n > 1 {
        create.define_song(slots[1], &chunks[1])?;
    }

    print_chunk_info(0, n, &chunks[0]);
    create.play_song(slots[0])?;
    let mut play_start = Instant::now();
    let mut playing_i = 0usize;

    loop {
        let dur = chunk_duration(&chunks[playing_i]);
        wait_for_chunk_end(&mut create, play_start, dur, playing_i);

        let next_i = playing_i + 1;
        if next_i >= n {
            break;
        }

        // Immediately start the next chunk — its slot was pre-loaded, so only
        // a 2-byte play_song command is needed (≈ 0.2 ms latency).
        print_chunk_info(next_i, n, &chunks[next_i]);
        create.play_song(slots[next_i % 2])?;
        play_start = Instant::now();

        // While the next chunk plays, pre-load the one after it into the now-free slot.
        let after_next_i = playing_i + 2;
        if after_next_i < n {
            create.define_song(slots[playing_i % 2], &chunks[after_next_i])?;
        }

        playing_i = next_i;
    }

    let _create = create.to_passive().map_err(|e| e.source)?;
    println!("Done!");
    Ok(())
}
