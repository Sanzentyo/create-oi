//! # libcreate
//!
//! A pure Rust implementation of the iRobot Create / Roomba
//! [Open Interface (OI)](https://www.irobot.com/about-irobot/stem/create-2)
//! protocol.
//!
//! ## Design
//!
//! - **Sans-IO**: Protocol encoding and decoding are completely independent
//!   of any I/O runtime. The [`protocol`] module works on plain `&[u8]`
//!   slices — zero allocation, zero copy.
//! - **TypeState**: The [`Robot`](robot::Robot) type encodes the OI mode
//!   (`Off`, `Passive`, `Safe`, `Full`) as a type parameter so the compiler
//!   prevents calling actuator commands while in Passive mode, etc.
//! - **Feature-gated transports**: Choose your I/O backend via Cargo features:
//!   - `serial` (default) — synchronous serial via [`serialport`](https://crates.io/crates/serialport)
//!   - `tokio-runtime` — async via [`tokio-serial`](https://crates.io/crates/tokio-serial)
//!   - `smol-runtime` — async via [`smol`](https://crates.io/crates/smol) + `async-io`
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use libcreate::prelude::*;
//! use libcreate::io::serial::SerialTransport;
//! use libcreate::types::RobotModel;
//!
//! let transport = SerialTransport::open("/dev/ttyUSB0", RobotModel::Create2).unwrap();
//! let robot = Robot::new(transport, RobotModel::Create2);
//! let robot = robot.start().unwrap();         // Off → Passive
//! let robot = robot.to_safe().unwrap();       // Passive → Safe
//! // robot.drive(Velocity::new(0.1).unwrap(), Radius::STRAIGHT).unwrap();
//! ```

pub mod error;
pub mod io;
pub mod mode;
pub mod protocol;
pub mod robot;
pub mod transport;
pub mod types;

/// Convenient re-exports for common usage.
pub mod prelude {
    pub use crate::error::{ConnectError, Error, TransitionError};
    pub use crate::mode::{Actuatable, Full, Mode, Off, Passive, Safe, SensorReadable};
    pub use crate::robot::Robot;
    pub use crate::transport::Transport;
    pub use crate::types::{
        LedIntensity, MotorPower, OiMode, PowerLedColor, Radius, RobotModel, SongNumber, Velocity,
    };
}
