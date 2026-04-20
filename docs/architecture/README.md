# libcreate-rs Architecture

## Overview

`libcreate-rs` is a **pure Rust** implementation of the iRobot Create / Roomba
[Open Interface (OI)](https://www.irobot.com/about-irobot/stem/create-2) protocol.
It supports Create 1, Create 2, and compatible Roomba robots over serial.

## Design Principles

- **Sans-IO**: Protocol encoding/decoding is completely independent of I/O.
  The `protocol` module works on plain `&[u8]` — zero allocation, zero copy.
- **TypeState**: The OI mode is encoded as a type parameter on `Create<M, T>`.
  Invalid operations are compile-time errors, not runtime panics.
- **Multi-crate workspace**: Core protocol is independent; transports are separate crates.
- **Minimal dependencies**: Core depends only on `thiserror`. No proc macros.
- **MIT OR Apache-2.0**: Independent implementation of the open OI spec.

## Workspace Structure

```
Cargo.toml                       # Virtual workspace manifest
crates/
├── create-oi/                   # Core: protocol, types, traits, Create<M,T>, AsyncCreate<M,T>
│   ├── src/
│   │   ├── lib.rs               # Public API + prelude
│   │   ├── error.rs             # Error, TransitionError<R>, ConnectError<T>
│   │   ├── types.rs             # Domain ADTs + validated newtypes
│   │   ├── mode.rs              # TypeState markers + sealed capability traits
│   │   ├── transport.rs         # Transport + AsyncTransport trait definitions
│   │   ├── robot.rs             # Create<M, T: Transport> — sync API
│   │   ├── async_robot.rs       # AsyncCreate<M, T: AsyncTransport> — async API
│   │   └── protocol/
│   │       ├── opcode.rs        # OI opcodes + sensor packet metadata table
│   │       ├── command.rs       # Command encoding → fixed-size byte arrays
│   │       ├── sensor.rs        # Sensor packet parsing from &[u8]
│   │       └── stream.rs        # Stream framing state machine
│   └── tests/
│       ├── mock_robot.rs        # 14 sync integration tests
│       └── mock_async_robot.rs  # 13 async integration tests
├── create-oi-serial/            # SerialTransport (sync)
├── create-oi-tokio/             # TokioTransport (async, tokio runtime)
├── create-oi-smol/              # SmolTransport (experimental, publish=false)
└── create-oi-dora/              # dora-rs dataflow node (publish=false)
```

## TypeState Pattern

The robot's OI mode is encoded in the type system. Both `Create<M, T>` (sync)
and `AsyncCreate<M, T>` (async) share the same TypeState model:

```
Create<Off, T> ─start()→ Create<Passive, T> ─to_safe()→ Create<Safe, T>
                               │                            │
                               └─to_full()→ Create<Full, T> ←┘
```

- Mode transitions **consume** `self` and return `Create<NewMode, T>`
- Invalid operations (e.g., `drive()` on `Create<Passive, _>`) are compile errors
- Failed transitions return `TransitionError { robot, source }` preserving the robot
- Failed connects return `ConnectError { transport, source }` preserving the transport
- Sealed capability traits (`SensorReadable`, `Actuatable`) gate method availability

## Algebraic Data Types

All domain values are proper Rust enums/newtypes:
- `RobotModel`: `Roomba400 | Create1 | Create2`
- `OiMode`: `Off | Passive | Safe | Full | Unknown(u8)`
- `ChargingState`: `NotCharging | Reconditioning | ... | Unknown(u8)`
- Sensor enums include `Unknown(u8)` for forward-compatibility

## Validated Newtypes

Physical quantities use validated newtypes with private inner fields:
- `Velocity(f32)` — range [-0.5, 0.5] m/s
- `AngularVelocity(f32)` — range [-π, π] rad/s
- `Radius(f32)` — range [-2.0, 2.0] m
- `MotorPower(f32)` — range [-1.0, 1.0]
- All reject NaN/infinity via `new()` and `TryFrom<f32>`

## Sans-IO Protocol Layer

The `protocol` module has zero I/O dependencies:

1. **Opcodes** (`opcode.rs`): Enum of all OI opcodes + packet metadata table
2. **Commands** (`command.rs`): Encode commands to fixed-size byte arrays
3. **Sensors** (`sensor.rs`): Decode sensor packets from `&[u8]` into `SensorData`
4. **Stream** (`stream.rs`): `StreamParser` with `feed(&[u8])` state machine

## Crates

| Crate | Description | Dependencies |
|-------|-------------|-------------|
| `create-oi` | Core protocol, types, traits | `thiserror` |
| `create-oi-serial` | Sync serial transport | `create-oi`, `serialport 4.9` |
| `create-oi-tokio` | Tokio async transport | `create-oi`, `tokio 1`, `tokio-serial 5.4` |
| `create-oi-smol` | Smol async transport (stub) | `create-oi`, `smol 2`, `async-io 2` |
| `create-oi-dora` | dora-rs dataflow node | `create-oi`, `dora-node-api 0.3` |

## Build & Test

```bash
just ci       # fmt-check + clippy + build + test
just check    # fast workspace check
just doc      # generate docs
```
