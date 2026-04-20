//! Tokio async transport for the robot.

use std::io;

use crate::transport::AsyncTransport;
use crate::types::RobotModel;

/// Async transport backed by `tokio-serial`.
#[derive(Debug)]
pub struct TokioTransport {
    port: tokio_serial::SerialStream,
}

impl TokioTransport {
    /// Open a serial port for the given model using tokio-serial.
    pub fn open(path: &str, model: RobotModel) -> io::Result<Self> {
        let builder = tokio_serial::new(path, model.baud())
            .data_bits(tokio_serial::DataBits::Eight)
            .parity(tokio_serial::Parity::None)
            .stop_bits(tokio_serial::StopBits::One)
            .flow_control(tokio_serial::FlowControl::None);
        let port = tokio_serial::SerialStream::open(&builder)?;
        Ok(Self { port })
    }
}

impl AsyncTransport for TokioTransport {
    async fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        tokio::io::AsyncWriteExt::write_all(&mut self.port, data).await
    }

    async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        tokio::io::AsyncReadExt::read(&mut self.port, buf).await
    }

    async fn flush(&mut self) -> io::Result<()> {
        tokio::io::AsyncWriteExt::flush(&mut self.port).await
    }

    async fn close(&mut self) -> io::Result<()> {
        tokio::io::AsyncWriteExt::shutdown(&mut self.port).await
    }

    async fn sleep(&self, duration: std::time::Duration) {
        tokio::time::sleep(duration).await;
    }
}
