# Progress Log

## Phase 1: Project Setup ✅
- Created Cargo workspace: `libcreate` (safe API) + `libcreate-sys` (FFI)
- Added libcreate C++ library as git submodule at `vendor/libcreate/`

## Phase 2: C Wrapper Layer ✅
- Created `wrapper.h` / `wrapper.cpp` — C shim around C++ API
- Every function wrapped with try/catch and mutex protection
- Opaque handle struct with `std::unique_ptr<create::Create>` + `std::mutex`
- Signal handler installation disabled
- Sensor snapshot for atomic multi-value reads

## Phase 3: Build System ✅
- `build.rs` using `cc` crate to compile all C++ sources
- Boost auto-detection for Homebrew (ARM/Intel)
- Created `boost_compat.h` to handle Boost 1.85+ API removals
  - `io_service` → `io_context` subclass
  - `deadline_timer` → `steady_timer` subclass
  - `posix_time::milliseconds` → `std::chrono::milliseconds`
- Resolved macOS 26 SDK libc++ shared_ptr compatibility issue (system compiler works)

## Phase 4: FFI Bindings ✅
- Complete `extern "C"` declarations in `libcreate-sys/src/lib.rs`
- `repr(C)` sensor snapshot struct matching C layout
- Constants for model IDs, modes, clean modes, return codes

## Phase 5: Safe Rust API ✅
- **TypeState pattern**: `Robot<Off>`, `Robot<Passive>`, `Robot<Safe>`, `Robot<Full>`
- **Sealed Mode trait** with capability sub-traits (`SensorReadable`, `Actuatable`)
- **ADTs**: `RobotModel`, `OiMode`, `ChargingState`, `CleanMode`, `DayOfWeek`, `IrChar`
- **Newtypes**: `Velocity`, `AngularVelocity`, `Radius`, `MotorPower` with NaN/range validation
- **TransitionError<M>**: preserves robot on failed mode transitions
- **SensorSnapshot**: rich sub-structs (Bumpers, Cliffs, Battery, Odometry, etc.)
- **Mode verification**: `verify_mode()` and `actual_mode()` for detecting async changes
- **!Send + !Sync** enforced with `static_assertions`
- Compile-time prevention of invalid operations (drive in Passive, etc.)

## Phase 6: Build Automation ✅
- `justfile` with recipes: build, release, test, clippy, fmt, ci, doc, clean

## Phase 7: Documentation ✅
- Architecture docs in `docs/architecture/`
- Progress log in `docs/progress.md`
- User inputs saved in `docs/user-inputs/`

## Phase 8: Testing ✅
- **40 unit tests** across `types` and `sensor` modules — all passing
  - Newtype validation (ranges, NaN, infinity, boundary values)
  - Enum conversions (known values, unknown/forward-compatible variants)
  - Sensor snapshot construction and conversion from raw FFI structs
  - Battery charge ratio edge cases, packet corruption rate
- **25 integration tests** in `tests/robot_integration.rs` — all `#[ignore]`
  - Require real robot hardware + `LIBCREATE_PORT` env var
  - Categories: connection, mode transitions, sensors, driving, LEDs, motors, songs, date, cleaning, docking, full mode, error recovery
- **2 doctests** — compile-only (no robot), verify API examples are valid
- **Testing guide** at `docs/testing-guide.md`
  - Unit test instructions, integration test prerequisites, safety warnings
  - Suggested incremental test order for first-time setup
  - Troubleshooting table
- `just ci` passes cleanly: fmt ✅ clippy ✅ build ✅ test ✅ doctest ✅
