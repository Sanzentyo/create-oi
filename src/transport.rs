//! Transport layer traits.
//!
//! These traits abstract over the physical connection to the robot.
//! The sync [`Transport`] trait uses `std::io::Read + Write`,
//! and the [`AsyncTransport`] trait uses `futures_io::AsyncRead + AsyncWrite`
//! (feature-gated behind `tokio-runtime` or `smol-runtime`).

use std::io;
use std::time::Duration;

/// Synchronous transport for communicating with the robot.
pub trait Transport: std::fmt::Debug + Send {
    /// Write all bytes to the transport.
    fn write_all(&mut self, data: &[u8]) -> io::Result<()>;

    /// Read available bytes into `buf`. Returns the number of bytes read.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;

    /// Flush the output buffer.
    fn flush(&mut self) -> io::Result<()>;

    /// Set the read timeout. `None` means blocking forever.
    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()>;

    /// Close the transport. After this, reads/writes should return errors.
    fn close(&mut self) -> io::Result<()>;
}

/// Asynchronous transport for communicating with the robot.
///
/// Requires the `tokio-runtime` or `smol-runtime` feature.
#[cfg(any(feature = "tokio-runtime", feature = "smol-runtime"))]
#[allow(async_fn_in_trait)]
pub trait AsyncTransport: std::fmt::Debug + Send {
    /// Write all bytes to the transport.
    async fn write_all(&mut self, data: &[u8]) -> io::Result<()>;

    /// Read available bytes into `buf`. Returns the number of bytes read.
    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;

    /// Flush the output buffer.
    async fn flush(&mut self) -> io::Result<()>;

    /// Close the transport.
    async fn close(&mut self) -> io::Result<()>;
}
