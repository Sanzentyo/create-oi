# Changelog

All notable changes to the `create-oi` workspace are documented here.

## [0.4.0] — first crates.io release

> **Note:** This is the first public release. The version 0.4.0 reflects
> approximately four major internal design iterations (pure Rust port, no_std
> support, TypeState API, async/embedded support) that occurred before the
> initial publication.

### create-oi-protocol

- Sans-IO wire protocol crate — fully `#![no_std]`, zero dependencies
- All OI opcodes encoded as `const fn` fixed-size byte arrays
- `SensorData`: big-endian parser for all sensor packet types
- `StreamParser`: incremental 7-byte-header stream framing

### create-oi

- `Create<M, T>`: TypeState sync API (`Off → Passive → Safe → Full`)
- `AsyncCreate<M, T>`: mirroring async API, runtime-agnostic via `AsyncTransport`
- `Transport` / `AsyncTransport` traits for custom I/O backends
- Validated newtypes: `Velocity`, `Radius`, `MotorPower`, `SongNote`, etc.
- `midi` feature: MIDI-to-OI song playback (Standard MIDI File → OI song commands)
- `no_std` support via feature flags (`std` / `alloc` / bare)
- **`Error::Disconnected`**: zero-byte transport reads now return a dedicated
  `Disconnected` variant instead of `Protocol(InsufficientData)`.  Applies to all
  six read sites in both sync and async paths.
- `Error<E>` and `MidiError` are intentionally exhaustive enums; callers should
  write complete `match` arms so the compiler catches unhandled variants.
- `Transport::write_all` / `AsyncTransport::write_all` contract clarified:
  implementations must submit bytes into the transmit path without requiring a
  subsequent `flush()` call for basic request–response correctness.

### create-oi-serial

- `SerialTransport`: synchronous transport backed by `serialport`
- Cross-platform (Linux, macOS, Windows)

### create-oi-tokio

- `TokioTransport`: async transport backed by `tokio-serial`
- Cross-platform (Linux, macOS, Windows)

### create-oi-embassy

- `EmbassyTransport` / `EmbassySplitTransport`: `#![no_std]` async transport
- Backed by `embedded-io-async` — works with Embassy on Cortex-M targets
- Verified on `thumbv7em-none-eabihf`

### create-oi-smol

- `SmolTransport`: async transport backed by `smol::Unblock<NativePort>`
- Cross-platform (Linux, macOS, Windows) via platform-specific `NativePort` type alias
- Reader and writer split into separate `Unblock<NativePort>` halves via `dup(2)` to
  eliminate sensor-query latency during concurrent streaming/MIDI playback.
- Fixed `TimedOut` panic on overlapping write when a background read task was in
  flight.
