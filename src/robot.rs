//! Synchronous robot API with TypeState mode tracking.
//!
//! `Robot<M, T>` wraps a [`Transport`](crate::transport::Transport) and
//! encodes the current OI mode as a type parameter. Commands that require
//! specific modes are only available on the relevant `Robot<Safe, T>` or
//! `Robot<Full, T>` specialisations.

use crate::error::{ConnectError, Error, TransitionError};
use crate::mode::{Actuatable, Full, Mode, Off, Passive, Safe, SensorReadable};
use crate::protocol::command;
use crate::protocol::sensor::{self, SensorData};
use crate::protocol::stream::StreamParser;
use crate::transport::Transport;
use crate::types::{
    LedIntensity, MotorPower, OiMode, PowerLedColor, Radius, RobotModel, SongNumber, Velocity,
};
use std::marker::PhantomData;

/// A synchronous robot handle, parameterised by OI mode `M` and transport `T`.
///
/// Mode transitions consume `self` and return a new `Robot` in the target mode,
/// ensuring the compiler enforces valid mode sequences.
#[derive(Debug)]
pub struct Robot<M: Mode, T: Transport> {
    transport: T,
    model: RobotModel,
    stream_parser: StreamParser,
    _mode: PhantomData<M>,
}

// ---------------------------------------------------------------------------
// Construction (Off mode)
// ---------------------------------------------------------------------------

impl<T: Transport> Robot<Off, T> {
    /// Create a new robot handle wrapping the given transport.
    /// The robot is assumed to be in the `Off` state.
    pub fn new(transport: T, model: RobotModel) -> Self {
        Self {
            transport,
            model,
            stream_parser: StreamParser::new(),
            _mode: PhantomData,
        }
    }

    /// Send the START command and transition to Passive mode.
    pub fn start(mut self) -> Result<Robot<Passive, T>, ConnectError<T>> {
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

impl<T: Transport> Robot<Passive, T> {
    /// Transition to Safe mode.
    pub fn to_safe(mut self) -> Result<Robot<Safe, T>, TransitionError<Self>> {
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
    pub fn to_full(mut self) -> Result<Robot<Full, T>, TransitionError<Self>> {
        if let Err(e) = self.send_cmd(&command::encode_full()) {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        self.sleep_mode_change();
        Ok(self.transition())
    }
}

impl<T: Transport> Robot<Safe, T> {
    /// Transition to Full mode.
    pub fn to_full(mut self) -> Result<Robot<Full, T>, TransitionError<Self>> {
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
    pub fn to_passive(mut self) -> Result<Robot<Passive, T>, TransitionError<Self>> {
        if let Err(e) = self.send_cmd(&command::encode_start()) {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        self.sleep_mode_change();
        Ok(self.transition())
    }
}

impl<T: Transport> Robot<Full, T> {
    /// Fall back to Safe mode.
    pub fn to_safe(mut self) -> Result<Robot<Safe, T>, TransitionError<Self>> {
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
    pub fn to_passive(mut self) -> Result<Robot<Passive, T>, TransitionError<Self>> {
        if let Err(e) = self.send_cmd(&command::encode_start()) {
            return Err(TransitionError {
                robot: self,
                source: e,
            });
        }
        self.sleep_mode_change();
        Ok(self.transition())
    }
}

// ---------------------------------------------------------------------------
// Sensor reading (Passive, Safe, Full)
// ---------------------------------------------------------------------------

impl<M: SensorReadable, T: Transport> Robot<M, T> {
    /// Query a single sensor packet by ID and return the raw bytes.
    pub fn query_sensor_raw(&mut self, packet_id: u8) -> Result<Vec<u8>, Error> {
        self.send_cmd(&command::encode_sensors(packet_id))?;

        let info = crate::protocol::opcode::packet_info(packet_id)
            .ok_or_else(|| Error::Protocol(format!("unknown packet id {packet_id}")))?;
        let mut buf = vec![0u8; info.len as usize];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Query a single sensor packet and decode it.
    pub fn query_sensor(&mut self, packet_id: u8) -> Result<SensorData, Error> {
        let raw = self.query_sensor_raw(packet_id)?;
        let mut sd = SensorData::default();
        sd.decode_packet(packet_id, &raw)?;
        Ok(sd)
    }

    /// Query multiple sensors at once and decode all of them.
    pub fn query_list(&mut self, packet_ids: &[u8]) -> Result<SensorData, Error> {
        self.send_cmd(&command::encode_query_list(packet_ids))?;

        let expected_len = sensor::expected_data_len(packet_ids)?;
        let mut buf = vec![0u8; expected_len];
        self.read_exact(&mut buf)?;

        let mut sd = SensorData::default();
        sd.decode_packets(packet_ids, &buf)?;
        Ok(sd)
    }

    /// Read the robot's current OI mode from sensor data.
    pub fn read_oi_mode(&mut self) -> Result<OiMode, Error> {
        let sd = self.query_sensor(35)?;
        sd.oi_mode
            .ok_or_else(|| Error::Protocol("missing OI mode".into()))
    }

    /// Start streaming the given packet IDs.
    pub fn start_stream(&mut self, packet_ids: &[u8]) -> Result<(), Error> {
        self.stream_parser.set_packet_ids(packet_ids);
        self.send_cmd(&command::encode_stream(packet_ids))
    }

    /// Pause or resume the sensor stream.
    pub fn toggle_stream(&mut self, enable: bool) -> Result<(), Error> {
        self.send_cmd(&command::encode_toggle_stream(enable))
    }

    /// Read bytes from the transport and try to parse stream frames.
    pub fn poll_stream(&mut self) -> Result<Vec<SensorData>, Error> {
        let mut buf = [0u8; 256];
        let n = self.transport.read(&mut buf).map_err(Error::Io)?;
        if n == 0 {
            return Ok(Vec::new());
        }
        let results = self.stream_parser.feed(&buf[..n]);
        results.into_iter().collect()
    }
}

// ---------------------------------------------------------------------------
// Actuator commands (Safe, Full)
// ---------------------------------------------------------------------------

impl<M: Actuatable, T: Transport> Robot<M, T> {
    /// Drive with a given velocity and radius.
    pub fn drive(&mut self, velocity: Velocity, radius: Radius) -> Result<(), Error> {
        self.send_cmd(&command::encode_drive(
            velocity.to_mm_per_sec(),
            radius.to_mm(),
        ))
    }

    /// Drive wheels directly with individual velocities.
    pub fn drive_direct(&mut self, right: Velocity, left: Velocity) -> Result<(), Error> {
        self.send_cmd(&command::encode_drive_direct(
            right.to_mm_per_sec(),
            left.to_mm_per_sec(),
        ))
    }

    /// Drive wheels with PWM values.
    pub fn drive_pwm(&mut self, right: MotorPower, left: MotorPower) -> Result<(), Error> {
        self.send_cmd(&command::encode_drive_pwm(right.to_pwm(), left.to_pwm()))
    }

    /// Stop all motors (drive 0, 0).
    pub fn stop(&mut self) -> Result<(), Error> {
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
    ) -> Result<(), Error> {
        let bits =
            (debris as u8) | ((spot as u8) << 1) | ((dock as u8) << 2) | ((check_robot as u8) << 3);
        self.send_cmd(&command::encode_leds(bits, color.get(), intensity.get()))
    }

    /// Display ASCII characters on the 7-segment displays.
    pub fn set_digit_leds(&mut self, d3: u8, d2: u8, d1: u8, d0: u8) -> Result<(), Error> {
        self.send_cmd(&command::encode_digit_leds_ascii(d3, d2, d1, d0))
    }

    /// Define a song.
    pub fn define_song(&mut self, number: SongNumber, notes: &[(u8, u8)]) -> Result<(), Error> {
        let cmd = command::encode_song(number.get(), notes);
        self.send_cmd(&cmd)
    }

    /// Play a previously defined song.
    pub fn play_song(&mut self, number: SongNumber) -> Result<(), Error> {
        self.send_cmd(&command::encode_play(number.get()))
    }

    /// Set motor PWM (main brush, side brush, vacuum).
    pub fn set_motors_pwm(
        &mut self,
        main_brush: i8,
        side_brush: i8,
        vacuum: i8,
    ) -> Result<(), Error> {
        self.send_cmd(&command::encode_motors_pwm(main_brush, side_brush, vacuum))
    }
}

// ---------------------------------------------------------------------------
// Common utilities (all modes)
// ---------------------------------------------------------------------------

impl<M: Mode, T: Transport> Robot<M, T> {
    /// Get the robot model.
    pub fn model(&self) -> RobotModel {
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
    fn send_cmd(&mut self, data: &[u8]) -> Result<(), Error> {
        self.transport.write_all(data).map_err(Error::Io)?;
        self.transport.flush().map_err(Error::Io)?;
        Ok(())
    }

    /// Read exactly `buf.len()` bytes from the transport.
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        let mut offset = 0;
        while offset < buf.len() {
            let n = self.transport.read(&mut buf[offset..]).map_err(Error::Io)?;
            if n == 0 {
                return Err(Error::InsufficientData {
                    need: buf.len(),
                    got: offset,
                });
            }
            offset += n;
        }
        Ok(())
    }

    fn sleep_mode_change(&self) {
        std::thread::sleep(self.model.mode_change_delay());
    }

    /// Transition to a different mode (zero-cost: just changes the type parameter).
    fn transition<N: Mode>(self) -> Robot<N, T> {
        Robot {
            transport: self.transport,
            model: self.model,
            stream_parser: self.stream_parser,
            _mode: PhantomData,
        }
    }
}
