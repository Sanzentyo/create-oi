//! Transport layer traits.
//!
//! These traits abstract over the physical connection to the robot.
//! They use associated error types for `no_std` compatibility.
//!
//! - [`AsyncTransport`] — async I/O + timer (works with Embassy, tokio, smol)
//! - [`Transport`] — blocking I/O (requires `std` feature)
//!
//! Concrete implementations live in their own crates:
//! - `create-oi-serial` — [`Transport`] via `serialport`
//! - `create-oi-tokio` — [`AsyncTransport`] via `tokio-serial`
//! - `create-oi-smol` — [`AsyncTransport`] via `smol` + `async-io`
//! - `create-oi-embassy` — [`AsyncTransport`] via Embassy UART

use core::fmt;
use core::time::Duration;

use create_oi_protocol::types::BaudRate;

/// Asynchronous transport for communicating with the robot.
///
/// This trait bundles async read/write/flush plus a timer abstraction.
/// It intentionally does **not** require `Send` — Embassy peripherals
/// are often `!Send` and pinned to a single executor.
///
/// # Cancellation safety
///
/// Async methods on this trait are **not** guaranteed to be cancellation-safe.
/// If a future returned by `write_all` or `read` is dropped mid-execution,
/// the transport may be left in an inconsistent state.
#[allow(async_fn_in_trait)] // Stable in edition 2024; no dyn dispatch needed here.
pub trait AsyncTransport: fmt::Debug {
    /// The error type for I/O operations.
    type Error: fmt::Debug + fmt::Display;

    /// Write all bytes to the transport.
    ///
    /// Implementations MUST submit all bytes into the transport's transmit
    /// path before returning, without requiring a subsequent
    /// [`flush`](AsyncTransport::flush) call to make progress.
    /// A following [`read`](AsyncTransport::read) call MUST be able to
    /// receive a response to the written bytes without an intervening flush.
    ///
    /// `flush()` is for waiting until hardware transmit buffers have drained
    /// (e.g. `tcdrain`), not for enabling basic request–response correctness.
    /// Implementations MUST NOT hold bytes back indefinitely.
    async fn write_all(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    /// Read available bytes into `buf`. Returns the number of bytes read.
    /// Must return at least 1 byte on success (0 indicates EOF/disconnect).
    ///
    /// Implementations **must not** propagate transport-internal idle timeouts
    /// (e.g. OS-level serial read timeouts) as errors; those should be retried
    /// transparently. Only genuine I/O errors should be returned.
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    /// Flush the output buffer, ensuring all written bytes are sent.
    async fn flush(&mut self) -> Result<(), Self::Error>;

    /// Sleep for the given duration using the runtime's timer.
    ///
    /// This abstracts over `tokio::time::sleep` / `smol::Timer::after` /
    /// `embassy_time::Timer::after` so that protocol-level delays
    /// (e.g. mode-change waits) don't depend on a specific runtime.
    async fn delay(&mut self, duration: Duration);
}

/// Synchronous (blocking) transport for communicating with the robot.
///
/// This trait is only available with the `std` feature. For embedded
/// targets, use [`AsyncTransport`] instead.
///
/// # Closing
///
/// This trait does **not** include a `close` method. Transports are closed
/// when dropped. Concrete types may provide their own consuming `close(self)`
/// method for explicit, fallible shutdown with flush.
#[cfg(feature = "std")]
pub trait Transport: fmt::Debug + Send {
    /// Write all bytes to the transport.
    ///
    /// Implementations MUST submit all bytes into the transport's transmit
    /// path before returning, without requiring a subsequent
    /// [`flush`](Transport::flush) call to make progress.
    /// A following [`read`](Transport::read) call MUST be able to
    /// receive a response to the written bytes without an intervening flush.
    ///
    /// `flush()` is for waiting until hardware transmit buffers have drained
    /// (e.g. `tcdrain`), not for enabling basic request–response correctness.
    /// Implementations MUST NOT hold bytes back indefinitely.
    fn write_all(&mut self, data: &[u8]) -> std::io::Result<()>;

    /// Read available bytes into `buf`. Returns the number of bytes read.
    /// Must return at least 1 byte on success (0 indicates EOF/disconnect).
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;

    /// Flush the output buffer.
    fn flush(&mut self) -> std::io::Result<()>;

    /// Set the read timeout. `None` means blocking forever.
    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()>;
}

/// Extension trait for [`Transport`] implementations that support runtime baud-rate switching.
///
/// Implement this alongside [`Transport`] if your serial driver supports changing
/// the baud rate after the port is opened. Transports that do not implement this trait
/// cannot use the [`Create::baud`](crate::create::Create::baud) command.
///
/// # Protocol sequence
///
/// 1. Send OI `BAUD` opcode + baud code byte.
/// 2. Wait 100 ms (the robot switches baud rate after this delay).
/// 3. Call `set_baud()` to reconfigure the host serial port.
#[cfg(feature = "std")]
pub trait BaudConfigurable: Transport {
    /// Reconfigure the serial connection to the given baud rate.
    fn set_baud(&mut self, rate: BaudRate) -> std::io::Result<()>;
}

/// Extension trait for [`AsyncTransport`] implementations that support runtime baud-rate switching.
///
/// Implement this alongside [`AsyncTransport`] if your async serial driver supports changing
/// the baud rate after the connection is opened. Transports that do not implement this trait
/// cannot use the [`AsyncCreate::baud`](crate::async_create::AsyncCreate::baud) command.
#[allow(async_fn_in_trait)]
pub trait AsyncBaudConfigurable: AsyncTransport {
    /// Reconfigure the serial connection to the given baud rate.
    async fn set_baud(&mut self, rate: BaudRate) -> Result<(), Self::Error>;
}
