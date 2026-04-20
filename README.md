# create-oi

A **pure Rust** implementation of the iRobot Create / Roomba
[Open Interface (OI)](https://www.irobot.com/about-irobot/stem/create-2)
protocol.

## Features

- **Sans-IO** — Protocol encoding/decoding on plain `&[u8]`, no I/O dependency
- **TypeState** — OI mode (`Off` → `Passive` → `Safe` → `Full`) enforced at compile time
- **Layered architecture** — Wire protocol (`create-oi-protocol`) separated from control (`create-oi`)
- **Async-ready** — `AsyncCreate<M, T>` mirrors the sync API, runtime-agnostic via trait
- **Validated newtypes** — `Velocity`, `Radius`, `MotorPower` reject invalid values at construction

## Workspace Crates

| Crate | Description |
|-------|-------------|
| [`create-oi-protocol`](crates/create-oi-protocol) | Sans-IO wire protocol (opcodes, commands, sensors, stream parser) |
| [`create-oi`](crates/create-oi) | TypeState control API + transport traits |
| [`create-oi-serial`](crates/create-oi-serial) | Sync serial transport (`serialport`) |
| [`create-oi-tokio`](crates/create-oi-tokio) | Async serial transport (Tokio) |
| [`create-oi-smol`](crates/create-oi-smol) | Async serial transport (Smol, experimental) |
| [`create-oi-dora`](crates/create-oi-dora) | dora-rs dataflow integration |

## Quick Start

```rust
use create_oi::prelude::*;
use create_oi_serial::SerialTransport;

let transport = SerialTransport::open("/dev/ttyUSB0", RobotModel::Create2)?;
let robot = Create::new(transport, RobotModel::Create2);
let robot = robot.start()?;          // Off → Passive
let robot = robot.to_safe()?;        // Passive → Safe
// robot.drive(Velocity::new(0.2)?, Radius::STRAIGHT)?;
```

## Build & Test

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

This project was informed by the [libcreate](https://github.com/AutonomyLab/libcreate)
C++ library by Jacob Perron (Autonomy Lab, Simon Fraser University), licensed under
BSD-3-Clause. See [NOTICE](NOTICE) for details. No code was copied; this is a clean-room
Rust implementation.
