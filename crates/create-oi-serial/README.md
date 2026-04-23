# create-oi-serial

Synchronous serial transport for [`create-oi`](https://crates.io/crates/create-oi) —
the iRobot Create / Roomba Open Interface library.

Provides `SerialTransport`, a synchronous [`Transport`] implementation backed by the
[`serialport`](https://crates.io/crates/serialport) crate.

## Usage

```toml
[dependencies]
create-oi        = "0.4"
create-oi-serial = "0.4"
```

```rust,no_run
use create_oi::prelude::*;
use create_oi_serial::SerialTransport;

let transport = SerialTransport::open("/dev/ttyUSB0", RobotModel::Create2)?;
let robot = Create::new(transport, RobotModel::Create2);
let robot = robot.start()?;       // Off → Passive
let robot = robot.to_safe()?;     // Passive → Safe
# Ok::<(), Box<dyn std::error::Error>>(())
```

## MIDI Playback

Enable the `midi` feature to use the bundled MIDI-to-OI song playback example:

```bash
cargo run -p create-oi-serial --example play_midi --features midi -- /dev/ttyUSB0 song.mid
```

See the [workspace README](https://github.com/Sanzentyo/create-oi) for full documentation.

## License

Licensed under either of Apache License 2.0 or MIT License at your option.
