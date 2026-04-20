//! Error types for the create-oi control layer.
//!
//! This module defines high-level errors that combine I/O failures,
//! protocol errors (from `create-oi-protocol`), and domain validation errors.

use create_oi_protocol::error::ProtocolError;
use thiserror::Error;

/// Errors that can occur when interacting with the robot.
#[derive(Debug, Error)]
pub enum Error {
    /// An underlying I/O operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A protocol-level error (checksum, parse, malformed data).
    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    /// A value was invalid for its domain type.
    #[error("invalid value for {field}: {reason}")]
    InvalidValue { field: &'static str, reason: String },

    /// The actual OI mode on the hardware does not match the expected TypeState.
    #[error("mode mismatch: expected {expected}, actual {actual}")]
    ModeMismatch {
        expected: &'static str,
        actual: &'static str,
    },

    /// Connection to the robot failed.
    #[error("connection failed: {0}")]
    Connection(String),

    /// The robot is not connected.
    #[error("robot not connected")]
    NotConnected,
}

/// Error returned when a mode transition fails, preserving the robot
/// so the caller can recover.
#[derive(Debug)]
pub struct TransitionError<R> {
    /// The robot, returned in its original mode.
    pub robot: R,
    /// The underlying error.
    pub source: Error,
}

impl<R> std::fmt::Display for TransitionError<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mode transition failed: {}", self.source)
    }
}

impl<R: std::fmt::Debug> std::error::Error for TransitionError<R> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Error returned when `connect()` fails, preserving the transport
/// so the caller can retry or reuse it.
#[derive(Debug)]
pub struct ConnectError<T> {
    /// The transport handle, returned to the caller.
    pub transport: T,
    /// The underlying error.
    pub source: Error,
}

impl<T> std::fmt::Display for ConnectError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "connect failed: {}", self.source)
    }
}

impl<T: std::fmt::Debug> std::error::Error for ConnectError<T> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}
