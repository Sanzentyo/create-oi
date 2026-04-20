//! Error types for the create-oi control layer.
//!
//! This module defines high-level errors that combine transport failures,
//! protocol errors (from `create-oi-protocol`), and domain validation errors.
//!
//! The main [`Error<E>`] type is generic over the transport's error type `E`,
//! allowing it to work with both `std::io::Error` and embedded HAL errors.

use core::fmt;
use create_oi_protocol::error::ProtocolError;

/// A domain validation error (independent of transport).
///
/// Used by newtype constructors (`Velocity::new`, `Radius::new`, etc.)
/// and `TryFrom` impls where no I/O is involved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// The field/type that failed validation.
    pub field: &'static str,
    /// A human-readable reason.
    pub reason: &'static str,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid value for {}: {}", self.field, self.reason)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ValidationError {}

/// Errors that can occur when interacting with the robot.
///
/// `E` is the transport's error type — for `std` users this is typically
/// `std::io::Error`, for embedded targets it is whatever the HAL provides.
#[derive(Debug)]
pub enum Error<E> {
    /// An underlying transport I/O operation failed.
    Io(E),

    /// A protocol-level error (checksum, parse, malformed data).
    Protocol(ProtocolError),

    /// A value was invalid for its domain type.
    Validation(ValidationError),
}

impl<E: fmt::Display> fmt::Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Protocol(e) => write!(f, "{e}"),
            Self::Validation(e) => write!(f, "{e}"),
        }
    }
}

#[cfg(feature = "std")]
impl<E: std::error::Error + 'static> std::error::Error for Error<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Protocol(e) => Some(e),
            Self::Validation(e) => Some(e),
        }
    }
}

impl<E> From<ProtocolError> for Error<E> {
    fn from(e: ProtocolError) -> Self {
        Self::Protocol(e)
    }
}

impl<E> From<ValidationError> for Error<E> {
    fn from(e: ValidationError) -> Self {
        Self::Validation(e)
    }
}

/// Type alias for errors with `std::io::Error` as the transport error.
#[cfg(feature = "std")]
pub type StdError = Error<std::io::Error>;

/// Error returned when a mode transition fails, preserving the robot
/// so the caller can recover.
#[derive(Debug)]
pub struct TransitionError<R, E> {
    /// The robot, returned in its original mode.
    pub robot: R,
    /// The underlying error.
    pub source: Error<E>,
}

impl<R, E: fmt::Display> fmt::Display for TransitionError<R, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mode transition failed: {}", self.source)
    }
}

#[cfg(feature = "std")]
impl<R: fmt::Debug, E: std::error::Error + 'static> std::error::Error for TransitionError<R, E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Error returned when `connect()` fails, preserving the transport
/// so the caller can retry or reuse it.
#[derive(Debug)]
pub struct ConnectError<T, E> {
    /// The transport handle, returned to the caller.
    pub transport: T,
    /// The underlying error.
    pub source: Error<E>,
}

impl<T, E: fmt::Display> fmt::Display for ConnectError<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "connect failed: {}", self.source)
    }
}

#[cfg(feature = "std")]
impl<T: fmt::Debug, E: std::error::Error + 'static> std::error::Error for ConnectError<T, E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}
