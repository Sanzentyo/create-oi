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
use crate::transport::{AsyncBaudConfigurable, AsyncTransport};
use crate::types::{
    AngularVelocity, ButtonBits, CleanMode, DayOfWeek, LedIntensity, MotorBits, MotorPower, OiMode,
    PowerLedColor, Radius, RobotModel, SongNote, SongNumber, Velocity, led_bits,
};
use core::marker::PhantomData;
use core::time::Duration;
use create_oi_protocol::command;
use create_oi_protocol::sensor::{self, SensorData};
use create_oi_protocol::stream::StreamParser;
use create_oi_protocol::types::{BaudRate, RadiusMm, VelocityMmPerSec};

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
    ///
    /// On Roomba 400, uses CONTROL (opcode 130) instead of SAFE (opcode 131) because
    /// Roomba 400 SCI does not implement the SAFE command for the Passive→Safe transition.
    pub async fn to_safe(
        mut self,
    ) -> Result<AsyncCreate<Safe, T>, TransitionError<Self, T::Error>> {
        let cmd = if self.model == RobotModel::Roomba400 {
            command::encode_control()
        } else {
            command::encode_safe()
        };
        if let Err(e) = self.send_cmd(&cmd).await {
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
    /// Per the OI spec, CLEAN/SPOT/MAX are valid from Passive, Safe, or Full mode.
    /// Also available from Safe and Full via the [`Actuatable`](crate::mode::Actuatable) impl.
    ///
    /// Cleaning modes:
    /// - [`CleanMode::Default`] — standard cleaning pattern
    /// - [`CleanMode::Spot`] — spot cleaning (small area)
    /// - [`CleanMode::Max`] — maximum cleaning (until battery depleted)
    ///
    /// **Note:** `CleanMode::Max` (opcode 136) is not available on Create 1; on that model
    /// opcode 136 triggers the Demo command instead. Returns `ValidationError` on Create 1.
    pub async fn clean(
        mut self,
        mode: CleanMode,
    ) -> Result<AsyncCreate<Passive, T>, TransitionError<Self, T::Error>> {
        if mode == CleanMode::Max && self.model == RobotModel::Create1 {
            return Err(TransitionError {
                create: self,
                source: Error::Validation(ValidationError {
                    field: "mode",
                    reason: "CleanMode::Max (OPCODE 136) is a Demo command on Create 1; use CleanMode::Default or CleanMode::Spot",
                }),
            });
        }
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
    /// Per the OI spec, SEEK_DOCK is valid from Passive, Safe, or Full mode.
    /// Also available from Safe and Full via the [`Actuatable`](crate::mode::Actuatable) impl.
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
        self.validate_packet_id(packet_id)?;
        let len = create_oi_protocol::opcode::packet_info(packet_id)
            .map(|p| p.len as usize)
            .or_else(|| create_oi_protocol::opcode::group_data_len(packet_id))
            .ok_or(Error::Protocol(
                create_oi_protocol::error::ProtocolError::UnknownPacketId(packet_id),
            ))?;
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
        self.validate_packet_id(packet_id)?;
        let len = create_oi_protocol::opcode::packet_info(packet_id)
            .map(|p| p.len as usize)
            .or_else(|| create_oi_protocol::opcode::group_data_len(packet_id))
            .ok_or(Error::Protocol(
                create_oi_protocol::error::ProtocolError::UnknownPacketId(packet_id),
            ))?;
        let mut buf = vec![0u8; len];
        self.send_cmd(&command::encode_sensors(packet_id)).await?;
        self.read_exact(&mut buf).await?;
        Ok(buf)
    }

    /// Query a single sensor packet and decode it.
    ///
    /// Group packet IDs (0-6, 100) are not supported by this typed decode API;
    /// use `query_sensor_raw_into()` to receive raw bytes for group packets.
    #[must_use = "query result must be used"]
    pub async fn query_sensor(&mut self, packet_id: u8) -> Result<SensorData, Error<T::Error>> {
        if create_oi_protocol::opcode::group_data_len(packet_id).is_some() {
            return Err(Error::Validation(ValidationError {
                field: "packet_id",
                reason: "group packet IDs (0-6, 100) are not decoded by query_sensor(); use query_sensor_raw_into() instead",
            }));
        }
        let mut buf = [0u8; 64]; // largest individual packet is well under 64 bytes
        let len = self.query_sensor_raw_into(packet_id, &mut buf).await?;
        let mut sd = SensorData::default();
        sd.decode_packet(packet_id, &buf[..len])?;
        Ok(sd)
    }

    /// Query multiple sensors at once and decode all of them.
    ///
    /// Validates all packet IDs before sending any bytes to the robot.
    /// Returns `ValidationError` if a sensor stream is currently active,
    /// if the list contains duplicate packet IDs, if the model does not support
    /// Query List (Roomba 400), or if a packet ID is not available on this model.
    ///
    /// Group packet IDs (0-6, 100) are accepted; the robot expands them and
    /// returns the constituent individual packets, which are then decoded.
    ///
    /// When the `alloc` feature is enabled this method accepts up to 255 IDs,
    /// matching the sync `Create::query_list` behaviour.  Without `alloc` the
    /// implementation uses a fixed stack buffer limited to 52 IDs (the size of
    /// Group-100); passing more IDs returns a `ValidationError`.
    #[cfg(feature = "alloc")]
    #[must_use = "query result must be used"]
    pub async fn query_list(&mut self, packet_ids: &[u8]) -> Result<SensorData, Error<T::Error>> {
        let expected_len = self.validate_query_list_common(packet_ids)?;
        let cmd = command::encode_query_list(packet_ids).map_err(Error::Protocol)?;
        self.send_cmd(&cmd).await?;

        let mut buf = vec![0u8; expected_len];
        self.read_exact(&mut buf).await?;

        let mut sd = SensorData::default();
        sd.decode_packets(packet_ids, &buf)?;
        Ok(sd)
    }

    /// Query multiple sensors at once and decode all of them (no-alloc variant).
    ///
    /// Uses a fixed stack buffer; limited to at most 52 packet IDs.
    /// Enable the `alloc` feature to remove this limit.
    #[cfg(not(feature = "alloc"))]
    #[must_use = "query result must be used"]
    pub async fn query_list(&mut self, packet_ids: &[u8]) -> Result<SensorData, Error<T::Error>> {
        let expected_len = self.validate_query_list_common(packet_ids)?;
        const ASYNC_MAX_IDS: usize = 52;
        if packet_ids.len() > ASYNC_MAX_IDS {
            return Err(Error::Validation(ValidationError {
                field: "packet_ids",
                reason: "async query_list supports at most 52 packet IDs without alloc; enable the alloc feature for longer lists",
            }));
        }
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
    /// if the packet ID list exceeds the protocol limit, if the total stream
    /// payload per cycle would exceed 255 bytes, if the list contains
    /// duplicate packet IDs, or if a packet ID is not available on this model.
    ///
    /// Group packet IDs (0-6, 100) are accepted; the per-cycle payload is
    /// computed as if each group were expanded to its constituent packets.
    ///
    /// When the `alloc` feature is enabled this method accepts up to 255 IDs.
    /// Without `alloc` the implementation uses a fixed stack buffer limited
    /// to 52 IDs (the size of Group-100).
    #[cfg(feature = "alloc")]
    pub async fn start_stream(&mut self, packet_ids: &[u8]) -> Result<(), Error<T::Error>> {
        self.validate_stream_init_params(packet_ids)?;
        let cmd = command::encode_stream(packet_ids).map_err(Error::Protocol)?;
        self.send_cmd(&cmd).await?;
        self.streaming = true;
        Ok(())
    }

    /// Start streaming the given packet IDs (no-alloc variant).
    ///
    /// Uses a fixed stack buffer; limited to at most 52 packet IDs.
    /// Enable the `alloc` feature to remove this limit.
    #[cfg(not(feature = "alloc"))]
    pub async fn start_stream(&mut self, packet_ids: &[u8]) -> Result<(), Error<T::Error>> {
        self.validate_stream_init_params(packet_ids)?;
        const ASYNC_MAX_IDS: usize = 52;
        if packet_ids.len() > ASYNC_MAX_IDS {
            return Err(Error::Validation(ValidationError {
                field: "packet_ids",
                reason: "async start_stream supports at most 52 packet IDs without alloc; enable the alloc feature for longer lists",
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
    #[must_use = "result must be checked"]
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
    ///
    /// Returns `ValidationError` if no stream is currently active (call
    /// `start_stream()` first).
    #[cfg(feature = "alloc")]
    pub async fn poll_stream(&mut self) -> Result<Vec<SensorData>, Error<T::Error>> {
        self.reject_if_not_streaming()?;
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
    /// Returns `ValidationError` if no stream is currently active.
    pub async fn poll_stream_with(
        &mut self,
        callback: impl FnMut(Result<SensorData, create_oi_protocol::error::ProtocolError>),
    ) -> Result<(), Error<T::Error>> {
        self.reject_if_not_streaming()?;
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

    /// Send the POWER command, powering down the robot.
    ///
    /// After this call the OI is in Off mode. The robot powers down and
    /// stops responding to OI commands. To resume, the robot must be
    /// physically woken (e.g. Clean button, dock) and then `start()` called.
    /// The stream state is cleared.
    pub async fn power_off(
        mut self,
    ) -> Result<AsyncCreate<Off, T>, TransitionError<Self, T::Error>> {
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
        self.send_cmd(&command::encode_drive(velocity.into(), radius.into()))
            .await
    }

    /// Drive wheels directly with individual velocities.
    ///
    /// Returns `ValidationError` if this model is Roomba 400 (OPCODE 145 is
    /// not supported; Roomba 400 only supports the Drive command, opcode 137).
    pub async fn drive_direct(
        &mut self,
        right: Velocity,
        left: Velocity,
    ) -> Result<(), Error<T::Error>> {
        if !self.model.supports_drive_direct() {
            return Err(Error::Validation(ValidationError {
                field: "model",
                reason: "drive_direct (OPCODE 145) requires Create 1 or Create 2; not supported on Roomba 400",
            }));
        }
        self.send_cmd(&command::encode_drive_direct(right.into(), left.into()))
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
        self.send_cmd(&command::encode_drive_pwm(right.into(), left.into()))
            .await
    }

    /// Stop all motors (both wheels to 0 mm/s).
    ///
    /// On Roomba 400, uses the Drive command (opcode 137) since Drive Direct
    /// (opcode 145) is not available. On Create 1 and Create 2, uses Drive Direct.
    pub async fn stop(&mut self) -> Result<(), Error<T::Error>> {
        if self.model == RobotModel::Roomba400 {
            // Roomba 400 does not support Drive Direct (opcode 145).
            // Drive at velocity 0 with the "straight" special radius (0x8000 wire value = i16::MIN).
            self.send_cmd(&command::encode_drive(
                VelocityMmPerSec::ZERO,
                RadiusMm::STRAIGHT,
            ))
            .await
        } else {
            self.send_cmd(&command::encode_drive_direct(
                VelocityMmPerSec::ZERO,
                VelocityMmPerSec::ZERO,
            ))
            .await
        }
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
        let bits = led_bits(self.model, debris, spot, dock, check_robot);
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
    /// - `vacuum`: range 0..=127. Values above 127 are invalid per the OI spec
    ///   (vacuum runs in one direction only) and are rejected without sending.
    ///
    /// Returns `ValidationError` if this model is not Create 2 (OPCODE 144 is
    /// not supported on Create 1 or Roomba 400).
    pub async fn set_motors_pwm(
        &mut self,
        main_brush: i8,
        side_brush: i8,
        vacuum: u8,
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
        if vacuum > 127 {
            return Err(Error::Validation(ValidationError {
                field: "vacuum",
                reason: "vacuum PWM must be 0..=127; values above 127 are invalid per OI spec",
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

    /// Set the scheduling LED indicators (opcode 162).
    ///
    /// - `day_leds`: bitmask for day-of-week LEDs; bits 0–6 = Sun–Sat. Bit 7 is reserved.
    /// - `schedule_leds`: bitmask for status icons; bit 0=colon, bit 1=AM/PM,
    ///   bit 2=clock icon, bit 3=schedule icon. Upper 4 bits are reserved.
    ///
    /// Returns `ValidationError` if this model is not Create 2 (OPCODE 162 is
    /// not supported on Create 1 or Roomba 400), or if reserved bits are set.
    pub async fn set_scheduling_leds(
        &mut self,
        day_leds: u8,
        schedule_leds: u8,
    ) -> Result<(), Error<T::Error>> {
        if !self.model.is_create2() {
            return Err(Error::Validation(ValidationError {
                field: "model",
                reason: "set_scheduling_leds (OPCODE 162) requires Create 2; not supported on Create 1 or Roomba 400",
            }));
        }
        if day_leds & 0x80 != 0 {
            return Err(Error::Validation(ValidationError {
                field: "day_leds",
                reason: "bit 7 of day_leds is reserved; only bits 0-6 (Sun-Sat) are valid",
            }));
        }
        if schedule_leds & 0xF0 != 0 {
            return Err(Error::Validation(ValidationError {
                field: "schedule_leds",
                reason: "upper 4 bits of schedule_leds are reserved; only bits 0-3 are valid",
            }));
        }
        self.send_cmd(&command::encode_scheduling_leds(day_leds, schedule_leds))
            .await
    }

    /// Drive using the unicycle (twist) model: linear velocity and angular velocity.
    ///
    /// Computes individual wheel speeds via differential drive kinematics:
    /// `right = v + ω × (axle/2)`, `left = v − ω × (axle/2)`.
    /// Wheel speeds are clamped to ±500 mm/s as required by the OI spec.
    ///
    /// Returns `ValidationError` if this model is Roomba 400 (uses Drive Direct
    /// internally, which is not supported on Roomba 400).
    pub async fn drive_twist(
        &mut self,
        velocity: Velocity,
        omega: AngularVelocity,
    ) -> Result<(), Error<T::Error>> {
        if !self.model.supports_drive_direct() {
            return Err(Error::Validation(ValidationError {
                field: "model",
                reason: "drive_twist (uses OPCODE 145) requires Create 1 or Create 2; not supported on Roomba 400",
            }));
        }
        let half_axle_mm = self.model.axle_length() * 500.0;
        let v_mm = velocity.to_mm_per_sec() as f32;
        let right = VelocityMmPerSec::from_raw(
            (libm::roundf(v_mm + omega.get() * half_axle_mm) as i16).clamp(-500, 500),
        );
        let left = VelocityMmPerSec::from_raw(
            (libm::roundf(v_mm - omega.get() * half_axle_mm) as i16).clamp(-500, 500),
        );
        self.send_cmd(&command::encode_drive_direct(right, left))
            .await
    }

    /// Initiate a cleaning cycle from Safe or Full mode. Transitions to Passive.
    ///
    /// Per the OI spec, CLEAN/SPOT/MAX are valid from Passive, Safe, or Full mode
    /// and always transition the robot to Passive mode.
    ///
    /// **Note:** `CleanMode::Max` (opcode 136) is not available on Create 1; on that model
    /// opcode 136 triggers the Demo command instead. Returns `ValidationError` on Create 1.
    pub async fn clean(
        mut self,
        mode: CleanMode,
    ) -> Result<AsyncCreate<Passive, T>, TransitionError<Self, T::Error>> {
        if mode == CleanMode::Max && self.model == RobotModel::Create1 {
            return Err(TransitionError {
                create: self,
                source: Error::Validation(ValidationError {
                    field: "mode",
                    reason: "CleanMode::Max (OPCODE 136) is a Demo command on Create 1; use CleanMode::Default or CleanMode::Spot",
                }),
            });
        }
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

    /// Seek the dock from Safe or Full mode. Transitions to Passive.
    ///
    /// Per the OI spec, SEEK_DOCK is valid from Passive, Safe, or Full mode
    /// and transitions the robot to Passive mode.
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
// Baud-rate switching (Passive, Safe, Full) — requires AsyncBaudConfigurable
// ---------------------------------------------------------------------------

impl<M: SensorReadable, T: AsyncTransport + AsyncBaudConfigurable> AsyncCreate<M, T> {
    /// Change the robot's baud rate (opcode 129).
    ///
    /// Sends the `BAUD` command, waits 100 ms for the robot to switch, then
    /// reconfigures the host serial connection to the new rate via
    /// [`AsyncBaudConfigurable::set_baud`].
    ///
    /// Available from Passive, Safe, and Full modes. The mode is unchanged after
    /// this call. All further commands must be sent at the new baud rate.
    ///
    /// # Note
    ///
    /// This method requires the transport to implement [`AsyncBaudConfigurable`].
    /// Tokio and smol transports support this; Embassy transports do not.
    pub async fn baud(&mut self, rate: BaudRate) -> Result<(), Error<T::Error>> {
        self.send_cmd(&command::encode_baud(rate)).await?;
        self.transport.delay(Duration::from_millis(100)).await;
        self.transport.set_baud(rate).await.map_err(Error::Io)
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
    /// Validates parameters common to both alloc and no-alloc `start_stream` variants.
    ///
    /// Checks: model supports streaming, no duplicate IDs, all IDs valid for this model,
    /// total per-cycle payload ≤ 255 bytes.  Returns `Ok(())` on success.
    fn validate_stream_init_params(&self, packet_ids: &[u8]) -> Result<(), Error<T::Error>> {
        if !self.model.supports_stream() {
            return Err(Error::Validation(ValidationError {
                field: "stream",
                reason: "sensor streaming is not supported by this robot model",
            }));
        }
        if sensor::has_duplicate_ids(packet_ids) {
            return Err(Error::Validation(ValidationError {
                field: "packet_ids",
                reason: "duplicate packet IDs are not allowed in start_stream",
            }));
        }
        for &id in packet_ids {
            self.validate_packet_id(id)?;
        }
        let payload_bytes =
            packet_ids
                .iter()
                .try_fold(0usize, |acc, &id| -> Result<usize, Error<T::Error>> {
                    if let Some(info) = create_oi_protocol::opcode::packet_info(id) {
                        Ok(acc + 1 + info.len as usize)
                    } else if let Some(members) = create_oi_protocol::opcode::group_packet_ids(id) {
                        let data_len = create_oi_protocol::opcode::group_data_len(id).unwrap_or(0);
                        Ok(acc + members.len() + data_len)
                    } else {
                        Err(Error::Protocol(
                            create_oi_protocol::error::ProtocolError::UnknownPacketId(id),
                        ))
                    }
                })?;
        if payload_bytes > 255 {
            return Err(Error::Validation(ValidationError {
                field: "packet_ids",
                reason: "stream payload per cycle exceeds OI limit of 255 bytes",
            }));
        }
        Ok(())
    }

    /// Validates parameters common to both alloc and no-alloc `query_list` variants.
    ///
    /// Checks: not currently streaming, model supports query_list, no duplicate IDs,
    /// all IDs valid for this model.  Returns the expected total response data length.
    fn validate_query_list_common(&self, packet_ids: &[u8]) -> Result<usize, Error<T::Error>> {
        self.reject_if_streaming()?;
        if !self.model.supports_query_list() {
            return Err(Error::Validation(ValidationError {
                field: "model",
                reason: "query_list (OPCODE 149) is not supported on Roomba 400; use query_sensor_raw with group IDs 0–3",
            }));
        }
        if sensor::has_duplicate_ids(packet_ids) {
            return Err(Error::Validation(ValidationError {
                field: "packet_ids",
                reason: "duplicate packet IDs are not allowed in query_list",
            }));
        }
        for &id in packet_ids {
            self.validate_packet_id(id)?;
        }
        Ok(sensor::expected_data_len(packet_ids)?)
    }

    fn reject_if_streaming(&self) -> Result<(), Error<T::Error>> {
        if self.streaming {
            return Err(Error::Validation(ValidationError {
                field: "stream",
                reason: "sensor queries cannot be sent while streaming; call toggle_stream(false) first",
            }));
        }
        Ok(())
    }

    fn reject_if_not_streaming(&self) -> Result<(), Error<T::Error>> {
        if !self.streaming {
            return Err(Error::Validation(ValidationError {
                field: "stream",
                reason: "poll_stream requires an active stream; call start_stream() first",
            }));
        }
        Ok(())
    }

    fn validate_packet_id(&self, packet_id: u8) -> Result<(), Error<T::Error>> {
        use create_oi_protocol::opcode;
        if opcode::group_data_len(packet_id).is_some() {
            if !self.model.supports_group_packet(packet_id) {
                return Err(Error::Validation(ValidationError {
                    field: "packet_id",
                    reason: "sensor group packet is not supported by this robot model",
                }));
            }
        } else if opcode::packet_info(packet_id).is_some() {
            if !self.model.supports_individual_sensor_packets() {
                return Err(Error::Validation(ValidationError {
                    field: "packet_id",
                    reason: "individual sensor packets are not supported by Roomba 400; use group IDs 0–3",
                }));
            }
            if packet_id > self.model.max_individual_sensor_packet_id() {
                return Err(Error::Validation(ValidationError {
                    field: "packet_id",
                    reason: "sensor packet ID is not available on this robot model",
                }));
            }
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

    async fn sleep_mode_change(&mut self) {
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
    /// cannot continue (e.g. Off mode, or power_off → Off).
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
