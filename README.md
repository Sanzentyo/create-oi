# create-oi

A **pure Rust** implementation of the iRobot Create / Roomba
[Open Interface (OI)](https://www.irobot.com/about-irobot/stem/create-2)
protocol.

## Features

- **Sans-IO** — Protocol encoding/decoding on plain `&[u8]`, no I/O dependency
- **TypeState** — OI mode (`Off` → `Passive` → `Safe` → `Full`) enforced at compile time
- **Layered architecture** — Wire protocol (`create-oi-protocol`) separated from control (`create-oi`)
- **`no_std` compatible** — Core protocol and async API work on embedded targets (Embassy, Cortex-M)
- **Async-ready** — `AsyncCreate<M, T>` mirrors the sync API, runtime-agnostic via trait
- **Validated newtypes** — `Velocity`, `Radius`, `MotorPower` reject invalid values at construction

## Workspace Crates

| Crate | Description |
|-------|-------------|
| [`create-oi-protocol`](crates/create-oi-protocol) | Sans-IO wire protocol (opcodes, commands, sensors, stream parser) |
| [`create-oi`](crates/create-oi) | TypeState control API + transport traits |
| [`create-oi-serial`](crates/create-oi-serial) | Sync serial transport (`serialport`) |
| [`create-oi-tokio`](crates/create-oi-tokio) | Async serial transport (Tokio) |
| [`create-oi-embassy`](crates/create-oi-embassy) | Async embedded transport (Embassy) |
| [`create-oi-smol`](crates/create-oi-smol) | Async serial transport (Smol, experimental) |
| [`create-oi-dora`](crates/create-oi-dora) | dora-rs dataflow integration |

## Feature Flags

The `create-oi` crate supports three tiers:

| Feature | What it enables |
|---------|----------------|
| `std` (default) | Sync `Create<M, T>` API, `std::error::Error` impls, implies `alloc` |
| `alloc` | Vec-returning convenience methods (e.g. `query_sensor_raw`) |
| *(none)* | Pure `no_std` async API only — suitable for Embassy on Cortex-M |

Embassy users: `create-oi = { version = "0.4", default-features = false }`

## Quick Start (Desktop)

```rust
use create_oi::prelude::*;
use create_oi_serial::SerialTransport;

let transport = SerialTransport::open("/dev/ttyUSB0", CreateRobotModel::Create2)?;
let robot = Create::new(transport, CreateRobotModel::Create2);
let robot = robot.start()?;          // Off → Passive
let robot = robot.to_safe()?;        // Passive → Safe
// robot.drive(Velocity::new(0.2)?, Radius::Straight)?;
```

## Quick Start (Embassy / Embedded)

```rust,ignore
use create_oi::prelude::*;
use create_oi_embassy::EmbassyTransport;

// uart: embassy_stm32::usart::Uart<'_, Async> configured at 115200 baud
let transport = EmbassyTransport::new(uart);
let robot = AsyncCreate::new(transport, CreateRobotModel::Create2);
let robot = robot.start().await.unwrap();   // Off → Passive
let mut robot = robot.to_safe().await.unwrap(); // Passive → Safe
// robot.drive(Velocity::new(0.2)?, Radius::Straight).await?;
```

## MIDI Playback

The `midi` feature converts Standard MIDI Files (SMF) into iRobot OI song commands and plays them through the robot's built-in buzzer.

### Enabling the feature

```toml
[dependencies]
create-oi        = { version = "0.4", features = ["midi"] }
create-oi-serial = { version = "0.4", features = ["midi"] }  # for the CLI example
```

### Running the bundled example

```bash
# Serial (sync) — plays the bundled CC0 game-over.mid
cargo run -p create-oi-serial --example play_midi --features midi -- /dev/ttyUSB0

# Async Tokio
cargo run -p create-oi-tokio  --example play_midi --features midi -- /dev/ttyUSB0

# Play a custom MIDI file with LED sync
cargo run -p create-oi-serial --example play_midi --features midi -- \
    /dev/ttyUSB0 song.mid --led-sync
```

### CLI options

| Option | Short | Description |
|--------|-------|-------------|
| `--bpm <N>` | `-b` | Override tempo (beats per minute) |
| `--channel <0-15>` | `-C` | Only play notes from one MIDI channel |
| `--merge-tracks` | `-m` | Collapse multi-track files to a single monophonic voice |
| `--voice <strategy>` | `-v` | Voice selection when merging: `highest` (default), `lowest`, `nearest`, `velocity` |
| `--max-voices <N>` | `-p` | Limit simultaneous voices before monophonization |
| `--no-rests` | | Skip silence gaps between notes |
| `--keep-start-silence` | | Preserve leading silence |
| `--keep-end-silence` | | Preserve trailing silence |
| `--led-sync` | `-L` | Sync power LED color (pitch → green→red gradient) and digit display (note name) to playback |

### Playback strategy (double-buffer)

The OI allows at most 16 notes per song slot.  The implementation uses two slots (0 and 1) as a **double-buffer**: while one chunk is playing, the next is silently pre-loaded, reducing inter-chunk silence from ~22 ms down to the sensor poll round-trip (~2 ms).

### Pitch range

The robot speaker accepts MIDI notes 31–127 (G1 – G9).  Notes outside this range are clamped.

### Using the API directly

```rust
use create_oi::midi::{MidiConfig, midi_initial_tempo, midi_to_notes, notes_to_chunks};

let smf_bytes = std::fs::read("song.mid")?;
let tempo = midi_initial_tempo(&smf_bytes)?;
let config = MidiConfig::default();
let notes = midi_to_notes(&smf_bytes, tempo, &config)?;
let chunks: Vec<Vec<SongNote>> = notes_to_chunks(&notes);
// then call robot.define_song() / robot.play_song() per chunk
```



```bash
just ci       # fmt-check + clippy + build + test
just check    # fast workspace check
just doc      # generate docs
```

See [`docs/verification.md`](docs/verification.md) for detailed verification instructions.

## Supported Robots

- iRobot Create 1
- iRobot Create 2
- iRobot Roomba 400/500/600 series (OI compatible)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Attribution

This project is a Rust port of [libcreate](https://github.com/AutonomyLab/libcreate)
by Jacob Perron (Autonomy Lab, Simon Fraser University), licensed under BSD-3-Clause.
See [NOTICE](NOTICE) for details.
