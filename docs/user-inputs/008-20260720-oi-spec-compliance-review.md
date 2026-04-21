# OI Spec Compliance Review Request

**Date**: 2026-07-20
**Request**: Third rubber-duck review round — audit all code against the iRobot Create 2 Open Interface specification. Find any discrepancies between our implementation and the OI spec.

## Scope

- Command encoding (opcodes, ranges, validated newtypes)
- Sensor decoding (packet IDs, bit layouts, signed/unsigned, accumulation semantics)
- Mode-gating (which commands are available in which modes)
- Protocol error handling
- Documentation accuracy

## Background

Prior two rubber-duck rounds fixed:
- Round 1 (commit 93334c1): `to_off()`, `#[must_use]`, async 52-ID cap, comment fixes
- Round 2 (commit 33ea673): EOF detection, motor PWM guard (i8::MIN), vacuum sign fix,
  song/play in Passive, trailing-bytes detection, `#[non_exhaustive]` on ProtocolError

## Intended Outcomes

Identify remaining OI spec violations, document limitations, and fix any critical discrepancies.
