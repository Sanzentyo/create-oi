//! # create-oi-embassy
//!
//! Embassy async transport adapter for the [`create-oi`] robot control library.
//!
//! This crate provides two adapters:
//!
//! - [`EmbassyTransport`] — wraps a combined read+write UART peripheral.
//! - [`EmbassySplitTransport`] — wraps separately-owned RX and TX halves
//!   (useful when the UART is split via `uart.split()`).
//!
//! Both implement [`AsyncTransport`](create_oi::transport::AsyncTransport).
//!
//! # Usage — combined UART
//!
//! ```rust,ignore
//! use create_oi::prelude::*;
//! use create_oi_embassy::EmbassyTransport;
//!
//! // `uart` is e.g. embassy_stm32::usart::Uart<'_, Async>
//! let transport = EmbassyTransport::new(uart);
//! let robot = AsyncCreate::new(transport, RobotModel::Create2);
//! let robot = robot.start().await.unwrap();
//! ```
//!
//! # Usage — split UART
//!
//! ```rust,ignore
//! use create_oi::prelude::*;
//! use create_oi_embassy::EmbassySplitTransport;
//!
//! // split an Embassy UART into (rx, tx) halves
//! let (rx, tx) = uart.split();
//! let transport = EmbassySplitTransport::new(rx, tx);
//! let robot = AsyncCreate::new(transport, RobotModel::Create2);
//! let robot = robot.start().await.unwrap();
//! ```
//!
//! # Baud Rate
//!
//! Configure the UART baud rate (57600 for Create 1, 115200 for Create 2)
//! **before** passing the peripheral to the transport. This crate does not
//! perform baud-rate configuration.

#![no_std]

use core::fmt;
use core::time::Duration;

use create_oi::transport::AsyncTransport;
use embassy_time::Timer;
use embedded_io_async::{ErrorType, Read, Write};

/// Embassy async transport adapter.
///
/// Wraps any Embassy peripheral implementing [`embedded_io_async::Read`] +
/// [`embedded_io_async::Write`] and uses [`embassy_time::Timer`] for delays.
///
/// The generic type `T` is typically a UART peripheral from an Embassy HAL,
/// e.g. `embassy_stm32::usart::Uart<'_, Async>` or
/// `embassy_rp::uart::BufferedUart<'_, UART0>`.
pub struct EmbassyTransport<T> {
    io: T,
}

impl<T: fmt::Debug> fmt::Debug for EmbassyTransport<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmbassyTransport")
            .field("io", &self.io)
            .finish()
    }
}

impl<T> EmbassyTransport<T> {
    /// Create a new Embassy transport wrapping the given I/O peripheral.
    ///
    /// The peripheral must already be configured with the correct baud rate
    /// (57600 for Create 1, 115200 for Create 2).
    pub fn new(io: T) -> Self {
        Self { io }
    }

    /// Consume the adapter and return the inner I/O peripheral.
    pub fn into_inner(self) -> T {
        self.io
    }

    /// Borrow the inner I/O peripheral.
    pub fn inner(&self) -> &T {
        &self.io
    }

    /// Mutably borrow the inner I/O peripheral.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.io
    }
}

impl<T> AsyncTransport for EmbassyTransport<T>
where
    T: Read + Write + fmt::Debug,
{
    type Error = T::Error;

    async fn write_all(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        <T as Write>::write_all(&mut self.io, data).await
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        <T as Read>::read(&mut self.io, buf).await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        <T as Write>::flush(&mut self.io).await
    }

    async fn delay(&self, duration: Duration) {
        // Clamp to u64::MAX micros to avoid truncation on large durations.
        // In practice OI delays are 100-300ms, so this never triggers.
        let micros = u64::try_from(duration.as_micros()).unwrap_or(u64::MAX);
        Timer::after(embassy_time::Duration::from_micros(micros)).await;
    }
}

/// Embassy split-UART transport adapter.
///
/// Use this when your Embassy UART peripheral exposes separate read and write
/// halves, for example after calling `uart.split()` on
/// `embassy_stm32::usart::Uart` or `embassy_rp::uart::Uart`.
///
/// Both halves must share the same error type, which is always true for any
/// standard Embassy UART.
///
/// # Type Parameters
///
/// - `R` — the read (RX) half; must implement [`embedded_io_async::Read`].
/// - `W` — the write (TX) half; must implement [`embedded_io_async::Write`].
/// - `E` — the shared error type (inferred from `R` and `W`).
pub struct EmbassySplitTransport<R, W> {
    rx: R,
    tx: W,
}

impl<R: fmt::Debug, W: fmt::Debug> fmt::Debug for EmbassySplitTransport<R, W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmbassySplitTransport")
            .field("rx", &self.rx)
            .field("tx", &self.tx)
            .finish()
    }
}

impl<R, W> EmbassySplitTransport<R, W> {
    /// Create a new split transport from separate RX and TX halves.
    pub fn new(rx: R, tx: W) -> Self {
        Self { rx, tx }
    }

    /// Consume the adapter and return the (rx, tx) halves.
    pub fn into_parts(self) -> (R, W) {
        (self.rx, self.tx)
    }

    /// Borrow the RX half.
    pub fn rx(&self) -> &R {
        &self.rx
    }

    /// Borrow the TX half.
    pub fn tx(&self) -> &W {
        &self.tx
    }

    /// Mutably borrow the RX half.
    pub fn rx_mut(&mut self) -> &mut R {
        &mut self.rx
    }

    /// Mutably borrow the TX half.
    pub fn tx_mut(&mut self) -> &mut W {
        &mut self.tx
    }
}

impl<R, W, E> AsyncTransport for EmbassySplitTransport<R, W>
where
    R: ErrorType<Error = E> + Read + fmt::Debug,
    W: ErrorType<Error = E> + Write + fmt::Debug,
    E: fmt::Debug + fmt::Display,
{
    type Error = E;

    async fn write_all(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        <W as Write>::write_all(&mut self.tx, data).await
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        <R as Read>::read(&mut self.rx, buf).await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        <W as Write>::flush(&mut self.tx).await
    }

    async fn delay(&self, duration: Duration) {
        let micros = u64::try_from(duration.as_micros()).unwrap_or(u64::MAX);
        Timer::after(embassy_time::Duration::from_micros(micros)).await;
    }
}
