//! Error types for the libcreate crate.

use thiserror::Error;

/// Main error type for libcreate operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to create the robot handle (allocation or C++ exception).
    #[error("failed to create robot handle")]
    HandleCreationFailed,

    /// Failed to connect to the robot over serial.
    #[error("connection failed on port `{port}`")]
    ConnectionFailed { port: String },

    /// The robot is not currently connected.
    #[error("robot is not connected")]
    NotConnected,

    /// A command sent to the robot failed at the FFI layer.
    #[error("command failed")]
    CommandFailed,

    /// The actual OI mode on the hardware does not match the expected TypeState.
    #[error("mode mismatch: expected {expected}, actual {actual}")]
    ModeMismatch {
        expected: &'static str,
        actual: &'static str,
    },

    /// A value was out of the valid range.
    #[error("value {value} out of range [{min}, {max}]")]
    OutOfRange { value: f32, min: f32, max: f32 },

    /// A floating-point value was NaN or infinite.
    #[error("value must be finite (got {0})")]
    NotFinite(f32),
}

/// Error returned when a mode transition fails, preserving the robot in its
/// original state so the caller can recover.
#[derive(Debug)]
pub struct TransitionError<M: crate::mode::Mode> {
    /// The robot, returned in its original mode.
    pub robot: crate::robot::Robot<M>,
    /// The underlying error.
    pub error: Error,
}

impl<M: crate::mode::Mode> std::fmt::Display for TransitionError<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mode transition failed: {}", self.error)
    }
}

impl<M: crate::mode::Mode> std::error::Error for TransitionError<M> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}
