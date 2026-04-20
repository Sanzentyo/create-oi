# no_std + Embassy Support Request

**Date**: 2026-07-20
**Request**: Make create-oi workspace no_std compatible for Embassy embedded async runtime

## User Requirements

1. TypeState pattern and ADT-style Rust idioms throughout
2. `create-oi-protocol` must be fully `#![no_std]` with zero dependencies
3. `create-oi` must support `no_std` async API via feature flags
4. Create `create-oi-embassy` adapter crate for Embassy UART
5. Use `embedded-io-async` traits for the Embassy transport
6. Use `embassy-time::Timer` for protocol-level delays
7. Feature hierarchy: `std` (default) → `alloc` → bare no_std
8. Embassy users use `default-features = false`
9. No `Send` bound on async transport (Embassy peripherals are `!Send`)
10. Preserve concrete HAL error types (no erasure to ErrorKind)
11. Update README.md and docs to reflect all changes

## Implementation Decisions

- `Error<E>` generic over transport error type
- `ValidationError` separate from `Error<E>` (no transport type needed for domain validation)
- `libm` for no_std `f32::round()` support
- Buffer-based `_into()` APIs always available; Vec convenience behind `alloc`
- `feed_with(data, callback)` pattern for stream parsing without allocation
- Workspace deps use `default-features = false` with explicit `features = ["std"]` opt-in

## Breaking Changes (0.4.0)

- `Error` → `Error<E>` (generic)
- `AsyncTransport::sleep()` → `AsyncTransport::delay()`
- `AsyncTransport::close()` removed
- `AsyncTransport` gains `type Error` associated type
- `TransitionError` and `ConnectError` gain second type parameter `E`
- `thiserror` removed from all crates
- `ProtocolError::Protocol(String)` → specific variants
