//! Sans-IO sensor packet parsing.
//!
//! Each sensor value is decoded from raw big-endian bytes using manual
//! `from_be_bytes` — zero allocation, zero copy.

use crate::error::ProtocolError;
use crate::types::{ChargingState, IrChar, OiMode};

use crate::opcode::packet_info;

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
    pub distance: Option<i16>,
    pub angle: Option<i16>,

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
    pub left_encoder_counts: Option<u16>,
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
    pub fn decode_packets(&mut self, ids: &[u8], data: &[u8]) -> Result<(), ProtocolError> {
        let mut offset = 0;
        for &id in ids {
            let consumed = self.decode_packet(id, &data[offset..])?;
            offset += consumed;
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

    /// Returns `true` if the stasis signal is toggling (robot is moving forward).
    #[inline(always)]
    pub const fn is_stasis_toggling(&self) -> Option<bool> {
        match self.stasis {
            Some(b) => Some(b & 0x01 != 0),
            None => None,
        }
    }

    /// Returns `true` if stasis is disabled (Create 1 backward-compatibility mode).
    #[inline(always)]
    pub const fn is_stasis_disabled(&self) -> Option<bool> {
        match self.stasis {
            Some(b) => Some(b & 0x02 != 0),
            None => None,
        }
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
            19 => self.distance = Some(decode_i16(data)?),
            20 => self.angle = Some(decode_i16(data)?),
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
pub const fn expected_data_len(ids: &[u8]) -> Result<usize, ProtocolError> {
    let mut total = 0usize;
    let mut i = 0;
    while i < ids.len() {
        match packet_info(ids[i]) {
            Some(p) => total += p.len as usize,
            None => return Err(ProtocolError::UnknownPacketId(ids[i])),
        }
        i += 1;
    }
    Ok(total)
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
}
