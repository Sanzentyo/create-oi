//! # create-oi
//!
//! A pure Rust, **sans-IO** implementation of the iRobot Create / Roomba
//! [Open Interface (OI)](https://www.irobot.com/about-irobot/stem/create-2)
//! protocol.
//!
//! ## Design
//!
//! - **Sans-IO**: Protocol encoding and decoding work on plain `&[u8]`
//!   slices — zero allocation, zero I/O dependency.
//! - **TypeState**: [`Create<M, T>`](robot::Create) encodes the OI mode
//!   (`Off`, `Passive`, `Safe`, `Full`) as a type parameter so the compiler
//!   prevents invalid operations at compile time.
//! - **Crate-level separation**: Transport implementations live in their own
//!   crates (`create-oi-serial`, `create-oi-tokio`, `create-oi-smol`).
//!
//! ## Quick Start (sync)
//!
//! ```rust,ignore
//! use create_oi::prelude::*;
//! use create_oi_serial::SerialTransport;
//! use create_oi::types::RobotModel;
//!
//! let transport = SerialTransport::open("/dev/ttyUSB0", RobotModel::Create2)?;
//! let robot = Create::new(transport, RobotModel::Create2);
//! let robot = robot.start()?;          // Off → Passive
//! let robot = robot.to_safe()?;        // Passive → Safe
//! // robot.drive(Velocity::new(0.1)?, Radius::STRAIGHT)?;
//! ```
//!
//! ## Quick Start (async / tokio)
//!
//! ```rust,ignore
//! use create_oi::prelude::*;
//! use create_oi_tokio::TokioTransport;
//! use create_oi::types::RobotModel;
//!
//! let transport = TokioTransport::open("/dev/ttyUSB0", RobotModel::Create2)?;
//! let robot = AsyncCreate::new(transport, RobotModel::Create2);
//! let robot = robot.start().await?;    // Off → Passive
//! let robot = robot.to_safe().await?;  // Passive → Safe
//! // robot.drive(Velocity::new(0.1)?, Radius::STRAIGHT).await?;
//! ```

pub mod async_robot;
pub mod error;
pub mod mode;
pub mod protocol;
pub mod robot;
pub mod transport;
pub mod types;

/// Convenient re-exports for common usage.
pub mod prelude {
    pub use crate::async_robot::AsyncCreate;
    pub use crate::error::{ConnectError, Error, TransitionError};
    pub use crate::mode::{Actuatable, Full, Mode, Off, Passive, Safe, SensorReadable};
    pub use crate::robot::Create;
    pub use crate::transport::{AsyncTransport, Transport};
    pub use crate::types::{
        LedIntensity, MotorPower, OiMode, PowerLedColor, Radius, RobotModel, SongNumber, Velocity,
    };
}
