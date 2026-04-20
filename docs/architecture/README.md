# libcreate-rs Architecture

## Overview

`libcreate-rs` is a safe, idiomatic Rust wrapper for [AutonomyLab/libcreate](https://github.com/AutonomyLab/libcreate), a C++ library for controlling iRobot Create 1, Create 2, and compatible Roomba robots over serial.

## Crate Structure

```
libcreate-rs/
├── libcreate-sys/       # Raw FFI bindings (-sys crate)
│   ├── csrc/
│   │   ├── wrapper.h    # C header (opaque handle + all functions)
│   │   ├── wrapper.cpp  # C++ implementation with try/catch
│   │   └── boost_compat.h  # Boost 1.85+ API compatibility shim
│   ├── build.rs         # cc-crate build script
│   └── src/lib.rs       # extern "C" declarations
├── src/                 # Safe Rust API
│   ├── lib.rs           # Public API, re-exports
│   ├── error.rs         # Error and TransitionError types
│   ├── types.rs         # ADTs: RobotModel, OiMode, ChargingState, newtypes
│   ├── mode.rs          # TypeState markers: Off, Passive, Safe, Full
│   ├── sensor.rs        # SensorSnapshot with rich sub-structs
│   └── robot.rs         # Robot<M: Mode> with typestate transitions
└── vendor/libcreate/    # Git submodule of upstream C++ library
```

## Design Patterns

### TypeState Pattern

The robot's Open Interface (OI) mode is encoded in the type system:

```
Robot<Off> ──connect()──> Robot<Passive> ──into_safe()──> Robot<Safe>
                              │                              │
                              └──into_full()──> Robot<Full> <┘
```

- Mode transitions **consume** `self` and return `Robot<NewMode>`
- Invalid operations (e.g., `drive()` on `Robot<Passive>`) are compile errors
- Failed transitions return `TransitionError<OldMode>` preserving the robot
- Capability traits (`SensorReadable`, `Actuatable`) gate method availability

### Algebraic Data Types (ADTs)

All domain values are proper Rust enums/newtypes:
- `RobotModel`: `Roomba400 | Create1 | Create2`
- `OiMode`: `Off | Passive | Safe | Full | Unknown(i32)`
- `ChargingState`: `NotCharging | Reconditioning | ... | Unknown(i32)`
- Sensor enums include `Unknown` variants for forward-compatibility

### Newtype Pattern

Physical quantities use validated newtypes with private inner fields:
- `Velocity(f32)` — range [-0.5, 0.5] m/s
- `AngularVelocity(f32)` — range [-4.25, 4.25] rad/s
- `MotorPower(f32)` — range [-1.0, 1.0]
- All reject NaN/infinity via `TryFrom<f32>` and `new()`

## FFI Safety Strategy

1. **Opaque C handle**: C++ `create::Create` is wrapped in an opaque struct with `std::mutex`
2. **Exception boundary**: Every C wrapper function has `try/catch(...)` — no C++ exceptions cross FFI
3. **Mutex protection**: All handle access is mutex-guarded in the C layer
4. **Signal handler disabled**: `install_signal_handler = false` in constructor
5. **!Send + !Sync**: `Robot` cannot be shared or sent across threads
6. **Null safety**: `Robot::new()` checks for null pointer from `create_robot_new()`

## Boost Compatibility

The `boost_compat.h` header provides API shims for Boost 1.85+ where:
- `boost::asio::io_service` → subclass of `io_context` with `reset()` re-added
- `boost::asio::deadline_timer` → subclass of `steady_timer` with `expires_from_now()`
- `boost::posix_time::milliseconds()` → `std::chrono::milliseconds`

## Build System

- `build.rs` uses the `cc` crate to compile all C++ sources
- Boost is auto-detected at Homebrew paths (ARM/Intel)
- Optional `ZIG_CXX` env var for zig-based C++ compilation
- `justfile` provides convenient build recipes
