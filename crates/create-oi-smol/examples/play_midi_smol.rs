//! Play a MIDI file on the robot using `SmolTransport`.
//!
//! See `play_midi_sync` for a full explanation of the sequential playback
//! strategy.  This variant uses smol async I/O and `smol::Timer::after`
//! for the chunk-duration wait.
//!
//! # Usage
//!
//! ```text
//! cargo run --example play_midi_smol --features midi -- <PORT> [OPTIONS]
//! ```

use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Parser;
use create_oi::midi::{
    MidiConfig, VoiceSelection, midi_initial_tempo, midi_to_notes, notes_to_chunks,
};
use create_oi::prelude::*;
use create_oi_smol::SmolTransport;

/// See `play_midi_sync` for rationale.  macOS USB-serial latency is typically
/// ≤10 ms; 20 ms provides a 2× margin.
const SONG_TIMING_BUFFER: Duration = Duration::from_millis(20);

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Play a MIDI file on the iRobot Create 2 (smol async)"
)]
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

    /// Merge all tracks into one monophonic voice (highest pitch wins)
    #[arg(short = 'm', long)]
    merge_tracks: bool,

    /// Include silence gaps between notes as rest notes (pitch 0)
    #[arg(short = 'r', long)]
    include_rests: bool,

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    smol::block_on(async {
        let path = args.file.clone().unwrap_or_else(|| {
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
            voice_selection: VoiceSelection::HighestPitch,
            channel: args.channel,
            include_rests: args.include_rests,
            trim_start: !args.keep_start_silence,
            trim_end: !args.keep_end_silence,
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
        let transport = SmolTransport::open(&args.port, RobotModel::Create2)?;
        let create = AsyncCreate::new(transport, RobotModel::Create2);
        let create = create.start().await.map_err(|e| e.source)?;
        let mut create = create.to_safe().await.map_err(|e| e.source)?;

        let slots = [SongNumber::new(0)?, SongNumber::new(1)?];

        for (i, chunk) in chunks.iter().enumerate() {
            let slot = slots[i % 2];
            let dur = chunk_duration(chunk);
            let pitch_min = chunk
                .iter()
                .filter(|n| !n.is_rest())
                .map(|n| n.midi_note())
                .min()
                .unwrap_or(0);
            let pitch_max = chunk
                .iter()
                .filter(|n| !n.is_rest())
                .map(|n| n.midi_note())
                .max()
                .unwrap_or(0);
            let rest_count = chunk.iter().filter(|n| n.is_rest()).count();
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
