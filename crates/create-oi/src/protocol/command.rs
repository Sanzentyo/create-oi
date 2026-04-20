//! Sans-IO command encoding.
//!
//! Each function returns a fixed-size byte array — zero allocation, zero copy.
//! The caller passes these bytes to a transport for transmission.

use super::opcode::Opcode;

/// Start the OI. Transitions to Passive mode.
pub fn encode_start() -> [u8; 1] {
    [Opcode::Start.as_u8()]
}

/// Reset the robot.
pub fn encode_reset() -> [u8; 1] {
    [Opcode::Reset.as_u8()]
}

/// Stop the OI. Transitions to Off mode.
pub fn encode_stop() -> [u8; 1] {
    [Opcode::Stop.as_u8()]
}

/// Enter Safe mode.
pub fn encode_safe() -> [u8; 1] {
    [Opcode::Safe.as_u8()]
}

/// Enter Full mode.
pub fn encode_full() -> [u8; 1] {
    [Opcode::Full.as_u8()]
}

/// Enter Control mode (same as Safe for Create 2).
pub fn encode_control() -> [u8; 1] {
    [Opcode::Control.as_u8()]
}

/// Power down the robot.
pub fn encode_power() -> [u8; 1] {
    [Opcode::Power.as_u8()]
}

/// Start default cleaning.
pub fn encode_clean() -> [u8; 1] {
    [Opcode::Clean.as_u8()]
}

/// Start max cleaning.
pub fn encode_max() -> [u8; 1] {
    [Opcode::Max.as_u8()]
}

/// Start spot cleaning.
pub fn encode_spot() -> [u8; 1] {
    [Opcode::Spot.as_u8()]
}

/// Seek dock.
pub fn encode_dock() -> [u8; 1] {
    [Opcode::Dock.as_u8()]
}

/// Drive with velocity (mm/s) and radius (mm).
/// Both values are signed 16-bit big-endian.
pub fn encode_drive(velocity_mm: i16, radius_mm: i16) -> [u8; 5] {
    let v = velocity_mm.to_be_bytes();
    let r = radius_mm.to_be_bytes();
    [Opcode::Drive.as_u8(), v[0], v[1], r[0], r[1]]
}

/// Drive wheels directly with individual velocities (mm/s).
pub fn encode_drive_direct(right_mm: i16, left_mm: i16) -> [u8; 5] {
    let r = right_mm.to_be_bytes();
    let l = left_mm.to_be_bytes();
    [Opcode::DriveDirect.as_u8(), r[0], r[1], l[0], l[1]]
}

/// Drive wheels with PWM values (-255 to 255).
pub fn encode_drive_pwm(right_pwm: i16, left_pwm: i16) -> [u8; 5] {
    let r = right_pwm.to_be_bytes();
    let l = left_pwm.to_be_bytes();
    [Opcode::DrivePwm.as_u8(), r[0], r[1], l[0], l[1]]
}

/// Set motor states (side brush, main brush, vacuum).
/// Bit 0: side brush, Bit 1: vacuum, Bit 2: main brush.
/// Bits 3,4: side/main brush direction (1 = default, 0 = reverse).
pub fn encode_motors(bits: u8) -> [u8; 2] {
    [Opcode::Motors.as_u8(), bits]
}

/// Set motor PWM values: main brush, side brush, vacuum.
/// Each is a signed byte (-127 to 127).
pub fn encode_motors_pwm(main_brush: i8, side_brush: i8, vacuum: i8) -> [u8; 4] {
    [
        Opcode::MotorsPwm.as_u8(),
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
    [Opcode::Leds.as_u8(), led_bits, power_color, power_intensity]
}

/// Set the 7-segment displays with ASCII characters.
pub fn encode_digit_leds_ascii(d3: u8, d2: u8, d1: u8, d0: u8) -> [u8; 5] {
    [Opcode::DigitLedsAscii.as_u8(), d3, d2, d1, d0]
}

/// Define a song.
/// `song_number`: 0-3
/// `notes`: pairs of (MIDI note, duration_64ths)
pub fn encode_song(song_number: u8, notes: &[(u8, u8)]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(3 + notes.len() * 2);
    buf.push(Opcode::Song.as_u8());
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
    [Opcode::Play.as_u8(), song_number]
}

/// Request a single sensor packet.
pub fn encode_sensors(packet_id: u8) -> [u8; 2] {
    [Opcode::Sensors.as_u8(), packet_id]
}

/// Request multiple sensor packets (query list).
pub fn encode_query_list(packet_ids: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(2 + packet_ids.len());
    buf.push(Opcode::QueryList.as_u8());
    buf.push(packet_ids.len() as u8);
    buf.extend_from_slice(packet_ids);
    buf
}

/// Start a sensor stream with the given packet IDs.
pub fn encode_stream(packet_ids: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(2 + packet_ids.len());
    buf.push(Opcode::Stream.as_u8());
    buf.push(packet_ids.len() as u8);
    buf.extend_from_slice(packet_ids);
    buf
}

/// Pause or resume the sensor stream.
pub fn encode_toggle_stream(enable: bool) -> [u8; 2] {
    [Opcode::ToggleStream.as_u8(), if enable { 1 } else { 0 }]
}

/// Set the date/time.
pub fn encode_date(day: u8, hour: u8, minute: u8) -> [u8; 4] {
    [Opcode::Date.as_u8(), day, hour, minute]
}

/// Change baud rate.
pub fn encode_baud(baud_code: u8) -> [u8; 2] {
    [Opcode::Baud.as_u8(), baud_code]
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
