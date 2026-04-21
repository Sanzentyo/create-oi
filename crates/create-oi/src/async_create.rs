//! Asynchronous Create API with TypeState mode tracking.
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

use crate::error::{ConnectError, Error, TransitionError, ValidationError};
use crate::mode::{Actuatable, Full, FullControl, Mode, Off, Passive, Safe, SensorReadable};
use crate::transport::AsyncTransport;
use crate::types::{
    AngularVelocity, ButtonBits, CleanMode, DayOfWeek, LedIntensity, MotorBits, MotorPower, OiMode,
    PowerLedColor, Radius, RobotModel, SongNote, SongNumber, Velocity,
};
use core::marker::PhantomData;
use create_oi_protocol::command;
use create_oi_protocol::sensor::{self, SensorData};
use create_oi_protocol::stream::StreamParser;

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use alloc::vec;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// An asynchronous Create handle, parameterised by OI mode `M` and transport `T`.
///
/// Mode transitions consume `self` and return a new `AsyncCreate` in the target
/// mode, ensuring the compiler enforces valid mode sequences.
///
/// # TypeState vs. actual robot mode
///
/// The mode type parameter tracks the **last commanded mode**, not the robot's
/// current hardware state. The robot can change mode autonomously (e.g. safety
/// events, button presses). Call [`read_oi_mode`](AsyncCreate::read_oi_mode) to
/// read the actual mode from the robot.
#[derive(Debug)]
pub struct AsyncCreate<M: Mode, T: AsyncTransport> {
    transport: T,
    model: RobotModel,
    stream_parser: StreamParser,
    /// `true` while a sensor stream is active (after `start_stream`, before `toggle_stream(false)`).
    streaming: bool,
    _mode: PhantomData<M>,
}

// ---------------------------------------------------------------------------
// Construction (Off mode)
// ---------------------------------------------------------------------------

impl<T: AsyncTransport> AsyncCreate<Off, T> {
    /// Create a new async robot handle wrapping the given transport.
    /// The robot is assumed to be in the `Off` state.
    pub fn new(transport: T, model: RobotModel) -> Self {
        Self {
            transport,
            model,
            stream_parser: StreamParser::new(),
            streaming: false,
            _mode: PhantomData,
        }
    }

    /// Send the START command and transition to Passive mode.
    pub async fn start(mut self) -> Result<AsyncCreate<Passive, T>, ConnectError<T, T::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_start()).await {
            return Err(ConnectError {
                transport: self.transport,
                source: e,
            });
        }
        self.sleep_mode_change().await;
        Ok(self.transition())
    }

    /// Soft-reset the robot. Available in all modes per the OI spec, including Off.
    ///
    /// After this call the robot reboots. The connection may need to be
    /// re-opened before creating a new `AsyncCreate::<Off, _>::new(transport, model)`.
    pub async fn reset(mut self) -> Result<T, ConnectError<T, T::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_reset()).await {
            return Err(ConnectError {
                transport: self.transport,
                source: e,
            });
        }
        Ok(self.transport)
    }
}

// ---------------------------------------------------------------------------
// Mode transitions (available from Passive, Safe, Full)
// ---------------------------------------------------------------------------

impl<T: AsyncTransport> AsyncCreate<Passive, T> {
    /// Transition to Safe mode.
    pub async fn to_safe(
        mut self,
    ) -> Result<AsyncCreate<Safe, T>, TransitionError<Self, T::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_safe()).await {
            return Err(TransitionError {
                create: self,
                source: e,
            });
        }
        self.sleep_mode_change().await;
        Ok(self.transition())
    }

    /// Transition to Full mode.
    pub async fn to_full(
        mut self,
    ) -> Result<AsyncCreate<Full, T>, TransitionError<Self, T::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_full()).await {
            return Err(TransitionError {
                create: self,
                source: e,
            });
        }
        self.sleep_mode_change().await;
        Ok(self.transition())
    }

    /// Send STOP and transition to Off mode.
    ///
    /// STOP (opcode 173) is a Create 2–only command. Returns `ValidationError`
    /// on Create 1 or Roomba 400. The OI session is ended; to reconnect,
    /// create a new `AsyncCreate::<Off, _>::new(transport, model)`.
    pub async fn to_off(mut self) -> Result<AsyncCreate<Off, T>, TransitionError<Self, T::Error>> {
        if !self.model.is_create2() {
            return Err(TransitionError {
                create: self,
                source: Error::Validation(ValidationError {
                    field: "model",
                    reason: "to_off (OPCODE 173 STOP) requires Create 2; not supported on Create 1 or Roomba 400",
                }),
            });
        }
        if let Err(e) = self.send_cmd(&command::encode_stop()).await {
            return Err(TransitionError {
                create: self,
                source: e,
            });
        }
        Ok(self.cleared_transition())
    }

    /// Initiate a cleaning cycle. Transitions the robot to Passive mode.
    ///
    /// Per the OI spec, CLEAN/SPOT/MAX are only valid from Passive mode.
    ///
    /// The OI spec defines three cleaning modes:
    /// - [`CleanMode::Default`] — standard cleaning pattern
    /// - [`CleanMode::Spot`] — spot cleaning (small area)
    /// - [`CleanMode::Max`] — maximum cleaning (until battery depleted)
    pub async fn clean(
        mut self,
        mode: CleanMode,
    ) -> Result<AsyncCreate<Passive, T>, TransitionError<Self, T::Error>> {
        let cmd = match mode {
            CleanMode::Default => command::encode_clean(),
            CleanMode::Spot => command::encode_spot(),
            CleanMode::Max => command::encode_max(),
        };
        if let Err(e) = self.send_cmd(&cmd).await {
            return Err(TransitionError {
                create: self,
                source: e,
            });
        }
        Ok(self.transition())
    }

    /// Seek the dock. Transitions the robot to Passive mode.
    ///
    /// Per the OI spec, SEEK_DOCK is only valid from Passive mode.
    pub async fn seek_dock(
        mut self,
    ) -> Result<AsyncCreate<Passive, T>, TransitionError<Self, T::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_dock()).await {
            return Err(TransitionError {
                create: self,
                source: e,
            });
        }
        Ok(self.transition())
    }
}

impl<T: AsyncTransport> AsyncCreate<Safe, T> {
    /// Transition to Full mode.
    pub async fn to_full(
        mut self,
    ) -> Result<AsyncCreate<Full, T>, TransitionError<Self, T::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_full()).await {
            return Err(TransitionError {
                create: self,
                source: e,
            });
        }
        self.sleep_mode_change().await;
        Ok(self.transition())
    }

    /// Fall back to Passive mode (sends START).
    pub async fn to_passive(
        mut self,
    ) -> Result<AsyncCreate<Passive, T>, TransitionError<Self, T::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_start()).await {
            return Err(TransitionError {
                create: self,
                source: e,
            });
        }
        self.sleep_mode_change().await;
        Ok(self.transition())
    }

    /// Send STOP and transition to Off mode.
    ///
    /// STOP (opcode 173) is a Create 2–only command. Returns `ValidationError`
    /// on Create 1 or Roomba 400.
    pub async fn to_off(mut self) -> Result<AsyncCreate<Off, T>, TransitionError<Self, T::Error>> {
        if !self.model.is_create2() {
            return Err(TransitionError {
                create: self,
                source: Error::Validation(ValidationError {
                    field: "model",
                    reason: "to_off (OPCODE 173 STOP) requires Create 2; not supported on Create 1 or Roomba 400",
                }),
            });
        }
        if let Err(e) = self.send_cmd(&command::encode_stop()).await {
            return Err(TransitionError {
                create: self,
                source: e,
            });
        }
        Ok(self.cleared_transition())
    }
}

impl<T: AsyncTransport> AsyncCreate<Full, T> {
    /// Fall back to Safe mode.
    pub async fn to_safe(
        mut self,
    ) -> Result<AsyncCreate<Safe, T>, TransitionError<Self, T::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_safe()).await {
            return Err(TransitionError {
                create: self,
                source: e,
            });
        }
        self.sleep_mode_change().await;
        Ok(self.transition())
    }

    /// Fall back to Passive mode (sends START).
    pub async fn to_passive(
        mut self,
    ) -> Result<AsyncCreate<Passive, T>, TransitionError<Self, T::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_start()).await {
            return Err(TransitionError {
                create: self,
                source: e,
            });
        }
        self.sleep_mode_change().await;
        Ok(self.transition())
    }

    /// Send STOP and transition to Off mode.
    ///
    /// STOP (opcode 173) is a Create 2–only command. Returns `ValidationError`
    /// on Create 1 or Roomba 400.
    pub async fn to_off(mut self) -> Result<AsyncCreate<Off, T>, TransitionError<Self, T::Error>> {
        if !self.model.is_create2() {
            return Err(TransitionError {
                create: self,
                source: Error::Validation(ValidationError {
                    field: "model",
                    reason: "to_off (OPCODE 173 STOP) requires Create 2; not supported on Create 1 or Roomba 400",
                }),
            });
        }
        if let Err(e) = self.send_cmd(&command::encode_stop()).await {
            return Err(TransitionError {
                create: self,
                source: e,
            });
        }
        Ok(self.cleared_transition())
    }
}

// ---------------------------------------------------------------------------
// Sensor reading (Passive, Safe, Full)
// ---------------------------------------------------------------------------

impl<M: SensorReadable, T: AsyncTransport> AsyncCreate<M, T> {
    /// Query a single sensor packet by ID into a caller-provided buffer.
    ///
    /// Validates the packet ID and buffer size before sending any bytes to the robot.
    /// Returns the number of bytes written.
    #[must_use = "query result must be used"]
    pub async fn query_sensor_raw_into(
        &mut self,
        packet_id: u8,
        buf: &mut [u8],
    ) -> Result<usize, Error<T::Error>> {
        self.reject_if_streaming()?;
        let info = create_oi_protocol::opcode::packet_info(packet_id).ok_or(Error::Protocol(
            create_oi_protocol::error::ProtocolError::UnknownPacketId(packet_id),
        ))?;
        let len = info.len as usize;
        if buf.len() < len {
            return Err(Error::Protocol(
                create_oi_protocol::error::ProtocolError::BufferTooSmall {
                    need: len,
                    got: buf.len(),
                },
            ));
        }
        self.send_cmd(&command::encode_sensors(packet_id)).await?;
        self.read_exact(&mut buf[..len]).await?;
        Ok(len)
    }

    /// Query a single sensor packet by ID and return the raw bytes.
    ///
    /// Validates the packet ID before sending any bytes to the robot.
    #[cfg(feature = "alloc")]
    #[must_use = "query result must be used"]
    pub async fn query_sensor_raw(&mut self, packet_id: u8) -> Result<Vec<u8>, Error<T::Error>> {
        self.reject_if_streaming()?;
        let info = create_oi_protocol::opcode::packet_info(packet_id).ok_or(Error::Protocol(
            create_oi_protocol::error::ProtocolError::UnknownPacketId(packet_id),
        ))?;
        let mut buf = vec![0u8; info.len as usize];
        self.send_cmd(&command::encode_sensors(packet_id)).await?;
        self.read_exact(&mut buf).await?;
        Ok(buf)
    }

    /// Query a single sensor packet and decode it.
    #[must_use = "query result must be used"]
    pub async fn query_sensor(&mut self, packet_id: u8) -> Result<SensorData, Error<T::Error>> {
        let mut buf = [0u8; 64]; // largest single packet is well under 64 bytes
        let len = self.query_sensor_raw_into(packet_id, &mut buf).await?;
        let mut sd = SensorData::default();
        sd.decode_packet(packet_id, &buf[..len])?;
        Ok(sd)
    }

    /// Query multiple sensors at once and decode all of them.
    ///
    /// Validates all packet IDs before sending any bytes to the robot.
    ///
    /// # Limits
    ///
    /// This async implementation uses a fixed stack buffer sized for the
    /// largest OI sensor group (Group-100, 52 packet IDs). Passing more
    /// than 52 IDs returns a `ValidationError`. For longer lists, use the
    /// sync `Create::query_list` which allocates a `Vec`.
    #[must_use = "query result must be used"]
    pub async fn query_list(&mut self, packet_ids: &[u8]) -> Result<SensorData, Error<T::Error>> {
        self.reject_if_streaming()?;
        // Cap at Group-100 size to match the stack buffer below.
        const ASYNC_MAX_IDS: usize = 52;
        if packet_ids.len() > ASYNC_MAX_IDS {
            return Err(Error::Validation(ValidationError {
                field: "packet_ids",
                reason: "async query_list supports at most 52 packet IDs; use sync API for longer lists",
            }));
        }
        // Validate packet IDs and compute expected response length BEFORE sending.
        let expected_len = sensor::expected_data_len(packet_ids)?;
        // Group-100 has 52 IDs — the largest valid group. Stack-buffer accordingly.
        const MAX_CMD: usize = 2 + ASYNC_MAX_IDS;
        let mut cmd_buf = [0u8; MAX_CMD];
        let cmd_len = command::encode_query_list_into(&mut cmd_buf, packet_ids)?;
        self.send_cmd(&cmd_buf[..cmd_len]).await?;

        let mut buf = [0u8; 256];
        self.read_exact(&mut buf[..expected_len]).await?;

        let mut sd = SensorData::default();
        sd.decode_packets(packet_ids, &buf[..expected_len])?;
        Ok(sd)
    }

    /// Read the robot's current OI mode from sensor data.
    #[must_use = "query result must be used"]
    pub async fn read_oi_mode(&mut self) -> Result<OiMode, Error<T::Error>> {
        let sd = self.query_sensor(35).await?;
        sd.oi_mode.ok_or(Error::Protocol(
            create_oi_protocol::error::ProtocolError::MissingSensorField { field: "oi_mode" },
        ))
    }

    /// Start streaming the given packet IDs.
    ///
    /// Returns an error if this robot model does not support sensor streaming,
    /// if the packet ID list exceeds the protocol limit, or if the total
    /// stream payload per cycle would exceed 255 bytes.
    ///
    /// # Limits
    ///
    /// This async implementation uses a fixed stack buffer sized for the
    /// largest OI sensor group (Group-100, 52 packet IDs). Passing more
    /// than 52 IDs returns a `ValidationError`.
    pub async fn start_stream(&mut self, packet_ids: &[u8]) -> Result<(), Error<T::Error>> {
        if !self.model.supports_stream() {
            return Err(Error::Validation(ValidationError {
                field: "stream",
                reason: "sensor streaming is not supported by this robot model",
            }));
        }
        let payload_bytes: usize = packet_ids
            .iter()
            .map(|&id| {
                1 + create_oi_protocol::opcode::packet_info(id).map_or(0, |p| p.len as usize)
            })
            .sum();
        if payload_bytes > 255 {
            return Err(Error::Validation(ValidationError {
                field: "packet_ids",
                reason: "stream payload per cycle exceeds OI limit of 255 bytes",
            }));
        }
        const ASYNC_MAX_IDS: usize = 52;
        if packet_ids.len() > ASYNC_MAX_IDS {
            return Err(Error::Validation(ValidationError {
                field: "packet_ids",
                reason: "async start_stream supports at most 52 packet IDs; use sync API for longer lists",
            }));
        }
        const MAX_CMD: usize = 2 + ASYNC_MAX_IDS;
        let mut buf = [0u8; MAX_CMD];
        let len = command::encode_stream_into(&mut buf, packet_ids)?;
        self.send_cmd(&buf[..len]).await?;
        self.streaming = true;
        Ok(())
    }

    /// Pause or resume the sensor stream.
    ///
    /// Returns an error if this robot model does not support sensor streaming.
    pub async fn toggle_stream(&mut self, enable: bool) -> Result<(), Error<T::Error>> {
        if !self.model.supports_stream() {
            return Err(Error::Validation(ValidationError {
                field: "stream",
                reason: "sensor streaming is not supported by this robot model",
            }));
        }
        self.send_cmd(&command::encode_toggle_stream(enable))
            .await?;
        self.streaming = enable;
        Ok(())
    }

    /// Read bytes from the transport and try to parse stream frames.
    #[cfg(feature = "alloc")]
    pub async fn poll_stream(&mut self) -> Result<Vec<SensorData>, Error<T::Error>> {
        let mut buf = [0u8; 256];
        let n = self.transport.read(&mut buf).await.map_err(Error::Io)?;
        if n == 0 {
            return Err(Error::Protocol(
                create_oi_protocol::error::ProtocolError::InsufficientData { need: 1, got: 0 },
            ));
        }
        let results = self.stream_parser.feed(&buf[..n]);
        results
            .into_iter()
            .map(|r| r.map_err(Error::Protocol))
            .collect()
    }

    /// Read bytes from the transport and parse stream frames via callback.
    ///
    /// This is the no-alloc equivalent of [`poll_stream`](Self::poll_stream).
    pub async fn poll_stream_with(
        &mut self,
        callback: impl FnMut(Result<SensorData, create_oi_protocol::error::ProtocolError>),
    ) -> Result<(), Error<T::Error>> {
        let mut buf = [0u8; 256];
        let n = self.transport.read(&mut buf).await.map_err(Error::Io)?;
        if n == 0 {
            return Err(Error::Protocol(
                create_oi_protocol::error::ProtocolError::InsufficientData { need: 1, got: 0 },
            ));
        }
        self.stream_parser.feed_with(&buf[..n], callback);
        Ok(())
    }

    /// Send the POWER command, putting the robot into Passive charging mode.
    ///
    /// After this call the robot enters Passive mode and begins charging.
    /// The returned handle can be used to continue interactions or gracefully
    /// transition to other modes. The stream state is cleared.
    pub async fn power_off(
        mut self,
    ) -> Result<AsyncCreate<Passive, T>, TransitionError<Self, T::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_power()).await {
            return Err(TransitionError {
                create: self,
                source: e,
            });
        }
        Ok(self.cleared_transition())
    }

    /// Reset the robot and return the underlying transport.
    ///
    /// After this call the robot reboots. The serial connection may need to be
    /// re-opened before creating a new `AsyncCreate::<Off, _>::new(transport, model)`.
    pub async fn reset(mut self) -> Result<T, TransitionError<Self, T::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_reset()).await {
            return Err(TransitionError {
                create: self,
                source: e,
            });
        }
        Ok(self.transport)
    }

    /// Define a song.
    ///
    /// Songs can be defined in Passive, Safe, and Full mode per the OI spec.
    /// Returns `ValidationError` if the song slot exceeds this model's maximum
    /// (Create 2: 0–4, Create 1 / Roomba 400: 0–15).
    ///
    /// Use [`SongNote::new`] to construct notes; MIDI note numbers must be 31..=127.
    pub async fn define_song(
        &mut self,
        number: SongNumber,
        notes: &[SongNote],
    ) -> Result<(), Error<T::Error>> {
        if number.get() > self.model.max_song_number() {
            return Err(Error::Validation(ValidationError {
                field: "number",
                reason: "song slot exceeds this model's maximum",
            }));
        }
        if notes.len() > 16 {
            return Err(Error::Protocol(
                create_oi_protocol::error::ProtocolError::TooManyItems {
                    max: 16,
                    got: notes.len(),
                },
            ));
        }
        if notes.is_empty() {
            return Err(Error::Protocol(
                create_oi_protocol::error::ProtocolError::TooFewItems { min: 1, got: 0 },
            ));
        }
        let mut raw = [(0u8, 0u8); 16];
        let count = notes.len().min(16);
        for (i, n) in notes.iter().enumerate().take(count) {
            raw[i] = (n.midi_note(), n.duration_64ths());
        }
        let mut buf = [0u8; 35]; // 1 opcode + 1 song_number + 1 count + 16*2 notes = 35
        let len = command::encode_song_into(&mut buf, number.get(), &raw[..count])?;
        self.send_cmd(&buf[..len]).await
    }

    /// Play a previously defined song.
    ///
    /// Songs can be played in Passive, Safe, and Full mode per the OI spec.
    /// Returns `ValidationError` if the song slot exceeds this model's maximum.
    pub async fn play_song(&mut self, number: SongNumber) -> Result<(), Error<T::Error>> {
        if number.get() > self.model.max_song_number() {
            return Err(Error::Validation(ValidationError {
                field: "number",
                reason: "song slot exceeds this model's maximum",
            }));
        }
        self.send_cmd(&command::encode_play(number.get())).await
    }
}

// ---------------------------------------------------------------------------
// Actuator commands (Safe, Full)
// ---------------------------------------------------------------------------

impl<M: Actuatable, T: AsyncTransport> AsyncCreate<M, T> {
    /// Drive with a given velocity and radius.
    pub async fn drive(
        &mut self,
        velocity: Velocity,
        radius: Radius,
    ) -> Result<(), Error<T::Error>> {
        self.send_cmd(&command::encode_drive(
            velocity.to_mm_per_sec(),
            radius.to_mm(),
        ))
        .await
    }

    /// Drive wheels directly with individual velocities.
    pub async fn drive_direct(
        &mut self,
        right: Velocity,
        left: Velocity,
    ) -> Result<(), Error<T::Error>> {
        self.send_cmd(&command::encode_drive_direct(
            right.to_mm_per_sec(),
            left.to_mm_per_sec(),
        ))
        .await
    }

    /// Drive wheels with PWM values.
    ///
    /// Returns `ValidationError` if this model is not Create 2 (OPCODE 146 is
    /// not supported on Create 1 or Roomba 400).
    pub async fn drive_pwm(
        &mut self,
        right: MotorPower,
        left: MotorPower,
    ) -> Result<(), Error<T::Error>> {
        if !self.model.is_create2() {
            return Err(Error::Validation(ValidationError {
                field: "model",
                reason: "drive_pwm (OPCODE 146) requires Create 2; not supported on Create 1 or Roomba 400",
            }));
        }
        self.send_cmd(&command::encode_drive_pwm(right.to_pwm(), left.to_pwm()))
            .await
    }

    /// Stop all motors (both wheels to 0 mm/s).
    pub async fn stop(&mut self) -> Result<(), Error<T::Error>> {
        self.send_cmd(&command::encode_drive_direct(0, 0)).await
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
    ) -> Result<(), Error<T::Error>> {
        let bits =
            (debris as u8) | ((spot as u8) << 1) | ((dock as u8) << 2) | ((check_robot as u8) << 3);
        self.send_cmd(&command::encode_leds(bits, color.get(), intensity.get()))
            .await
    }

    /// Display ASCII characters on the 7-segment displays.
    ///
    /// Each character must be a printable ASCII byte (32–126). Non-printable
    /// bytes are rejected before any bytes are sent to the robot.
    ///
    /// Returns `ValidationError` if this model is not Create 2 (OPCODE 164 is
    /// not supported on Create 1 or Roomba 400).
    pub async fn set_digit_leds(
        &mut self,
        d3: u8,
        d2: u8,
        d1: u8,
        d0: u8,
    ) -> Result<(), Error<T::Error>> {
        if !self.model.is_create2() {
            return Err(Error::Validation(ValidationError {
                field: "model",
                reason: "set_digit_leds (OPCODE 164) requires Create 2; not supported on Create 1 or Roomba 400",
            }));
        }
        for (name, val) in [("d3", d3), ("d2", d2), ("d1", d1), ("d0", d0)] {
            if !(32..=126).contains(&val) {
                return Err(Error::Validation(ValidationError {
                    field: name,
                    reason: "digit LED character must be printable ASCII (32–126)",
                }));
            }
        }
        self.send_cmd(&command::encode_digit_leds_ascii(d3, d2, d1, d0))
            .await
    }

    /// Set motor PWM (main brush, side brush, vacuum).
    ///
    /// - `main_brush` and `side_brush`: range -127..=127 (i8::MIN = -128 is rejected).
    ///   Positive = forward direction, negative = reverse.
    /// - `vacuum`: range 0..=127. Negative values are invalid per the OI spec
    ///   (vacuum runs in one direction only) and are rejected without sending.
    ///
    /// Returns `ValidationError` if this model is not Create 2 (OPCODE 144 is
    /// not supported on Create 1 or Roomba 400).
    pub async fn set_motors_pwm(
        &mut self,
        main_brush: i8,
        side_brush: i8,
        vacuum: i8,
    ) -> Result<(), Error<T::Error>> {
        if !self.model.is_create2() {
            return Err(Error::Validation(ValidationError {
                field: "model",
                reason: "set_motors_pwm (OPCODE 144) requires Create 2; not supported on Create 1 or Roomba 400",
            }));
        }
        for (name, val) in [("main_brush", main_brush), ("side_brush", side_brush)] {
            if val == i8::MIN {
                return Err(Error::Validation(ValidationError {
                    field: name,
                    reason: "motor PWM value -128 is not valid; range is -127..=127",
                }));
            }
        }
        if vacuum < 0 {
            return Err(Error::Validation(ValidationError {
                field: "vacuum",
                reason: "vacuum PWM must be 0..=127; negative values are invalid per OI spec",
            }));
        }
        self.send_cmd(&command::encode_motors_pwm(main_brush, side_brush, vacuum))
            .await
    }

    /// Enable or disable motors with direction control.
    pub async fn set_motors(&mut self, motors: MotorBits) -> Result<(), Error<T::Error>> {
        self.send_cmd(&command::encode_motors(motors.to_raw()))
            .await
    }

    /// Set raw 7-segment digit LEDs.
    ///
    /// Each byte controls one digit: bits 0–6 = segments A–G, bit 7 = decimal point.
    /// `d3` is the leftmost digit and `d0` is the rightmost.
    ///
    /// Returns `ValidationError` if this model is not Create 2 (OPCODE 163 is
    /// not supported on Create 1 or Roomba 400).
    pub async fn set_digit_leds_raw(
        &mut self,
        d3: u8,
        d2: u8,
        d1: u8,
        d0: u8,
    ) -> Result<(), Error<T::Error>> {
        if !self.model.is_create2() {
            return Err(Error::Validation(ValidationError {
                field: "model",
                reason: "set_digit_leds_raw (OPCODE 163) requires Create 2; not supported on Create 1 or Roomba 400",
            }));
        }
        self.send_cmd(&command::encode_digit_leds_raw(d3, d2, d1, d0))
            .await
    }

    /// Drive using the unicycle (twist) model: linear velocity and angular velocity.
    ///
    /// Computes individual wheel speeds via differential drive kinematics:
    /// `right = v + ω × (axle/2)`, `left = v − ω × (axle/2)`.
    /// Wheel speeds are clamped to ±500 mm/s as required by the OI spec.
    pub async fn drive_twist(
        &mut self,
        velocity: Velocity,
        omega: AngularVelocity,
    ) -> Result<(), Error<T::Error>> {
        let half_axle_mm = self.model.axle_length() * 500.0;
        let v_mm = velocity.to_mm_per_sec() as f32;
        let right_mm = (libm::roundf(v_mm + omega.get() * half_axle_mm) as i16).clamp(-500, 500);
        let left_mm = (libm::roundf(v_mm - omega.get() * half_axle_mm) as i16).clamp(-500, 500);
        self.send_cmd(&command::encode_drive_direct(right_mm, left_mm))
            .await
    }
}

// ---------------------------------------------------------------------------
// Full-control commands (Full only)
// ---------------------------------------------------------------------------

impl<M: FullControl, T: AsyncTransport> AsyncCreate<M, T> {
    /// Simulate button presses on the robot (Full mode only).
    ///
    /// Returns `ValidationError` if this model is not Create 2 (OPCODE 165 is
    /// not supported on Create 1 or Roomba 400).
    pub async fn simulate_buttons(&mut self, buttons: ButtonBits) -> Result<(), Error<T::Error>> {
        if !self.model.is_create2() {
            return Err(Error::Validation(ValidationError {
                field: "model",
                reason: "simulate_buttons (OPCODE 165) requires Create 2; not supported on Create 1 or Roomba 400",
            }));
        }
        self.send_cmd(&command::encode_buttons(buttons.to_raw()))
            .await
    }
}

// ---------------------------------------------------------------------------
// Scheduling commands (Passive, Safe, Full) — Create 2 only
// ---------------------------------------------------------------------------

impl<M: SensorReadable, T: AsyncTransport> AsyncCreate<M, T> {
    /// Set the robot's internal date and time.
    ///
    /// Per the OI spec, SET_DAY_TIME (opcode 168) is available in
    /// Passive, Safe, and Full modes. Returns `ValidationError` if this model
    /// is not Create 2, or if `hour`/`minute` are out of range.
    pub async fn set_date(
        &mut self,
        day: DayOfWeek,
        hour: u8,
        minute: u8,
    ) -> Result<(), Error<T::Error>> {
        if !self.model.is_create2() {
            return Err(Error::Validation(ValidationError {
                field: "model",
                reason: "set_date (OPCODE 168) requires Create 2; not supported on Create 1 or Roomba 400",
            }));
        }
        if hour > 23 {
            return Err(Error::Validation(ValidationError {
                field: "hour",
                reason: "hour must be in range 0-23",
            }));
        }
        if minute > 59 {
            return Err(Error::Validation(ValidationError {
                field: "minute",
                reason: "minute must be in range 0-59",
            }));
        }
        self.send_cmd(&command::encode_date(day.to_raw(), hour, minute))
            .await
    }

    /// Set the weekly cleaning schedule.
    ///
    /// Per the OI spec, SCHEDULE (opcode 167) is available in
    /// Passive, Safe, and Full modes. Returns `ValidationError` if this model
    /// is not Create 2.
    ///
    /// `days`: bitmask of scheduled days (bit 0=Sunday, bit 6=Saturday). Bit 7 must be 0.
    /// `times`: (hour, minute) for each day, starting with Sunday.
    pub async fn set_schedule(
        &mut self,
        days: u8,
        times: [(u8, u8); 7],
    ) -> Result<(), Error<T::Error>> {
        if !self.model.is_create2() {
            return Err(Error::Validation(ValidationError {
                field: "model",
                reason: "set_schedule (OPCODE 167) requires Create 2; not supported on Create 1 or Roomba 400",
            }));
        }
        if days & !0x7F != 0 {
            return Err(Error::Validation(ValidationError {
                field: "days",
                reason: "days bitmask must only use bits 0-6 (Sunday=0 … Saturday=6)",
            }));
        }
        for &(h, m) in &times {
            if h > 23 {
                return Err(Error::Validation(ValidationError {
                    field: "hour",
                    reason: "hour must be in range 0-23",
                }));
            }
            if m > 59 {
                return Err(Error::Validation(ValidationError {
                    field: "minute",
                    reason: "minute must be in range 0-59",
                }));
            }
        }
        self.send_cmd(&command::encode_schedule(days, times)).await
    }
}

// ---------------------------------------------------------------------------
// Common utilities (all modes)
// ---------------------------------------------------------------------------

impl<M: Mode, T: AsyncTransport> AsyncCreate<M, T> {
    /// Get the robot model.
    #[must_use]
    pub fn model(&self) -> RobotModel {
        self.model
    }

    /// Consume the robot handle and return the underlying transport.
    ///
    /// # Note on robot state
    ///
    /// Dropping the returned transport (or this `AsyncCreate` handle directly) does
    /// **not** send any stop or shutdown command to the robot. The robot will
    /// continue executing the last commanded motion indefinitely. Call
    /// `stop()` or `power_off()` before releasing the handle if safe shutdown is needed.
    #[must_use]
    pub fn into_transport(self) -> T {
        self.transport
    }

    /// Borrow the underlying transport.
    #[must_use]
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Borrow the underlying transport mutably.
    ///
    /// # Warning
    ///
    /// Direct transport access bypasses the TypeState mode invariants, the
    fn reject_if_streaming(&self) -> Result<(), Error<T::Error>> {
        if self.streaming {
            return Err(Error::Validation(ValidationError {
                field: "stream",
                reason: "sensor queries cannot be sent while streaming; call toggle_stream(false) first",
            }));
        }
        Ok(())
    }

    /// Send raw bytes to the robot.
    async fn send_cmd(&mut self, data: &[u8]) -> Result<(), Error<T::Error>> {
        self.transport.write_all(data).await.map_err(Error::Io)?;
        self.transport.flush().await.map_err(Error::Io)?;
        Ok(())
    }

    /// Read exactly `buf.len()` bytes from the transport.
    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error<T::Error>> {
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
        self.transport.delay(self.model.mode_change_delay()).await;
    }

    /// Transition to a different mode (zero-cost: just changes the type parameter).
    #[inline(always)]
    fn transition<N: Mode>(self) -> AsyncCreate<N, T> {
        AsyncCreate {
            transport: self.transport,
            model: self.model,
            stream_parser: self.stream_parser,
            streaming: self.streaming,
            _mode: PhantomData,
        }
    }

    /// Like `transition`, but also resets streaming state.
    ///
    /// Use this when transitioning to a mode where the current stream session
    /// cannot continue (e.g. Off mode, or power_off → Passive).
    #[inline(always)]
    fn cleared_transition<N: Mode>(self) -> AsyncCreate<N, T> {
        AsyncCreate {
            transport: self.transport,
            model: self.model,
            stream_parser: StreamParser::new(),
            streaming: false,
            _mode: PhantomData,
        }
    }
}
