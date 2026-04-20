# libcreate-rs Progress

## Multi-Crate Workspace (exp/full-port branch)

### Completed
- [x] Pure Rust rewrite (removed all C++ artifacts)
- [x] Core types: error.rs, types.rs, mode.rs
- [x] Protocol opcodes: all OI opcodes + sensor packet metadata table
- [x] Protocol commands: sans-IO command encoding (fixed-size byte arrays)
- [x] Protocol sensors: manual big-endian parsing into SensorData
- [x] Protocol stream: StreamParser with feed(&[u8]) state machine
- [x] Transport traits: Transport (sync) + AsyncTransport (async, runtime-agnostic sleep)
- [x] Create<M, T>: TypeState sync API with mode transitions
- [x] AsyncCreate<M, T>: TypeState async API mirroring Create
- [x] Split into multi-crate workspace (5 crates)
- [x] create-oi-serial: serialport sync transport
- [x] create-oi-tokio: tokio-serial async transport
- [x] create-oi-smol: stub (requires unsafe fd extraction)
- [x] create-oi-dora: dora-rs dataflow integration crate + example
- [x] Mock transport tests: 14 sync + 13 async integration tests
- [x] Unit tests: 51 tests across protocol + types
- [x] Examples: basic_sync, basic_tokio, dora_create_driver
- [x] Justfile: workspace commands
- [x] Architecture docs: updated for workspace structure
- [x] User-input docs: ID-date naming convention

### Test Summary
- 51 unit tests (protocol + types)
- 14 sync mock robot integration tests
- 13 async mock robot integration tests
- `just ci` passes: fmt ✅ clippy ✅ build ✅ test ✅

### Remaining
- [ ] SmolTransport: full implementation (needs safe fd extraction)
- [ ] Hardware integration tests (requires physical robot)
- [ ] Publish to crates.io
