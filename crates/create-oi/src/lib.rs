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
//! - **TypeState**: [`Create<M, T>`](create::Create) encodes the OI mode
//!   (`Off`, `Passive`, `Safe`, `Full`) as a type parameter so the compiler
//!   prevents invalid operations at compile time.
//! - **Layered crates**:
//!   - [`create-oi-protocol`](create_oi_protocol) — wire format encoding/decoding
//!   - This crate — TypeState control API + transport abstraction
//!   - `create-oi-serial`, `create-oi-tokio`, `create-oi-smol` — transports
//!
//! ## Quick Start (sync)
//!
//! ```rust,ignore
//! use create_oi::prelude::*;
//! use create_oi_serial::SerialTransport;
//! use create_oi::types::CreateRobotModel;
//!
//! let transport = SerialTransport::open("/dev/ttyUSB0", CreateRobotModel::Create2)?;
//! let robot = Create::new(transport, CreateRobotModel::Create2);
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
//! use create_oi::types::CreateRobotModel;
//!
//! let transport = TokioTransport::open("/dev/ttyUSB0", CreateRobotModel::Create2)?;
//! let robot = AsyncCreate::new(transport, CreateRobotModel::Create2);
//! let robot = robot.start().await?;    // Off → Passive
//! let robot = robot.to_safe().await?;  // Passive → Safe
//! // robot.drive(Velocity::new(0.1)?, Radius::STRAIGHT).await?;
//! ```

// TransitionError/ConnectError intentionally store the robot/transport handle
// for recovery, making them large. This is by design.
#![allow(clippy::result_large_err)]

pub mod async_create;
pub mod create;
pub mod error;
pub mod mode;
pub mod transport;
pub mod types;

/// Re-export protocol crate for direct access.
pub use create_oi_protocol as protocol;

/// Convenient re-exports for common usage.
pub mod prelude {
    pub use crate::async_create::AsyncCreate;
    pub use crate::create::Create;
    pub use crate::error::{ConnectError, Error, TransitionError};
    pub use crate::mode::{Actuatable, Full, Mode, Off, Passive, Safe, SensorReadable};
    pub use crate::transport::{AsyncTransport, Transport};
    pub use crate::types::{
        CreateRobotModel, LedIntensity, MotorPower, PowerLedColor, Radius, SongNumber, Velocity,
    };

    // Selective protocol re-exports
    pub use create_oi_protocol::opcode::Opcode;
    pub use create_oi_protocol::sensor::SensorData;
    pub use create_oi_protocol::stream::StreamParser;
    pub use create_oi_protocol::types::OiMode;
}
