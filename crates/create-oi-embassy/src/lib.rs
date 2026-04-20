//! # create-oi-embassy
//!
//! Embassy async transport adapter for the [`create-oi`] robot control library.
//!
//! This crate provides [`EmbassyTransport`], a thin wrapper that implements
//! [`AsyncTransport`](create_oi::transport::AsyncTransport) for any Embassy
//! peripheral (or HAL type) that implements [`embedded_io_async::Read`] +
//! [`embedded_io_async::Write`].
//!
//! # Usage
//!
//! ```rust,ignore
//! use create_oi::prelude::*;
//! use create_oi_embassy::EmbassyTransport;
//!
//! // `uart` is e.g. embassy_stm32::usart::Uart<'_, Async>
//! let transport = EmbassyTransport::new(uart);
//! let robot = AsyncCreate::new(transport, CreateRobotModel::Create2);
//! let robot = robot.start().await.unwrap();
//! ```
//!
//! # Baud Rate
//!
//! Configure the UART baud rate (57600 for Create 1, 115200 for Create 2)
//! **before** passing it to [`EmbassyTransport::new`]. This crate does not
//! perform baud-rate configuration.

#![no_std]

use core::fmt;
use core::time::Duration;

use create_oi::transport::AsyncTransport;
use embassy_time::Timer;
use embedded_io_async::{Read, Write};

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
