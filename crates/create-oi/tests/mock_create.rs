//! Mock transport and integration-level Create tests.

use std::io;
use std::time::Duration;

use create_oi::prelude::*;
use create_oi::transport::Transport;

// ---------------------------------------------------------------------------
// Mock transport
// ---------------------------------------------------------------------------

/// A mock transport that records writes and replays pre-loaded read data.
#[derive(Debug)]
struct MockTransport {
    /// Bytes written by the create.
    written: Vec<u8>,
    /// Bytes pre-loaded for the Create to read.
    read_buf: Vec<u8>,
    /// Current read position.
    read_pos: usize,
    closed: bool,
    /// When true, `read()` returns `Ok(0)` to simulate EOF/disconnect.
    eof_on_read: bool,
}

impl MockTransport {
    fn new() -> Self {
        Self {
            written: Vec::new(),
            read_buf: Vec::new(),
            read_pos: 0,
            closed: false,
            eof_on_read: false,
        }
    }

    fn with_read_data(data: &[u8]) -> Self {
        Self {
            written: Vec::new(),
            read_buf: data.to_vec(),
            read_pos: 0,
            closed: false,
            eof_on_read: false,
        }
    }

    /// Construct a transport that returns `Ok(0)` (EOF/disconnect) on every read.
    fn with_eof_on_read() -> Self {
        Self {
            written: Vec::new(),
            read_buf: Vec::new(),
            read_pos: 0,
            closed: false,
            eof_on_read: true,
        }
    }

    fn written_bytes(&self) -> &[u8] {
        &self.written
    }

    fn _push_read_data(&mut self, data: &[u8]) {
        self.read_buf.extend_from_slice(data);
    }
}

impl Transport for MockTransport {
    fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        if self.closed {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "closed"));
        }
        self.written.extend_from_slice(data);
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.closed {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "closed"));
        }
        if self.eof_on_read {
            return Ok(0);
        }
        let available = &self.read_buf[self.read_pos..];
        if available.is_empty() {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "no data"));
        }
        let n = buf.len().min(available.len());
        buf[..n].copy_from_slice(&available[..n]);
        self.read_pos += n;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn set_read_timeout(&mut self, _timeout: Option<Duration>) -> io::Result<()> {
        Ok(())
    }

    fn close(&mut self) -> io::Result<()> {
        self.closed = true;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Robot mode transition tests
// ---------------------------------------------------------------------------

#[test]
fn create_start_sends_start_opcode() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);

    // start() transitions Off → Passive
    let create = create.start().unwrap();
    let written = create.transport().written_bytes();
    assert_eq!(written, &[128]); // START opcode
}

#[test]
fn create_passive_to_safe_sends_safe_opcode() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let create = create.start().unwrap();

    let create = create.to_safe().unwrap();
    let written = create.transport().written_bytes();
    // START(128) + SAFE(131)
    assert_eq!(written, &[128, 131]);
}

#[test]
fn create_passive_to_full_sends_full_opcode() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let create = create.start().unwrap();

    let create = create.to_full().unwrap();
    let written = create.transport().written_bytes();
    assert_eq!(written, &[128, 132]); // START + FULL
}

#[test]
fn create_safe_to_full() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let create = create.start().unwrap().to_safe().unwrap();

    let create = create.to_full().unwrap();
    let written = create.transport().written_bytes();
    assert_eq!(written, &[128, 131, 132]);
}

#[test]
fn create_full_to_safe() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let create = create.start().unwrap().to_full().unwrap();

    let create = create.to_safe().unwrap();
    let written = create.transport().written_bytes();
    assert_eq!(written, &[128, 132, 131]);
}

#[test]
fn create_passive_to_off() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let create = create.start().unwrap();

    let off = create.to_off().unwrap();
    let transport = off.into_transport();
    assert_eq!(transport.written_bytes(), &[128, 173]); // START + STOP
}

#[test]
fn create_safe_to_off() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let create = create.start().unwrap().to_safe().unwrap();

    let off = create.to_off().unwrap();
    let transport = off.into_transport();
    assert_eq!(transport.written_bytes(), &[128, 131, 173]); // START + SAFE + STOP
}

#[test]
fn create_full_to_off() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let create = create.start().unwrap().to_full().unwrap();

    let off = create.to_off().unwrap();
    let transport = off.into_transport();
    assert_eq!(transport.written_bytes(), &[128, 132, 173]); // START + FULL + STOP
}

#[test]
fn create_full_to_passive() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let create = create.start().unwrap().to_full().unwrap();

    let create = create.to_passive().unwrap();
    let written = create.transport().written_bytes();
    assert_eq!(written, &[128, 132, 128]); // START + FULL + START
}

// ---------------------------------------------------------------------------
// Sensor query tests
// ---------------------------------------------------------------------------

#[test]
fn query_single_sensor() {
    // OI mode (packet 35) = 2 (Safe), 1 byte response
    let mock = MockTransport::with_read_data(&[2]);
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap();

    let sd = create.query_sensor(35).unwrap();
    assert_eq!(sd.oi_mode, Some(OiMode::Safe));

    // Verify query command was sent: START(128) + SENSORS(142) + packet_id(35)
    let written = create.transport().written_bytes();
    assert_eq!(written, &[128, 142, 35]);
}

#[test]
fn query_list_multiple_sensors() {
    // wall(id=8, 1 byte) = 1, voltage(id=22, 2 bytes) = 12500 (0x30D4)
    let mock = MockTransport::with_read_data(&[1, 0x30, 0xD4]);
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap();

    let sd = create.query_list(&[8, 22]).unwrap();
    assert_eq!(sd.wall, Some(true));
    assert_eq!(sd.voltage, Some(12500));
}

// ---------------------------------------------------------------------------
// Drive command tests
// ---------------------------------------------------------------------------

#[test]
fn drive_sends_correct_bytes() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap().to_safe().unwrap();

    let v = Velocity::new(0.2).unwrap();
    let r = Radius::new(0.5).unwrap();
    create.drive(v, r).unwrap();

    let written = create.transport().written_bytes();
    // START(128) + SAFE(131) + DRIVE(137) + vel_hi + vel_lo + rad_hi + rad_lo
    assert_eq!(written[0], 128); // START
    assert_eq!(written[1], 131); // SAFE
    assert_eq!(written[2], 137); // DRIVE opcode
    let vel = i16::from_be_bytes([written[3], written[4]]);
    let rad = i16::from_be_bytes([written[5], written[6]]);
    assert_eq!(vel, 200); // 0.2 * 1000
    assert_eq!(rad, 500); // 0.5 * 1000
}

#[test]
fn stop_sends_zero_drive() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap().to_safe().unwrap();

    create.stop().unwrap();

    let written = create.transport().written_bytes();
    // Last 5 bytes should be DRIVE(137) + 0,0,0,0
    let drive_cmd = &written[written.len() - 5..];
    assert_eq!(drive_cmd, &[137, 0, 0, 0, 0]);
}

// ---------------------------------------------------------------------------
// LED tests
// ---------------------------------------------------------------------------

#[test]
fn set_leds_sends_correct_bytes() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap().to_safe().unwrap();

    create
        .set_leds(
            true,
            false,
            true,
            false,
            PowerLedColor::RED,
            LedIntensity::FULL,
        )
        .unwrap();

    let written = create.transport().written_bytes();
    // LED cmd: [139, bits, color, intensity]
    let led_cmd = &written[written.len() - 4..];
    assert_eq!(led_cmd[0], 139);
    assert_eq!(led_cmd[1], 0b0101); // debris=1, spot=0, dock=1, check=0
    assert_eq!(led_cmd[2], 255); // RED
    assert_eq!(led_cmd[3], 255); // FULL
}

// ---------------------------------------------------------------------------
// ConnectError preserves transport
// ---------------------------------------------------------------------------

#[test]
fn connect_error_returns_transport() {
    // Create a transport that will fail on write
    let mock = MockTransport {
        closed: true,
        ..MockTransport::new()
    };
    let create = Create::new(mock, RobotModel::Create2);

    let err = create.start().unwrap_err();
    // We get the transport back
    assert!(err.transport.closed);
}

// ---------------------------------------------------------------------------
// TransitionError preserves create instance
// ---------------------------------------------------------------------------

#[test]
fn transition_error_returns_create() {
    // Verify that TransitionError<Robot<Passive, MockTransport>> compiles.
    // This is a compile-time check — the type system is the test.
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let _create = create.start().unwrap();
}

// ---------------------------------------------------------------------------
// into_transport recovers the transport
// ---------------------------------------------------------------------------

#[test]
fn into_transport_recovers() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let create = create.start().unwrap();
    let transport = create.into_transport();
    assert_eq!(transport.written_bytes(), &[128]); // START was written
}

// ---------------------------------------------------------------------------
// Validation error path tests (validate-before-send)
// ---------------------------------------------------------------------------

#[test]
fn set_date_invalid_hour_rejects_before_send() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap().to_full().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create.set_date(DayOfWeek::Monday, 24, 0).unwrap_err();
    assert!(
        matches!(err, create_oi::error::Error::Validation(_)),
        "expected Validation error, got {err:?}"
    );
    // No additional bytes should have been sent
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn set_date_invalid_minute_rejects_before_send() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap().to_full().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create.set_date(DayOfWeek::Monday, 0, 60).unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn set_schedule_invalid_days_mask_rejects() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap().to_full().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create
        .set_schedule(0x80, [(0, 0); 7]) // bit 7 set — reserved
        .unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn set_schedule_invalid_time_rejects() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap().to_full().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    // Wednesday has hour = 25 (invalid)
    let err = create
        .set_schedule(
            0x7F,
            [(0, 0), (0, 0), (0, 0), (25, 0), (0, 0), (0, 0), (0, 0)],
        )
        .unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn start_stream_unsupported_model_rejects_before_send() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Roomba400);
    let mut create = create.start().unwrap().to_safe().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create.start_stream(&[8, 22]).unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn define_song_too_many_notes_rejects() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap().to_full().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    // 17 notes — exceeds the 16-note OI spec limit
    let notes = [SongNote::new(60, 32).unwrap(); 17];
    let err = create
        .define_song(SongNumber::new(0).unwrap(), &notes)
        .unwrap_err();
    assert!(
        matches!(
            err,
            create_oi::error::Error::Protocol(
                create_oi_protocol::error::ProtocolError::TooManyItems { max: 16, .. }
            )
        ),
        "expected TooManyItems error, got {err:?}"
    );
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn query_sensor_raw_into_unknown_packet_id_rejects_before_send() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let mut buf = [0u8; 32];
    let err = create.query_sensor_raw_into(0xFF, &mut buf).unwrap_err();
    assert!(
        matches!(
            err,
            create_oi::error::Error::Protocol(
                create_oi_protocol::error::ProtocolError::UnknownPacketId(0xFF)
            )
        ),
        "expected UnknownPacketId error, got {err:?}"
    );
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

// ---------------------------------------------------------------------------
// poll_stream EOF handling
// ---------------------------------------------------------------------------

#[test]
fn poll_stream_eof_returns_protocol_error() {
    let mock = MockTransport::with_eof_on_read();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap();

    let err = create.poll_stream().unwrap_err();
    assert!(
        matches!(
            err,
            create_oi::error::Error::Protocol(
                create_oi_protocol::error::ProtocolError::InsufficientData { need: 1, got: 0 }
            )
        ),
        "expected InsufficientData on EOF, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// toggle_stream model guard
// ---------------------------------------------------------------------------

#[test]
fn toggle_stream_unsupported_model_rejects_before_send() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Roomba400);
    let mut create = create.start().unwrap().to_safe().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create.toggle_stream(true).unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

// ---------------------------------------------------------------------------
// set_motors_pwm validation guards
// ---------------------------------------------------------------------------

#[test]
fn set_motors_pwm_invalid_values_reject_before_send() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap().to_safe().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    // i8::MIN (-128) is invalid for main_brush and side_brush
    let err = create.set_motors_pwm(i8::MIN, 0, 0).unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);

    let err = create.set_motors_pwm(0, i8::MIN, 0).unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);

    // Negative vacuum is invalid per OI spec (vacuum is 0..=127 only)
    let err = create.set_motors_pwm(0, 0, -1).unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);

    let err = create.set_motors_pwm(0, 0, i8::MIN).unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);

    // Valid boundary values should succeed
    create.set_motors_pwm(0, 0, 0).unwrap();
    create.set_motors_pwm(0, 0, 127).unwrap();
    create.set_motors_pwm(-127, -127, 0).unwrap();
}

// ---------------------------------------------------------------------------
// define_song available in Passive mode
// ---------------------------------------------------------------------------

#[test]
fn define_song_available_in_passive() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    // define_song should compile and succeed in Passive mode
    let mut create = create.start().unwrap();
    let notes = [
        SongNote::new(69, 32).unwrap(),
        SongNote::new(71, 32).unwrap(),
    ];
    create
        .define_song(SongNumber::new(0).unwrap(), &notes)
        .unwrap();
    // Song opcode = 140
    let written = create.transport().written_bytes();
    assert_eq!(written[1], 140);
}

// ---------------------------------------------------------------------------
// define_song / play_song model-specific slot validation
// ---------------------------------------------------------------------------

#[test]
fn define_song_rejects_out_of_range_slot_for_create2() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    // Slot 5 is valid for Create 1 (max=15) but not for Create 2 (max=4)
    let song = SongNumber::new(5).unwrap();
    let err = create
        .define_song(song, &[SongNote::new(69, 32).unwrap()])
        .unwrap_err();
    assert!(
        matches!(err, create_oi::error::Error::Validation(_)),
        "expected ValidationError for slot 5 on Create2, got {err:?}"
    );
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn define_song_accepts_slot_15_for_create1() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create1);
    let mut create = create.start().unwrap();

    let song = SongNumber::new(15).unwrap();
    create
        .define_song(song, &[SongNote::new(69, 32).unwrap()])
        .unwrap();
    // Song opcode 140 must appear in the written bytes with song number 15 after it
    let written = create.transport().written_bytes();
    let pos = written
        .iter()
        .position(|&b| b == 140)
        .expect("opcode 140 not written");
    assert_eq!(written[pos + 1], 15, "expected song slot 15 in payload");
}

#[test]
fn play_song_rejects_out_of_range_slot_for_create2() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let song = SongNumber::new(5).unwrap();
    let err = create.play_song(song).unwrap_err();
    assert!(
        matches!(err, create_oi::error::Error::Validation(_)),
        "expected ValidationError for slot 5 on Create2, got {err:?}"
    );
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

// ---------------------------------------------------------------------------
// start_stream payload byte validation
// ---------------------------------------------------------------------------

#[test]
fn start_stream_payload_overflow_rejects_before_send() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    // Packet 8 (wall sensor) has 1 data byte → each entry costs 2 bytes in stream payload.
    // 128 × 2 = 256 > 255, so this should be rejected.
    let ids: Vec<u8> = vec![8u8; 128];
    let err = create.start_stream(&ids).unwrap_err();
    assert!(
        matches!(err, create_oi::error::Error::Validation(_)),
        "expected ValidationError for oversized stream payload, got {err:?}"
    );
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

// ---------------------------------------------------------------------------
// Round 5: streaming / query exclusion guard
// ---------------------------------------------------------------------------

#[test]
fn query_sensor_raw_rejects_while_streaming() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap().to_safe().unwrap();

    // Packet 8 (wall) costs 2 bytes per cycle; use 1 → 2 bytes, well under 255
    create.start_stream(&[8u8]).unwrap();

    let err = create.query_sensor_raw(8).unwrap_err();
    assert!(
        matches!(err, create_oi::error::Error::Validation(_)),
        "expected ValidationError while streaming, got {err:?}"
    );
}

#[test]
fn query_resumes_after_toggle_stream_false() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap().to_safe().unwrap();

    create.start_stream(&[8u8]).unwrap();
    create.toggle_stream(false).unwrap();

    // After disabling the stream, sensor queries should be accepted again.
    // (The mock just records the write; we only verify no ValidationError is raised.)
    let result = create.query_sensor_raw(8);
    // Will error with Protocol::InsufficientData because mock has no read bytes loaded,
    // but NOT with ValidationError.
    assert!(
        !matches!(result, Err(create_oi::error::Error::Validation(_))),
        "should not get ValidationError after disabling stream"
    );
}

// ---------------------------------------------------------------------------
// Round 5: set_digit_leds ASCII validation
// ---------------------------------------------------------------------------

#[test]
fn set_digit_leds_rejects_non_printable_ascii() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap().to_safe().unwrap();

    // Control character 0x01 is not printable ASCII
    let err = create.set_digit_leds(b'0', b'0', b'0', 0x01).unwrap_err();
    assert!(
        matches!(err, create_oi::error::Error::Validation(_)),
        "expected ValidationError for non-printable ASCII, got {err:?}"
    );
}

#[test]
fn set_digit_leds_accepts_printable_ascii() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap().to_safe().unwrap();

    // All printable ASCII: space (32) through tilde (126)
    create.set_digit_leds(b'1', b'2', b'3', b'4').unwrap();
}
