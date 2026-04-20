# create-oi Progress

## Multi-Crate Workspace (feat/no-std branch)

### Completed
- [x] Pure Rust rewrite (removed all C++ artifacts)
- [x] Core types: error.rs, types.rs, mode.rs
- [x] Protocol opcodes: all OI opcodes + sensor packet metadata table
- [x] Protocol commands: sans-IO command encoding (fixed-size byte arrays)
- [x] Protocol sensors: manual big-endian parsing into SensorData
- [x] Protocol stream: StreamParser with feed_with() callback pattern
- [x] Transport traits: Transport (sync, std) + AsyncTransport (async, no_std)
- [x] Create<M, T>: TypeState sync API with mode transitions
- [x] AsyncCreate<M, T>: TypeState async API mirroring Create
- [x] Split into multi-crate workspace (7 crates)
- [x] **create-oi-protocol**: Sans-IO wire format crate — fully `#![no_std]`
- [x] create-oi: Control layer with TypeState + transport abstraction — `#![no_std]` compatible
- [x] create-oi-serial: serialport sync transport
- [x] create-oi-tokio: tokio-serial async transport
- [x] **create-oi-embassy**: Embassy async transport adapter (embedded-io-async + embassy-time)
- [x] create-oi-smol: stub (requires unsafe fd extraction)
- [x] create-oi-dora: dora-rs dataflow integration crate + example
- [x] Mock transport tests: 14 sync + 13 async integration tests
- [x] Unit tests: 56 tests across protocol + types
- [x] Examples: basic_sync, basic_tokio, dora_create_driver
- [x] Justfile: workspace commands
- [x] Architecture docs: updated for no_std layered structure
- [x] User-input docs: ID-date naming convention
- [x] **Error type split**: ProtocolError (wire) vs Error<E> (control, generic over transport)
- [x] **ValidationError**: Transport-independent domain validation errors
- [x] **Removed thiserror**: Manual Display impls for no_std compatibility
- [x] **no_std Phase 1**: create-oi-protocol fully no_std (zero dependencies)
- [x] **no_std Phase 2**: create-oi generic transport, Error<E>, feature flags
- [x] **no_std Phase 3**: create-oi-embassy crate with embedded-io-async transport
- [x] **Embedded verified**: Builds for thumbv7em-none-eabihf (Cortex-M4F)
- [x] **File renames**: robot.rs → create.rs, async_robot.rs → async_create.rs
- [x] **resolver = "3"**: workspace Cargo.toml

### no_std Feature Hierarchy
- `std` (default) → implies `alloc` + `create-oi-protocol/std`
- `alloc` → enables Vec convenience APIs
- bare → pure no_std async API only (Embassy compatible)

### Test Summary
- 37 unit tests (protocol)
- 19 unit tests (types + control)
- 14 sync mock robot integration tests
- 13 async mock robot integration tests
- 1 protocol doc test
- `just ci` passes: fmt ✅ clippy ✅ build ✅ test ✅

### Remaining
- [ ] SmolTransport: full implementation (needs safe fd extraction)
- [ ] Hardware integration tests (requires physical robot)
- [ ] Publish to crates.io
- [ ] Version bump coordination (0.4.0)
