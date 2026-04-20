# Testing Guide

## Overview

libcreate-rs has two layers of tests:

| Layer | Requires Robot | Command |
|-------|---------------|---------|
| Unit tests | No | `cargo test` |
| Integration tests | **Yes** | `LIBCREATE_PORT=/dev/ttyUSB0 cargo test --test robot_integration -- --ignored` |

## Unit Tests

Unit tests validate pure Rust logic without any hardware:

- **Newtype validation** — `Velocity`, `AngularVelocity`, `Radius`, `MotorPower`, `SongNumber`
  - Valid ranges, boundary values
  - NaN and infinity rejection
  - `TryFrom` trait implementations
- **Enum conversions** — `OiMode`, `ChargingState`, `IrChar`
  - Known raw values → correct variants
  - Unknown raw values → `Unknown(x)` variant
- **Sensor snapshot** — `SensorSnapshot::from(raw)`
  - Default values
  - Boolean conversion (0 = false, any non-zero = true)
  - IR character decoding
  - Light bumper signal forwarding
- **Battery calculations** — `charge_ratio()`, edge cases (0 capacity, overflow)
- **Packet statistics** — `corruption_rate()`, zero total

```sh
# Run all unit tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific module tests
cargo test types::tests
cargo test sensor::tests
```

## Integration Tests

Integration tests exercise the full stack: Rust API → FFI → C++ → serial → robot hardware.

### Prerequisites

1. **Hardware**: iRobot Create 2, Roomba 600+ series, or compatible
2. **Serial cable**: USB-to-serial adapter or Create 2's micro-USB cable
3. **Robot powered on**: Battery charged, robot on a safe surface
4. **Serial port identified**:
   - Linux: typically `/dev/ttyUSB0` or `/dev/ttyACM0`
   - macOS: `/dev/tty.usbserial-*` or `/dev/tty.usbmodem*`
   - Check with `ls /dev/tty.*` (macOS) or `ls /dev/ttyUSB*` (Linux)

### Running Integration Tests

```sh
# Set your serial port
export LIBCREATE_PORT=/dev/ttyUSB0

# Run ALL integration tests (robot will move!)
cargo test --test robot_integration -- --ignored

# Run a single test
cargo test --test robot_integration test_connect_and_disconnect -- --ignored

# Run with verbose output
cargo test --test robot_integration -- --ignored --nocapture

# Run tests matching a pattern
cargo test --test robot_integration test_drive -- --ignored --nocapture
```

### Test Categories

#### 1. Connection Lifecycle
| Test | Description |
|------|-------------|
| `test_connect_and_disconnect` | Basic connect → verify → disconnect |
| `test_reconnect_after_disconnect` | Connect → disconnect → reconnect |
| `test_connection_failure_recovery` | Connect to invalid port, verify robot handle is recoverable |

#### 2. Mode Transitions
| Test | Description |
|------|-------------|
| `test_passive_to_safe_and_back` | Passive ↔ Safe |
| `test_passive_to_full_and_back` | Passive ↔ Full |
| `test_safe_to_full_and_back` | Safe ↔ Full |
| `test_verify_mode_matches` | `verify_mode()` returns Ok in correct modes |

#### 3. Sensor Reading
| Test | Description |
|------|-------------|
| `test_read_sensors_passive` | Read all sensors in Passive mode, verify battery > 0 |
| `test_read_sensors_safe` | Read sensors in Safe mode, print bumper/cliff/packet stats |
| `test_sensor_polling_loop` | Read sensors 10× in rapid succession |
| `test_comprehensive_sensor_report` | Full sensor dump to stdout |

#### 4. Driving Commands ⚠️ **Robot will move!**
| Test | Description |
|------|-------------|
| `test_drive_forward_and_stop` | Drive forward 1s at 0.1 m/s, verify odometry |
| `test_drive_wheels_independently` | Spin in place using differential wheel speeds |
| `test_drive_radius` | Drive in an arc for 2s |
| `test_drive_pwm` | Drive using PWM mode |

#### 5. LED Control
| Test | Description |
|------|-------------|
| `test_leds_cycle` | Toggle debris, spot, dock, check-robot LEDs |
| `test_power_led_colors` | Sweep power LED from green to red |
| `test_digits_ascii` | Display "RUST" on 7-segment display |

#### 6. Motor Control ⚠️ **Brushes will spin!**
| Test | Description |
|------|-------------|
| `test_brush_motors` | Test side brush, main brush, vacuum individually |
| `test_all_motors_combined` | All three motors at once |

#### 7. Songs 🔊
| Test | Description |
|------|-------------|
| `test_define_and_play_song` | Define a 4-note melody and play it |

#### 8. Date/Clock
| Test | Description |
|------|-------------|
| `test_set_date` | Set the robot's internal clock |

#### 9. Cleaning and Docking ⚠️ **Robot will move autonomously!**
| Test | Description |
|------|-------------|
| `test_clean_default` | Start default cleaning (3s then stop) |
| `test_dock` | Send dock-seeking command (5s) |

#### 10. Full Mode ⚠️ **No safety limits!**
| Test | Description |
|------|-------------|
| `test_full_mode_drive` | Drive in Full mode (0.05 m/s, 0.5s) |

### Safety Warnings

⚠️ **Before running driving tests**:
- Place the robot on a flat, open surface
- Keep hands and obstacles clear
- Be ready to pick up the robot to stop it
- Tests use low velocities (0.05–0.1 m/s) for safety

⚠️ **Full mode tests** (`test_full_mode_drive`):
- Cliff/bump sensors are **disabled** — the robot will drive off edges
- Only run on a safe, enclosed surface

⚠️ **Cleaning/docking tests**:
- The robot will move autonomously for several seconds
- Ensure the area is clear

### Suggested Test Order

For first-time testing, run tests incrementally:

```sh
# 1. Verify connection works
cargo test --test robot_integration test_connect -- --ignored --nocapture

# 2. Verify mode transitions
cargo test --test robot_integration test_passive -- --ignored --nocapture

# 3. Read sensors (non-destructive)
cargo test --test robot_integration test_read_sensors -- --ignored --nocapture
cargo test --test robot_integration test_comprehensive -- --ignored --nocapture

# 4. LEDs (visual only, no movement)
cargo test --test robot_integration test_leds -- --ignored --nocapture
cargo test --test robot_integration test_power_led -- --ignored --nocapture
cargo test --test robot_integration test_digits -- --ignored --nocapture

# 5. Songs (audio only)
cargo test --test robot_integration test_define_and_play -- --ignored --nocapture

# 6. Driving (robot will move!)
cargo test --test robot_integration test_drive_forward -- --ignored --nocapture

# 7. Everything
cargo test --test robot_integration -- --ignored --nocapture
```

### Troubleshooting

| Problem | Solution |
|---------|----------|
| "connection failed" | Check serial port path, permissions (`sudo chmod 666 /dev/ttyUSB0`), robot power |
| "command failed" | Robot may have gone to sleep — press Clean button to wake |
| Mode mismatch | Bump in Safe mode causes Passive transition — expected behavior |
| Garbled sensor data | Check baud rate (115200 for Create 2) |
| Permission denied | Add user to `dialout` group: `sudo usermod -a -G dialout $USER` |

### Using `just` for Testing

```sh
# Unit tests only
just test

# Full CI (includes unit tests)
just ci
```

Integration tests are not included in `just ci` because they require hardware.
