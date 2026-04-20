//! Protocol-level error types.
//!
//! These errors represent failures in encoding/decoding the OI wire format,
//! independent of any I/O or transport concerns.

use thiserror::Error;

/// Errors from protocol encoding/decoding operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    /// Not enough bytes to decode a sensor packet or stream frame.
    #[error("insufficient data: need {need} bytes, got {got}")]
    InsufficientData { need: usize, got: usize },

    /// A stream frame checksum did not match.
    #[error("checksum mismatch: expected {expected:#04x}, got {actual:#04x}")]
    Checksum { expected: u8, actual: u8 },

    /// A protocol-level invariant was violated.
    #[error("protocol error: {0}")]
    Protocol(String),
}
