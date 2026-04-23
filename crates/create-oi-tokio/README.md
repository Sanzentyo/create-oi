# create-oi-tokio

Async Tokio transport for [`create-oi`](https://crates.io/crates/create-oi) —
the iRobot Create / Roomba Open Interface library.

Provides `TokioTransport`, an [`AsyncTransport`] implementation backed by
[`tokio-serial`](https://crates.io/crates/tokio-serial).

## Usage

```toml
[dependencies]
create-oi       = "0.4"
create-oi-tokio = "0.4"
tokio           = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust,no_run
use create_oi::prelude::*;
use create_oi_tokio::TokioTransport;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = TokioTransport::open("/dev/ttyUSB0", RobotModel::Create2)?;
    let robot = AsyncCreate::new(transport, RobotModel::Create2);
    let robot = robot.start().await?;       // Off → Passive
    let robot = robot.to_safe().await?;     // Passive → Safe
    Ok(())
}
```

## MIDI Playback

Enable the `midi` feature to use the bundled MIDI-to-OI song playback example:

```bash
cargo run -p create-oi-tokio --example play_midi --features midi -- /dev/ttyUSB0 song.mid
```

See the [workspace README](https://github.com/Sanzentyo/create-oi) for full documentation.

## License

Licensed under either of Apache License 2.0 or MIT License at your option.
