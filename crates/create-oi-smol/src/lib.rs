//! Smol async transport for the Create/Roomba.
//!
//! Uses [`smol::Unblock`] to wrap the native serial port for non-blocking
//! async I/O on the smol runtime.  Blocking I/O is dispatched to a thread
//! pool, keeping the executor free.
//!
//! | Platform | Native type |
//! |----------|-------------|
//! | Unix     | `serialport::TTYPort` |
//! | Windows  | `serialport::COMPort` |
//!
//! Other platforms are not supported.

#[cfg(windows)]
use serialport::COMPort as NativePort;
#[cfg(unix)]
use serialport::TTYPort as NativePort;
#[cfg(not(any(unix, windows)))]
compile_error!("create-oi-smol requires Unix or Windows; other platforms are not yet supported");

use std::io;
use std::time::Duration;

use create_oi::protocol::types::BaudRate;
use create_oi::transport::{AsyncBaudConfigurable, AsyncTransport};
use create_oi::types::RobotModel;
use smol::Unblock;
use smol::io::{AsyncReadExt, AsyncWriteExt};

/// Re-export core types for convenience.
pub use create_oi;

/// Async transport for the smol runtime.
///
/// Wraps a native serial port in [`smol::Unblock`], which runs
/// blocking serial I/O on a thread pool without blocking the async executor.
#[derive(Debug)]
pub struct SmolTransport {
    port: Unblock<NativePort>,
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

    async fn delay(&mut self, duration: Duration) {
        smol::Timer::after(duration).await;
    }
}

impl AsyncBaudConfigurable for SmolTransport {
    async fn set_baud(&mut self, rate: BaudRate) -> Result<(), io::Error> {
        use serialport::SerialPort;
        // get_mut() waits until in-flight thread-pool operations complete and returns &mut NativePort.
        let port = self.port.get_mut().await;
        port.set_baud_rate(rate.baud_u32())
            .map_err(io::Error::other)
    }
}
