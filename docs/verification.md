# Verification Guide

How to verify that `create-oi` workspace crates compile, pass tests, and function correctly.

## Prerequisites

- **Rust**: 1.85+ (edition 2024)
- **just** (command runner): `cargo install just` or `brew install just`

## Quick Verification

```bash
# Full CI check: format, clippy, build, test
just ci
```

This runs the same checks used in development:
1. `cargo fmt --all -- --check` — formatting
2. `cargo clippy --workspace --all-targets -- -D warnings` — lints
3. `cargo build --workspace` — compile all default members
4. `cargo test --workspace` — run all tests

## Individual Steps

### Compile check (fast)

```bash
just check
# or: cargo check --workspace --all-targets
```

### Build

```bash
just build
# or: cargo build --workspace
```

### Run tests

```bash
just test
# or: cargo test --workspace
```

Expected output:
- **79 tests total** (may increase as development continues)
  - 15 protocol unit tests (command encoding, sensor decoding, stream parsing)
  - 22 control unit tests (validated newtypes, error types)
  - 14 sync integration tests (mock transport)
  - 13 async integration tests (mock async transport)
  - 1+ doctests

### Clippy (lints)

```bash
just clippy
# or: cargo clippy --workspace --all-targets -- -D warnings
```

Should report **0 warnings** (warnings are treated as errors).

### Format check

```bash
just fmt-check
# or: cargo fmt --all -- --check
```

## What the Tests Verify

### Protocol layer (`create-oi-protocol`)

| Test area | What it verifies |
|-----------|-----------------|
| Command encoding | Each OI command produces correct byte sequence per spec |
| Sensor decoding | Raw bytes → `SensorData` fields (signed/unsigned, big-endian) |
| Stream parsing | Byte-wise state machine handles framing, checksums, partial data |
| Opcode values | `#[repr(u8)]` enum values match OI spec opcodes |

### Control layer (`create-oi`)

| Test area | What it verifies |
|-----------|-----------------|
| TypeState transitions | `Off→Passive→Safe→Full` sends correct opcodes |
| Sensor queries | `query_sensor()` / `query_list()` send correct command and decode response |
| Drive commands | `drive()` / `stop()` encode velocity/radius as big-endian mm/s and mm |
| LED commands | `set_leds()` encodes bit fields correctly |
| Error recovery | `ConnectError` preserves transport, `TransitionError` preserves robot |
| Transport recovery | `into_transport()` returns the transport layer to the caller |

### Async mirror (`AsyncCreate`)

All control tests are duplicated for the async API using `#[tokio::test]` and `MockAsyncTransport`.

## Hardware Verification (requires physical robot)

If you have a Create 2 robot connected via USB serial:

```bash
# Sync example
cargo run -p create-oi-serial --example basic_sync -- /dev/ttyUSB0

# Async (tokio) example
cargo run -p create-oi-tokio --example basic_tokio -- /dev/ttyUSB0
```

These examples will:
1. Connect to the robot (enter Passive mode)
2. Transition to Safe mode
3. Query battery voltage
4. Drive forward for 2 seconds
5. Stop and return to Passive mode

**⚠️  Place the robot on a safe surface before running drive commands.**

## API Usage Verification

### Compile-time safety (TypeState)

The following code should **not compile** — this is by design:

```rust,compile_fail
use create_oi::prelude::*;

fn test(robot: Create<Passive, impl Transport>) {
    // ERROR: drive() requires Safe or Full mode
    robot.drive(Velocity::new(0.1).unwrap(), Radius::STRAIGHT).unwrap();
}
```

### Validated newtypes

```rust
use create_oi::types::Velocity;

// Valid
assert!(Velocity::new(0.5).is_ok());
assert!(Velocity::new(-0.5).is_ok());

// Invalid: out of range
assert!(Velocity::new(1.0).is_err());

// Invalid: NaN/infinity
assert!(Velocity::new(f32::NAN).is_err());
assert!(Velocity::new(f32::INFINITY).is_err());
```

## Documentation

```bash
just doc
# or: cargo doc --workspace --no-deps --open
```

Opens generated API documentation in the browser.
