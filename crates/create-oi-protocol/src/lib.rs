//! # create-oi-protocol
//!
//! A **pure Sans-IO** implementation of the iRobot Create / Roomba
//! [Open Interface (OI)](https://www.irobot.com/about-irobot/stem/create-2)
//! wire protocol.
//!
//! This crate handles:
//! - **Command encoding** — OI commands → fixed-size byte arrays
//! - **Sensor decoding** — raw `&[u8]` → structured [`SensorData`](sensor::SensorData)
//! - **Stream framing** — byte-wise state machine for sensor stream parsing
//!
//! It has **zero I/O dependencies** — all functions operate on plain byte slices.
//! Suitable for embedded, `no_std`, or any environment.
//!
//! ## Features
//!
//! - `std` (default) — enables `std::error::Error` impls and `alloc`
//! - `alloc` — enables `Vec`-returning convenience APIs
//! - Neither — fully `no_std`, no heap: only buffer-based APIs
//!
//! ## Usage
//!
//! ```rust
//! use create_oi_protocol::command;
//! use create_oi_protocol::opcode::Opcode;
//!
//! // Encode a "start" command
//! let bytes = command::encode_start();
//! assert_eq!(bytes, [Opcode::Start as u8]);
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod command;
pub mod error;
pub mod opcode;
pub mod sensor;
pub mod stream;
pub mod types;

/// Convenience re-exports of commonly used protocol items.
pub mod prelude {
    pub use crate::error::ProtocolError;
    pub use crate::opcode::Opcode;
    pub use crate::sensor::SensorData;
    pub use crate::stream::StreamParser;
    pub use crate::types::{ChargingState, IrChar, OiMode};
}
