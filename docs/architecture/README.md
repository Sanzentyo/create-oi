# create-oi Architecture

## Overview

`create-oi` is a **pure Rust** implementation of the iRobot Create / Roomba
[Open Interface (OI)](https://www.irobot.com/about-irobot/stem/create-2) protocol.
It supports Create 1, Create 2, and compatible Roomba robots over serial.

## Design Principles

- **Sans-IO**: Protocol encoding/decoding is completely independent of I/O.
  The `create-oi-protocol` crate works on plain `&[u8]` — zero allocation, zero copy.
- **TypeState**: The OI mode is encoded as a type parameter on `Create<M, T>`.
  Invalid operations are compile-time errors, not runtime panics.
- **`no_std` first**: Both protocol and async control crates compile on embedded
  targets (Cortex-M4F with no heap). `std` and `alloc` are opt-in features.
- **Layered architecture**: Wire protocol is separate from transport+control.
- **Multi-crate workspace**: Core protocol and control are independent; transports are separate crates.
- **Minimal dependencies**: Protocol crate has zero dependencies. Control crate adds only `libm`.
- **MIT OR Apache-2.0**: Independent implementation of the open OI spec.

## Workspace Structure

```
Cargo.toml                       # Virtual workspace manifest (resolver = "3")
crates/
├── create-oi-protocol/          # Sans-IO wire format: opcodes, commands, sensors, stream
│   └── src/
│       ├── lib.rs               # Module declarations + prelude
│       ├── error.rs             # ProtocolError (specific variants, no String)
│       ├── types.rs             # Wire-level enums: OiMode, ChargingState, IrChar, etc.
│       ├── opcode.rs            # OI opcodes (#[repr(u8)]) + sensor packet metadata
│       ├── command.rs           # Command encoding → fixed-size byte arrays + _into() APIs
│       ├── sensor.rs            # Sensor packet parsing from &[u8] → SensorData
│       └── stream.rs            # StreamParser: byte-wise framing state machine
├── create-oi/                   # Control layer: TypeState API + transport traits
│   ├── src/
│   │   ├── lib.rs               # Public API + prelude + protocol re-exports
│   │   ├── error.rs             # Error<E> generic over transport error + ValidationError
│   │   ├── types.rs             # RobotModel + validated newtypes (Velocity, Radius, etc.)
│   │   ├── mode.rs              # TypeState markers + sealed capability traits
│   │   ├── transport.rs         # AsyncTransport (no_std) + Transport (std) trait definitions
│   │   ├── create.rs            # Create<M, T: Transport> — sync API (std only)
│   │   └── async_create.rs      # AsyncCreate<M, T: AsyncTransport> — async API (no_std)
│   └── tests/
│       ├── mock_robot.rs        # 14 sync integration tests
│       └── mock_async_robot.rs  # 13 async integration tests
├── create-oi-serial/            # SerialTransport (sync, std)
├── create-oi-tokio/             # TokioTransport (async, std, tokio runtime)
├── create-oi-embassy/           # EmbassyTransport (async, no_std, Embassy runtime)
├── create-oi-smol/              # SmolTransport (experimental, publish=false)
└── create-oi-dora/              # dora-rs dataflow node (publish=false)
```

## Feature Flags

The `create-oi` crate supports three tiers via feature flags:

| Feature | Implies | Enables |
|---------|---------|---------|
| `std` (default) | `alloc` | Sync `Create<M, T>`, `std::error::Error` impls |
| `alloc` | — | Vec convenience APIs (`query_sensor_raw`, `poll_stream`) |
| *(bare)* | — | Pure async no_std: `AsyncCreate`, buffer-based APIs only |

### Embassy / Embedded usage

```toml
[dependencies]
create-oi = { version = "0.4", default-features = false }
create-oi-embassy = "0.4"
```

## Layer Separation

### `create-oi-protocol` — Wire Format (Sans-IO)

Pure encoding/decoding with no transport dependency:
- `Opcode` — `#[repr(u8)]` enum, cast via `as u8`
- `command::encode_*()` — returns `[u8; N]` (always available)
- `command::encode_*_into(buf)` — writes to caller-provided buffer
- `SensorData::decode_packet()` — parses from `&[u8]`
- `StreamParser::feed_with(data, callback)` — byte-wise state machine, no alloc
- `ProtocolError` — `UnknownPacketId(u8)`, `InvalidStreamLength(u8)`, `BufferTooSmall`, etc.

### `create-oi` — Control Layer

Transport-aware TypeState API:
- `Create<M, T>` / `AsyncCreate<M, T>` — mode as type parameter
- `AsyncTransport` trait — associated `type Error`, no `Send` requirement
- `Transport` trait — std-only, `Send` required
- `Error<E>` — generic over transport error, wraps `ProtocolError` + `ValidationError`
- `ValidationError` — transport-independent domain validation errors
- Validated newtypes (Velocity, Radius, MotorPower, etc.)
- `mode.rs` — sealed traits gating method availability

### `create-oi-embassy` — Embassy Transport Adapter

Thin wrapper implementing `AsyncTransport` for any `embedded_io_async::Read + Write`:
- Uses `embassy_time::Timer::after()` for `delay()`
- Preserves the concrete HAL error type (no erasure)
- Zero-cost abstraction over Embassy UART peripherals

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
- `CreateRobotModel`: `Roomba400 | Create1 | Create2`
- `OiMode`: `Off | Passive | Safe | Full | Unknown(u8)`
- `ChargingState`: `NotCharging | Reconditioning | ... | Unknown(u8)`
- Sensor enums include `Unknown(u8)` for forward-compatibility

## Validated Newtypes

Physical quantities use validated newtypes with private inner fields:
- `Velocity(f32)` — range [-0.5, 0.5] m/s, rounds to nearest mm/s for OI
- `AngularVelocity(f32)` — range [-π, π] rad/s
- `Radius` — enum: `Straight | TurnInPlaceCw | TurnInPlaceCcw | Curve(f32)`
  - `Curve` range: [-2.0, 2.0] m; special OI values are distinct variants
  - `to_mm()` maps directly to OI protocol i16 (0x7FFF for straight, ±1 for in-place)
- `MotorPower(f32)` — range [-1.0, 1.0], rounds to nearest PWM value
- All float newtypes reject NaN/infinity via `new()` and `TryFrom<f32>`
- All protocol constants (velocities, radii, PWM limits) are named with doc comments

## Transport Traits

### `AsyncTransport` (no_std, all platforms)

```rust
pub trait AsyncTransport: fmt::Debug {
    type Error: fmt::Debug + fmt::Display;
    async fn write_all(&mut self, data: &[u8]) -> Result<(), Self::Error>;
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
    async fn flush(&mut self) -> Result<(), Self::Error>;
    async fn delay(&self, duration: Duration);
}
```

- No `Send` bound — Embassy peripherals are `!Send`
- Associated error type — preserves concrete errors from each runtime
- `delay()` bundles timer with transport for protocol-level waits

### `Transport` (std only)

```rust
pub trait Transport: fmt::Debug + Send {
    fn write_all(&mut self, data: &[u8]) -> io::Result<()>;
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    fn flush(&mut self) -> io::Result<()>;
    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()>;
    fn close(&mut self) -> io::Result<()>;
}
```

## Crates

| Crate | Description | Key Dependencies |
|-------|-------------|-----------------|
| `create-oi-protocol` | Sans-IO wire protocol | *(none)* |
| `create-oi` | TypeState control API + transport traits | `create-oi-protocol`, `libm` |
| `create-oi-serial` | Sync serial transport | `serialport 4.9` |
| `create-oi-tokio` | Tokio async transport | `tokio 1`, `tokio-serial 5.4` |
| `create-oi-embassy` | Embassy async transport | `embedded-io-async 0.7`, `embassy-time 0.5` |
| `create-oi-smol` | Smol async transport (stub) | `smol 2`, `async-io 2` |
| `create-oi-dora` | dora-rs dataflow node | `dora-node-api 0.3` |

## Build & Test

```bash
just ci       # fmt-check + clippy + build + test
just check    # fast workspace check
just doc      # generate docs
```
