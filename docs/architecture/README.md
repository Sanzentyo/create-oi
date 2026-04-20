# libcreate-rs Architecture

## Overview

`libcreate-rs` is a **pure Rust** implementation of the iRobot Create / Roomba
[Open Interface (OI)](https://www.irobot.com/about-irobot/stem/create-2) protocol.
It supports Create 1, Create 2, and compatible Roomba robots over serial.

## Design Principles

- **Sans-IO**: Protocol encoding/decoding is completely independent of I/O.
  The `create-oi-protocol` crate works on plain `&[u8]` — zero allocation, zero copy.
- **TypeState**: The OI mode is encoded as a type parameter on `Create<M, T>`.
  Invalid operations are compile-time errors, not runtime panics.
- **Layered architecture**: Wire protocol is separate from transport+control.
- **Multi-crate workspace**: Core protocol and control are independent; transports are separate crates.
- **Minimal dependencies**: Protocol crate depends only on `thiserror`. No proc macros.
- **MIT OR Apache-2.0**: Independent implementation of the open OI spec.

## Workspace Structure

```
Cargo.toml                       # Virtual workspace manifest (resolver = "3")
crates/
├── create-oi-protocol/          # Sans-IO wire format: opcodes, commands, sensors, stream
│   └── src/
│       ├── lib.rs               # Module declarations + prelude
│       ├── error.rs             # ProtocolError (Checksum, InsufficientData, Protocol)
│       ├── types.rs             # Wire-level enums: OiMode, ChargingState, IrChar, etc.
│       ├── opcode.rs            # OI opcodes (#[repr(u8)]) + sensor packet metadata
│       ├── command.rs           # Command encoding → fixed-size byte arrays
│       ├── sensor.rs            # Sensor packet parsing from &[u8] → SensorData
│       └── stream.rs            # StreamParser: byte-wise framing state machine
├── create-oi/                   # Control layer: TypeState API + transport traits
│   ├── src/
│   │   ├── lib.rs               # Public API + prelude + protocol re-exports
│   │   ├── error.rs             # Error (wraps ProtocolError + Io + domain errors)
│   │   ├── types.rs             # RobotModel + validated newtypes (Velocity, Radius, etc.)
│   │   ├── mode.rs              # TypeState markers + sealed capability traits
│   │   ├── transport.rs         # Transport + AsyncTransport trait definitions
│   │   ├── create.rs            # Create<M, T: Transport> — sync API
│   │   └── async_create.rs      # AsyncCreate<M, T: AsyncTransport> — async API
│   └── tests/
│       ├── mock_robot.rs        # 14 sync integration tests
│       └── mock_async_robot.rs  # 13 async integration tests
├── create-oi-serial/            # SerialTransport (sync)
├── create-oi-tokio/             # TokioTransport (async, tokio runtime)
├── create-oi-smol/              # SmolTransport (experimental, publish=false)
└── create-oi-dora/              # dora-rs dataflow node (publish=false)
```

## Layer Separation

### `create-oi-protocol` — Wire Format (Sans-IO)

Pure encoding/decoding with no transport dependency:
- `Opcode` — `#[repr(u8)]` enum, cast via `as u8`
- `command::encode_*()` — returns `[u8; N]` or `Vec<u8>`
- `SensorData::decode_packet()` — parses from `&[u8]`
- `StreamParser::feed(&[u8])` — byte-wise state machine
- `ProtocolError` — Checksum, InsufficientData, Protocol

### `create-oi` — Control Layer

Transport-aware TypeState API:
- `Create<M, T>` / `AsyncCreate<M, T>` — mode as type parameter
- `Transport` / `AsyncTransport` traits
- `Error` — wraps `ProtocolError` via `#[from]`, adds Io/Connection/etc.
- Validated newtypes (Velocity, Radius, MotorPower, etc.)
- `mode.rs` — sealed traits gating method availability

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

## Crates

| Crate | Description | Dependencies |
|-------|-------------|-------------|
| `create-oi-protocol` | Sans-IO wire protocol | `thiserror` |
| `create-oi` | TypeState control API + transport traits | `create-oi-protocol`, `thiserror` |
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
