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
//!
//! # LEDs
//!
//! The following snippets assume you are inside an `#[embassy_executor::main]`
//! task with `robot: AsyncCreate<Safe, EmbassyTransport<...>>` already in
//! scope.
//!
//! ```rust,ignore
//! // All status LEDs on, power LED red at full brightness
//! robot.set_leds(true, true, true, true, PowerLedColor::RED, LedIntensity::new(255)).await?;
//!
//! // Day-of-week and schedule icon LEDs (Create 2 only)
//! // day_leds: bits 0-6 = Sun-Sat; schedule_leds: bit0=colon, bit1=AM/PM
//! robot.set_scheduling_leds(0b000_1010, 0b0000_0011).await?; // Mon + Wed + colon + AM/PM
//! robot.set_scheduling_leds(0, 0).await?; // clear
//!
//! // Four-digit ASCII display (Create 2 only)
//! robot.set_digit_leds(b'O', b'I', b' ', b' ').await?;
//!
//! // Four-digit raw segment bits — each byte encodes segments A-G (bits 0-6)
//! robot.set_digit_leds_raw(0x7F, 0x7F, 0x7F, 0x7F).await?; // all segments on
//! robot.set_digit_leds_raw(0, 0, 0, 0).await?; // clear
//! ```
//!
//! # Songs
//!
//! ```rust,ignore
//! // Define song 0: C major scale (MIDI 60-72, 0.5 s per note at 32/64 s)
//! let scale = [
//!     SongNote::new(60, 32)?, SongNote::new(62, 32)?, SongNote::new(64, 32)?,
//!     SongNote::new(65, 32)?, SongNote::new(67, 32)?, SongNote::new(69, 32)?,
//!     SongNote::new(71, 32)?, SongNote::new(72, 32)?,
//! ];
//! robot.define_song(SongNumber::new(0)?, &scale).await?;
//! robot.play_song(SongNumber::new(0)?).await?;
//! ```
//!
//! # Full Mode
//!
//! ```rust,ignore
//! // Transition Safe → Full (no safety cutoffs)
//! let mut robot = robot.to_full().await.map_err(|e| e.source)?;
//!
//! // Per-wheel velocity: right 0.15 m/s, left 0.08 m/s → gentle left arc
//! robot.drive_direct(Velocity::new(0.15)?, Velocity::new(0.08)?).await?;
//!
//! // cmd_vel-style: 0.2 m/s forward, 0.5 rad/s left turn
//! robot.drive_twist(Velocity::new(0.2)?, AngularVelocity::new(0.5)?).await?;
//!
//! // Cleaning brushes on
//! robot.set_motors(MotorBits { side_brush: true, vacuum: true, main_brush: true,
//!                              side_brush_backward: false, main_brush_backward: false }).await?;
//! robot.set_motors_pwm(64, 64, 64).await?; // ~50% PWM (Create 2 only)
//! robot.set_motors(MotorBits::default()).await?; // all off
//!
//! // Simulate Spot button press (Full mode only)
//! robot.simulate_buttons(ButtonBits { spot: true, ..ButtonBits::default() }).await?;
//! robot.simulate_buttons(ButtonBits::default()).await?; // release
//!
//! // Return to Safe
//! let robot = robot.to_safe().await.map_err(|e| e.source)?;
//! ```
//!
//! # Sensor Streaming
//!
//! ```rust,ignore
//! // Subscribe to bumps (7), voltage (22), OI mode (35)
//! robot.start_stream(&[7, 22, 35]).await?;
//!
//! let mut frames = 0u32;
//! let mut paused = false;
//! while frames < 30 {
//!     robot.poll_stream_with(|result| {
//!         if let Ok(sd) = result {
//!             frames += 1;
//!             // process sd.voltage, sd.is_right_bump(), sd.oi_mode, …
//!         }
//!     }).await?;
//!
//!     if frames >= 15 && !paused {
//!         paused = true;
//!         robot.toggle_stream(false).await?; // pause
//!         // … wait …
//!         robot.toggle_stream(true).await?;  // resume
//!     }
//! }
//! robot.toggle_stream(false).await?;
//! ```
//!
//! # Sensor Queries
//!
//! ```rust,ignore
//! // Single packet: battery voltage (packet 22)
//! let sd = robot.query_sensor(22).await?;
//! // sd.voltage is Option<u16>
//!
//! // Multiple packets in one round-trip
//! let sd = robot.query_list(&[22, 23, 25, 26, 35]).await?;
//! // sd.voltage, sd.current, sd.battery_charge, sd.battery_capacity, sd.oi_mode
//!
//! // Read OI mode directly
//! let mode = robot.read_oi_mode().await?;
//! ```
//!
//! # Scheduling
//!
//! ```rust,ignore
//! // Set robot clock to Monday 09:30 (Create 2 only)
//! robot.set_date(DayOfWeek::Monday, 9, 30).await?;
//!
//! // Program weekly schedule: Monday 09:30, Thursday 18:00
//! // Bitmask: bit 0 = Sunday, bit 1 = Monday, …, bit 6 = Saturday
//! let days = 0b001_0010_u8; // Monday + Thursday
//! let times: [(u8, u8); 7] = [
//!     (0, 0), (9, 30), (0, 0), (0, 0), (18, 0), (0, 0), (0, 0),
//! ];
//! robot.set_schedule(days, times).await?;
//! robot.set_schedule(0, [(0, 0); 7]).await?; // clear
//! ```
//!
//! # Autonomous Commands
//!
//! ```rust,ignore
//! // Start a spot-clean cycle — transitions Safe → Passive
//! let robot = robot.clean(CleanMode::Spot).await.map_err(|e| e.source)?;
//!
//! // Reclaim control (Safe aborts any ongoing autonomous operation)
//! let mut robot = robot.to_safe().await.map_err(|e| e.source)?;
//! robot.stop().await?;
//!
//! // Send robot to charging dock — also transitions Safe → Passive
//! let _robot = robot.seek_dock().await.map_err(|e| e.source)?;
//! // Poll packet 21 (ChargingState) to detect when docking is complete.
//! ```

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

    async fn delay(&mut self, duration: Duration) {
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

    async fn delay(&mut self, duration: Duration) {
        let micros = u64::try_from(duration.as_micros()).unwrap_or(u64::MAX);
        Timer::after(embassy_time::Duration::from_micros(micros)).await;
    }
}
