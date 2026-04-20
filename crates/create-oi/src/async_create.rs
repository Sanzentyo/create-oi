//! Asynchronous robot API with TypeState mode tracking.
//!
//! [`AsyncCreate<M, T>`] is the async counterpart to
//! [`Create<M, T>`](crate::create::Create). It provides the same TypeState
//! guarantees (the OI mode is encoded as a type parameter) but all I/O
//! operations are `async`.
//!
//! # Cancellation safety
//!
//! Most methods on `AsyncCreate` are **not** cancellation-safe. If a future
//! is dropped after the command bytes have been partially or fully written
//! but before the response is read, the transport and robot state may be
//! inconsistent. Prefer running transitions to completion or discarding
//! the robot handle after cancellation.

use crate::error::{ConnectError, Error, TransitionError};
use crate::mode::{Actuatable, Full, Mode, Off, Passive, Safe, SensorReadable};
use crate::transport::AsyncTransport;
use crate::types::{
    CreateRobotModel, LedIntensity, MotorPower, OiMode, PowerLedColor, Radius, SongNumber, Velocity,
};
use create_oi_protocol::command;
use create_oi_protocol::sensor::{self, SensorData};
use create_oi_protocol::stream::StreamParser;
use std::marker::PhantomData;

/// An asynchronous robot handle, parameterised by OI mode `M` and transport `T`.
///
/// Mode transitions consume `self` and return a new `AsyncCreate` in the target
/// mode, ensuring the compiler enforces valid mode sequences.
#[derive(Debug)]
pub struct AsyncCreate<M: Mode, T: AsyncTransport> {
    transport: T,
    model: CreateRobotModel,
    stream_parser: StreamParser,
    _mode: PhantomData<M>,
}

// ---------------------------------------------------------------------------
// Construction (Off mode)
// ---------------------------------------------------------------------------

impl<T: AsyncTransport> AsyncCreate<Off, T> {
    /// Create a new async robot handle wrapping the given transport.
    /// The robot is assumed to be in the `Off` state.
    pub fn new(transport: T, model: CreateRobotModel) -> Self {
        Self {
            transport,
            model,
            stream_parser: StreamParser::new(),
            _mode: PhantomData,
        }
    }

    /// Send the START command and transition to Passive mode.
    pub async fn start(mut self) -> Result<AsyncCreate<Passive, T>, ConnectError<T>> {
        if let Err(e) = self.send_cmd(&command::encode_start()).await {
            return Err(ConnectError {
                transport: self.transport,
                source: e,
            });
        }
        self.sleep_mode_change().await;
        Ok(self.transition())
    }
}

// ---------------------------------------------------------------------------
// Mode transitions (available from Passive, Safe, Full)
// ---------------------------------------------------------------------------

impl<T: AsyncTransport> AsyncCreate<Passive, T> {
    /// Transition to Safe mode.
    pub async fn to_safe(mut self) -> Result<AsyncCreate<Safe, T>, TransitionError<Self>> {
        if let Err(e) = self.send_cmd(&command::encode_safe()).await {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        self.sleep_mode_change().await;
        Ok(self.transition())
    }

    /// Transition to Full mode.
    pub async fn to_full(mut self) -> Result<AsyncCreate<Full, T>, TransitionError<Self>> {
        if let Err(e) = self.send_cmd(&command::encode_full()).await {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        self.sleep_mode_change().await;
        Ok(self.transition())
    }
}

impl<T: AsyncTransport> AsyncCreate<Safe, T> {
    /// Transition to Full mode.
    pub async fn to_full(mut self) -> Result<AsyncCreate<Full, T>, TransitionError<Self>> {
        if let Err(e) = self.send_cmd(&command::encode_full()).await {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        self.sleep_mode_change().await;
        Ok(self.transition())
    }

    /// Fall back to Passive mode (sends START).
    pub async fn to_passive(mut self) -> Result<AsyncCreate<Passive, T>, TransitionError<Self>> {
        if let Err(e) = self.send_cmd(&command::encode_start()).await {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        self.sleep_mode_change().await;
        Ok(self.transition())
    }
}

impl<T: AsyncTransport> AsyncCreate<Full, T> {
    /// Fall back to Safe mode.
    pub async fn to_safe(mut self) -> Result<AsyncCreate<Safe, T>, TransitionError<Self>> {
        if let Err(e) = self.send_cmd(&command::encode_safe()).await {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        self.sleep_mode_change().await;
        Ok(self.transition())
    }

    /// Fall back to Passive mode (sends START).
    pub async fn to_passive(mut self) -> Result<AsyncCreate<Passive, T>, TransitionError<Self>> {
        if let Err(e) = self.send_cmd(&command::encode_start()).await {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        self.sleep_mode_change().await;
        Ok(self.transition())
    }
}

// ---------------------------------------------------------------------------
// Sensor reading (Passive, Safe, Full)
// ---------------------------------------------------------------------------

impl<M: SensorReadable, T: AsyncTransport> AsyncCreate<M, T> {
    /// Query a single sensor packet by ID and return the raw bytes.
    pub async fn query_sensor_raw(&mut self, packet_id: u8) -> Result<Vec<u8>, Error> {
        self.send_cmd(&command::encode_sensors(packet_id)).await?;

        let info = create_oi_protocol::opcode::packet_info(packet_id).ok_or_else(|| {
            Error::Protocol(create_oi_protocol::error::ProtocolError::Protocol(format!(
                "unknown packet id {packet_id}"
            )))
        })?;
        let mut buf = vec![0u8; info.len as usize];
        self.read_exact(&mut buf).await?;
        Ok(buf)
    }

    /// Query a single sensor packet and decode it.
    pub async fn query_sensor(&mut self, packet_id: u8) -> Result<SensorData, Error> {
        let raw = self.query_sensor_raw(packet_id).await?;
        let mut sd = SensorData::default();
        sd.decode_packet(packet_id, &raw)?;
        Ok(sd)
    }

    /// Query multiple sensors at once and decode all of them.
    pub async fn query_list(&mut self, packet_ids: &[u8]) -> Result<SensorData, Error> {
        self.send_cmd(&command::encode_query_list(packet_ids))
            .await?;

        let expected_len = sensor::expected_data_len(packet_ids)?;
        let mut buf = vec![0u8; expected_len];
        self.read_exact(&mut buf).await?;

        let mut sd = SensorData::default();
        sd.decode_packets(packet_ids, &buf)?;
        Ok(sd)
    }

    /// Read the robot's current OI mode from sensor data.
    pub async fn read_oi_mode(&mut self) -> Result<OiMode, Error> {
        let sd = self.query_sensor(35).await?;
        sd.oi_mode.ok_or_else(|| {
            Error::Protocol(create_oi_protocol::error::ProtocolError::Protocol(
                "missing OI mode".into(),
            ))
        })
    }

    /// Start streaming the given packet IDs.
    pub async fn start_stream(&mut self, packet_ids: &[u8]) -> Result<(), Error> {
        self.stream_parser.set_packet_ids(packet_ids);
        self.send_cmd(&command::encode_stream(packet_ids)).await
    }

    /// Pause or resume the sensor stream.
    pub async fn toggle_stream(&mut self, enable: bool) -> Result<(), Error> {
        self.send_cmd(&command::encode_toggle_stream(enable)).await
    }

    /// Read bytes from the transport and try to parse stream frames.
    pub async fn poll_stream(&mut self) -> Result<Vec<SensorData>, Error> {
        let mut buf = [0u8; 256];
        let n = self.transport.read(&mut buf).await.map_err(Error::Io)?;
        if n == 0 {
            return Ok(Vec::new());
        }
        let results = self.stream_parser.feed(&buf[..n]);
        results
            .into_iter()
            .map(|r| r.map_err(Error::Protocol))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Actuator commands (Safe, Full)
// ---------------------------------------------------------------------------

impl<M: Actuatable, T: AsyncTransport> AsyncCreate<M, T> {
    /// Drive with a given velocity and radius.
    pub async fn drive(&mut self, velocity: Velocity, radius: Radius) -> Result<(), Error> {
        self.send_cmd(&command::encode_drive(
            velocity.to_mm_per_sec(),
            radius.to_mm(),
        ))
        .await
    }

    /// Drive wheels directly with individual velocities.
    pub async fn drive_direct(&mut self, right: Velocity, left: Velocity) -> Result<(), Error> {
        self.send_cmd(&command::encode_drive_direct(
            right.to_mm_per_sec(),
            left.to_mm_per_sec(),
        ))
        .await
    }

    /// Drive wheels with PWM values.
    pub async fn drive_pwm(&mut self, right: MotorPower, left: MotorPower) -> Result<(), Error> {
        self.send_cmd(&command::encode_drive_pwm(right.to_pwm(), left.to_pwm()))
            .await
    }

    /// Stop all motors (drive 0, 0).
    pub async fn stop(&mut self) -> Result<(), Error> {
        self.send_cmd(&command::encode_drive(0, 0)).await
    }

    /// Set LEDs.
    pub async fn set_leds(
        &mut self,
        debris: bool,
        spot: bool,
        dock: bool,
        check_robot: bool,
        color: PowerLedColor,
        intensity: LedIntensity,
    ) -> Result<(), Error> {
        let bits =
            (debris as u8) | ((spot as u8) << 1) | ((dock as u8) << 2) | ((check_robot as u8) << 3);
        self.send_cmd(&command::encode_leds(bits, color.get(), intensity.get()))
            .await
    }

    /// Display ASCII characters on the 7-segment displays.
    pub async fn set_digit_leds(&mut self, d3: u8, d2: u8, d1: u8, d0: u8) -> Result<(), Error> {
        self.send_cmd(&command::encode_digit_leds_ascii(d3, d2, d1, d0))
            .await
    }

    /// Define a song.
    pub async fn define_song(
        &mut self,
        number: SongNumber,
        notes: &[(u8, u8)],
    ) -> Result<(), Error> {
        let cmd = command::encode_song(number.get(), notes);
        self.send_cmd(&cmd).await
    }

    /// Play a previously defined song.
    pub async fn play_song(&mut self, number: SongNumber) -> Result<(), Error> {
        self.send_cmd(&command::encode_play(number.get())).await
    }

    /// Set motor PWM (main brush, side brush, vacuum).
    pub async fn set_motors_pwm(
        &mut self,
        main_brush: i8,
        side_brush: i8,
        vacuum: i8,
    ) -> Result<(), Error> {
        self.send_cmd(&command::encode_motors_pwm(main_brush, side_brush, vacuum))
            .await
    }
}

// ---------------------------------------------------------------------------
// Common utilities (all modes)
// ---------------------------------------------------------------------------

impl<M: Mode, T: AsyncTransport> AsyncCreate<M, T> {
    /// Get the robot model.
    pub fn model(&self) -> CreateRobotModel {
        self.model
    }

    /// Consume the robot and return the underlying transport.
    pub fn into_transport(self) -> T {
        self.transport
    }

    /// Borrow the underlying transport.
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Borrow the underlying transport mutably.
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// Send raw bytes to the robot.
    async fn send_cmd(&mut self, data: &[u8]) -> Result<(), Error> {
        self.transport.write_all(data).await.map_err(Error::Io)?;
        self.transport.flush().await.map_err(Error::Io)?;
        Ok(())
    }

    /// Read exactly `buf.len()` bytes from the transport.
    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        let mut offset = 0;
        while offset < buf.len() {
            let n = self
                .transport
                .read(&mut buf[offset..])
                .await
                .map_err(Error::Io)?;
            if n == 0 {
                return Err(Error::Protocol(
                    create_oi_protocol::error::ProtocolError::InsufficientData {
                        need: buf.len(),
                        got: offset,
                    },
                ));
            }
            offset += n;
        }
        Ok(())
    }

    async fn sleep_mode_change(&self) {
        self.transport.sleep(self.model.mode_change_delay()).await;
    }

    /// Transition to a different mode (zero-cost: just changes the type parameter).
    fn transition<N: Mode>(self) -> AsyncCreate<N, T> {
        AsyncCreate {
            transport: self.transport,
            model: self.model,
            stream_parser: self.stream_parser,
            _mode: PhantomData,
        }
    }
}
