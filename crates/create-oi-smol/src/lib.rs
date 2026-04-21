//! Smol async transport for the Create/Roomba.
//!
//! Uses [`smol::Unblock`] to wrap the native `serialport::TTYPort` for
//! non-blocking async I/O on the smol runtime. Blocking I/O is dispatched
//! to a thread pool, keeping the executor free.
//!
//! # Platform Support
//!
//! `serialport::TTYPort` is Unix-only. On non-Unix platforms this module
//! is empty and `SmolTransport` is not available.

#![cfg(unix)]

use std::io;
use std::time::Duration;

use create_oi::transport::AsyncTransport;
use create_oi::types::RobotModel;
use smol::Unblock;
use smol::io::{AsyncReadExt, AsyncWriteExt};

/// Re-export core types for convenience.
pub use create_oi;

/// Async transport for the smol runtime.
///
/// Wraps a native `serialport::TTYPort` in [`smol::Unblock`], which runs
/// blocking serial I/O on a thread pool without blocking the async executor.
#[derive(Debug)]
pub struct SmolTransport {
    port: Unblock<serialport::TTYPort>,
}

impl SmolTransport {
    /// Open a serial port for the given robot model.
    ///
    /// Uses the native baud rate for the model:
    /// - Create 2: 115200
    /// - Create 1: 57600
    pub fn open(path: &str, model: RobotModel) -> io::Result<Self> {
        Self::open_with_baud(path, model.baud())
    }

    /// Open a serial port with a custom baud rate.
    pub fn open_with_baud(path: &str, baud: u32) -> io::Result<Self> {
        let port = serialport::new(path, baud)
            .timeout(Duration::from_millis(100))
            .open_native()
            .map_err(io::Error::other)?;
        Ok(Self {
            port: Unblock::new(port),
        })
    }
}

impl AsyncTransport for SmolTransport {
    type Error = io::Error;

    async fn write_all(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.port.write_all(data).await
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.port.read(buf).await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.port.flush().await
    }

    async fn delay(&self, duration: Duration) {
        smol::Timer::after(duration).await;
    }
}
