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

### Remaining
