# create-oi-embassy

Embassy async transport adapter for [`create-oi`](https://crates.io/crates/create-oi) —
the iRobot Create / Roomba Open Interface library.

Provides `EmbassyTransport` and `EmbassySplitTransport`, both implementing
[`AsyncTransport`] for use on embedded targets (e.g. STM32, nRF52) running
[Embassy](https://embassy.dev/).

This crate is `#![no_std]` and has no `alloc` dependency.

## Usage

```toml
[dependencies]
create-oi         = { version = "0.4", default-features = false }
create-oi-embassy = "0.4"
```

```rust,ignore
use create_oi::prelude::*;
use create_oi_embassy::EmbassyTransport;

// Configure UART at 115200 baud (Create 2) or 57600 baud (Create 1)
let transport = EmbassyTransport::new(uart);
let robot = AsyncCreate::new(transport, RobotModel::Create2);
let robot = robot.start().await.unwrap();
```

For split UART halves, use `EmbassySplitTransport::new(rx, tx)`.

## Supported Targets

Any target supported by Embassy with `embedded-io-async` UART I/O.
Verified on `thumbv7em-none-eabihf` (Cortex-M4F with FPU).

See the [workspace README](https://github.com/Sanzentyo/create-oi) for full documentation.

## License

Licensed under either of Apache License 2.0 or MIT License at your option.
