//! Sans-IO sensor packet parsing.
//!
//! Each sensor value is decoded from raw big-endian bytes using manual
//! `from_be_bytes` — zero allocation, zero copy.

use crate::error::ProtocolError;
use crate::types::{ChargingState, IrChar, OiMode};

use crate::opcode::{group_data_len, group_packet_ids, packet_info};

// ---------------------------------------------------------------------------
// Raw sensor value decoding
// ---------------------------------------------------------------------------

/// Decode a single unsigned value from big-endian bytes.
#[inline(always)]
pub const fn decode_u8(data: &[u8]) -> Result<u8, ProtocolError> {
    if data.is_empty() {
        Err(ProtocolError::InsufficientData { need: 1, got: 0 })
    } else {
        Ok(data[0])
    }
}

/// Decode a signed 8-bit value.
#[inline(always)]
pub const fn decode_i8(data: &[u8]) -> Result<i8, ProtocolError> {
    if data.is_empty() {
        Err(ProtocolError::InsufficientData { need: 1, got: 0 })
    } else {
        Ok(data[0] as i8)
    }
}

/// Decode a 16-bit unsigned value from big-endian bytes.
#[inline(always)]
pub const fn decode_u16(data: &[u8]) -> Result<u16, ProtocolError> {
    if data.len() < 2 {
        Err(ProtocolError::InsufficientData {
            need: 2,
            got: data.len(),
        })
    } else {
        Ok(u16::from_be_bytes([data[0], data[1]]))
    }
}

/// Decode a 16-bit signed value from big-endian bytes.
#[inline(always)]
pub const fn decode_i16(data: &[u8]) -> Result<i16, ProtocolError> {
    if data.len() < 2 {
        Err(ProtocolError::InsufficientData {
            need: 2,
            got: data.len(),
        })
    } else {
        Ok(i16::from_be_bytes([data[0], data[1]]))
    }
}

// ---------------------------------------------------------------------------
// Typed sensor values
// ---------------------------------------------------------------------------

/// Raw sensor data parsed from a query or stream response.
///
/// This struct holds the fully decoded sensor values. Fields are `Option`
/// to support partial queries (not all packets may be present).
#[derive(Debug, Clone, Default)]
pub struct SensorData {
    // --- Group 1: bumps, cliffs, wall (IDs 7–16) ---
    pub bumps_and_wheeldrops: Option<u8>,
    pub wall: Option<bool>,
    pub cliff_left: Option<bool>,
    pub cliff_front_left: Option<bool>,
    pub cliff_front_right: Option<bool>,
    pub cliff_right: Option<bool>,
    pub virtual_wall: Option<bool>,
    pub overcurrents: Option<u8>,
    pub dirt_detect_left: Option<u8>,
    pub dirt_detect_right: Option<u8>,

    // --- Group 2: IR, buttons, motion (IDs 17–20) ---
    pub ir_omni: Option<IrChar>,
    pub buttons: Option<u8>,
    /// Estimated distance traveled since this field was last read, in mm.
    ///
    /// **This is a delta accumulator**, not an absolute position. The robot
    /// resets the internal counter each time this packet is read (or streamed).
    /// Positive = forward, negative = backward. Accumulates as a signed 16-bit
    /// integer and may saturate at ±32767 mm between reads.
    pub distance_delta_mm: Option<i16>,
    /// Estimated heading change since this field was last read, in degrees.
    ///
    /// **This is a delta accumulator**, not an absolute heading. The robot
    /// resets the internal counter each time this packet is read (or streamed).
    /// Positive = counter-clockwise, negative = clockwise. Accumulates as a
    /// signed 16-bit integer and may saturate at ±32767° between reads.
    pub angle_delta_deg: Option<i16>,

    // --- Group 3: battery (IDs 21–26) ---
    pub charging_state: Option<ChargingState>,
    pub voltage: Option<u16>,
    pub current: Option<i16>,
    pub temperature: Option<i8>,
    pub battery_charge: Option<u16>,
    pub battery_capacity: Option<u16>,

    // --- Group 4: analog/digital (IDs 27–34) ---
    pub wall_signal: Option<u16>,
    pub cliff_left_signal: Option<u16>,
    pub cliff_front_left_signal: Option<u16>,
    pub cliff_front_right_signal: Option<u16>,
    pub cliff_right_signal: Option<u16>,
    pub cargo_bay_digital_inputs: Option<u8>,
    pub cargo_bay_analog_signal: Option<u16>,
    pub charging_sources: Option<u8>,

    // --- Group 5: OI state (IDs 35–42) ---
    pub oi_mode: Option<OiMode>,
    pub song_number: Option<u8>,
    pub song_playing: Option<bool>,
    pub num_stream_packets: Option<u8>,
    pub requested_velocity: Option<i16>,
    pub requested_radius: Option<i16>,
    pub requested_right_velocity: Option<i16>,
    pub requested_left_velocity: Option<i16>,

    // --- Encoders & light bumper (IDs 43–51) ---
    /// Left wheel encoder count (packet 43). Wraps modulo 65536 (u16 rollover).
    ///
    /// To compute a signed delta across rollover: `wrapping_sub` the previous reading.
    pub left_encoder_counts: Option<u16>,
    /// Right wheel encoder count (packet 44). Wraps modulo 65536 (u16 rollover).
    ///
    /// To compute a signed delta across rollover: `wrapping_sub` the previous reading.
    pub right_encoder_counts: Option<u16>,
    pub light_bumper: Option<u8>,
    pub light_bump_left_signal: Option<u16>,
    pub light_bump_front_left_signal: Option<u16>,
    pub light_bump_center_left_signal: Option<u16>,
    pub light_bump_center_right_signal: Option<u16>,
    pub light_bump_front_right_signal: Option<u16>,
    pub light_bump_right_signal: Option<u16>,

    // --- IR left/right, motor currents, stasis (IDs 52–58) ---
    pub ir_left: Option<IrChar>,
    pub ir_right: Option<IrChar>,
    pub left_motor_current: Option<i16>,
    pub right_motor_current: Option<i16>,
    pub main_brush_motor_current: Option<i16>,
    pub side_brush_motor_current: Option<i16>,
    pub stasis: Option<u8>,
}

impl SensorData {
    /// Decode a single packet from `data` starting at offset 0,
    /// and store the result into the appropriate field.
    ///
    /// Returns the number of bytes consumed.
    pub fn decode_packet(&mut self, id: u8, data: &[u8]) -> Result<usize, ProtocolError> {
        let info = packet_info(id).ok_or(ProtocolError::UnknownPacketId(id))?;
        let len = info.len as usize;
        if data.len() < len {
            return Err(ProtocolError::InsufficientData {
                need: len,
                got: data.len(),
            });
        }
        let slice = &data[..len];
        self.store_value(id, slice)?;
        Ok(len)
    }

    /// Decode a sequence of packets (e.g., from a query list response).
    /// `ids` is the ordered list of packet IDs. `data` is the concatenated bytes.
    ///
    /// Group packet IDs (0-6, 100) are expanded to their constituent individual
    /// packet IDs before decoding; the robot always returns individual packet data
    /// in the same order.
    pub fn decode_packets(&mut self, ids: &[u8], data: &[u8]) -> Result<(), ProtocolError> {
        let mut offset = 0;
        for &id in ids {
            if let Some(members) = group_packet_ids(id) {
                for &mid in members {
                    let consumed = self.decode_packet(mid, &data[offset..])?;
                    offset += consumed;
                }
            } else {
                let consumed = self.decode_packet(id, &data[offset..])?;
                offset += consumed;
            }
        }
        if offset != data.len() {
            return Err(ProtocolError::UnexpectedData {
                trailing: data.len() - offset,
            });
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Bumps and wheeldrops (packet 7)
    // -----------------------------------------------------------------------

    /// Returns `true` if the right bump sensor is active.
    #[inline(always)]
    pub const fn is_right_bump(&self) -> Option<bool> {
        match self.bumps_and_wheeldrops {
            Some(b) => Some(b & 0x01 != 0),
            None => None,
        }
    }

    /// Returns `true` if the left bump sensor is active.
    #[inline(always)]
    pub const fn is_left_bump(&self) -> Option<bool> {
        match self.bumps_and_wheeldrops {
            Some(b) => Some(b & 0x02 != 0),
            None => None,
        }
    }

    /// Returns `true` if the right wheel has dropped.
    #[inline(always)]
    pub const fn is_right_wheeldrop(&self) -> Option<bool> {
        match self.bumps_and_wheeldrops {
            Some(b) => Some(b & 0x04 != 0),
            None => None,
        }
    }

    /// Returns `true` if the left wheel has dropped.
    #[inline(always)]
    pub const fn is_left_wheeldrop(&self) -> Option<bool> {
        match self.bumps_and_wheeldrops {
            Some(b) => Some(b & 0x08 != 0),
            None => None,
        }
    }

    // -----------------------------------------------------------------------
    // Buttons (packet 18)
    // -----------------------------------------------------------------------

    /// Returns `true` if the Clean button is pressed.
    #[inline(always)]
    pub const fn is_button_clean(&self) -> Option<bool> {
        match self.buttons {
            Some(b) => Some(b & 0x01 != 0),
            None => None,
        }
    }

    /// Returns `true` if the Spot button is pressed.
    #[inline(always)]
    pub const fn is_button_spot(&self) -> Option<bool> {
        match self.buttons {
            Some(b) => Some(b & 0x02 != 0),
            None => None,
        }
    }

    /// Returns `true` if the Dock button is pressed.
    #[inline(always)]
    pub const fn is_button_dock(&self) -> Option<bool> {
        match self.buttons {
            Some(b) => Some(b & 0x04 != 0),
            None => None,
        }
    }

    /// Returns `true` if the Minute button is pressed.
    #[inline(always)]
    pub const fn is_button_minute(&self) -> Option<bool> {
        match self.buttons {
            Some(b) => Some(b & 0x08 != 0),
            None => None,
        }
    }

    /// Returns `true` if the Hour button is pressed.
    #[inline(always)]
    pub const fn is_button_hour(&self) -> Option<bool> {
        match self.buttons {
            Some(b) => Some(b & 0x10 != 0),
            None => None,
        }
    }

    /// Returns `true` if the Day button is pressed.
    #[inline(always)]
    pub const fn is_button_day(&self) -> Option<bool> {
        match self.buttons {
            Some(b) => Some(b & 0x20 != 0),
            None => None,
        }
    }

    /// Returns `true` if the Schedule button is pressed.
    #[inline(always)]
    pub const fn is_button_schedule(&self) -> Option<bool> {
        match self.buttons {
            Some(b) => Some(b & 0x40 != 0),
            None => None,
        }
    }

    /// Returns `true` if the Clock button is pressed.
    #[inline(always)]
    pub const fn is_button_clock(&self) -> Option<bool> {
        match self.buttons {
            Some(b) => Some(b & 0x80 != 0),
            None => None,
        }
    }

    // -----------------------------------------------------------------------
    // Overcurrents (packet 14)
    // -----------------------------------------------------------------------

    /// Returns `true` if the side brush motor is drawing excessive current.
    #[inline(always)]
    pub const fn is_overcurrent_side_brush(&self) -> Option<bool> {
        match self.overcurrents {
            Some(b) => Some(b & 0x01 != 0),
            None => None,
        }
    }

    /// Returns `true` if the main brush motor is drawing excessive current.
    #[inline(always)]
    pub const fn is_overcurrent_main_brush(&self) -> Option<bool> {
        match self.overcurrents {
            Some(b) => Some(b & 0x04 != 0),
            None => None,
        }
    }

    /// Returns `true` if the right wheel motor is drawing excessive current.
    #[inline(always)]
    pub const fn is_overcurrent_right_wheel(&self) -> Option<bool> {
        match self.overcurrents {
            Some(b) => Some(b & 0x08 != 0),
            None => None,
        }
    }

    /// Returns `true` if the left wheel motor is drawing excessive current.
    #[inline(always)]
    pub const fn is_overcurrent_left_wheel(&self) -> Option<bool> {
        match self.overcurrents {
            Some(b) => Some(b & 0x10 != 0),
            None => None,
        }
    }

    // -----------------------------------------------------------------------
    // Light bumper (packet 45)
    // -----------------------------------------------------------------------

    /// Returns `true` if the left light bumper is detecting an obstacle.
    #[inline(always)]
    pub const fn is_light_bump_left(&self) -> Option<bool> {
        match self.light_bumper {
            Some(b) => Some(b & 0x01 != 0),
            None => None,
        }
    }

    /// Returns `true` if the front-left light bumper is detecting an obstacle.
    #[inline(always)]
    pub const fn is_light_bump_front_left(&self) -> Option<bool> {
        match self.light_bumper {
            Some(b) => Some(b & 0x02 != 0),
            None => None,
        }
    }

    /// Returns `true` if the center-left light bumper is detecting an obstacle.
    #[inline(always)]
    pub const fn is_light_bump_center_left(&self) -> Option<bool> {
        match self.light_bumper {
            Some(b) => Some(b & 0x04 != 0),
            None => None,
        }
    }

    /// Returns `true` if the center-right light bumper is detecting an obstacle.
    #[inline(always)]
    pub const fn is_light_bump_center_right(&self) -> Option<bool> {
        match self.light_bumper {
            Some(b) => Some(b & 0x08 != 0),
            None => None,
        }
    }

    /// Returns `true` if the front-right light bumper is detecting an obstacle.
    #[inline(always)]
    pub const fn is_light_bump_front_right(&self) -> Option<bool> {
        match self.light_bumper {
            Some(b) => Some(b & 0x10 != 0),
            None => None,
        }
    }

    /// Returns `true` if the right light bumper is detecting an obstacle.
    #[inline(always)]
    pub const fn is_light_bump_right(&self) -> Option<bool> {
        match self.light_bumper {
            Some(b) => Some(b & 0x20 != 0),
            None => None,
        }
    }

    // -----------------------------------------------------------------------
    // Charging sources (packet 34)
    // -----------------------------------------------------------------------

    /// Returns `true` if charging via the home base dock.
    #[inline(always)]
    pub const fn is_charging_home_base(&self) -> Option<bool> {
        match self.charging_sources {
            Some(b) => Some(b & 0x01 != 0),
            None => None,
        }
    }

    /// Returns `true` if charging via the internal charger.
    #[inline(always)]
    pub const fn is_charging_internal(&self) -> Option<bool> {
        match self.charging_sources {
            Some(b) => Some(b & 0x02 != 0),
            None => None,
        }
    }

    // -----------------------------------------------------------------------
    // Stasis (packet 58)
    // -----------------------------------------------------------------------

    /// Returns `true` if the robot is making forward progress (Create 2).
    ///
    /// Per the Create 2 OI spec, packet 58 (stasis) encodes:
    /// - Bit 0 = 1: the robot is making forward progress (wheels turning, not
    ///   slipping, sensor clean).
    /// - Bit 0 = 0: no forward progress (stopped, spinning in place, or sensor
    ///   dirty).
    ///
    /// Bits 1–7 are reserved and should be ignored.
    #[inline(always)]
    pub const fn is_making_forward_progress(&self) -> Option<bool> {
        match self.stasis {
            Some(b) => Some(b & 0x01 != 0),
            None => None,
        }
    }

    /// Returns `true` if the robot is making forward progress.
    ///
    /// # Deprecated
    ///
    /// The name "stasis_detected" is misleading — this function returns `true`
    /// when the robot IS moving, not when it is stationary. Use
    /// [`is_making_forward_progress`](Self::is_making_forward_progress) instead.
    #[deprecated(since = "0.4.0", note = "use `is_making_forward_progress` instead")]
    #[inline(always)]
    pub const fn is_stasis_detected(&self) -> Option<bool> {
        self.is_making_forward_progress()
    }

    // -----------------------------------------------------------------------
    // Cargo bay digital inputs (packet 32)
    // -----------------------------------------------------------------------

    /// Returns `true` if cargo bay digital input 0 (pin 18) is active (high).
    #[inline(always)]
    pub const fn is_cargo_bay_di0(&self) -> Option<bool> {
        match self.cargo_bay_digital_inputs {
            Some(b) => Some(b & 0x01 != 0),
            None => None,
        }
    }

    /// Returns `true` if cargo bay digital input 1 is active (high).
    #[inline(always)]
    pub const fn is_cargo_bay_di1(&self) -> Option<bool> {
        match self.cargo_bay_digital_inputs {
            Some(b) => Some(b & 0x02 != 0),
            None => None,
        }
    }

    /// Returns `true` if cargo bay digital input 2 is active (high).
    #[inline(always)]
    pub const fn is_cargo_bay_di2(&self) -> Option<bool> {
        match self.cargo_bay_digital_inputs {
            Some(b) => Some(b & 0x04 != 0),
            None => None,
        }
    }

    /// Returns `true` if cargo bay digital input 3 is active (high).
    #[inline(always)]
    pub const fn is_cargo_bay_di3(&self) -> Option<bool> {
        match self.cargo_bay_digital_inputs {
            Some(b) => Some(b & 0x08 != 0),
            None => None,
        }
    }

    /// Returns `true` if the home base dock is electrically connected via the
    /// cargo bay Device Detect pin (bit 4, Create 2 / V3 only).
    #[inline(always)]
    pub const fn is_home_base_connected(&self) -> Option<bool> {
        match self.cargo_bay_digital_inputs {
            Some(b) => Some(b & 0x10 != 0),
            None => None,
        }
    }

    // -----------------------------------------------------------------------
    // Battery utilities
    // -----------------------------------------------------------------------

    /// Returns the battery state of charge as a percentage in `0.0..=100.0`.
    ///
    /// Returns `None` if either [`battery_charge`] or [`battery_capacity`] is
    /// not available, or if the reported capacity is zero (undefined denominator).
    ///
    /// The raw ratio `charge / capacity` is returned without clamping.
    /// Some robot firmware versions occasionally report `battery_charge` slightly
    /// above `battery_capacity`, which can produce a value above 100.0.
    pub fn battery_charge_percent(&self) -> Option<f32> {
        let charge = self.battery_charge?;
        let capacity = self.battery_capacity?;
        if capacity == 0 {
            return None;
        }
        Some(charge as f32 / capacity as f32 * 100.0)
    }

    // -----------------------------------------------------------------------
    // Merge / accumulation
    // -----------------------------------------------------------------------

    /// Update this `SensorData` with any fields that are `Some` in `other`.
    ///
    /// Fields where `other` has `None` are left unchanged. Fields where `other`
    /// has `Some` overwrite the existing value regardless of whether `self`
    /// already had a value.
    ///
    /// This is useful for building a **latest-known snapshot** from a stream
    /// of partial `SensorData` packets, where each packet only populates the
    /// sensor IDs that were registered with `start_stream`.
    ///
    /// # Note on delta fields
    ///
    /// [`distance_delta_mm`](Self::distance_delta_mm) and
    /// [`angle_delta_deg`](Self::angle_delta_deg) are **delta accumulators** —
    /// the robot resets their internal counter after each read. This method
    /// overwrites those fields rather than summing them. If you need cumulative
    /// odometry, sum the deltas yourself from each individual packet rather
    /// than relying on a merged snapshot.
    pub fn merge_from(&mut self, other: &SensorData) {
        self.bumps_and_wheeldrops = other.bumps_and_wheeldrops.or(self.bumps_and_wheeldrops);
        self.wall = other.wall.or(self.wall);
        self.cliff_left = other.cliff_left.or(self.cliff_left);
        self.cliff_front_left = other.cliff_front_left.or(self.cliff_front_left);
        self.cliff_front_right = other.cliff_front_right.or(self.cliff_front_right);
        self.cliff_right = other.cliff_right.or(self.cliff_right);
        self.virtual_wall = other.virtual_wall.or(self.virtual_wall);
        self.overcurrents = other.overcurrents.or(self.overcurrents);
        self.dirt_detect_left = other.dirt_detect_left.or(self.dirt_detect_left);
        self.dirt_detect_right = other.dirt_detect_right.or(self.dirt_detect_right);
        self.ir_omni = other.ir_omni.or(self.ir_omni);
        self.buttons = other.buttons.or(self.buttons);
        self.distance_delta_mm = other.distance_delta_mm.or(self.distance_delta_mm);
        self.angle_delta_deg = other.angle_delta_deg.or(self.angle_delta_deg);
        self.charging_state = other.charging_state.or(self.charging_state);
        self.voltage = other.voltage.or(self.voltage);
        self.current = other.current.or(self.current);
        self.temperature = other.temperature.or(self.temperature);
        self.battery_charge = other.battery_charge.or(self.battery_charge);
        self.battery_capacity = other.battery_capacity.or(self.battery_capacity);
        self.wall_signal = other.wall_signal.or(self.wall_signal);
        self.cliff_left_signal = other.cliff_left_signal.or(self.cliff_left_signal);
        self.cliff_front_left_signal = other
            .cliff_front_left_signal
            .or(self.cliff_front_left_signal);
        self.cliff_front_right_signal = other
            .cliff_front_right_signal
            .or(self.cliff_front_right_signal);
        self.cliff_right_signal = other.cliff_right_signal.or(self.cliff_right_signal);
        self.cargo_bay_digital_inputs = other
            .cargo_bay_digital_inputs
            .or(self.cargo_bay_digital_inputs);
        self.cargo_bay_analog_signal = other
            .cargo_bay_analog_signal
            .or(self.cargo_bay_analog_signal);
        self.charging_sources = other.charging_sources.or(self.charging_sources);
        self.oi_mode = other.oi_mode.or(self.oi_mode);
        self.song_number = other.song_number.or(self.song_number);
        self.song_playing = other.song_playing.or(self.song_playing);
        self.num_stream_packets = other.num_stream_packets.or(self.num_stream_packets);
        self.requested_velocity = other.requested_velocity.or(self.requested_velocity);
        self.requested_radius = other.requested_radius.or(self.requested_radius);
        self.requested_right_velocity = other
            .requested_right_velocity
            .or(self.requested_right_velocity);
        self.requested_left_velocity = other
            .requested_left_velocity
            .or(self.requested_left_velocity);
        self.left_encoder_counts = other.left_encoder_counts.or(self.left_encoder_counts);
        self.right_encoder_counts = other.right_encoder_counts.or(self.right_encoder_counts);
        self.light_bumper = other.light_bumper.or(self.light_bumper);
        self.light_bump_left_signal = other.light_bump_left_signal.or(self.light_bump_left_signal);
        self.light_bump_front_left_signal = other
            .light_bump_front_left_signal
            .or(self.light_bump_front_left_signal);
        self.light_bump_center_left_signal = other
            .light_bump_center_left_signal
            .or(self.light_bump_center_left_signal);
        self.light_bump_center_right_signal = other
            .light_bump_center_right_signal
            .or(self.light_bump_center_right_signal);
        self.light_bump_front_right_signal = other
            .light_bump_front_right_signal
            .or(self.light_bump_front_right_signal);
        self.light_bump_right_signal = other
            .light_bump_right_signal
            .or(self.light_bump_right_signal);
        self.ir_left = other.ir_left.or(self.ir_left);
        self.ir_right = other.ir_right.or(self.ir_right);
        self.left_motor_current = other.left_motor_current.or(self.left_motor_current);
        self.right_motor_current = other.right_motor_current.or(self.right_motor_current);
        self.main_brush_motor_current = other
            .main_brush_motor_current
            .or(self.main_brush_motor_current);
        self.side_brush_motor_current = other
            .side_brush_motor_current
            .or(self.side_brush_motor_current);
        self.stasis = other.stasis.or(self.stasis);
    }

    fn store_value(&mut self, id: u8, data: &[u8]) -> Result<(), ProtocolError> {
        match id {
            7 => self.bumps_and_wheeldrops = Some(decode_u8(data)?),
            8 => self.wall = Some(decode_u8(data)? != 0),
            9 => self.cliff_left = Some(decode_u8(data)? != 0),
            10 => self.cliff_front_left = Some(decode_u8(data)? != 0),
            11 => self.cliff_front_right = Some(decode_u8(data)? != 0),
            12 => self.cliff_right = Some(decode_u8(data)? != 0),
            13 => self.virtual_wall = Some(decode_u8(data)? != 0),
            14 => self.overcurrents = Some(decode_u8(data)?),
            15 => self.dirt_detect_left = Some(decode_u8(data)?),
            16 => self.dirt_detect_right = Some(decode_u8(data)?),
            17 => self.ir_omni = Some(IrChar::from_raw(decode_u8(data)?)),
            18 => self.buttons = Some(decode_u8(data)?),
            19 => self.distance_delta_mm = Some(decode_i16(data)?),
            20 => self.angle_delta_deg = Some(decode_i16(data)?),
            21 => self.charging_state = Some(ChargingState::from_raw(decode_u8(data)?)),
            22 => self.voltage = Some(decode_u16(data)?),
            23 => self.current = Some(decode_i16(data)?),
            24 => self.temperature = Some(decode_i8(data)?),
            25 => self.battery_charge = Some(decode_u16(data)?),
            26 => self.battery_capacity = Some(decode_u16(data)?),
            27 => self.wall_signal = Some(decode_u16(data)?),
            28 => self.cliff_left_signal = Some(decode_u16(data)?),
            29 => self.cliff_front_left_signal = Some(decode_u16(data)?),
            30 => self.cliff_front_right_signal = Some(decode_u16(data)?),
            31 => self.cliff_right_signal = Some(decode_u16(data)?),
            32 => self.cargo_bay_digital_inputs = Some(decode_u8(data)?),
            33 => self.cargo_bay_analog_signal = Some(decode_u16(data)?),
            34 => self.charging_sources = Some(decode_u8(data)?),
            35 => self.oi_mode = Some(OiMode::from_raw(decode_u8(data)?)),
            36 => self.song_number = Some(decode_u8(data)?),
            37 => self.song_playing = Some(decode_u8(data)? != 0),
            38 => self.num_stream_packets = Some(decode_u8(data)?),
            39 => self.requested_velocity = Some(decode_i16(data)?),
            40 => self.requested_radius = Some(decode_i16(data)?),
            41 => self.requested_right_velocity = Some(decode_i16(data)?),
            42 => self.requested_left_velocity = Some(decode_i16(data)?),
            43 => self.left_encoder_counts = Some(decode_u16(data)?),
            44 => self.right_encoder_counts = Some(decode_u16(data)?),
            45 => self.light_bumper = Some(decode_u8(data)?),
            46 => self.light_bump_left_signal = Some(decode_u16(data)?),
            47 => self.light_bump_front_left_signal = Some(decode_u16(data)?),
            48 => self.light_bump_center_left_signal = Some(decode_u16(data)?),
            49 => self.light_bump_center_right_signal = Some(decode_u16(data)?),
            50 => self.light_bump_front_right_signal = Some(decode_u16(data)?),
            51 => self.light_bump_right_signal = Some(decode_u16(data)?),
            52 => self.ir_left = Some(IrChar::from_raw(decode_u8(data)?)),
            53 => self.ir_right = Some(IrChar::from_raw(decode_u8(data)?)),
            54 => self.left_motor_current = Some(decode_i16(data)?),
            55 => self.right_motor_current = Some(decode_i16(data)?),
            56 => self.main_brush_motor_current = Some(decode_i16(data)?),
            57 => self.side_brush_motor_current = Some(decode_i16(data)?),
            58 => self.stasis = Some(decode_u8(data)?),
            _ => return Err(ProtocolError::UnknownPacketId(id)),
        }
        Ok(())
    }
}

/// Compute the expected total data length for a list of packet IDs.
/// Total expected response length (bytes) for the given list of packet IDs.
///
/// Both individual packet IDs (7-58) and group packet IDs (0-6, 100) are
/// accepted; group IDs are expanded to their constituent packets.
pub const fn expected_data_len(ids: &[u8]) -> Result<usize, ProtocolError> {
    let mut total = 0usize;
    let mut i = 0;
    while i < ids.len() {
        match packet_info(ids[i]) {
            Some(p) => total += p.len as usize,
            None => match group_data_len(ids[i]) {
                Some(len) => total += len,
                None => return Err(ProtocolError::UnknownPacketId(ids[i])),
            },
        }
        i += 1;
    }
    Ok(total)
}

/// Returns `true` if `ids` contains any duplicate packet ID.
///
/// Uses a 256-bit stack bitset — no allocation required.
pub fn has_duplicate_ids(ids: &[u8]) -> bool {
    let mut seen = [0u8; 32];
    for &id in ids {
        let bit = 1u8 << (id & 7);
        let byte = (id >> 3) as usize;
        if seen[byte] & bit != 0 {
            return true;
        }
        seen[byte] |= bit;
    }
    false
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_u8_single_byte() {
        assert_eq!(decode_u8(&[42]).unwrap(), 42);
    }

    #[test]
    fn decode_u8_empty() {
        assert!(decode_u8(&[]).is_err());
    }

    #[test]
    fn decode_u16_big_endian() {
        assert_eq!(decode_u16(&[0x01, 0x00]).unwrap(), 256);
        assert_eq!(decode_u16(&[0xFF, 0xFF]).unwrap(), 65535);
    }

    #[test]
    fn decode_i16_negative() {
        // -1 in big-endian is 0xFF, 0xFF
        assert_eq!(decode_i16(&[0xFF, 0xFF]).unwrap(), -1);
        // -256
        assert_eq!(decode_i16(&[0xFF, 0x00]).unwrap(), -256);
    }

    #[test]
    fn decode_single_packet_wall() {
        let mut sd = SensorData::default();
        let consumed = sd.decode_packet(8, &[1]).unwrap();
        assert_eq!(consumed, 1);
        assert_eq!(sd.wall, Some(true));
    }

    #[test]
    fn decode_single_packet_voltage() {
        let mut sd = SensorData::default();
        // 12500 mV = 0x30D4
        let consumed = sd.decode_packet(22, &[0x30, 0xD4]).unwrap();
        assert_eq!(consumed, 2);
        assert_eq!(sd.voltage, Some(12500));
    }

    #[test]
    fn decode_packet_insufficient_data() {
        let mut sd = SensorData::default();
        let result = sd.decode_packet(22, &[0x30]); // needs 2 bytes
        assert!(result.is_err());
    }

    #[test]
    fn decode_packet_unknown_id() {
        let mut sd = SensorData::default();
        let result = sd.decode_packet(200, &[0x00]);
        assert!(result.is_err());
    }

    #[test]
    fn decode_packets_query_list() {
        let mut sd = SensorData::default();
        // Decode wall (id 8, 1 byte) + voltage (id 22, 2 bytes)
        let data = [1, 0x30, 0xD4];
        sd.decode_packets(&[8, 22], &data).unwrap();
        assert_eq!(sd.wall, Some(true));
        assert_eq!(sd.voltage, Some(12500));
    }

    #[test]
    fn decode_packets_trailing_bytes_returns_error() {
        let mut sd = SensorData::default();
        // wall (id 8, 1 byte) + 2 extra bytes
        let data = [1, 0xFF, 0xFF];
        let err = sd.decode_packets(&[8], &data).unwrap_err();
        assert!(
            matches!(err, ProtocolError::UnexpectedData { trailing: 2 }),
            "expected UnexpectedData(2), got {err:?}"
        );
    }

    #[test]
    fn decode_oi_mode() {
        let mut sd = SensorData::default();
        sd.decode_packet(35, &[2]).unwrap();
        assert_eq!(sd.oi_mode, Some(OiMode::Safe));
    }

    #[test]
    fn decode_signed_current() {
        let mut sd = SensorData::default();
        // -500 mA = 0xFE0C
        sd.decode_packet(23, &[0xFE, 0x0C]).unwrap();
        assert_eq!(sd.current, Some(-500));
    }

    #[test]
    fn decode_temperature() {
        let mut sd = SensorData::default();
        // -10°C = 0xF6 as i8
        sd.decode_packet(24, &[0xF6]).unwrap();
        assert_eq!(sd.temperature, Some(-10));
    }

    #[test]
    fn expected_data_len_computes() {
        // wall(1) + voltage(2) + distance(2) = 5
        assert_eq!(expected_data_len(&[8, 22, 19]).unwrap(), 5);
    }

    #[test]
    fn decode_all_individual_packets() {
        use crate::opcode::SENSOR_PACKETS;
        // Verify every packet in SENSOR_PACKETS can be decoded without panic.
        let mut sd = SensorData::default();
        for p in SENSOR_PACKETS {
            let data: Vec<u8> = vec![0; p.len as usize];
            sd.decode_packet(p.id, &data).unwrap();
        }
    }

    #[test]
    fn decode_distance_angle_fields() {
        let mut sd = SensorData::default();
        // distance = +500 mm = 0x01F4
        sd.decode_packet(19, &[0x01, 0xF4]).unwrap();
        assert_eq!(sd.distance_delta_mm, Some(500));
        // angle = -90 deg = 0xFFA6
        sd.decode_packet(20, &[0xFF, 0xA6]).unwrap();
        assert_eq!(sd.angle_delta_deg, Some(-90));
    }

    #[test]
    fn stasis_detected_accessor() {
        let mut sd = SensorData::default();
        // bit 0 set → forward progress detected
        sd.decode_packet(58, &[0x01]).unwrap();
        assert_eq!(sd.is_making_forward_progress(), Some(true));
        // bit 0 clear → no forward progress
        sd.decode_packet(58, &[0x00]).unwrap();
        assert_eq!(sd.is_making_forward_progress(), Some(false));
        // reserved bits set but bit 0 clear
        sd.decode_packet(58, &[0xFE]).unwrap();
        assert_eq!(sd.is_making_forward_progress(), Some(false));
        // reserved bits + bit 0 → detected
        sd.decode_packet(58, &[0xFF]).unwrap();
        assert_eq!(sd.is_making_forward_progress(), Some(true));
    }

    #[test]
    #[allow(deprecated)]
    fn stasis_deprecated_alias_works() {
        let mut sd = SensorData::default();
        sd.decode_packet(58, &[0x01]).unwrap();
        assert_eq!(sd.is_stasis_detected(), Some(true));
    }

    // Round 14: has_duplicate_ids tests
    #[test]
    fn has_duplicate_ids_empty() {
        assert!(!has_duplicate_ids(&[]), "empty slice has no duplicates");
    }

    #[test]
    fn has_duplicate_ids_no_dups() {
        assert!(
            !has_duplicate_ids(&[7, 8, 22, 19]),
            "distinct IDs have no duplicates"
        );
    }

    #[test]
    fn has_duplicate_ids_adjacent_dup() {
        assert!(has_duplicate_ids(&[7, 7]), "adjacent duplicate detected");
    }

    #[test]
    fn has_duplicate_ids_non_adjacent_dup() {
        assert!(
            has_duplicate_ids(&[7, 8, 22, 7]),
            "non-adjacent duplicate detected"
        );
    }

    #[test]
    fn has_duplicate_ids_group_ids_not_same_as_individuals() {
        // Group ID 0 is different from individual IDs 7-26; not a duplicate
        assert!(
            !has_duplicate_ids(&[0, 7, 8]),
            "group ID 0 is not the same as ID 7 or 8"
        );
    }

    // Round 14: expected_data_len with group IDs
    #[test]
    fn expected_data_len_group_0() {
        // Group 0 = packets 7-26; total = sum of their individual lengths
        let len = expected_data_len(&[0]).unwrap();
        // Validate it's non-zero and matches group_data_len
        assert_eq!(Some(len), group_data_len(0));
    }

    #[test]
    fn expected_data_len_group_100() {
        let len = expected_data_len(&[100]).unwrap();
        assert_eq!(Some(len), group_data_len(100));
    }

    #[test]
    fn expected_data_len_mixed_individual_and_group() {
        // Mix a group ID with an individual packet
        let group_len = group_data_len(6).unwrap();
        let individual_len = packet_info(8).unwrap().len as usize;
        let total = expected_data_len(&[6, 8]).unwrap();
        assert_eq!(total, group_len + individual_len);
    }

    #[test]
    fn expected_data_len_unknown_id_fails() {
        let result = expected_data_len(&[200]);
        assert!(result.is_err(), "unknown ID 200 should return Err");
    }

    // Round 34: battery_charge_percent
    #[test]
    fn battery_charge_percent_normal() {
        let mut sd = SensorData::default();
        sd.battery_charge = Some(2000);
        sd.battery_capacity = Some(4000);
        let pct = sd.battery_charge_percent().unwrap();
        assert!((pct - 50.0_f32).abs() < 0.01, "expected 50.0, got {pct}");
    }

    #[test]
    fn battery_charge_percent_full() {
        let mut sd = SensorData::default();
        sd.battery_charge = Some(4000);
        sd.battery_capacity = Some(4000);
        let pct = sd.battery_charge_percent().unwrap();
        assert!((pct - 100.0_f32).abs() < 0.01, "expected 100.0, got {pct}");
    }

    #[test]
    fn battery_charge_percent_over_capacity() {
        // Some firmware reports charge slightly above capacity; no clamping.
        let mut sd = SensorData::default();
        sd.battery_charge = Some(4050);
        sd.battery_capacity = Some(4000);
        let pct = sd.battery_charge_percent().unwrap();
        assert!(pct > 100.0, "should not clamp above 100%");
    }

    #[test]
    fn battery_charge_percent_zero_capacity() {
        let mut sd = SensorData::default();
        sd.battery_charge = Some(0);
        sd.battery_capacity = Some(0);
        assert!(
            sd.battery_charge_percent().is_none(),
            "zero capacity must return None"
        );
    }

    #[test]
    fn battery_charge_percent_missing_fields() {
        let sd = SensorData::default();
        assert!(sd.battery_charge_percent().is_none());
    }

    // Round 34: cargo bay digital input accessors
    #[test]
    fn cargo_bay_di_accessors() {
        let mut sd = SensorData::default();
        // bits 0..3 set, bit 4 (device detect) clear
        sd.decode_packet(32, &[0x0F]).unwrap();
        assert_eq!(sd.is_cargo_bay_di0(), Some(true));
        assert_eq!(sd.is_cargo_bay_di1(), Some(true));
        assert_eq!(sd.is_cargo_bay_di2(), Some(true));
        assert_eq!(sd.is_cargo_bay_di3(), Some(true));
        assert_eq!(sd.is_home_base_connected(), Some(false));
    }

    #[test]
    fn home_base_connected_accessor() {
        let mut sd = SensorData::default();
        sd.decode_packet(32, &[0x10]).unwrap(); // only bit 4 set
        assert_eq!(sd.is_cargo_bay_di0(), Some(false));
        assert_eq!(sd.is_home_base_connected(), Some(true));
    }

    #[test]
    fn cargo_bay_di_none_when_absent() {
        let sd = SensorData::default();
        assert_eq!(sd.is_cargo_bay_di0(), None);
        assert_eq!(sd.is_home_base_connected(), None);
    }

    // Round 34: merge_from
    #[test]
    fn merge_from_updates_some_preserves_existing() {
        let mut base = SensorData::default();
        base.voltage = Some(12000);
        base.wall = Some(false);

        let mut patch = SensorData::default();
        patch.wall = Some(true);
        patch.oi_mode = Some(OiMode::Safe);

        base.merge_from(&patch);

        assert_eq!(base.voltage, Some(12000), "voltage must be preserved");
        assert_eq!(base.wall, Some(true), "wall updated from patch");
        assert_eq!(base.oi_mode, Some(OiMode::Safe), "oi_mode set from patch");
    }

    #[test]
    fn merge_from_none_does_not_clear() {
        let mut base = SensorData::default();
        base.voltage = Some(12000);

        let empty = SensorData::default();
        base.merge_from(&empty);

        assert_eq!(
            base.voltage,
            Some(12000),
            "voltage must not be cleared by None"
        );
    }
}
