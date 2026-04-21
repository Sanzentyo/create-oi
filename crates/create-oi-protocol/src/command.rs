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
#[inline(always)]
pub const fn encode_start() -> [u8; 1] {
    [Opcode::Start as u8]
}

/// Reset the robot.
#[inline(always)]
pub const fn encode_reset() -> [u8; 1] {
    [Opcode::Reset as u8]
}

/// Stop the OI. Transitions to Off mode.
#[inline(always)]
pub const fn encode_stop() -> [u8; 1] {
    [Opcode::Stop as u8]
}

/// Enter Safe mode.
#[inline(always)]
pub const fn encode_safe() -> [u8; 1] {
    [Opcode::Safe as u8]
}

/// Enter Full mode.
#[inline(always)]
pub const fn encode_full() -> [u8; 1] {
    [Opcode::Full as u8]
}

/// Enter Control mode (same as Safe for Create 2).
#[inline(always)]
pub const fn encode_control() -> [u8; 1] {
    [Opcode::Control as u8]
}

/// Power down the robot.
#[inline(always)]
pub const fn encode_power() -> [u8; 1] {
    [Opcode::Power as u8]
}

/// Start default cleaning.
#[inline(always)]
pub const fn encode_clean() -> [u8; 1] {
    [Opcode::Clean as u8]
}

/// Start max cleaning.
#[inline(always)]
pub const fn encode_max() -> [u8; 1] {
    [Opcode::Max as u8]
}

/// Start spot cleaning.
#[inline(always)]
pub const fn encode_spot() -> [u8; 1] {
    [Opcode::Spot as u8]
}

/// Seek dock.
#[inline(always)]
pub const fn encode_dock() -> [u8; 1] {
    [Opcode::Dock as u8]
}

/// Drive with velocity (mm/s) and radius (mm).
/// Both values are signed 16-bit big-endian.
#[inline(always)]
pub const fn encode_drive(velocity_mm: i16, radius_mm: i16) -> [u8; 5] {
    let v = velocity_mm.to_be_bytes();
    let r = radius_mm.to_be_bytes();
    [Opcode::Drive as u8, v[0], v[1], r[0], r[1]]
}

/// Drive wheels directly with individual velocities (mm/s).
#[inline(always)]
pub const fn encode_drive_direct(right_mm: i16, left_mm: i16) -> [u8; 5] {
    let r = right_mm.to_be_bytes();
    let l = left_mm.to_be_bytes();
    [Opcode::DriveDirect as u8, r[0], r[1], l[0], l[1]]
}

/// Drive wheels with PWM values (-255 to 255).
#[inline(always)]
pub const fn encode_drive_pwm(right_pwm: i16, left_pwm: i16) -> [u8; 5] {
    let r = right_pwm.to_be_bytes();
    let l = left_pwm.to_be_bytes();
    [Opcode::DrivePwm as u8, r[0], r[1], l[0], l[1]]
}

/// Set motor states (side brush, main brush, vacuum).
/// Bit 0: side brush, Bit 1: vacuum, Bit 2: main brush.
/// Bits 3,4: side/main brush direction (1 = default, 0 = reverse).
#[inline(always)]
pub const fn encode_motors(bits: u8) -> [u8; 2] {
    [Opcode::Motors as u8, bits]
}

/// Set motor PWM values: main brush, side brush, vacuum.
/// Each is a signed byte (-127 to 127).
#[inline(always)]
pub const fn encode_motors_pwm(main_brush: i8, side_brush: i8, vacuum: i8) -> [u8; 4] {
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
#[inline(always)]
pub const fn encode_leds(led_bits: u8, power_color: u8, power_intensity: u8) -> [u8; 4] {
    [Opcode::Leds as u8, led_bits, power_color, power_intensity]
}

/// Set the 7-segment displays with ASCII characters.
#[inline(always)]
pub const fn encode_digit_leds_ascii(d3: u8, d2: u8, d1: u8, d0: u8) -> [u8; 5] {
    [Opcode::DigitLedsAscii as u8, d3, d2, d1, d0]
}

/// Set the scheduling LEDs (opcode 162).
///
/// `day_leds`: bits 0–6 select the Sun–Sat day LEDs.
/// `schedule_leds`: bit 0=colon, bit 1=AM/PM indicator, bit 2=clock icon, bit 3=schedule icon.
#[inline(always)]
pub const fn encode_scheduling_leds(day_leds: u8, schedule_leds: u8) -> [u8; 3] {
    [Opcode::SchedulingLeds as u8, day_leds, schedule_leds]
}

/// Set raw 7-segment digit LEDs (opcode 163).
///
/// Each argument directly controls the 7 segments and decimal point of one digit:
/// bits 0–6 = segments A–G, bit 7 = decimal point.
/// `d3` is the leftmost digit and `d0` is the rightmost.
#[inline(always)]
pub const fn encode_digit_leds_raw(d3: u8, d2: u8, d1: u8, d0: u8) -> [u8; 5] {
    [Opcode::DigitLedsRaw as u8, d3, d2, d1, d0]
}

/// Simulate button presses (opcode 165; Full mode only).
///
/// Bits: 0=clean, 1=spot, 2=dock, 3=minute, 4=hour, 5=day, 6=schedule, 7=clock.
/// Setting a bit to 1 simulates pressing that button.
#[inline(always)]
pub const fn encode_buttons(bits: u8) -> [u8; 2] {
    [Opcode::Buttons as u8, bits]
}

/// Set the weekly cleaning schedule (opcode 167).
///
/// `days`: bitmask of scheduled days (bit 0=Sunday, bit 6=Saturday).
/// `times`: (hour, minute) for each day of the week, starting with Sunday.
///
/// Note: firmware support for this command varies across robot models.
#[inline(always)]
pub const fn encode_schedule(days: u8, times: [(u8, u8); 7]) -> [u8; 16] {
    let mut buf = [0u8; 16];
    buf[0] = Opcode::Schedule as u8;
    buf[1] = days;
    let mut i = 0;
    while i < 7 {
        buf[2 + i * 2] = times[i].0;
        buf[2 + i * 2 + 1] = times[i].1;
        i += 1;
    }
    buf
}

/// Define a song. Writes into `buf` and returns the number of bytes written.
///
/// `song_number`: 0-3
/// `notes`: pairs of (MIDI note, duration_64ths). Maximum 16 notes per OI spec.
///
/// Required buffer size: `3 + notes.len() * 2`
pub fn encode_song_into(
    buf: &mut [u8],
    song_number: u8,
    notes: &[(u8, u8)],
) -> Result<usize, ProtocolError> {
    if notes.len() > 16 {
        return Err(ProtocolError::TooManyItems {
            max: 16,
            got: notes.len(),
        });
    }
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
/// `notes`: pairs of (MIDI note, duration_64ths). Maximum 16 notes per OI spec.
#[cfg(feature = "alloc")]
pub fn encode_song(song_number: u8, notes: &[(u8, u8)]) -> Result<Vec<u8>, ProtocolError> {
    if notes.len() > 16 {
        return Err(ProtocolError::TooManyItems {
            max: 16,
            got: notes.len(),
        });
    }
    let mut buf = Vec::with_capacity(3 + notes.len() * 2);
    buf.push(Opcode::Song as u8);
    buf.push(song_number);
    buf.push(notes.len() as u8);
    for &(note, duration) in notes {
        buf.push(note);
        buf.push(duration);
    }
    Ok(buf)
}

/// Play a previously defined song.
#[inline(always)]
pub const fn encode_play(song_number: u8) -> [u8; 2] {
    [Opcode::Play as u8, song_number]
}

/// Request a single sensor packet.
#[inline(always)]
pub const fn encode_sensors(packet_id: u8) -> [u8; 2] {
    [Opcode::Sensors as u8, packet_id]
}

/// Request multiple sensor packets (query list). Writes into `buf`.
///
/// Returns `TooManyItems` if `packet_ids.len() > 255`.
/// Required buffer size: `2 + packet_ids.len()`
pub fn encode_query_list_into(buf: &mut [u8], packet_ids: &[u8]) -> Result<usize, ProtocolError> {
    if packet_ids.len() > 255 {
        return Err(ProtocolError::TooManyItems {
            max: 255,
            got: packet_ids.len(),
        });
    }
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
///
/// Returns `TooManyItems` if `packet_ids.len() > 255`.
#[cfg(feature = "alloc")]
pub fn encode_query_list(packet_ids: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if packet_ids.len() > 255 {
        return Err(ProtocolError::TooManyItems {
            max: 255,
            got: packet_ids.len(),
        });
    }
    let mut buf = Vec::with_capacity(2 + packet_ids.len());
    buf.push(Opcode::QueryList as u8);
    buf.push(packet_ids.len() as u8);
    buf.extend_from_slice(packet_ids);
    Ok(buf)
}

/// Start a sensor stream with the given packet IDs. Writes into `buf`.
///
/// Returns `TooManyItems` if `packet_ids.len() > 255`.
/// Required buffer size: `2 + packet_ids.len()`
pub fn encode_stream_into(buf: &mut [u8], packet_ids: &[u8]) -> Result<usize, ProtocolError> {
    if packet_ids.len() > 255 {
        return Err(ProtocolError::TooManyItems {
            max: 255,
            got: packet_ids.len(),
        });
    }
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
///
/// Returns `TooManyItems` if `packet_ids.len() > 255`.
#[cfg(feature = "alloc")]
pub fn encode_stream(packet_ids: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    if packet_ids.len() > 255 {
        return Err(ProtocolError::TooManyItems {
            max: 255,
            got: packet_ids.len(),
        });
    }
    let mut buf = Vec::with_capacity(2 + packet_ids.len());
    buf.push(Opcode::Stream as u8);
    buf.push(packet_ids.len() as u8);
    buf.extend_from_slice(packet_ids);
    Ok(buf)
}

/// Pause or resume the sensor stream.
#[inline(always)]
pub const fn encode_toggle_stream(enable: bool) -> [u8; 2] {
    [Opcode::ToggleStream as u8, if enable { 1 } else { 0 }]
}

/// Set the date/time.
#[inline(always)]
pub const fn encode_date(day: u8, hour: u8, minute: u8) -> [u8; 4] {
    [Opcode::Date as u8, day, hour, minute]
}

/// Change baud rate.
#[inline(always)]
pub const fn encode_baud(baud_code: u8) -> [u8; 2] {
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
        let cmd = encode_song(0, &notes).unwrap();
        assert_eq!(cmd, [140, 0, 2, 60, 32, 64, 32]);
    }

    #[test]
    fn song_encode_too_many_notes() {
        let notes = [(60u8, 32u8); 17];
        assert!(encode_song(0, &notes).is_err());
    }

    #[test]
    fn sensors_group_100() {
        let cmd = encode_sensors(100);
        assert_eq!(cmd, [142, 100]);
    }

    #[test]
    fn query_list() {
        let cmd = encode_query_list(&[7, 8, 35]).unwrap();
        assert_eq!(cmd, [149, 3, 7, 8, 35]);
    }

    #[test]
    fn stream_encode() {
        let cmd = encode_stream(&[7, 8, 9]).unwrap();
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

    #[test]
    fn scheduling_leds() {
        // day_leds = 0b0100010 (Mon + Thu), schedule_leds = 0b0011 (colon + AM/PM)
        let cmd = encode_scheduling_leds(0b0100010, 0b0011);
        assert_eq!(cmd, [162, 0b0100010, 0b0011]);
    }

    #[test]
    fn digit_leds_raw() {
        let cmd = encode_digit_leds_raw(0x7F, 0x00, 0x41, 0x63);
        assert_eq!(cmd, [163, 0x7F, 0x00, 0x41, 0x63]);
    }

    #[test]
    fn buttons_encode() {
        // clean + dock = bits 0 and 2
        let cmd = encode_buttons(0b0000_0101);
        assert_eq!(cmd, [165, 0b0000_0101]);
    }

    #[test]
    fn schedule_encodes_all_days() {
        // All days enabled, all set to 08:00
        let times = [(8u8, 0u8); 7];
        let cmd = encode_schedule(0b0111_1111, times);
        assert_eq!(cmd[0], 167); // opcode
        assert_eq!(cmd[1], 0b0111_1111); // days
        // Sun = byte 2,3; Sat = byte 14,15
        assert_eq!(cmd[2], 8);
        assert_eq!(cmd[3], 0);
        assert_eq!(cmd[14], 8);
        assert_eq!(cmd[15], 0);
        assert_eq!(cmd.len(), 16);
    }

    #[test]
    fn schedule_per_day_times() {
        let times = [
            (9, 0),  // Sun
            (7, 30), // Mon
            (7, 30), // Tue
            (7, 30), // Wed
            (7, 30), // Thu
            (8, 0),  // Fri
            (10, 0), // Sat
        ];
        let cmd = encode_schedule(0b0111_1110, times); // Mon–Sat
        assert_eq!(cmd[2], 9); // Sun hour
        assert_eq!(cmd[3], 0); // Sun min
        assert_eq!(cmd[4], 7); // Mon hour
        assert_eq!(cmd[5], 30); // Mon min
        assert_eq!(cmd[14], 10); // Sat hour
        assert_eq!(cmd[15], 0); // Sat min
    }
}
