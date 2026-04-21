//! Protocol-level error types.
//!
//! These errors represent failures in encoding/decoding the OI wire format,
//! independent of any I/O or transport concerns.

use core::fmt;

/// Errors from protocol encoding/decoding operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    /// Not enough bytes to decode a sensor packet or stream frame.
    InsufficientData {
        /// Minimum bytes required.
        need: usize,
        /// Bytes actually available.
        got: usize,
    },

    /// A stream frame checksum did not match.
    Checksum {
        /// Expected checksum value.
        expected: u8,
        /// Actual computed checksum value.
        actual: u8,
    },

    /// An unrecognised sensor packet ID was encountered.
    UnknownPacketId(u8),

    /// A required sensor field was not present in the decoded data.
    MissingSensorField {
        /// Human-readable name of the missing field.
        field: &'static str,
    },

    /// The provided buffer was too small for the encoded output.
    BufferTooSmall {
        /// Minimum buffer size required.
        need: usize,
        /// Actual buffer size provided.
        got: usize,
    },

    /// The input contains more items than the OI protocol allows.
    TooManyItems {
        /// Maximum allowed number of items.
        max: usize,
        /// Number of items provided.
        got: usize,
    },
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientData { need, got } => {
                write!(f, "insufficient data: need {need} bytes, got {got}")
            }
            Self::Checksum { expected, actual } => {
                write!(
                    f,
                    "checksum mismatch: expected {expected:#04x}, got {actual:#04x}"
                )
            }
            Self::UnknownPacketId(id) => {
                write!(f, "unknown packet id: {id}")
            }
            Self::MissingSensorField { field } => {
                write!(f, "missing sensor field: {field}")
            }
            Self::BufferTooSmall { need, got } => {
                write!(f, "buffer too small: need {need} bytes, got {got}")
            }
            Self::TooManyItems { max, got } => {
                write!(f, "too many items: max {max}, got {got}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ProtocolError {}
