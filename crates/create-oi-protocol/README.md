# create-oi-protocol

Sans-IO wire protocol implementation for the iRobot Create / Roomba
[Open Interface (OI)](https://www.irobot.com/about-irobot/stem/create-2).

This crate is fully `#![no_std]` with no dependencies and provides:

- **Opcode encoding** — all OI opcodes as type-safe byte array encoders (`const fn`)
- **Sensor decoding** — big-endian parsing of all sensor packet types into `SensorData`
- **Stream framing** — `StreamParser` for incremental 7-byte-header stream framing

## Usage

Add to `Cargo.toml`:

```toml
[dependencies]
create-oi-protocol = { version = "0.4", default-features = false }
```

For `no_std` without alloc, the default `std` feature is not needed:

```toml
[dependencies]
create-oi-protocol = { version = "0.4", default-features = false }
```

For hosted environments with `alloc`:

```toml
[dependencies]
create-oi-protocol = { version = "0.4", features = ["alloc"] }
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `std` (default) | Enables `std::error::Error` impls |
| `alloc` | Enables `alloc`-backed helpers |
| *(none)* | Pure `no_std` — works on bare-metal Cortex-M |

## Design

This crate is a **sans-IO** library: all encoding/decoding operates on plain `[u8]`
slices with no I/O operations or async runtime dependencies. The higher-level
[`create-oi`](https://crates.io/crates/create-oi) crate provides the TypeState
robot control API on top of this wire protocol layer.

## License

Licensed under either of Apache License 2.0 or MIT License at your option.
