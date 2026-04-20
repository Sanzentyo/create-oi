# Workspace Restructure Request

## Original Message (paraphrased)

Split the single-crate `libcreate` into a multi-crate workspace:
- Separate crates for independent development and optional dependencies
- Rename `Robot`/`AsyncRobot` to `Create`/`AsyncCreate` (more specific names)
- Add dora-rs integration as an example crate
- Fix user-input file naming to `NNN-YYYYMMDD-HHMM-description.md` (ID first)
- Restore original user messages from git log

## Decisions

- **Robot → Create**, **AsyncRobot → AsyncCreate** — user preference
- **Virtual manifest** at workspace root — no root package
- **`create-oi-smol`** and **`create-oi-dora`** are `publish = false` (experimental)
- **`default-members`** excludes smol and dora — they're opt-in
- **AsyncTransport trait always available** — no feature gates in core crate
- **dora-rs integration** lives in workspace, not a separate repo
