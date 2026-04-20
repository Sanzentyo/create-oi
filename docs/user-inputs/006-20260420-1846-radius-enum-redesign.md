# 006: Radius Enum Redesign & Magic Number Elimination

- **Date**: 2026-04-20
- **Session**: Radius redesign, magic number cleanup

## User Request

- Fix magic numbers in `types.rs`
- Fix the `Radius::STRAIGHT` type design issue (struct wrapping f32 that relied on saturation behavior)
- Use rubber-duck agent for validation
- Follow rust-best-practices skill

## Key Decisions

1. **Radius redesigned** from `struct Radius(f32)` to `enum Radius { Straight, TurnInPlaceCw, TurnInPlaceCcw, Curve(f32) }`
   - Explicitly encodes OI protocol semantics (special values as distinct variants)
   - `Radius::STRAIGHT` kept as const alias for backward compatibility
   - `Radius::new()` rejects values colliding with OI special raw values (±1mm)

2. **Named constants extracted** (~20) with OI spec section references
   - e.g., `OI_MAX_VELOCITY_MM_S: i16 = 500` (§5.5)
   - e.g., `OI_RADIUS_STRAIGHT_RAW: i16 = 0x7FFF` (§5.5)

3. **Added `.round()` before all `f32 as i16` casts**
   - Fixes truncation bias (e.g., 127.5 → 128 instead of 127)

4. **Doc comments fixed**: `RobotModel` → `CreateRobotModel` in lib.rs examples

## Verification

- 83 tests pass (19 types unit + 14 sync + 13 async + 36 protocol + 1 integration)
- 0 clippy warnings
- Committed as `f3e82d8` on master
