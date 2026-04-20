# User Inputs Index

All user requests and decisions are stored in this directory.

## Naming Convention

```
NNN-YYYYMMDD-HHMM-short-description.md
```

- `NNN` — sequential ID (zero-padded 3-digit)
- `YYYYMMDD` — date (year, month, day)
- `HHMM` — time (24h, hour + minute)
- `short-description` — kebab-case summary

Files sort by sequential ID first, then chronologically.

## Entries

| ID  | Date       | Time  | Description |
|-----|------------|-------|-------------|
| 001 | 2025-04-20 | 13:41 | [Initial request](001-20250420-1341-initial-request.md) — Create Rust wrapper for libcreate with TypeState/ADT, zigbuild |
| 002 | 2025-07-17 | 14:36 | [Pure Rust port request](002-20250717-1436-pure-rust-port-request.md) — Rewrite as pure Rust, dual async runtime support |
| 003 | 2026-04-20 | 15:41 | [Async robot + docs organization](003-20260420-1541-async-robot-and-docs.md) — Implement AsyncRobot, organize user-inputs |
| 004 | 2026-04-20 | —     | [Workspace restructure](004-20260420-workspace-restructure.md) — Split into multi-crate workspace, rename Robot→Create, add dora-rs |
