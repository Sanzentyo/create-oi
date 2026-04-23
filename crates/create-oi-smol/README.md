# create-oi-smol

Async smol transport for [`create-oi`](https://crates.io/crates/create-oi) —
the iRobot Create / Roomba Open Interface library.

Provides `SmolTransport`, an [`AsyncTransport`] implementation that wraps the
native serial port in [`smol::Unblock`], dispatching blocking I/O to a thread
pool so the smol executor stays free.

| Platform | Native serial type |
|----------|--------------------|
| Unix     | `serialport::TTYPort` |
| Windows  | `serialport::COMPort` |

## Usage

```toml
[dependencies]
create-oi      = "0.4"
create-oi-smol = "0.4"
smol           = "2"
```

```rust,no_run
use create_oi_smol::SmolTransport;
use create_oi_smol::create_oi::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    smol::block_on(async {
        let transport = SmolTransport::open("/dev/ttyUSB0", RobotModel::Create2)?;
        let robot = AsyncCreate::new(transport, RobotModel::Create2);
        let robot = robot.start().await.map_err(|e| e.source)?;
        let mut robot = robot.to_safe().await.map_err(|e| e.source)?;
        // robot.drive(Velocity::new(0.2)?, Radius::STRAIGHT).await?;
        Ok(())
    })
}
```

## MIDI Playback

Enable the `midi` feature to use the bundled MIDI-to-OI song playback example:

```bash
cargo run -p create-oi-smol --example play_midi --features midi -- /dev/ttyUSB0 song.mid
```

See the [workspace README](https://github.com/Sanzentyo/create-oi) for full documentation.

## License

Licensed under either of Apache License 2.0 or MIT License at your option.
