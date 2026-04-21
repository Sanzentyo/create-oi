//! Play a MIDI file on the robot using `TokioTransport`.
//!
//! See `create-oi-serial`'s `play_midi` for a full description of the
//! double-buffer playback strategy.  This variant uses Tokio async I/O.
//!
//! # Usage
//!
//! ```text
//! cargo run -p create-oi-tokio --example play_midi --features midi -- <PORT> [OPTIONS]
//! ```

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::{Parser, ValueEnum};
use create_oi::midi::{
    MidiConfig, VoiceSelection, midi_initial_tempo, midi_to_notes, notes_to_chunks,
};
use create_oi::prelude::*;
use create_oi_tokio::TokioTransport;

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
#[command(
    version,
    about = "Play a MIDI file on the iRobot Create 2 (Tokio async)"
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

    /// Sync power LED color and digit display to playback (pitch → color, note name display)
    #[arg(short = 'L', long)]
    led_sync: bool,
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

/// One LED state update, timed relative to the start of a song chunk.
struct LedFrame {
    offset: Duration,
    color: u8,
    intensity: u8,
    display: [u8; 4],
}

fn pitch_to_display(pitch: u8) -> [u8; 4] {
    const NAMES: [u8; 12] = [
        b'C', b'C', b'D', b'D', b'E', b'F', b'F', b'G', b'G', b'A', b'A', b'B',
    ];
    const IS_SHARP: [bool; 12] = [
        false, true, false, true, false, false, true, false, true, false, true, false,
    ];
    let semitone = (pitch % 12) as usize;
    let oct_char = b'0' + (pitch / 12).saturating_sub(1).min(9);
    [
        NAMES[semitone],
        if IS_SHARP[semitone] { b'#' } else { b' ' },
        b' ',
        oct_char,
    ]
}

fn chunk_led_frames(chunk: &[SongNote]) -> Vec<LedFrame> {
    let mut frames = Vec::with_capacity(chunk.len());
    let mut offset = Duration::ZERO;
    for note in chunk {
        let dur = Duration::from_micros(u64::from(note.duration_64ths()) * 15_625);
        let (color, intensity, display) = if note.is_rest() {
            (0u8, 0u8, *b"    ")
        } else {
            let p = note.midi_note();
            let color = ((u32::from(p.saturating_sub(31)) * 255) / (127 - 31)) as u8;
            (color, 200u8, pitch_to_display(p))
        };
        frames.push(LedFrame {
            offset,
            color,
            intensity,
            display,
        });
        offset += dur;
    }
    frames
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let transport = TokioTransport::open(&args.port, RobotModel::Create2)?;
    let create = AsyncCreate::new(transport, RobotModel::Create2);
    let create = create.start().await.map_err(|e| e.source)?;
    let mut create = create.to_safe().await.map_err(|e| e.source)?;

    let slots = [SongNumber::new(0)?, SongNumber::new(1)?];

    // Double-buffer setup: pre-load the first two chunks before starting playback.
    create.define_song(slots[0], &chunks[0]).await?;
    if n > 1 {
        create.define_song(slots[1], &chunks[1]).await?;
    }

    print_chunk_info(0, n, &chunks[0]);
    create.play_song(slots[0]).await?;
    let mut play_start = Instant::now();
    let mut playing_i = 0usize;

    loop {
        let dur = chunk_duration(&chunks[playing_i]);
        let led_frames = if args.led_sync {
            chunk_led_frames(&chunks[playing_i])
        } else {
            vec![]
        };

        // Unified event loop: drive LEDs + poll sensor until chunk ends.
        let timeout_at = play_start + dur + SONG_POLL_TIMEOUT;
        let poll_start = play_start + dur.saturating_sub(SONG_POLL_EARLY);
        let mut consumed = 0usize;
        let mut saw_playing = false;
        loop {
            let now = Instant::now();
            if now >= timeout_at {
                eprintln!(
                    "Warning: chunk {} timed out waiting for song end; advancing",
                    playing_i + 1
                );
                break;
            }

            // Drive LEDs: emit only the latest overdue frame (coalesces missed).
            let elapsed = now.saturating_duration_since(play_start);
            let ready = led_frames.partition_point(|f| f.offset <= elapsed);
            if ready > consumed {
                consumed = ready;
                let f = &led_frames[ready - 1];
                let _ = create
                    .set_leds(
                        false,
                        false,
                        false,
                        false,
                        PowerLedColor::new(f.color),
                        LedIntensity::new(f.intensity),
                    )
                    .await;
                let _ = create
                    .set_digit_leds(f.display[0], f.display[1], f.display[2], f.display[3])
                    .await;
            }

            if now >= poll_start {
                match create.query_sensor(37).await {
                    Ok(sd) => {
                        let playing = sd.song_playing.unwrap_or(false);
                        saw_playing |= playing;
                        if !playing && (saw_playing || elapsed >= dur) {
                            break;
                        }
                    }
                    Err(e) => eprintln!(
                        "Warning: sensor query failed during chunk {}: {e}",
                        playing_i + 1
                    ),
                }
            }

            let next_led = led_frames.get(consumed).map(|f| play_start + f.offset);
            let next_poll = if now < poll_start {
                poll_start
            } else {
                now + SONG_POLL_INTERVAL
            };
            let wake_at = next_led.map_or(next_poll, |t| t.min(next_poll));
            let sleep_for = wake_at.saturating_duration_since(Instant::now());
            if sleep_for > Duration::ZERO {
                tokio::time::sleep(sleep_for).await;
            }
        }

        let next_i = playing_i + 1;
        if next_i >= n {
            break;
        }

        // Immediately start the next chunk — already pre-loaded, so only
        // a 2-byte play_song command is needed (≈ 0.2 ms latency).
        print_chunk_info(next_i, n, &chunks[next_i]);
        create.play_song(slots[next_i % 2]).await?;
        play_start = Instant::now();

        // While the next chunk plays, pre-load the one after it into the now-free slot.
        let after_next_i = playing_i + 2;
        if after_next_i < n {
            create
                .define_song(slots[playing_i % 2], &chunks[after_next_i])
                .await?;
        }

        playing_i = next_i;
    }

    if args.led_sync {
        let _ = create
            .set_leds(
                false,
                false,
                false,
                false,
                PowerLedColor::GREEN,
                LedIntensity::OFF,
            )
            .await;
        let _ = create.set_digit_leds(b' ', b' ', b' ', b' ').await;
    }
    let _create = create.to_passive().await.map_err(|e| e.source)?;
    println!("Done!");
    Ok(())
}
