//! Smol async transport for the robot.
//!
//! Uses `async-io` to wrap the synchronous `serialport` handle for
//! non-blocking operation on the smol runtime.

use std::io;

use crate::types::RobotModel;

/// Async transport for the smol runtime.
///
/// This wraps a `serialport` file descriptor with `async-io::Async`
/// for non-blocking I/O on the smol executor.
#[derive(Debug)]
pub struct SmolTransport {
    #[expect(
        dead_code,
        reason = "stub: will be used once fd extraction is implemented"
    )]
    port: async_io::Async<crate::io::serial::SerialTransport>,
}

impl SmolTransport {
    /// Open a serial port for the given model.
    ///
    /// # Note
    /// This currently requires `unsafe` to extract the raw fd from the serial port.
    /// We gate behind a feature flag and document this caveat.
    pub fn open(path: &str, model: RobotModel) -> io::Result<Self> {
        let _inner = crate::io::serial::SerialTransport::open(path, model)?;
        // async-io::Async requires the inner type to implement AsRawFd (Unix)
        // or AsRawSocket (Windows). Since SerialTransport wraps Box<dyn SerialPort>,
        // we cannot directly satisfy this. This is a known limitation.
        //
        // For now, we provide a stub. A production implementation would need to
        // either extract the fd from serialport or use a different approach.
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "SmolTransport requires OS-specific fd extraction; not yet implemented",
        ))
    }
}
