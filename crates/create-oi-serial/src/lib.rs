//! Serial port transport using the `serialport` crate.
//!
//! Provides [`SerialTransport`], a synchronous [`Transport`] implementation
//! for communicating with iRobot Create / Roomba robots over a serial port.

use std::io;
use std::time::Duration;

use create_oi::transport::Transport;
use create_oi::types::RobotModel;

/// Re-export core types for convenience.
pub use create_oi;

/// Synchronous serial port transport.
#[derive(Debug)]
pub struct SerialTransport {
    port: Option<Box<dyn serialport::SerialPort>>,
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
        Ok(Self { port: Some(port) })
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
        Ok(Self { port: Some(port) })
    }

    fn port_mut(&mut self) -> io::Result<&mut Box<dyn serialport::SerialPort>> {
        self.port
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "serial port is closed"))
    }
}

impl Transport for SerialTransport {
    fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        io::Write::write_all(self.port_mut()?, data)
    }

    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        io::Read::read(self.port_mut()?, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.port.as_mut() {
            Some(p) => io::Write::flush(p),
            None => Ok(()),
        }
    }

    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.port_mut()?
            .set_timeout(timeout.unwrap_or(Duration::from_secs(u64::MAX)))
            .map_err(io::Error::other)
    }

    /// Close the transport.
    ///
    /// Flushes pending output and drops the port handle (closing the OS file
    /// descriptor). After this call, `read` and `write_all` return
    /// [`io::ErrorKind::NotConnected`]. Calling `close` again is a no-op.
    ///
    /// Note: if the flush fails, the port is still closed and the error is
    /// returned, but subsequent calls will see `NotConnected` regardless.
    fn close(&mut self) -> io::Result<()> {
        if let Some(mut p) = self.port.take() {
            io::Write::flush(&mut p)?;
        }
        Ok(())
    }
}
