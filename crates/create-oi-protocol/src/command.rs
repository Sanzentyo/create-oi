//! Sans-IO command encoding.
//!
//! Each function returns a fixed-size byte array — zero allocation, zero copy.
//! The caller passes these bytes to a transport for transmission.
//!
//! Variable-length commands (song, query_list, stream) have two variants:
//! - `encode_*_into(buf, ...)` — writes into a caller-provided buffer (always available)
//! - `encode_*(...)` — returns a `Vec<u8>` (requires `alloc` feature)

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::error::ProtocolError;
use crate::opcode::Opcode;

/// Start the OI. Transitions to Passive mode.
pub fn encode_start() -> [u8; 1] {
    [Opcode::Start as u8]
}

/// Reset the robot.
pub fn encode_reset() -> [u8; 1] {
    [Opcode::Reset as u8]
}

/// Stop the OI. Transitions to Off mode.
pub fn encode_stop() -> [u8; 1] {
    [Opcode::Stop as u8]
}

/// Enter Safe mode.
pub fn encode_safe() -> [u8; 1] {
    [Opcode::Safe as u8]
}

/// Enter Full mode.
pub fn encode_full() -> [u8; 1] {
    [Opcode::Full as u8]
}

/// Enter Control mode (same as Safe for Create 2).
pub fn encode_control() -> [u8; 1] {
    [Opcode::Control as u8]
}

/// Power down the robot.
pub fn encode_power() -> [u8; 1] {
    [Opcode::Power as u8]
}

/// Start default cleaning.
pub fn encode_clean() -> [u8; 1] {
    [Opcode::Clean as u8]
}

/// Start max cleaning.
pub fn encode_max() -> [u8; 1] {
    [Opcode::Max as u8]
}

/// Start spot cleaning.
pub fn encode_spot() -> [u8; 1] {
    [Opcode::Spot as u8]
}

/// Seek dock.
pub fn encode_dock() -> [u8; 1] {
    [Opcode::Dock as u8]
}

/// Drive with velocity (mm/s) and radius (mm).
/// Both values are signed 16-bit big-endian.
pub fn encode_drive(velocity_mm: i16, radius_mm: i16) -> [u8; 5] {
    let v = velocity_mm.to_be_bytes();
    let r = radius_mm.to_be_bytes();
    [Opcode::Drive as u8, v[0], v[1], r[0], r[1]]
}

/// Drive wheels directly with individual velocities (mm/s).
pub fn encode_drive_direct(right_mm: i16, left_mm: i16) -> [u8; 5] {
    let r = right_mm.to_be_bytes();
    let l = left_mm.to_be_bytes();
    [Opcode::DriveDirect as u8, r[0], r[1], l[0], l[1]]
}

/// Drive wheels with PWM values (-255 to 255).
pub fn encode_drive_pwm(right_pwm: i16, left_pwm: i16) -> [u8; 5] {
    let r = right_pwm.to_be_bytes();
    let l = left_pwm.to_be_bytes();
    [Opcode::DrivePwm as u8, r[0], r[1], l[0], l[1]]
}

/// Set motor states (side brush, main brush, vacuum).
/// Bit 0: side brush, Bit 1: vacuum, Bit 2: main brush.
/// Bits 3,4: side/main brush direction (1 = default, 0 = reverse).
pub fn encode_motors(bits: u8) -> [u8; 2] {
    [Opcode::Motors as u8, bits]
}

/// Set motor PWM values: main brush, side brush, vacuum.
/// Each is a signed byte (-127 to 127).
pub fn encode_motors_pwm(main_brush: i8, side_brush: i8, vacuum: i8) -> [u8; 4] {
    [
        Opcode::MotorsPwm as u8,
        main_brush as u8,
        side_brush as u8,
        vacuum as u8,
    ]
}

/// Set LEDs.
/// `led_bits`: Bit 0=debris, 1=spot, 2=dock, 3=check_robot
/// `power_color`: 0=green, 255=red
/// `power_intensity`: 0=off, 255=full
pub fn encode_leds(led_bits: u8, power_color: u8, power_intensity: u8) -> [u8; 4] {
    [Opcode::Leds as u8, led_bits, power_color, power_intensity]
}

/// Set the 7-segment displays with ASCII characters.
pub fn encode_digit_leds_ascii(d3: u8, d2: u8, d1: u8, d0: u8) -> [u8; 5] {
    [Opcode::DigitLedsAscii as u8, d3, d2, d1, d0]
}

/// Define a song. Writes into `buf` and returns the number of bytes written.
///
/// `song_number`: 0-3
/// `notes`: pairs of (MIDI note, duration_64ths)
///
/// Required buffer size: `3 + notes.len() * 2`
pub fn encode_song_into(
    buf: &mut [u8],
    song_number: u8,
    notes: &[(u8, u8)],
) -> Result<usize, ProtocolError> {
    let need = 3 + notes.len() * 2;
    if buf.len() < need {
        return Err(ProtocolError::BufferTooSmall {
            need,
            got: buf.len(),
        });
    }
    buf[0] = Opcode::Song as u8;
    buf[1] = song_number;
    buf[2] = notes.len() as u8;
    for (i, &(note, duration)) in notes.iter().enumerate() {
        buf[3 + i * 2] = note;
        buf[3 + i * 2 + 1] = duration;
    }
    Ok(need)
}

/// Define a song. Returns a `Vec<u8>`.
///
/// `song_number`: 0-3
/// `notes`: pairs of (MIDI note, duration_64ths)
#[cfg(feature = "alloc")]
pub fn encode_song(song_number: u8, notes: &[(u8, u8)]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(3 + notes.len() * 2);
    buf.push(Opcode::Song as u8);
    buf.push(song_number);
    buf.push(notes.len() as u8);
    for &(note, duration) in notes {
        buf.push(note);
        buf.push(duration);
    }
    buf
}

/// Play a previously defined song.
pub fn encode_play(song_number: u8) -> [u8; 2] {
    [Opcode::Play as u8, song_number]
}

/// Request a single sensor packet.
pub fn encode_sensors(packet_id: u8) -> [u8; 2] {
    [Opcode::Sensors as u8, packet_id]
}

/// Request multiple sensor packets (query list). Writes into `buf`.
///
/// Required buffer size: `2 + packet_ids.len()`
pub fn encode_query_list_into(buf: &mut [u8], packet_ids: &[u8]) -> Result<usize, ProtocolError> {
    let need = 2 + packet_ids.len();
    if buf.len() < need {
        return Err(ProtocolError::BufferTooSmall {
            need,
            got: buf.len(),
        });
    }
    buf[0] = Opcode::QueryList as u8;
    buf[1] = packet_ids.len() as u8;
    buf[2..need].copy_from_slice(packet_ids);
    Ok(need)
}

/// Request multiple sensor packets (query list). Returns a `Vec<u8>`.
#[cfg(feature = "alloc")]
pub fn encode_query_list(packet_ids: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(2 + packet_ids.len());
    buf.push(Opcode::QueryList as u8);
    buf.push(packet_ids.len() as u8);
    buf.extend_from_slice(packet_ids);
    buf
}

/// Start a sensor stream with the given packet IDs. Writes into `buf`.
///
/// Required buffer size: `2 + packet_ids.len()`
pub fn encode_stream_into(buf: &mut [u8], packet_ids: &[u8]) -> Result<usize, ProtocolError> {
    let need = 2 + packet_ids.len();
    if buf.len() < need {
        return Err(ProtocolError::BufferTooSmall {
            need,
            got: buf.len(),
        });
    }
    buf[0] = Opcode::Stream as u8;
    buf[1] = packet_ids.len() as u8;
    buf[2..need].copy_from_slice(packet_ids);
    Ok(need)
}

/// Start a sensor stream with the given packet IDs. Returns a `Vec<u8>`.
#[cfg(feature = "alloc")]
pub fn encode_stream(packet_ids: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(2 + packet_ids.len());
    buf.push(Opcode::Stream as u8);
    buf.push(packet_ids.len() as u8);
    buf.extend_from_slice(packet_ids);
    buf
}

/// Pause or resume the sensor stream.
pub fn encode_toggle_stream(enable: bool) -> [u8; 2] {
    [Opcode::ToggleStream as u8, if enable { 1 } else { 0 }]
}

/// Set the date/time.
pub fn encode_date(day: u8, hour: u8, minute: u8) -> [u8; 4] {
    [Opcode::Date as u8, day, hour, minute]
}

/// Change baud rate.
pub fn encode_baud(baud_code: u8) -> [u8; 2] {
    [Opcode::Baud as u8, baud_code]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_encodes_correctly() {
        assert_eq!(encode_start(), [128]);
    }

    #[test]
    fn safe_encodes_correctly() {
        assert_eq!(encode_safe(), [131]);
    }

    #[test]
    fn full_encodes_correctly() {
        assert_eq!(encode_full(), [132]);
    }

    #[test]
    fn drive_positive() {
        // velocity = 200 mm/s, radius = 500 mm
        let cmd = encode_drive(200, 500);
        assert_eq!(cmd[0], 137); // opcode
        assert_eq!(i16::from_be_bytes([cmd[1], cmd[2]]), 200);
        assert_eq!(i16::from_be_bytes([cmd[3], cmd[4]]), 500);
    }

    #[test]
    fn drive_negative() {
        let cmd = encode_drive(-300, -1000);
        assert_eq!(cmd[0], 137);
        assert_eq!(i16::from_be_bytes([cmd[1], cmd[2]]), -300);
        assert_eq!(i16::from_be_bytes([cmd[3], cmd[4]]), -1000);
    }

    #[test]
    fn drive_direct() {
        let cmd = encode_drive_direct(100, -100);
        assert_eq!(cmd[0], 145);
        assert_eq!(i16::from_be_bytes([cmd[1], cmd[2]]), 100);
        assert_eq!(i16::from_be_bytes([cmd[3], cmd[4]]), -100);
    }

    #[test]
    fn leds_encode() {
        let cmd = encode_leds(0b1010, 128, 255);
        assert_eq!(cmd, [139, 0b1010, 128, 255]);
    }

    #[test]
    fn song_encode() {
        let notes = [(60, 32), (64, 32)];
        let cmd = encode_song(0, &notes);
        assert_eq!(cmd, [140, 0, 2, 60, 32, 64, 32]);
    }

    #[test]
    fn sensors_group_100() {
        let cmd = encode_sensors(100);
        assert_eq!(cmd, [142, 100]);
    }

    #[test]
    fn query_list() {
        let cmd = encode_query_list(&[7, 8, 35]);
        assert_eq!(cmd, [149, 3, 7, 8, 35]);
    }

    #[test]
    fn stream_encode() {
        let cmd = encode_stream(&[7, 8, 9]);
        assert_eq!(cmd, [148, 3, 7, 8, 9]);
    }

    #[test]
    fn toggle_stream_enable() {
        assert_eq!(encode_toggle_stream(true), [150, 1]);
        assert_eq!(encode_toggle_stream(false), [150, 0]);
    }

    #[test]
    fn date_encode() {
        let cmd = encode_date(1, 14, 30);
        assert_eq!(cmd, [168, 1, 14, 30]);
    }

    #[test]
    fn digit_leds_ascii() {
        let cmd = encode_digit_leds_ascii(b'R', b'U', b'S', b'T');
        assert_eq!(cmd, [164, b'R', b'U', b'S', b'T']);
    }
}
