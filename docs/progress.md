# libcreate-rs Progress

## Pure Rust Port (exp/full-port branch)

### Completed
- [x] Remove C++ binding artifacts (libcreate-sys, vendor, .gitmodules)
- [x] Restructure as single crate with feature flags
- [x] Add MIT + Apache-2.0 license files
- [x] Core types: error.rs, types.rs, mode.rs
- [x] Protocol opcodes: all OI opcodes + sensor packet metadata table
- [x] Protocol commands: sans-IO command encoding (fixed-size byte arrays)
- [x] Protocol sensors: manual big-endian parsing into SensorData
- [x] Protocol stream: StreamParser with feed(&[u8]) state machine
- [x] Transport traits: Transport (sync) + AsyncTransport (async, with runtime-agnostic sleep)
- [x] Robot<M, T>: TypeState sync API with mode transitions
- [x] AsyncRobot<M, T>: TypeState async API mirroring Robot (feature-gated)
- [x] SerialTransport: serialport-based sync transport
- [x] TokioTransport: tokio-serial async transport
- [x] SmolTransport: stub (requires unsafe fd extraction)
- [x] Mock transport tests: 14 sync + 13 async integration tests
- [x] Unit tests: 51 tests across protocol + types
- [x] Example: basic_sync usage
- [x] Justfile: updated for new structure
- [x] Architecture docs: updated
- [x] User-input docs: reorganized with date-time-ID naming

### Test Summary
- 51 unit tests (protocol + types)
- 14 sync mock robot integration tests
- 13 async mock robot integration tests
- 1 doctest
- `just ci-all` passes: fmt ✅ clippy ✅ build ✅ test ✅

### Remaining
- [ ] SmolTransport: full implementation (needs safe fd extraction)
- [ ] Hardware integration tests (requires physical robot)
- [ ] Async tokio example
- [ ] Publish to crates.io
