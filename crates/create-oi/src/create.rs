//! Synchronous robot API with TypeState mode tracking.
//!
//! `Create<M, T>` wraps a [`Transport`](crate::transport::Transport) and
//! encodes the current OI mode as a type parameter. Commands that require
//! specific modes are only available on the relevant `Create<Safe, T>` or
//! `Create<Full, T>` specialisations.
//!
//! This module requires the `std` feature (blocking I/O + `thread::sleep`).

use crate::error::{ConnectError, Error, TransitionError, ValidationError};
use crate::mode::{Actuatable, Full, FullControl, Mode, Off, Passive, Safe, SensorReadable};
use crate::transport::Transport;
use crate::types::{
    AngularVelocity, ButtonBits, CleanMode, CreateRobotModel, DayOfWeek, LedIntensity, MotorBits,
    MotorPower, OiMode, PowerLedColor, Radius, SongNote, SongNumber, Velocity,
};
use create_oi_protocol::command;
use create_oi_protocol::sensor::{self, SensorData};
use create_oi_protocol::stream::StreamParser;
use std::marker::PhantomData;

/// A synchronous robot handle, parameterised by OI mode `M` and transport `T`.
///
/// Mode transitions consume `self` and return a new `Create` in the target mode,
/// ensuring the compiler enforces valid mode sequences.
///
/// # TypeState vs. actual robot mode
///
/// The mode type parameter tracks the **last commanded mode**, not the robot's
/// current hardware state. The robot can change mode autonomously (e.g. safety
/// events, button presses). Call [`read_oi_mode`](Create::read_oi_mode) to
/// read the actual mode from the robot.
#[derive(Debug)]
pub struct Create<M: Mode, T: Transport> {
    transport: T,
    model: CreateRobotModel,
    stream_parser: StreamParser,
    /// `true` while a sensor stream is active (after `start_stream`, before `toggle_stream(false)`).
    streaming: bool,
    _mode: PhantomData<M>,
}

// ---------------------------------------------------------------------------
// Construction (Off mode)
// ---------------------------------------------------------------------------

impl<T: Transport> Create<Off, T> {
    /// Create a new robot handle wrapping the given transport.
    /// The robot is assumed to be in the `Off` state.
    pub fn new(transport: T, model: CreateRobotModel) -> Self {
        Self {
            transport,
            model,
            stream_parser: StreamParser::new(),
            streaming: false,
            _mode: PhantomData,
        }
    }

    /// Send the START command and transition to Passive mode.
    pub fn start(mut self) -> Result<Create<Passive, T>, ConnectError<T, std::io::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_start()) {
            return Err(ConnectError {
                transport: self.transport,
                source: e,
            });
        }
        self.sleep_mode_change();
        Ok(self.transition())
    }
}

// ---------------------------------------------------------------------------
// Mode transitions (available from Passive, Safe, Full)
// ---------------------------------------------------------------------------

impl<T: Transport> Create<Passive, T> {
    /// Transition to Safe mode.
    pub fn to_safe(mut self) -> Result<Create<Safe, T>, TransitionError<Self, std::io::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_safe()) {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        self.sleep_mode_change();
        Ok(self.transition())
    }

    /// Transition to Full mode.
    pub fn to_full(mut self) -> Result<Create<Full, T>, TransitionError<Self, std::io::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_full()) {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        self.sleep_mode_change();
        Ok(self.transition())
    }

    /// Send STOP and transition to Off mode.
    ///
    /// The OI session is ended. To reconnect, create a new
    /// `Create::<Off, _>::new(transport, model)`.
    pub fn to_off(mut self) -> Result<Create<Off, T>, TransitionError<Self, std::io::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_stop()) {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        Ok(self.transition())
    }
}

impl<T: Transport> Create<Safe, T> {
    /// Transition to Full mode.
    pub fn to_full(mut self) -> Result<Create<Full, T>, TransitionError<Self, std::io::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_full()) {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        self.sleep_mode_change();
        Ok(self.transition())
    }

    /// Fall back to Passive mode (sends START).
    pub fn to_passive(
        mut self,
    ) -> Result<Create<Passive, T>, TransitionError<Self, std::io::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_start()) {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        self.sleep_mode_change();
        Ok(self.transition())
    }

    /// Send STOP and transition to Off mode.
    pub fn to_off(mut self) -> Result<Create<Off, T>, TransitionError<Self, std::io::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_stop()) {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        Ok(self.transition())
    }
}

impl<T: Transport> Create<Full, T> {
    /// Fall back to Safe mode.
    pub fn to_safe(mut self) -> Result<Create<Safe, T>, TransitionError<Self, std::io::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_safe()) {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        self.sleep_mode_change();
        Ok(self.transition())
    }

    /// Fall back to Passive mode (sends START).
    pub fn to_passive(
        mut self,
    ) -> Result<Create<Passive, T>, TransitionError<Self, std::io::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_start()) {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        self.sleep_mode_change();
        Ok(self.transition())
    }

    /// Send STOP and transition to Off mode.
    pub fn to_off(mut self) -> Result<Create<Off, T>, TransitionError<Self, std::io::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_stop()) {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        Ok(self.transition())
    }
}

// ---------------------------------------------------------------------------
// Sensor reading (Passive, Safe, Full)
// ---------------------------------------------------------------------------

impl<M: SensorReadable, T: Transport> Create<M, T> {
    /// Query a single sensor packet by ID and return the raw bytes.
    ///
    /// Validates the packet ID before sending any bytes to the robot.
    /// Returns `ValidationError` if a sensor stream is currently active;
    /// call `toggle_stream(false)` first to pause the stream.
    #[must_use = "query result must be used"]
    pub fn query_sensor_raw(&mut self, packet_id: u8) -> Result<Vec<u8>, Error<std::io::Error>> {
        self.reject_if_streaming()?;
        let info = create_oi_protocol::opcode::packet_info(packet_id).ok_or(Error::Protocol(
            create_oi_protocol::error::ProtocolError::UnknownPacketId(packet_id),
        ))?;
        let mut buf = vec![0u8; info.len as usize];
        self.send_cmd(&command::encode_sensors(packet_id))?;
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Query a single sensor packet by ID into a caller-provided buffer.
    ///
    /// Validates the packet ID and buffer size before sending any bytes to the robot.
    /// Returns the number of bytes written.
    /// Returns `ValidationError` if a sensor stream is currently active.
    #[must_use = "query result must be used"]
    pub fn query_sensor_raw_into(
        &mut self,
        packet_id: u8,
        buf: &mut [u8],
    ) -> Result<usize, Error<std::io::Error>> {
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
        self.send_cmd(&command::encode_sensors(packet_id))?;
        self.read_exact(&mut buf[..len])?;
        Ok(len)
    }

    /// Query a single sensor packet and decode it.
    #[must_use = "query result must be used"]
    pub fn query_sensor(&mut self, packet_id: u8) -> Result<SensorData, Error<std::io::Error>> {
        let raw = self.query_sensor_raw(packet_id)?;
        let mut sd = SensorData::default();
        sd.decode_packet(packet_id, &raw)?;
        Ok(sd)
    }

    /// Query multiple sensors at once and decode all of them.
    ///
    /// Validates all packet IDs before sending any bytes to the robot.
    /// Returns `ValidationError` if a sensor stream is currently active.
    #[must_use = "query result must be used"]
    pub fn query_list(&mut self, packet_ids: &[u8]) -> Result<SensorData, Error<std::io::Error>> {
        self.reject_if_streaming()?;
        let expected_len = sensor::expected_data_len(packet_ids)?;
        let cmd = command::encode_query_list(packet_ids).map_err(Error::Protocol)?;
        self.send_cmd(&cmd)?;

        let mut buf = vec![0u8; expected_len];
        self.read_exact(&mut buf)?;

        let mut sd = SensorData::default();
        sd.decode_packets(packet_ids, &buf)?;
        Ok(sd)
    }

    /// Read the robot's current OI mode from sensor data.
    #[must_use = "query result must be used"]
    pub fn read_oi_mode(&mut self) -> Result<OiMode, Error<std::io::Error>> {
        let sd = self.query_sensor(35)?;
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
    /// Sets the internal streaming flag; while streaming, sensor query methods
    /// (`query_sensor_raw`, `query_sensor_raw_into`, `query_list`) return
    /// `ValidationError`. Call `toggle_stream(false)` to pause the stream first.
    pub fn start_stream(&mut self, packet_ids: &[u8]) -> Result<(), Error<std::io::Error>> {
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
        let cmd = command::encode_stream(packet_ids).map_err(Error::Protocol)?;
        self.send_cmd(&cmd)?;
        self.streaming = true;
        Ok(())
    }

    /// Pause or resume the sensor stream.
    ///
    /// Returns an error if this robot model does not support sensor streaming.
    /// Updates the internal streaming flag (`enable = false` allows sensor queries again).
    pub fn toggle_stream(&mut self, enable: bool) -> Result<(), Error<std::io::Error>> {
        if !self.model.supports_stream() {
            return Err(Error::Validation(ValidationError {
                field: "stream",
                reason: "sensor streaming is not supported by this robot model",
            }));
        }
        self.send_cmd(&command::encode_toggle_stream(enable))?;
        self.streaming = enable;
        Ok(())
    }

    /// Read bytes from the transport and try to parse stream frames.
    pub fn poll_stream(&mut self) -> Result<Vec<SensorData>, Error<std::io::Error>> {
        let mut buf = [0u8; 256];
        let n = self.transport.read(&mut buf).map_err(Error::Io)?;
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
    pub fn poll_stream_with(
        &mut self,
        callback: impl FnMut(Result<SensorData, create_oi_protocol::error::ProtocolError>),
    ) -> Result<(), Error<std::io::Error>> {
        let mut buf = [0u8; 256];
        let n = self.transport.read(&mut buf).map_err(Error::Io)?;
        if n == 0 {
            return Err(Error::Protocol(
                create_oi_protocol::error::ProtocolError::InsufficientData { need: 1, got: 0 },
            ));
        }
        self.stream_parser.feed_with(&buf[..n], callback);
        Ok(())
    }

    /// Initiate a cleaning cycle. Transitions the robot to Passive mode.
    ///
    /// The OI spec defines three cleaning modes:
    /// - [`CleanMode::Default`] — standard cleaning pattern
    /// - [`CleanMode::Spot`] — spot cleaning (small area)
    /// - [`CleanMode::Max`] — maximum cleaning (until battery depleted)
    pub fn clean(
        mut self,
        mode: CleanMode,
    ) -> Result<Create<Passive, T>, TransitionError<Self, std::io::Error>> {
        let cmd = match mode {
            CleanMode::Default => command::encode_clean(),
            CleanMode::Spot => command::encode_spot(),
            CleanMode::Max => command::encode_max(),
        };
        if let Err(e) = self.send_cmd(&cmd) {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        Ok(self.transition())
    }

    /// Seek the dock. Transitions the robot to Passive mode.
    pub fn seek_dock(
        mut self,
    ) -> Result<Create<Passive, T>, TransitionError<Self, std::io::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_dock()) {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        Ok(self.transition())
    }

    /// Power off the robot and return the underlying transport.
    ///
    /// After this call the robot is powered down. To reconnect, wrap the
    /// returned transport in a new `Create::<Off, _>::new(transport, model)`.
    pub fn power_off(mut self) -> Result<T, TransitionError<Self, std::io::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_power()) {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        Ok(self.transport)
    }

    /// Reset the robot and return the underlying transport.
    ///
    /// After this call the robot reboots. The serial connection may need to be
    /// re-opened before creating a new `Create::<Off, _>::new(transport, model)`.
    pub fn reset(mut self) -> Result<T, TransitionError<Self, std::io::Error>> {
        if let Err(e) = self.send_cmd(&command::encode_reset()) {
            return Err(TransitionError {
                robot: self,
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
    pub fn define_song(
        &mut self,
        number: SongNumber,
        notes: &[SongNote],
    ) -> Result<(), Error<std::io::Error>> {
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
        let mut raw = [(0u8, 0u8); 16];
        let count = notes.len().min(16);
        for (i, n) in notes.iter().enumerate().take(count) {
            raw[i] = (n.midi_note, n.duration_64ths);
        }
        let cmd = command::encode_song(number.get(), &raw[..count]).map_err(Error::Protocol)?;
        self.send_cmd(&cmd)
    }

    /// Play a previously defined song.
    ///
    /// Songs can be played in Passive, Safe, and Full mode per the OI spec.
    /// Returns `ValidationError` if the song slot exceeds this model's maximum.
    pub fn play_song(&mut self, number: SongNumber) -> Result<(), Error<std::io::Error>> {
        if number.get() > self.model.max_song_number() {
            return Err(Error::Validation(ValidationError {
                field: "number",
                reason: "song slot exceeds this model's maximum",
            }));
        }
        self.send_cmd(&command::encode_play(number.get()))
    }
}

// ---------------------------------------------------------------------------
// Actuator commands (Safe, Full)
// ---------------------------------------------------------------------------

impl<M: Actuatable, T: Transport> Create<M, T> {
    /// Drive with a given velocity and radius.
    pub fn drive(
        &mut self,
        velocity: Velocity,
        radius: Radius,
    ) -> Result<(), Error<std::io::Error>> {
        self.send_cmd(&command::encode_drive(
            velocity.to_mm_per_sec(),
            radius.to_mm(),
        ))
    }

    /// Drive wheels directly with individual velocities.
    pub fn drive_direct(
        &mut self,
        right: Velocity,
        left: Velocity,
    ) -> Result<(), Error<std::io::Error>> {
        self.send_cmd(&command::encode_drive_direct(
            right.to_mm_per_sec(),
            left.to_mm_per_sec(),
        ))
    }

    /// Drive wheels with PWM values.
    pub fn drive_pwm(
        &mut self,
        right: MotorPower,
        left: MotorPower,
    ) -> Result<(), Error<std::io::Error>> {
        self.send_cmd(&command::encode_drive_pwm(right.to_pwm(), left.to_pwm()))
    }

    /// Stop all motors (drive 0, 0).
    pub fn stop(&mut self) -> Result<(), Error<std::io::Error>> {
        self.send_cmd(&command::encode_drive(0, 0))
    }

    /// Set LEDs.
    pub fn set_leds(
        &mut self,
        debris: bool,
        spot: bool,
        dock: bool,
        check_robot: bool,
        color: PowerLedColor,
        intensity: LedIntensity,
    ) -> Result<(), Error<std::io::Error>> {
        let bits =
            (debris as u8) | ((spot as u8) << 1) | ((dock as u8) << 2) | ((check_robot as u8) << 3);
        self.send_cmd(&command::encode_leds(bits, color.get(), intensity.get()))
    }

    /// Display ASCII characters on the 7-segment displays.
    ///
    /// Each character must be a printable ASCII byte (32–126). Non-printable
    /// bytes are rejected before any bytes are sent to the robot.
    pub fn set_digit_leds(
        &mut self,
        d3: u8,
        d2: u8,
        d1: u8,
        d0: u8,
    ) -> Result<(), Error<std::io::Error>> {
        for (name, val) in [("d3", d3), ("d2", d2), ("d1", d1), ("d0", d0)] {
            if !(32..=126).contains(&val) {
                return Err(Error::Validation(ValidationError {
                    field: name,
                    reason: "digit LED character must be printable ASCII (32–126)",
                }));
            }
        }
        self.send_cmd(&command::encode_digit_leds_ascii(d3, d2, d1, d0))
    }

    /// Set motor PWM (main brush, side brush, vacuum).
    ///
    /// - `main_brush` and `side_brush`: range -127..=127 (i8::MIN = -128 is rejected).
    ///   Positive = forward direction, negative = reverse.
    /// - `vacuum`: range 0..=127. Negative values are invalid per the OI spec
    ///   (vacuum runs in one direction only) and are rejected without sending.
    pub fn set_motors_pwm(
        &mut self,
        main_brush: i8,
        side_brush: i8,
        vacuum: i8,
    ) -> Result<(), Error<std::io::Error>> {
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
    }

    /// Enable or disable motors with direction control.
    pub fn set_motors(&mut self, motors: MotorBits) -> Result<(), Error<std::io::Error>> {
        self.send_cmd(&command::encode_motors(motors.to_raw()))
    }

    /// Set raw 7-segment digit LEDs.
    ///
    /// Each byte controls one digit: bits 0–6 = segments A–G, bit 7 = decimal point.
    /// `d3` is the leftmost digit and `d0` is the rightmost.
    pub fn set_digit_leds_raw(
        &mut self,
        d3: u8,
        d2: u8,
        d1: u8,
        d0: u8,
    ) -> Result<(), Error<std::io::Error>> {
        self.send_cmd(&command::encode_digit_leds_raw(d3, d2, d1, d0))
    }

    /// Drive using the unicycle (twist) model: linear velocity and angular velocity.
    ///
    /// Computes individual wheel speeds via differential drive kinematics:
    /// `right = v + ω × (axle/2)`, `left = v − ω × (axle/2)`.
    /// Wheel speeds are clamped to ±500 mm/s as required by the OI spec.
    pub fn drive_twist(
        &mut self,
        velocity: Velocity,
        omega: AngularVelocity,
    ) -> Result<(), Error<std::io::Error>> {
        let half_axle_mm = self.model.axle_length() * 500.0;
        let v_mm = velocity.to_mm_per_sec() as f32;
        let right_mm = (libm::roundf(v_mm + omega.get() * half_axle_mm) as i16).clamp(-500, 500);
        let left_mm = (libm::roundf(v_mm - omega.get() * half_axle_mm) as i16).clamp(-500, 500);
        self.send_cmd(&command::encode_drive_direct(right_mm, left_mm))
    }
}

// ---------------------------------------------------------------------------
// Full-control commands (Full only)
// ---------------------------------------------------------------------------

impl<M: FullControl, T: Transport> Create<M, T> {
    /// Simulate button presses on the robot (Full mode only).
    pub fn simulate_buttons(&mut self, buttons: ButtonBits) -> Result<(), Error<std::io::Error>> {
        self.send_cmd(&command::encode_buttons(buttons.to_raw()))
    }

    /// Set the robot's internal date and time (Full mode only).
    ///
    /// Returns an error if `hour` is not 0–23 or `minute` is not 0–59.
    pub fn set_date(
        &mut self,
        day: DayOfWeek,
        hour: u8,
        minute: u8,
    ) -> Result<(), Error<std::io::Error>> {
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
    }

    /// Set the weekly cleaning schedule (Full mode only).
    ///
    /// `days`: bitmask of scheduled days (bit 0=Sunday, bit 6=Saturday). Bits 7 must be 0.
    /// `times`: (hour, minute) for each day, starting with Sunday.
    ///
    /// Returns an error if `days` has reserved bits set or any time is out of range.
    pub fn set_schedule(
        &mut self,
        days: u8,
        times: [(u8, u8); 7],
    ) -> Result<(), Error<std::io::Error>> {
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
        self.send_cmd(&command::encode_schedule(days, times))
    }
}

// ---------------------------------------------------------------------------
// Common utilities (all modes)
// ---------------------------------------------------------------------------

impl<M: Mode, T: Transport> Create<M, T> {
    /// Get the robot model.
    #[must_use]
    pub fn model(&self) -> CreateRobotModel {
        self.model
    }

    /// Consume the robot and return the underlying transport.
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
    /// # Caution
    ///
    /// Directly writing to or reading from the transport bypasses both the
    /// TypeState mode invariants and the internal [`StreamParser`] state.
    /// Only use this for low-level diagnostics or protocol extensions where
    /// you can guarantee correctness externally.
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// Send raw bytes to the robot.
    fn send_cmd(&mut self, data: &[u8]) -> Result<(), Error<std::io::Error>> {
        self.transport.write_all(data).map_err(Error::Io)?;
        self.transport.flush().map_err(Error::Io)?;
        Ok(())
    }

    /// Read exactly `buf.len()` bytes from the transport.
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error<std::io::Error>> {
        let mut offset = 0;
        while offset < buf.len() {
            let n = self.transport.read(&mut buf[offset..]).map_err(Error::Io)?;
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

    fn sleep_mode_change(&self) {
        std::thread::sleep(self.model.mode_change_delay());
    }

    fn reject_if_streaming(&self) -> Result<(), Error<std::io::Error>> {
        if self.streaming {
            return Err(Error::Validation(ValidationError {
                field: "stream",
                reason: "sensor queries cannot be sent while streaming; call toggle_stream(false) first",
            }));
        }
        Ok(())
    }

    /// Transition to a different mode (zero-cost: just changes the type parameter).
    #[inline(always)]
    fn transition<N: Mode>(self) -> Create<N, T> {
        Create {
            transport: self.transport,
            model: self.model,
            stream_parser: self.stream_parser,
            streaming: self.streaming,
            _mode: PhantomData,
        }
    }
}
