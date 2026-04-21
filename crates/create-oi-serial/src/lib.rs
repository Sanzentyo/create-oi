//! Serial port transport using the `serialport` crate.
//!
//! Provides [`SerialTransport`], a synchronous [`Transport`] implementation
//! for communicating with iRobot Create / Roomba robots over a serial port.

use std::io;
use std::time::Duration;

use create_oi::protocol::types::BaudRate;
use create_oi::transport::{BaudConfigurable, Transport};
use create_oi::types::RobotModel;

/// Re-export core types for convenience.
pub use create_oi;

/// Synchronous serial port transport.
#[derive(Debug)]
pub struct SerialTransport {
    port: Box<dyn serialport::SerialPort>,
}

impl SerialTransport {
    /// Open a serial port with settings appropriate for the given robot model.
    pub fn open(path: &str, model: RobotModel) -> io::Result<Self> {
        let port = serialport::new(path, model.baud())
            .data_bits(serialport::DataBits::Eight)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .flow_control(serialport::FlowControl::None)
            .timeout(Duration::from_secs(1))
            .open()?;
        Ok(Self { port })
    }

    /// Open a serial port with a custom baud rate.
    pub fn open_with_baud(path: &str, baud: u32) -> io::Result<Self> {
        let port = serialport::new(path, baud)
            .data_bits(serialport::DataBits::Eight)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .flow_control(serialport::FlowControl::None)
            .timeout(Duration::from_secs(1))
            .open()?;
        Ok(Self { port })
    }

    /// Flush pending output and explicitly close the port.
    ///
    /// This is an inherent consuming method for callers that want explicit,
    /// fallible shutdown. Dropping the transport also closes the port via
    /// `Drop`, but without the opportunity to observe a flush error.
    pub fn close(mut self) -> io::Result<()> {
        io::Write::flush(&mut self.port)
    }
}

impl Transport for SerialTransport {
    fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        io::Write::write_all(&mut self.port, data)
    }

    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        io::Read::read(&mut self.port, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        io::Write::flush(&mut self.port)
    }

    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.port
            .set_timeout(timeout.unwrap_or(Duration::from_secs(u64::MAX)))
            .map_err(io::Error::other)
    }
}

impl BaudConfigurable for SerialTransport {
    fn set_baud(&mut self, rate: BaudRate) -> io::Result<()> {
        self.port
            .set_baud_rate(rate.baud_u32())
            .map_err(io::Error::other)
    }
}
