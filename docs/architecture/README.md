# libcreate-rs Architecture

## Overview

`libcreate-rs` is a **pure Rust** implementation of the iRobot Create / Roomba
[Open Interface (OI)](https://www.irobot.com/about-irobot/stem/create-2) protocol.
It supports Create 1, Create 2, and compatible Roomba robots over serial.

## Design Principles

- **Sans-IO**: Protocol encoding/decoding is completely independent of I/O.
  The `protocol` module works on plain `&[u8]` — zero allocation, zero copy.
- **TypeState**: The OI mode is encoded as a type parameter on `Robot<M, T>`.
  Invalid operations are compile-time errors, not runtime panics.
- **Feature-gated transports**: Sync serial, tokio async, smol async — pick your runtime.
- **Minimal dependencies**: Core depends only on `thiserror`. No proc macros.
- **MIT OR Apache-2.0**: Independent implementation of the open OI spec.

## Module Structure

```
src/
├── lib.rs                  # Public API, feature-gate re-exports, prelude
├── error.rs                # Error, TransitionError<R>, ConnectError<T>
├── types.rs                # Domain ADTs + validated newtypes
├── mode.rs                 # TypeState markers + sealed capability traits
├── protocol.rs             # Sans-IO protocol module root
├── protocol/
│   ├── opcode.rs           # All OI opcodes + sensor packet metadata table
│   ├── command.rs          # Command encoding → fixed-size byte arrays
│   ├── sensor.rs           # Sensor packet parsing from &[u8]
│   └── stream.rs           # Stream framing state machine (feed(&[u8]))
├── transport.rs            # Transport + AsyncTransport trait definitions
├── robot.rs                # Robot<M, T: Transport> — sync API
├── async_robot.rs          # AsyncRobot<M, T: AsyncTransport> — async API
├── io.rs                   # Concrete transport module root
└── io/
    ├── serial.rs           # SerialTransport (feature = "serial")
    ├── tokio.rs            # TokioTransport (feature = "tokio-runtime")
    └── smol.rs             # SmolTransport (feature = "smol-runtime")
```

## TypeState Pattern

The robot's OI mode is encoded in the type system. Both `Robot<M, T>` (sync)
and `AsyncRobot<M, T>` (async) share the same TypeState model:

```
Robot<Off, T> ─start()→ Robot<Passive, T> ─to_safe()→ Robot<Safe, T>
                              │                            │
                              └─to_full()→ Robot<Full, T> ←┘
```

- Mode transitions **consume** `self` and return `Robot<NewMode, T>`
- Invalid operations (e.g., `drive()` on `Robot<Passive, _>`) are compile errors
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

## Feature Flags

| Feature | Dependencies | Description |
|---------|-------------|-------------|
| `serial` (default) | `serialport 4.9` | Synchronous serial transport |
| `tokio-runtime` | `tokio 1`, `tokio-serial 5.4`, `futures-io 0.3` | Tokio async transport |
| `smol-runtime` | `smol 2`, `async-io 2`, `futures-io 0.3` | Smol async transport |

## Build & Test

```bash
just ci       # fmt-check + clippy + build + test
just ci-all   # same but with --all-features
```
