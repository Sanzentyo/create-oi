# 005 — Protocol Crate Split & Architecture Refinement

**Date**: 2026-04-20 17:29
**Branch**: exp/full-port

## User Request

Split `create-oi` into two layers:
1. `create-oi-protocol` — Sans-IO wire format (opcodes, commands, sensors, stream)
2. `create-oi` — Transport + TypeState control API

Additional refinements:
- Rename `robot.rs` → `create.rs`, `async_robot.rs` → `async_create.rs` (match type names)
- Remove redundant `Opcode::as_u8()` (use `as u8` on `#[repr(u8)]`)
- Split error types: `ProtocolError` vs `Error`
- Move wire-level types (OiMode, ChargingState, IrChar) to protocol crate
- Remove `no-std` category (crate uses std)
- Add `resolver = "3"` to workspace

## Rationale (from rubber-duck architecture review)

- Sans-IO protocol should be independently usable without the TypeState control layer
- Error types should reflect their layer (wire errors vs control errors)
- `mode.rs` is NOT protocol — it's a control-layer opinionated abstraction
- File names should match the types they define

## Outcome

All changes implemented. 79 tests pass, 0 clippy warnings. Workspace now has 6 crates.
