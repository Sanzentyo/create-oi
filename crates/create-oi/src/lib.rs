//! # create-oi
//!
//! A pure Rust, **sans-IO** implementation of the iRobot Create / Roomba
//! [Open Interface (OI)](https://www.irobot.com/about-irobot/stem/create-2)
//! protocol.
//!
//! ## Features
//!
//! - `std` (default) — enables the sync [`Create`](create::Create) API and
//!   the `std`-based [`Transport`](transport::Transport) trait.
//! - `alloc` — enables `Vec`-returning convenience methods on `AsyncCreate`.
//! - *(no features)* — pure `no_std` async API only; suitable for Embassy.
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
//! ## Quick Start (sync, requires `std` feature)
//!
//! ```rust,ignore
//! use create_oi::prelude::*;
//! use create_oi_serial::SerialTransport;
//!
//!
//! let transport = SerialTransport::open("/dev/ttyUSB0", RobotModel::Create2)?;
//! let create = Create::new(transport, RobotModel::Create2);
//! let create = create.start()?;          // Off → Passive
//! let create = create.to_safe()?;        // Passive → Safe
//! ```
//!
//! ## Quick Start (async / tokio)
//!
//! ```rust,ignore
//! use create_oi::prelude::*;
//! use create_oi_tokio::TokioTransport;
//!
//!
//! let transport = TokioTransport::open("/dev/ttyUSB0", RobotModel::Create2)?;
//! let create = AsyncCreate::new(transport, RobotModel::Create2);
//! let create = create.start().await?;    // Off → Passive
//! let create = create.to_safe().await?;  // Passive → Safe
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
// TransitionError/ConnectError intentionally store the Create/transport handle
// for recovery, making them large. This is by design.
#![allow(clippy::result_large_err)]

pub mod async_create;
#[cfg(feature = "std")]
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
    #[cfg(feature = "std")]
    pub use crate::create::Create;
    pub use crate::error::{ConnectError, Error, TransitionError};
    pub use crate::mode::{
        Actuatable, Full, FullControl, Mode, Off, Passive, Safe, SensorReadable,
    };
    pub use crate::transport::AsyncBaudConfigurable;
    pub use crate::transport::AsyncTransport;
    #[cfg(feature = "std")]
    pub use crate::transport::BaudConfigurable;
    #[cfg(feature = "std")]
    pub use crate::transport::Transport;
    pub use crate::types::{
        AngularVelocity, ButtonBits, CleanMode, CurveRadius, DayOfWeek, LedIntensity, MotorBits,
        MotorPower, PowerLedColor, Radius, RobotModel, SongNote, SongNumber, Velocity,
    };

    // Selective protocol re-exports
    pub use create_oi_protocol::opcode::Opcode;
    pub use create_oi_protocol::sensor::SensorData;
    pub use create_oi_protocol::stream::StreamParser;
    pub use create_oi_protocol::types::{BaudRate, OiMode};
}
