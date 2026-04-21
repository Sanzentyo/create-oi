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
- [x] **create-oi-smol**: Real implementation via `smol::Unblock<TTYPort>` (Unix-only)
- [x] create-oi-dora: dora-rs dataflow integration crate + example
- [x] Mock transport tests: 14 sync + 13 async integration tests
- [x] Unit tests: 94 tests across all crates
- [x] Examples: basic_sync, basic_tokio, dora_create_driver, basic_smol, round_smol
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
- [x] **const fn phase**: All eligible encode/decode/accessor functions made `const fn` + `#[inline(always)]`
  - `command.rs`: all 25+ fixed-array encoders; `encode_schedule` rewritten with `while` loop
  - `opcode.rs`: `packet_info`, `group_packet_ids`, `group_data_len`, `all_sensors_data_len` all `const fn`
  - `sensor.rs`: `decode_u8/i8/u16/i16`, `expected_data_len`, all 20 `SensorData::is_*` bit-field accessors
  - `protocol/types.rs`: `OiMode::from_raw/name`, `ChargingState::from_raw`, `DayOfWeek::to_raw`, `IrChar::from_raw`
  - `create-oi/types.rs`: `CreateRobotModel` 7 methods, all getters, `PowerLedColor/LedIntensity/SongNumber`, `MotorBits/ButtonBits::to_raw`
- [x] **Second-round correctness + API (dual-agent: explore + rubber-duck)**:
  - **Bug fix (critical)**: `query_sensor_raw`, `query_sensor_raw_into`, `query_list` now validate ALL preconditions BEFORE sending bytes to the robot (validate-before-send); affected sync + async
  - **Bug fix (critical)**: `encode_song_into/encode_query_list_into/encode_stream_into` validate item count BEFORE computing buffer `need` (prevents usize overflow in debug builds)
  - **Bug fix (high)**: `encode_song/encode_query_list/encode_stream` (Vec variants) now return `Result<Vec<u8>, ProtocolError>` — `TooManyItems { max, got }` variant added to `ProtocolError`
  - **Bug fix (high)**: Async `query_list/start_stream` cmd_buf: 28 bytes → 54 bytes (`2 + MAX_PACKET_IDS(52)`) — Group-100 (52 IDs) no longer errors with `BufferTooSmall`
  - **Feature**: `start_stream` (sync + async) now checks `model.supports_stream()` and returns `ValidationError` for unsupported models (Roomba 400)
  - **Feature**: `set_date`/`set_schedule` (sync + async) now validate `hour ≤ 23`, `minute ≤ 59`, `days & !0x7F == 0`
  - **API**: Added `query_sensor_raw_into` to sync `Create<M,T>` (was async-only)
  - **API**: `#[must_use]` added to all query methods (sync + async)
  - **API**: `#[inline(always)]` added to `transition()` in both `Create` and `AsyncCreate`
  - **Display impls (protocol layer)**: `OiMode`, `ChargingState`, `DayOfWeek`, `IrChar` (delegates to `name()`, Unknown variants show numeric value); `DayOfWeek::name()` const fn added
  - **Display impls (control layer)**: `CreateRobotModel`, `Velocity` (`N.NNN m/s`), `AngularVelocity` (`N.NNN rad/s`), `Radius` (straight/turn-cw/turn-ccw/`N.NNN m`), `MotorPower`, `PowerLedColor`, `LedIntensity`, `SongNumber`
  - **Bug fix**: StreamParser overflow guard `N-2` → `N-3`; oversized frames now correctly rejected + resync
  - **Bug fix**: Checksum error `expected`/`actual` were swapped and formula wrong; now correctly reports received vs correct byte
  - **Soundness**: Capability traits (`SensorReadable`, `Actuatable`, `FullControl`) now properly sealed via private `cap_sealed` module; external code cannot bypass TypeState by implementing traits for wrong modes
  - **Cleanup**: Removed dead `Error<E>` variants (`ModeMismatch`, `Connection`, `NotConnected`) — never constructed
  - **Docs**: `transport_mut()` now has explicit caution warning in both `Create` and `AsyncCreate`
  - **API**: `ChargingState::name()` and `IrChar::name()` added (consistent with `OiMode::name()`)
  - **API**: `From<PowerLedColor> for u8`, `From<LedIntensity> for u8`, `From<SongNumber> for u8` added
  - **Examples**: `create-oi-smol` now has `basic_smol` and `round_smol` examples

### no_std Feature Hierarchy
- `std` (default) → implies `alloc` + `create-oi-protocol/std`
- `alloc` → enables Vec convenience APIs
- bare → pure no_std async API only (Embassy compatible)

### Test Summary
- 51 unit tests (protocol, +7 from Round 2+3)
- 33 unit tests (types + control, +8 from Round 2+3)
- 28 sync mock robot integration tests (+7 from Round 2+3)
- 27 async mock robot integration tests (+8 from Round 2+3)
- 1 protocol doc test
- `just ci` passes: fmt ✅ clippy ✅ build ✅ test ✅
- `just check-nostd` passes: no_std + Embassy thumbv7em-none-eabihf ✅

- [x] **Round 2 correctness (rubber-duck review: EOF, vacuum guard, song in Passive, trailing bytes)**:
  - **Bug fix**: `MockAsyncTransport::read()` now returns `Ok(0)` when `eof_on_read == true` (same as sync)
  - **Bug fix**: `set_motors_pwm` in sync+async rejects i8::MIN before send (validate-before-send)
  - **Feature**: `define_song` now available in Passive mode (was incorrectly gated behind Actuatable)
  - **Bug fix**: `decode_packets` returns `ProtocolError::UnexpectedData { trailing }` on trailing bytes
  - Added 4 async integration tests + 1 protocol unit test
- [x] **Round 3 OI spec compliance (rubber-duck reviews, Groups A + B)**:
  - **Bug fix (blocking)**: `set_motors_pwm` vacuum parameter: added guard `vacuum < 0 → ValidationError`; OI spec defines vacuum range as 0..=127 (no reverse direction), previously negative values slipped through
  - **Bug fix**: `OI_MAX_SONG_NUMBER` was 3, now 4 (Create 2 OI spec §5.13 allows song slots 0–4)
  - **Spec fix**: Stasis accessors (`is_stasis_toggling`, `is_stasis_disabled`) replaced with `is_stasis_detected()` per Create 2 spec (bit 0 = forward progress; bit 1 was incorrectly modeled)
  - **Rename (semantic clarity)**: `SensorData::distance` → `distance_delta_mm`, `angle` → `angle_delta_deg`; both are delta accumulators that reset on each read, not absolute values
  - **Doc fix**: Added detailed doc comments to `distance_delta_mm` and `angle_delta_deg` explaining accumulator-reset semantics
  - **Doc fix**: Added doc comments to `left_encoder_counts`/`right_encoder_counts` noting u16 wraparound and how to compute signed deltas
  - **Bug fix**: `StreamParser` default capacity 256 → 258; maximum valid OI frame is 258 bytes (N=255 payload + 3 framing bytes); previously N=254/255 frames were rejected as oversized
  - Updated tests: vacuum guard tests extended (negative + boundary), song number tests updated, new `decode_distance_angle_fields` + `stasis_detected_accessor` tests
- [x] **Round 4 model-aware songs + stream payload validation**:
  - **Feature**: `CreateRobotModel::max_song_number() -> u8` — Create2=4, Create1/Roomba400=15
  - **Change**: `OI_MAX_SONG_NUMBER` now 15 (most permissive global max); `SongNumber::new()` accepts 0–15
  - **Feature**: `define_song` + `play_song` (sync + async) reject slots > `model.max_song_number()` before sending
  - **Bug fix**: `start_stream` (sync + async) now validates total stream payload bytes per cycle ≤ 255; previously only packet count was checked, not byte payload size
  - Updated tests: `song_number_valid/invalid`, new `model_max_song_number`, `define_song_rejects_out_of_range_slot_for_create2`, `define_song_accepts_slot_15_for_create1`, `play_song_rejects_out_of_range_slot_for_create2`, `start_stream_payload_overflow_rejects_before_send` (sync + async)

### Test Summary
- 51 unit tests (protocol)
- 34 unit tests (types + control, +1 from Round 4)
- 32 sync mock robot integration tests (+4 from Round 4)
- 31 async mock robot integration tests (+4 from Round 4)
- 1 protocol doc test
- Total: **149 tests** | `just ci` passes: fmt ✅ clippy ✅ build ✅ test ✅

- [x] **Round 5 spec compliance + streaming guard**:
  - **Feature**: `SongNote { midi_note: u8, duration_64ths: u8 }` validated newtype; `new()` validates `midi_note` in `31..=127` per OI spec §5.13
  - **Breaking**: `define_song()` signature changed from `&[(u8,u8)]` to `&[SongNote]` (semver-breaking: bumps to 0.5 when published)
  - **Feature**: Streaming/query mutual exclusion — `streaming: bool` field on `Create` + `AsyncCreate`; `start_stream()` sets it; `toggle_stream(false)` clears it; `query_sensor_raw*` and `query_list` return `ValidationError` while streaming is active; flag survives mode transitions
  - **Bug fix**: `AngularVelocity` MAX corrected to `±(2 × 0.5 / 0.235) ≈ ±4.255 rad/s` (was ±π, incorrect for Create 2 geometry)
  - **Bug fix**: `define_song` now returns `TooManyItems` error for > 16 notes (was silently truncating, breaking the spec's own error detection)
  - **Feature**: `set_digit_leds` now validates each character is printable ASCII (32–126); returns `ValidationError` for out-of-range bytes (sync + async)
  - **Prelude**: `SongNote` added to `create_oi::prelude`
  - Added 8 integration tests (4 sync + 4 async): streaming guard on query, resume after toggle, digit LED ASCII validation
  - All 7 existing `define_song` call sites in tests updated to `SongNote::new(...).unwrap()`

### Test Summary (Round 5)
- 51 unit tests (protocol)
- 34 unit tests (types + control)
- 36 sync mock robot integration tests (+4 from Round 5)
- 35 async mock robot integration tests (+4 from Round 5)
- 1 protocol doc test
- Total: **157 tests** | `just ci` passes: fmt ✅ clippy ✅ build ✅ test ✅ | commit `3bac273`

- [x] **Embassy split transport** (`EmbassySplitTransport`):
  - Added `EmbassySplitTransport<R, W>` to `create-oi-embassy`, complementing the existing `EmbassyTransport<T>`
  - Accepts separate RX + TX halves (e.g. from `uart.split()` on Embassy STM32/RP2040)
  - Shared error type `E` constrained via `ErrorType<Error = E>` on both halves
  - Accessors: `new(rx, tx)`, `into_parts()`, `rx()`, `tx()`, `rx_mut()`, `tx_mut()`
  - `delay()` uses `embassy_time::Timer::after()` (same as `EmbassyTransport`)
  - `just check-nostd` and `just ci` pass with no regressions

- [x] **Round 6 OI spec compliance audit**:
  - **Bug fix (critical)**: `MotorBits::to_raw()` had bits 3 and 4 reversed; per OI spec §5.6 bit 3 = side brush direction (clockwise), bit 4 = main brush direction (outward). All direction commands were sending the wrong bit to the wrong motor.
  - **New tests**: `motor_bits_side_brush_backward_is_bit3` and `motor_bits_main_brush_backward_is_bit4` verify each direction bit individually (existing `motor_bits_reverse` masked the bug by setting both flags simultaneously).
  - **New error variant**: `ProtocolError::TooFewItems { min, got }` added to `#[non_exhaustive]` enum.
  - **Validation**: `encode_song[_into]` now rejects 0-note songs (OI spec §5.13: song length must be 1–16); `define_song` in sync + async APIs propagates the error.
  - **Validation**: `encode_query_list[_into]` and `encode_stream[_into]` now reject empty packet ID lists.
  - **Rename**: `SensorData::is_stasis_detected()` → `is_making_forward_progress()` (old name is a deprecated alias); the previous name implied the robot was stationary when bit 0 = 1 actually means forward progress.
  - **Doc fixes**: `encode_motors` comment now correctly states bit 3 = side brush direction, bit 4 = main brush direction; `encode_motors_pwm` clarifies vacuum is 0–127 unsigned; song doc comments correct the song number range to 0–15 / 0–4.
  - Commit: `0cdc873`

### Test Summary (Round 6)
- 56 unit tests (protocol, +5 from Round 6)
- 36 unit tests (types + control, +2 from Round 6)
- 36 sync mock robot integration tests
- 35 async mock robot integration tests
- 1 protocol doc test
- Total: **163 tests** | `just ci` passes: fmt ✅ clippy ✅ build ✅ test ✅ | `just check-nostd` ✅ | commit `0cdc873`

- [x] **Naming cleanup — `robot` → `create`** (aligning with libcreate's `class RobotModel` terminology):
  - **Type rename**: `CreateRobotModel` → `RobotModel`; deprecated alias `type CreateRobotModel = RobotModel` retained in `types.rs` (not in prelude)
  - **Field rename**: `TransitionError::robot` → `TransitionError::create`
  - **File renames**: `tests/mock_robot.rs` → `tests/mock_create.rs`, `tests/mock_async_robot.rs` → `tests/mock_async_create.rs`
  - **Test function renames**: `robot_*` / `async_robot_*` → `create_*` / `async_create_*`; `robot_model_*` → `model_*`
  - **Variable renames**: `robot` → `create` in all tests and examples
  - **Doc comment updates**: "Synchronous/Asynchronous robot API" → "…Create API"; struct docs updated; transport crates "for the robot" → "for the Create/Roomba"; hardware-describing comments kept as-is
  - Commit: `e23c4fd`

- [x] **Round 7 — Exploratory 3-duck audit + validation bypass fixes**:
  - **3 rubber-duck agents** ran a parallel exploratory audit (API design, protocol, transport).
  - **Bug fix (HIGH)**: `Radius::Curve(f32)` was a public enum variant, allowing `Radius::Curve(f32::NAN)` to bypass `Radius::new()` validation. Fixed by introducing an opaque `CurveRadius` newtype with a private `f32` field; `Radius::Curve(CurveRadius)` is now only constructable via `Radius::new()`. `CurveRadius` is exported from the prelude; pattern-matching still works via `r.as_meters()`.
  - **Bug fix (HIGH)**: `SongNote.midi_note` and `.duration_64ths` were `pub` fields, allowing struct-literal construction that bypassed the 31–127 MIDI range check. Fields are now private; `pub const fn midi_note(self) -> u8` and `duration_64ths(self) -> u8` accessors added.
  - **Bug fix (MED)**: `Radius::new(0.0)` was accepted (rounds to 0 mm, not a valid OI arc radius). Now returns `ValidationError` with a clear message distinct from the ±1/32767 special-value rejections. Any value rounding to 0 mm is rejected.
  - **Bug fix (MED)**: `SerialTransport::close()` only flushed, leaving the port open. Refactored to `Option<Box<dyn SerialPort>>`; `close()` calls `take()`, flushing and dropping the OS handle. Subsequent `read`/`write_all` calls return `io::ErrorKind::NotConnected`. `close()` is idempotent.
  - **New tests**: `radius_zero_rejected`, `radius_smallest_valid_curve`, `song_note_accessors`, `song_note_invalid_midi`, `curve_radius_as_meters` (+5).
  - Commit: `9dfe1b7`

### Test Summary (Round 7)
- 56 unit tests (protocol)
- 41 unit tests (types + control, +5 from Round 7)
- 36 sync mock integration tests
- 35 async mock integration tests
- 1 protocol doc test
- Total: **168 tests** | `just ci` passes: fmt ✅ clippy ✅ build ✅ test ✅ | commit `9dfe1b7`

### Remaining

## Round 8 — Transport trait cleanup (commit 1a76d44)

### Goal
Evaluate and implement TypeState for the Transport layer.

### Decision
After rubber-duck review, full TypeState for `SerialTransport<Open/Closed>` was
rejected as over-engineering. Key reasons:
- `SerialTransport<Closed>` has nowhere to store the dropped port, forcing `Option<Box>` back
- TypeState only benefits users who call `into_transport()` — a narrow surface area
- Async transports already have no `close()`, making TypeState asymmetric

### Changes
- **Removed `fn close(&mut self)`** from `Transport` trait — brings sync/async into
  consistency; port closes on `Drop` (idiomatic Rust)
- **Reverted `Option<Box<dyn SerialPort>>`** back to `Box<dyn SerialPort>` in
  `SerialTransport` — the Option complexity was only needed for the old `close()` pattern
- **Added inherent `SerialTransport::close(self) -> io::Result<()>`** — consuming
  method for explicit flush-before-drop; not part of the trait
- **Removed `fn close(&mut self)`** from `MockTransport` test implementation

### Result
168 tests pass, CI green. Transport API is now simpler and consistent with `AsyncTransport`.

## Round 9 — Model-gated opcodes, stop() fix, reset() in Off mode (commit 3d42fa3)

### Goal
Fix OI spec compliance issues found in Round 1 audit: 7 opcodes that are Create 2–only
were missing model guards, `stop()` used wrong opcode, and `reset()` was unavailable
in Off mode.

### Changes
- **`RobotModel::is_create2()`** — centralized helper in `types.rs`; replaces ad-hoc
  model checks throughout the codebase
- **Model gates on 7 Create 2–only opcodes** (both sync and async):
  `drive_pwm`, `set_digit_leds`, `set_motors_pwm`, `set_digit_leds_raw`,
  `simulate_buttons`, `set_date`, `set_schedule` — return `ValidationError`
  on Create1 or Roomba400 before any bytes are sent
- **`stop()` opcode fix**: was `encode_drive(0, 0)` (opcode 137, DRIVE) — now
  `encode_drive_direct(0, 0)` (opcode 145, DRIVE_DIRECT). Radius=0 is not a valid OI
  DRIVE value; DRIVE_DIRECT(0,0) directly expresses "zero both wheels"
- **`reset()` in Off mode** (sync and async): OI spec says RESET (opcode 7) is
  available at any time; returns the transport wrapped in `ConnectError<T, E>` for
  recovery, matching the `start()` API
- **Removed `transport_mut()`** — had zero callers, bypassed TypeState guarantees;
  deleted entirely rather than making it dead code

### Tests added (18 new)
- `reset_available_in_off_mode` / `async_reset_available_in_off_mode`
- `drive_pwm_rejects_create1_before_send` / `_roomba400_before_send` (sync + async = 4)
- `set_motors_pwm_rejects_create1_before_send` (sync + async = 2)
- `set_digit_leds_rejects_create1_before_send` (sync + async = 2)
- `set_digit_leds_raw_rejects_create1_before_send` (sync + async = 2)
- `simulate_buttons_rejects_create1_before_send` (sync + async = 2)
- `set_schedule_rejects_create1_before_send` (sync + async = 2)
- `set_date_rejects_create1_before_send` (sync + async = 2)
- Updated `stop_sends_zero_drive` / `async_stop_sends_zero_drive` to assert opcode 145

### Result
186 tests pass (41 unit + 45 sync + 44 async + 56 protocol). CI green.

---

## Round 10 — OI spec mode-transition fixes (commit `d2e15bc`)

**Trigger:** rubber-duck OI audit round 2 (HIGH/MEDIUM findings)

### Changes

- **`cleared_transition()` helper** — new internal helper in `Create`/`AsyncCreate`
  that resets `streaming=false` and `stream_parser=StreamParser::new()` when
  transitioning across OI session boundaries
- **`to_off()` model gate** (sync + async, Passive/Safe/Full = 6 impls):
  rejects Create 1 and Roomba 400 with `ValidationError` before sending any bytes;
  now uses `cleared_transition()` instead of `transition()` to reset stream state
- **`power_off()` return type** changed from `Result<T, ...>` to
  `Result<Create<Passive, T>, ...>` (sync + async): POWER (opcode 133) puts the
  robot into Passive charging mode, not Off; stream state cleared via
  `cleared_transition()`
- **`clean()` / `seek_dock()` moved to `Create<Passive, T>` only** (sync + async):
  per OI spec, CLEAN/SPOT/MAX/DOCK commands are valid in Passive mode only; removed
  from the `SensorReadable` impl block
- **`set_date()` / `set_schedule()` moved from `FullControl` to `SensorReadable`**
  (sync + async): per OI spec, SET_DAY_TIME (168) and SCHEDULE (167) are available
  in Passive, Safe, and Full modes; `simulate_buttons()` remains `FullControl`-only

### Tests added (26 new)

- `to_off_rejects_create1/roomba400_before_send_from_passive/safe/full` (×8 sync+async)
- `to_off_succeeds_on_create2_sends_stop_opcode` (sync + async = 2)
- `power_off_returns_passive_handle` (sync + async = 2)
- `power_off_clears_streaming_state` (sync + async = 2)
- `set_date_available_in_passive/safe_mode` (sync + async = 4)
- `set_schedule_available_in_passive/safe_mode` (sync + async = 4)
- `clean_available_in_passive_mode` (sync + async = 2)
- `seek_dock_available_in_passive_mode` (sync + async = 2)

### Result
212 tests pass (41 unit + 57 sync + 58 async + 56 protocol). CI green.

---

## Round 11 — start_stream validation + scheduling LEDs API (commit TBD)

**Trigger:** rubber-duck OI audit round 3 (HIGH/MEDIUM findings)

### Changes

- **`start_stream()` unknown-ID validation** (sync + async): previously used
  `packet_info(id).map_or(0, ...)` so unknown/group IDs contributed 0 payload bytes
  and still passed validation — the invalid ID was sent to the robot and the
  `StreamParser` would later choke on it. Fixed with `try_fold` that returns
  `Error::Protocol(UnknownPacketId)` on the first unrecognised packet ID, before any
  bytes are written to the transport.
- **Fixed existing test**: `async_start_stream_too_many_ids_rejects_before_send` was
  using `(7..60)` (included invalid ID 59); updated to `[8u8; 53]` (53 valid IDs).
- **`set_scheduling_leds()` API added** (opcode 162, sync + async): Create 2–only
  (returns `ValidationError` on Create 1 / Roomba 400); available in Safe and Full
  modes (`Actuatable` impl block); parameters `day_leds: u8` (bits 0–6 = Sun–Sat) and
  `schedule_leds: u8` (bits 0–3 = colon/AM-PM/clock/schedule icons).

### Tests added (14 new)

- `start_stream_rejects_unknown_packet_id_before_send` (sync + async = 2)
- `start_stream_rejects_group_packet_id_before_send` (sync + async = 2)
- `start_stream_accepts_valid_packet_ids` (sync + async = 2)
- `set_scheduling_leds_sends_correct_bytes` (sync + async = 2)
- `set_scheduling_leds_rejects_create1` (sync + async = 2)
- `set_scheduling_leds_rejects_roomba400` (sync + async = 2)

### Not fixed this round (deferred)

- **`baud()` command** (opcode 129): The protocol encoder (`encode_baud`) exists but
  exposing a public `baud()` API requires transport-level baud-rate reconfiguration
  which the current `Transport`/`AsyncTransport` traits do not support. Deferred to
  a future transport-trait extension.

### Result
228 tests pass (41 unit + 64 sync + 63 async + 56 protocol + 1 doc). CI green.

---

## Round 12 OI Spec Compliance Audit

**Date**: 2026-07-20
**Commit**: (pending)

### Changes

#### `AsyncTransport::delay` signature
- Changed `async fn delay(&self, ...)` → `async fn delay(&mut self, ...)`
- Consistent with all other transport methods (`write_all`, `read`, `flush` all take `&mut self`)
- Updated all implementations: EmbassyTransport, EmbassySplitTransport, TokioTransport, SmolTransport, MockAsyncTransport
- `sleep_mode_change(&self)` in async_create.rs updated to `&mut self`

#### `power_off()` return type — **Breaking change**
- Changed from `Create<Passive, T>` / `AsyncCreate<Passive, T>` → `Create<Off, T>` / `AsyncCreate<Off, T>`
- Per OI spec: opcode 133 transitions the OI to **Off mode** (robot powers down)
- Returning Passive was misleading; robot stops responding to OI commands after power-off
- To resume: physically wake robot (Clean button or dock), then call `start()`

#### `clean()` and `seek_dock()` — available in Safe and Full modes
- Added to `impl<M: Actuatable, T: Transport> Create<M, T>` and async equivalent
- Per OI spec these commands are valid from Passive, Safe, or Full modes
- Previously only available from Passive mode (spec gap)
- Updated doc comments on Passive versions to note cross-mode availability

#### `query_sensor_raw` / `query_sensor_raw_into` — group packet support
- Now accepts group packet IDs (0-6, 100) via `group_data_len()` fallback
- Uses correct byte count for the full group response
- Unknown IDs (neither individual 7-58 nor group 0-6/100) still return `UnknownPacketId`
- `query_sensor()` (typed) still requires individual packet IDs (group decoding is ambiguous)

### Tests added (11 sync + 11 async = 22 new tests)

- `power_off_returns_off_handle` / `async_power_off_returns_off_handle` (replaces Passive variant)
- `power_off_clears_streaming_state` / `async_power_off_clears_streaming_state` (updated)
- `clean_available_in_safe_mode` / `async_clean_available_in_safe_mode`
- `clean_available_in_full_mode` / `async_clean_available_in_full_mode`
- `seek_dock_available_in_safe_mode` / `async_seek_dock_available_in_safe_mode`
- `seek_dock_available_in_full_mode` / `async_seek_dock_available_in_full_mode`
- `query_sensor_raw_with_group_id_zero` / `async_query_sensor_raw_with_group_id_zero`
- `query_sensor_raw_into_with_group_id_100` / `async_query_sensor_raw_into_with_group_id_100`
- `query_sensor_raw_still_rejects_truly_unknown_id` / `async_query_sensor_raw_rejects_unknown_id`

### Not fixed this round (deferred)

- **`baud()` command**: requires transport-level baud reconfiguration (still deferred)
- **`control()` opcode 130**: Legacy Create 1 alias for Safe mode; no public API needed
- **`query_sensor()` with group IDs**: would require returning multiple `SensorData` — deferred

### Result
239 tests pass (41 unit + 71 sync + 70 async + 56 protocol + 1 doc). CI green. no_std builds pass.

---

## Round 13: baud() command + OI spec type/buffer fixes

### Changes

#### `BaudRate` enum + extension traits
- Added `BaudRate` (codes 0-10, 300–115200 bps) to `create-oi-protocol/src/types.rs`
- `encode_baud` in `command.rs` updated to accept `BaudRate` instead of raw `u8`
- `BaudConfigurable` (sync, `#[cfg(feature="std")]`) and `AsyncBaudConfigurable` (async) extension traits added to `create-oi/src/transport.rs`
- `baud()` available on `Create<M: SensorReadable, T: Transport + BaudConfigurable>` and async mirror
- Per spec: sends BAUD opcode → waits 100ms → calls `transport.set_baud()`
- `SerialTransport`, `TokioTransport`, `SmolTransport` all implement the extension trait
- Mock transports updated with `last_set_baud: Option<BaudRate>` for test assertions

#### `encode_motors_pwm` vacuum type: `i8` → `u8`
- `vacuum` parameter changed from `i8` to `u8` at both protocol and high-level API layers
- API validation changed from `vacuum < 0` to `vacuum > 127`
- This is a breaking change: callers passing `-1` or `i8::MIN` must now pass `128` or `255`
- Tests updated to use `vacuum = 128` and `vacuum = 255` as invalid values

#### async `query_sensor()` — explicit group ID rejection
- Added upfront validation: group packet IDs (0-6, 100) return `ValidationError` with helpful message
- Previously would fail at `decode_packet()` with confusing `UnknownPacketId` error
- Same fix applied to sync `query_sensor()` in `create.rs`

#### async `query_list()` / `start_stream()` — alloc removes 52-ID cap
- `#[cfg(feature = "alloc")]` path uses `Vec`-based command buffers (up to 255 IDs)
- `#[cfg(not(feature = "alloc"))]` path keeps 52-ID stack buffer with updated error message
- Tests updated: "too many IDs" tests now use protocol-level limits (256 IDs for `query_list`, 128×packet-8 for `start_stream` payload > 255 bytes)

### Tests added (6 sync baud + 4 async baud = 10 new tests)

- `baud_rate_from_code_round_trip` / `baud_rate_baud_u32_all_codes` (protocol unit tests, counted above)
- `baud_sends_correct_bytes_and_calls_set_baud` / `async_baud_sends_correct_bytes_and_calls_set_baud`
- `baud_available_from_passive_mode` / `async_baud_available_from_passive_mode`
- `baud_available_from_safe_mode` / `async_baud_available_from_safe_mode`
- `baud_available_from_full_mode` / `async_baud_available_from_full_mode`

### Result
248 tests pass (41 unit + 77 sync + 74 async + 56 protocol + 1 doc). CI green. no_std builds pass.

## Round 14 — Group packet support, duplicate ID rejection, poll_stream guard, scheduling_leds reserved bits

**Findings fixed (6):**

1. **High: Group packet IDs accepted in `query_list`/`start_stream`**
   - `decode_packets()` in `sensor.rs`: expands group IDs to constituent packets before decoding
   - `expected_data_len()`: falls back to `group_data_len()` for group IDs (const fn)
   - `start_stream()` payload calculation: group ID byte count = `members.len() + group_data_len` (accounts for per-member ID prefix in stream frames)
   - Applies to both sync (`create.rs`) and async (`async_create.rs`, both alloc/no-alloc variants)

2. **Medium: Duplicate packet IDs rejected**
   - Added `has_duplicate_ids(ids: &[u8]) -> bool` in `sensor.rs` (256-bit stack bitset, O(n), no_std)
   - `query_list()` and `start_stream()` now validate for duplicates before sending any bytes
   - Both sync and async (alloc + no-alloc) variants updated

3. **Medium: `query_sensor()` error message corrected**
   - `query_sensor()` now says `query_sensor_raw_into()` (not `query_sensor_raw()` which is alloc-only)
   - Fixed in both `create.rs` and `async_create.rs`

4. **Medium: `poll_stream`/`poll_stream_with` require active stream**
   - Added `reject_if_not_streaming()` helper to both `create.rs` and `async_create.rs`
   - `poll_stream` / `poll_stream_with` return `ValidationError` if `start_stream()` was not called

5. **Low: `set_scheduling_leds` rejects reserved bits**
   - `day_leds & 0x80 != 0` → `ValidationError { field: "day_leds" }`
   - `schedule_leds & 0xF0 != 0` → `ValidationError { field: "schedule_leds" }`
   - Both sync and async updated with descriptive doc comments

**Tests:** 271 total (was 248 → +23 new tests)
- Protocol unit: `has_duplicate_ids` (5), `expected_data_len` with group IDs (4)
- Sync integration: duplicate ID rejection (2), poll_stream guard (2), scheduling_leds reserved bits (3), group ID acceptance (1)
- Async integration: same set (9)
- Updated: `poll_stream_eof_returns_protocol_error` (sync + async) to call `start_stream` first
- Updated: `start_stream_rejects_group_packet_id_before_send` → `start_stream_accepts_group_packet_id` (behavior change)

---

## Round 15: OI Spec Compliance Guards (Two-Agent Parallel Audit + Cross-Review)

**Process:** Dual parallel audit — spec-researcher (general-purpose agent) researched OI spec differences per model against primary sources (Roomba 400 SCI v1, Create 1 OI v2, Create 2 OI v3), and rubber-duck (duck-round15) performed an independent library audit. Results were cross-reviewed for conflicts and new findings.

**Part A (spec-researcher commit 45703ca):** Model-specific OI compliance guards

1. **High: `to_safe()` Passive→Safe wrong opcode for Roomba 400**
   - Roomba 400 SCI: Passive→Safe requires opcode 130 (CONTROL), not 131 (SAFE)
   - `Create<Passive, T>::to_safe()` now sends CONTROL for Roomba400, SAFE for Create1/2
   - Both sync and async updated

2. **High: `CleanMode::Max` sends Demo opcode on Create 1**
   - Opcode 136 = "Demo" (runs built-in demos) on Create 1; = "Max" on Roomba400/Create2
   - `clean(CleanMode::Max)` now returns `ValidationError` for Create1

3. **High: `drive_direct()` / `drive_twist()` not guarded for Roomba 400**
   - Opcode 145 (DRIVE_DIRECT) does not exist in Roomba 400 SCI
   - Both reject `Roomba400` with `ValidationError`

4. **High: `stop()` uses DRIVE_DIRECT (opcode 145) on Roomba 400**
   - Roomba 400: `stop()` now uses DRIVE (137) with 0x8000 straight sentinel (v=0, straight)
   - Create 1/2: unchanged (still uses DRIVE_DIRECT)

5. **High: `query_list()` not guarded for Roomba 400**
   - Opcode 149 (QUERY_LIST) not in Roomba 400 SCI
   - Returns `ValidationError` for Roomba400

6. **High: Sensor packet IDs not model-filtered**
   - Packets 43–58 are Create 2-only; groups 4–6 not in Roomba 400; group 100 not in Create 1/Roomba400
   - Added `RobotModel` helpers: `supports_individual_sensor_packets()`, `max_individual_sensor_packet_id()`, `supports_query_list()`, `supports_group_packet()`
   - All query/stream methods validate against model capability

**Part B (Round 15 continuation):** Wire encoding bugs found via cross-review

7. **High: `BaudRate` codes 9–11 wrong for all models**
   - OI spec Table 3 has 12 codes (0–11): code 9=38400, 10=57600, 11=115200
   - Old code had 11 entries (0–10) with 9=57600, 10=115200 — missing 38400, wrong codes for 57600/115200
   - Added `BaudRate::Baud38400`, updated `Baud57600` to code 10, `Baud115200` to code 11

8. **Medium: Straight-driving sentinel canonicalized to i16::MIN (0x8000)**
   - OI spec lists `32768` (bytes `[0x80, 0x00]`) as primary sentinel for "straight"; 32767 (0x7FFF) is an alias
   - Changed `OI_RADIUS_STRAIGHT_RAW` from `0x7FFF` to `i16::MIN` for canonical wire encoding
   - `stop()` on Roomba400 was already using `i16::MIN`; `Radius::Straight` now also uses it

9. **Medium: `set_leds()` used wrong bit layout for Roomba 400**
   - Roomba 400 SCI LEDS byte: debris=bit3, check_robot=bit4, dock=bit5, spot=bit6
   - Create 1/2 LEDS byte: debris=bit0, spot=bit1, dock=bit2, check_robot=bit3
   - Added `pub(crate) fn led_bits(model, ...)` helper in `types.rs`; both sync and async `set_leds()` dispatch to it

**Tests:** 301 total (was 291 → +10 new tests)
- Baud code correctness: Baud38400 sends code 9, Baud57600 sends code 10, Baud115200 sends code 11 (sync + async)
- LED bit layout: Create2 uses low bits (0–3), Roomba400 uses high bits (3–6) (sync + async)
- Straight sentinel: Radius::Straight encodes to i16::MIN (bytes [0x80, 0x00])
- Updated baud round-trip test: from_code now covers 0–11

---

## Round 16: Comprehensive Examples Coverage

**Process:** Lateral thinking audit (waterfall approach after Round 15) identified missing example coverage across all implemented features. Parallel agents hit API rate limits; work executed directly. Rubber-duck review caught 3 bugs (bitmask, stream multi-frame, round_sync semantics).

**Added examples:**

| Crate | File | Features demonstrated |
|-------|------|-----------------------|
| create-oi-serial | `leds_sync.rs` | `set_leds`, `set_scheduling_leds`, `set_digit_leds`, `set_digit_leds_raw` |
| create-oi-serial | `songs_sync.rs` | `define_song`, `play_song`, `SongNote`, `SongNumber` |
| create-oi-serial | `full_sync.rs` | `to_full`, `drive_direct`, `drive_twist`, `drive_pwm`, `set_motors`, `set_motors_pwm`, `simulate_buttons` |
| create-oi-serial | `stream_sync.rs` | `start_stream`, `poll_stream_with`, `toggle_stream` |
| create-oi-serial | `sensors_sync.rs` | `query_sensor`, `query_list`, `read_oi_mode` |
| create-oi-serial | `schedule_sync.rs` | `set_date`, `set_schedule`, `DayOfWeek` |
| create-oi-serial | `round_sync.rs` | **Replaced duplicate** with `clean(Spot)` + `seek_dock` |
| create-oi-tokio | `stream_tokio.rs` | Async streaming (was missing from tokio crate) |
| create-oi-tokio | `leds_tokio.rs` | Async LED control |

**Bugs caught by rubber-duck review:**
1. `schedule_sync.rs`: day bitmask `0b000_1010` ≠ Monday+Thursday; fixed to `0b001_0010`
2. `stream_sync.rs` / `stream_tokio.rs`: `== 15` frame check could be skipped on multi-frame reads; fixed to `>= 15` with `paused` flag
3. `round_sync.rs`: doc comment said "observe while cleaning" but code reclaims control; clarified

**Verification:** All 9 examples compile clean under `cargo check --all-targets` and `just ci` (301 tests).

---

## Round 17: MIDI Feature (`create-oi/midi`)

**Design reviewed by rubber-duck agent before implementation. Key findings addressed:**
- `midi = ["alloc"]` (not `std`) — avoids forcing `std` on no_std users via feature unification
- Global tempo map (all tracks) + piecewise integration across tempo changes
- Reject MIDI Format 2 (sequential) explicitly
- `u128` arithmetic for duration to prevent overflow
- Use `create_oi_protocol::MAX_SONG_NOTES` constant instead of hardcoding 16

### New crate: `create-oi/src/midi.rs` (feature-gated: `midi`)

**Types:**
- `MidiConfig { track: Option<usize>, tempo_micros_per_beat: Option<u32> }`
- `MidiError` — `Parse` / `NoNotes` / `UnsupportedTiming` / `UnsupportedFormat` / `InvalidTiming`

**Functions:**
- `midi_to_notes(&[u8], &MidiConfig) -> Result<Vec<SongNote>, MidiError>`
  - Parses SMF, builds global tempo map (all tracks), auto-detects melody track,
    performs monophonic extraction (new NoteOn cuts previous note), piecewise duration
    integration across tempo changes; drops rests (robot has no silence representation)
- `notes_to_chunks(Vec<SongNote>) -> Vec<Vec<SongNote>>` — splits into ≤16-note chunks

### Feature flags

| Crate | Change |
|-------|--------|
| `create-oi` | `midi = ["alloc", "dep:midly"]` added; `midly` = `{ version = "0.5", default-features = false, features = ["alloc"], optional = true }` |
| `create-oi-serial` | `midi = ["create-oi/midi"]` feature added |
| `create-oi-tokio` | `midi = ["create-oi/midi"]` feature added |
| `create-oi-smol` | `midi = ["create-oi/midi"]` feature added |
| `create-oi-embassy` | Unchanged (`default-features = false`, no midi) |

### Examples added (all `required-features = ["midi"]`)

| Crate | File | Runtime |
|-------|------|---------|
| `create-oi-serial` | `play_midi_sync.rs` | sync std::thread |
| `create-oi-tokio` | `play_midi_tokio.rs` | tokio |
| `create-oi-smol` | `play_midi_smol.rs` | smol::block_on |

### Tests added (16 new, run via `cargo test -p create-oi --features midi`)

- `test_single_quarter_note_120bpm` — basic note, 120 BPM, expected 32 robot units
- `test_noteon_vel0_acts_as_noteoff` — vel=0 NoteOn equals NoteOff
- `test_format1_tempo_from_conductor_track` — SMF format 1 with tempo in track 0, note in track 1
- `test_tempo_override` — MidiConfig::tempo_micros_per_beat overrides file tempo
- `test_out_of_range_pitches_return_no_notes` — pitch < 31 → NoNotes
- `test_out_of_range_pitches_skipped_valid_remain` — mixed valid/invalid pitches
- `test_smpte_timing_rejected` — SMPTE header → UnsupportedTiming
- `test_format2_rejected` — Format 2 → UnsupportedFormat
- `test_dangling_active_note_at_end_of_track` — unclosed note; duration clamped to 1
- `test_monophonic_cut_on_new_noteon` — chord cut to 2 sequential notes
- `test_explicit_track_selection` — MidiConfig::track selects specific track
- `test_chunks_single` / `test_chunks_exactly_max` / `test_chunks_over_max` / `test_chunks_empty`
- `test_duration_very_long_clamped_to_255` — 2-beat note at 20 BPM → clamp to 255

### CI update (justfile)

```just
test: build
    cargo test --workspace
    cargo test -p create-oi --features midi
```

### Result
301 tests pass (`--workspace`); 62 pass with `--features midi`. no_std builds unaffected.
Embassy crate unchanged (uses `default-features = false`).


---

## Round 18: Typed Drive Wrappers + TryFrom Validation

**Design reviewed by rubber-duck agent before implementation. Key findings addressed:**
- Rename `VelocityMmS` → `VelocityMmPerSec` (clearer)
- Drop `From<i16>` (would defeat type safety); keep `from_raw()` for unchecked construction
- Implement `From<Velocity>` etc. in `create-oi` (infallible since source types already validated)
- Include all missed call sites (`stop()`, `drive_twist()`)

### New protocol types in `create-oi-protocol/src/types.rs`

| Type | Constant(s) | `TryFrom<i16>` valid range |
|------|-------------|---------------------------|
| `VelocityMmPerSec(i16)` | `ZERO` | `−500..=500` |
| `RadiusMm(i16)` | `STRAIGHT`, `TURN_CW`, `TURN_CCW` | `i16::MIN` ∪ `−2000..=−1` ∪ `1..=2000` (0 rejected) |
| `WheelPwm(i16)` | `STOP` | `−255..=255` |

All three implement `TryFrom<i16>` with OI spec range validation; `Err(v)` returns the rejected raw value (matching the `DayOfWeek::try_from` pattern).

### Updated encode functions

`encode_drive`, `encode_drive_direct`, `encode_drive_pwm` in `command.rs` now accept
`VelocityMmPerSec`/`RadiusMm`/`WheelPwm` instead of raw `i16`.

### Conversions in `create-oi/src/types.rs`

Infallible conversions added (high-level → protocol types):
- `From<Velocity> for VelocityMmPerSec`
- `From<Radius> for RadiusMm`
- `From<MotorPower> for WheelPwm`

### Call site updates

All 5 drive call sites in `create.rs` and 5 in `async_create.rs` updated to use `.into()`,
`VelocityMmPerSec::ZERO`, `RadiusMm::STRAIGHT`, or `VelocityMmPerSec::from_raw()`.

### CC0 demo MIDI asset

`assets/midi/game-over.mid` added — 22-note CC0 file from
[m-malandro/CC0-midis](https://github.com/m-malandro/CC0-midis); ideal for example code
(familiar concept, 2 song chunks, only 969 bytes).

### Tests added (12 new in `create-oi-protocol/src/types.rs`)

- `velocity_mm_per_sec_try_from` — boundary tests ±500, ±501, i16::MAX
- `radius_mm_try_from` — sentinel, boundary, zero rejection, out-of-range
- `wheel_pwm_try_from` — boundary ±255, ±256

### Prelude

`VelocityMmPerSec`, `RadiusMm`, `WheelPwm` re-exported from `create_oi::prelude`.

## Round 19 — Double-Buffer MIDI Playback

### Problem

With the single-slot polling approach (Round 18), each inter-chunk gap was:
- Detection delay: 0–30 ms (30 ms poll interval)
- `define_song` write: ~3 ms (34 bytes at 115200 baud)
- `play_song` write: ~0.2 ms

Total gap: ~33 ms per chunk. For 0.25 s chunks that is 13% silence, clearly audible as choppiness when testing with `ssg_17.mid` (656 chunks).

### Solution

Double-buffer architecture using 2 song slots (0 and 1):

1. Pre-load `chunk[0]` into slot 0, `chunk[1]` into slot 1, then `play_song(slot 0)`
2. For each chunk `i`, poll `SONG_PLAYING` at **5 ms** intervals
3. On detection: `play_song(preloaded_slot)` immediately (2-byte write only, ~0.2 ms)
4. Swap playing/preloaded slot references
5. During the newly-started chunk's playback, `define_song(free_slot, chunks[i+2])`

Gap = detection delay (0–5 ms) + `play_song` (~0.2 ms) ≈ **5 ms total** vs ~33 ms before —
a 6.6× improvement. For 0.25 s chunks: ~2% silence instead of ~13%.

### Files changed

- `crates/create-oi-serial/examples/play_midi_sync.rs` — double-buffer + 5 ms poll
- `crates/create-oi-tokio/examples/play_midi_tokio.rs` — same (tokio async)
- `crates/create-oi-smol/examples/play_midi_smol.rs` — same (smol async)

### Edge cases handled

- `n == 0`: early return before opening serial port
- `n == 1`: no preloaded_slot setup, no swap
- `i + 2 >= n`: skip `define_song` pre-load (nothing left to pre-load)

## Round 20 — MIDI Root-Cause Analysis & Time-Based Fix

### Root Cause (confirmed via parallel agent investigation)

Two independent agents (explore + rubber-duck) identified the same root cause:

1. **macOS USB-to-serial driver buffers `read()` data for 10–20 ms** in the kernel
   before delivering it to userspace — regardless of poll interval.  Each
   `query_sensor` round-trip therefore has ~10–20 ms of irreducible latency.

2. **OI sensor refresh rate = 15 ms** (64 Hz).  Polling at 5 ms was illusory;
   the sensor value can only change every ~15.625 ms anyway.

3. **`play_song` interrupts**: confirmed by the OI specification — sending
   `play_song` while a song is already playing immediately interrupts the
   current song and starts the new one.  There is no queue.  This rules out
   the "burst window" strategy (sending play_song slightly before expected end
   would cut off the current chunk).

### Solution: time-based chunk transitions (no polling)

- Record `Instant::now()` immediately after each `play_song` call.
- Sleep for `chunk_duration + SONG_TIMING_BUFFER (3 ms)`.
- On wakeup, swap slots and immediately `play_song(preloaded_slot)`.
- `define_song(free_slot, chunks[i+2])` runs during the new chunk's playback.

Expected gap = sleep jitter (±2 ms) + serial write (~1 ms) ≈ **1–5 ms** vs
**10–30 ms** with polling.

The 3 ms buffer ensures `play_song` never arrives while the robot is still
playing (which would interrupt the tail of the current chunk).

### Files changed

- `crates/create-oi-serial/examples/play_midi_sync.rs` — time-based, no polling
- `crates/create-oi-tokio/examples/play_midi_tokio.rs` — same; fix `std::fs::read`
- `crates/create-oi-smol/examples/play_midi_smol.rs` — same

### Hardware/OS limits

On macOS USB-to-serial: **minimum achievable gap ≈ 1–5 ms** (OS timer jitter).
For guaranteed sub-millisecond gaps, a direct UART (e.g., Raspberry Pi GPIO)
or an FTDI chip with its latency timer set to 1 ms would be required.

---

## Round 21 — MIDI Monophonization (sweep-line)

### Root cause confirmed

`ssg_17.mid` produced 656 song chunks (10 496 notes, all `duration_64ths=1 =
15.625 ms`) because the auto-detected single track had rapid polyphonic NoteOn
events. Each new NoteOn cut the previous note at the point of arrival, turning
every simultaneous chord change into a 1-unit fragment.

### Solution: multi-track sweep-line monophonization

Added `MidiConfig::merge_all_tracks: bool` (default `false`, fully backwards
compatible). When `true`:

1. **All** MIDI tracks are scanned for NoteOn/NoteOff events.
2. Events are sorted by `(abs_tick, is_off, pitch, channel)` — NoteOff sorts
   before NoteOn at the same tick for clean note handoffs.
3. A sweep-line maintains a `BTreeMap<pitch, count>` of sounding notes.
4. At every state change the highest (or lowest, via `VoiceSelection`) pitch is
   compared to the previous winner; if it changed, the previous segment is
   emitted as a `SongNote`.
5. Zero-duration segments (two NoteOns at the same tick) are silently discarded.
6. Same-pitch overlapping notes from different tracks are handled by reference
   counting; they merge into a single longer note automatically.

### New API surface

```rust
pub enum VoiceSelection { HighestPitch, LowestPitch }

pub struct MidiConfig {
    pub merge_all_tracks: bool,           // false = old single-track mode
    pub voice_selection: VoiceSelection,  // HighestPitch (default)
    pub filter_percussion: bool,          // true = skip MIDI channel 10
    // existing fields unchanged
    pub track: Option<usize>,
    pub tempo_micros_per_beat: Option<u32>,
}
```

### Files changed

- `crates/create-oi/src/midi.rs` — new types, `collect_note_events`,
  `monophonize_events`, `single_track_notes`; 7 new unit tests
- `crates/create-oi-serial/examples/play_midi_sync.rs` — use `merge_all_tracks: true`
- `crates/create-oi-tokio/examples/play_midi_tokio.rs` — same
- `crates/create-oi-smol/examples/play_midi_smol.rs` — same
