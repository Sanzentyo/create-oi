//! Mock transport and integration-level Create tests.

use std::io;
use std::time::Duration;

use create_oi::prelude::*;
use create_oi::transport::{BaudConfigurable, Transport};
use create_oi_protocol::types::BaudRate;

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
    /// Last baud rate passed to `set_baud`.
    last_set_baud: Option<BaudRate>,
    /// Number of times `flush()` was called.
    flush_count: u32,
}

impl MockTransport {
    fn new() -> Self {
        Self {
            written: Vec::new(),
            read_buf: Vec::new(),
            read_pos: 0,
            closed: false,
            eof_on_read: false,
            last_set_baud: None,
            flush_count: 0,
        }
    }

    fn with_read_data(data: &[u8]) -> Self {
        Self {
            written: Vec::new(),
            read_buf: data.to_vec(),
            read_pos: 0,
            closed: false,
            eof_on_read: false,
            last_set_baud: None,
            flush_count: 0,
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
            last_set_baud: None,
            flush_count: 0,
        }
    }

    fn written_bytes(&self) -> &[u8] {
        &self.written
    }

    fn flush_count(&self) -> u32 {
        self.flush_count
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
        self.flush_count += 1;
        Ok(())
    }

    fn set_read_timeout(&mut self, _timeout: Option<Duration>) -> io::Result<()> {
        Ok(())
    }
}

impl BaudConfigurable for MockTransport {
    fn set_baud(&mut self, rate: BaudRate) -> io::Result<()> {
        self.last_set_baud = Some(rate);
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
    // Last 5 bytes should be DRIVE_DIRECT(145) + 0,0,0,0
    let drive_cmd = &written[written.len() - 5..];
    assert_eq!(drive_cmd, &[145, 0, 0, 0, 0]);
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
fn poll_stream_eof_returns_disconnected() {
    let mock = MockTransport::with_eof_on_read();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap();

    // Must start a stream before poll_stream is callable
    create.start_stream(&[8]).unwrap(); // write succeeds even with eof_on_read
    let err = create.poll_stream().unwrap_err();
    assert!(
        matches!(err, create_oi::error::Error::Disconnected),
        "expected Disconnected on EOF, got {err:?}"
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

    // Values > 127 are invalid per OI spec (vacuum is 0..=127 only)
    let err = create.set_motors_pwm(0, 0, 128).unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);

    let err = create.set_motors_pwm(0, 0, 255).unwrap_err();
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

// ---------------------------------------------------------------------------
// Create 2–only model gate tests
// ---------------------------------------------------------------------------

#[test]
fn drive_pwm_rejects_create1_before_send() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create1);
    let mut create = create.start().unwrap().to_safe().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create
        .drive_pwm(
            MotorPower::try_from(0.5).unwrap(),
            MotorPower::try_from(-0.5).unwrap(),
        )
        .unwrap_err();
    assert!(
        matches!(err, create_oi::error::Error::Validation(_)),
        "expected Validation error, got {err:?}"
    );
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn drive_pwm_rejects_roomba400_before_send() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Roomba400);
    let mut create = create.start().unwrap().to_safe().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create
        .drive_pwm(
            MotorPower::try_from(0.0).unwrap(),
            MotorPower::try_from(0.0).unwrap(),
        )
        .unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn set_motors_pwm_rejects_create1_before_send() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create1);
    let mut create = create.start().unwrap().to_safe().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create.set_motors_pwm(0, 0, 0).unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn set_digit_leds_rejects_create1_before_send() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create1);
    let mut create = create.start().unwrap().to_safe().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create.set_digit_leds(b'A', b'B', b'C', b'D').unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn set_digit_leds_raw_rejects_create1_before_send() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create1);
    let mut create = create.start().unwrap().to_safe().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create
        .set_digit_leds_raw(0xFF, 0x00, 0xFF, 0x00)
        .unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn simulate_buttons_rejects_create1_before_send() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create1);
    let mut create = create.start().unwrap().to_full().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create.simulate_buttons(ButtonBits::default()).unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn set_schedule_rejects_create1_before_send() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create1);
    let mut create = create.start().unwrap().to_full().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create
        .set_schedule(
            0b0000001,
            [(8, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)],
        )
        .unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn set_date_rejects_create1_before_send() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create1);
    let mut create = create.start().unwrap().to_full().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create.set_date(DayOfWeek::Monday, 10, 30).unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

// ---------------------------------------------------------------------------
// reset() available in Off mode
// ---------------------------------------------------------------------------

#[test]
fn reset_available_in_off_mode() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    // reset() is available before start() — the OI spec allows RESET at any time
    let transport = create.reset().unwrap();
    assert_eq!(transport.written_bytes(), &[7]); // OPCODE 7 = RESET
}

// ---------------------------------------------------------------------------
// Round 10: to_off() model gate
// ---------------------------------------------------------------------------

#[test]
fn to_off_rejects_create1_before_send_from_passive() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create1);
    let create = create.start().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create.to_off().unwrap_err();
    assert!(
        matches!(err.source, create_oi::error::Error::Validation(_)),
        "expected ValidationError, got {err:?}"
    );
    assert_eq!(err.create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn to_off_rejects_roomba400_before_send_from_passive() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Roomba400);
    let create = create.start().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create.to_off().unwrap_err();
    assert!(
        matches!(err.source, create_oi::error::Error::Validation(_)),
        "expected ValidationError, got {err:?}"
    );
    assert_eq!(err.create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn to_off_rejects_create1_before_send_from_safe() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create1);
    let create = create.start().unwrap().to_safe().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create.to_off().unwrap_err();
    assert!(matches!(err.source, create_oi::error::Error::Validation(_)));
    assert_eq!(err.create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn to_off_rejects_create1_before_send_from_full() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create1);
    let create = create.start().unwrap().to_full().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create.to_off().unwrap_err();
    assert!(matches!(err.source, create_oi::error::Error::Validation(_)));
    assert_eq!(err.create.transport().written_bytes().len(), bytes_before);
}

#[test]
fn to_off_succeeds_on_create2_sends_stop_opcode() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let create = create.start().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let off = create.to_off().unwrap();
    let written = off.transport().written_bytes();
    assert_eq!(
        written[bytes_before], 173,
        "expected STOP opcode 173, got {}",
        written[bytes_before]
    );
}

// ---------------------------------------------------------------------------
// Round 12: power_off() now returns Create<Off, T> per OI spec (opcode 133 → Off mode)
// ---------------------------------------------------------------------------

#[test]
fn power_off_returns_off_handle() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let create = create.start().unwrap();
    let bytes_before = create.transport().written_bytes().len();

    // power_off() must return Create<Off, _> — the robot powers down
    let off = create.power_off().unwrap();
    let written = off.transport().written_bytes();
    // POWER opcode = 133
    assert_eq!(
        written[bytes_before], 133,
        "expected POWER opcode 133, got {}",
        written[bytes_before]
    );
    // Verify the handle is Off — start() is only callable on Create<Off, _>
    let _passive = off.start().unwrap();
}

#[test]
fn power_off_clears_streaming_state() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap();

    // Start a stream first
    create.start_stream(&[8u8]).unwrap();
    // After power_off(), the Off handle must have streaming=false
    let off = create.power_off().unwrap();
    // Call start() to get an interactive handle; the new Passive handle inherits Off's cleared state
    let mut passive = off.start().unwrap();
    // query_sensor_raw should NOT return a streaming ValidationError
    let result = passive.query_sensor_raw(8);
    assert!(
        !matches!(result, Err(create_oi::error::Error::Validation(_))),
        "power_off should have cleared streaming state, got ValidationError"
    );
}

// ---------------------------------------------------------------------------
// Round 10: set_date / set_schedule available in Passive and Safe modes
// ---------------------------------------------------------------------------

#[test]
fn set_date_available_in_passive_mode() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap();

    // Must compile and succeed (no FullControl requirement)
    create.set_date(DayOfWeek::Monday, 10, 30).unwrap();
    // SET_DAY_TIME opcode = 168
    let written = create.transport().written_bytes();
    assert!(written.contains(&168), "expected opcode 168 in payload");
}

#[test]
fn set_date_available_in_safe_mode() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap().to_safe().unwrap();

    create.set_date(DayOfWeek::Friday, 9, 0).unwrap();
    let written = create.transport().written_bytes();
    assert!(written.contains(&168), "expected opcode 168 in payload");
}

#[test]
fn set_schedule_available_in_passive_mode() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap();

    // Must compile and succeed (no FullControl requirement)
    create
        .set_schedule(
            0b0000001,
            [(8, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)],
        )
        .unwrap();
    // SCHEDULE opcode = 167
    let written = create.transport().written_bytes();
    assert!(written.contains(&167), "expected opcode 167 in payload");
}

#[test]
fn set_schedule_available_in_safe_mode() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let mut create = create.start().unwrap().to_safe().unwrap();

    create
        .set_schedule(
            0b0000010,
            [(0, 0), (7, 30), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)],
        )
        .unwrap();
    let written = create.transport().written_bytes();
    assert!(written.contains(&167), "expected opcode 167 in payload");
}

// ---------------------------------------------------------------------------
// Round 10: clean() / seek_dock() Passive-only
// ---------------------------------------------------------------------------

#[test]
fn clean_available_in_passive_mode() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let create = create.start().unwrap();

    let passive = create.clean(CleanMode::Default).unwrap();
    // CLEAN opcode = 135
    let written = passive.transport().written_bytes();
    assert!(written.contains(&135), "expected opcode 135 in payload");
}

#[test]
fn seek_dock_available_in_passive_mode() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2);
    let create = create.start().unwrap();

    let passive = create.seek_dock().unwrap();
    // DOCK opcode = 143
    let written = passive.transport().written_bytes();
    assert!(
        written.contains(&143),
        "expected DOCK opcode 143 in payload"
    );
}

// ---------------------------------------------------------------------------
// Round 11: start_stream unknown packet validation, set_scheduling_leds
// ---------------------------------------------------------------------------

#[test]
fn start_stream_rejects_unknown_packet_id_before_send() {
    let transport = MockTransport::new();
    let mut robot = Create::new(transport, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    let written_before = robot.transport().written_bytes().to_vec();
    let result = robot.start_stream(&[8, 99, 22]); // 99 is not a valid packet ID
    assert!(result.is_err(), "should reject unknown packet ID 99");
    let written_after = robot.transport().written_bytes().to_vec();
    assert_eq!(
        written_before, written_after,
        "no bytes should be sent when an unknown ID is present"
    );
}

#[test]
fn start_stream_accepts_group_packet_id() {
    let transport = MockTransport::new();
    let mut robot = Create::new(transport, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    // Group 0 covers packets 7-26; payload fits within 255 bytes
    let result = robot.start_stream(&[0]);
    assert!(
        result.is_ok(),
        "group packet ID 0 should be accepted; got {result:?}"
    );
    assert!(
        robot.transport().written_bytes().contains(&148),
        "STREAM opcode 148 should be sent"
    );
}

#[test]
fn start_stream_accepts_valid_packet_ids() {
    let transport = MockTransport::new();
    let mut robot = Create::new(transport, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    let result = robot.start_stream(&[8, 22, 19]); // wall, voltage, distance — all valid
    assert!(result.is_ok(), "should accept valid packet IDs");
    assert!(
        robot.transport().written_bytes().contains(&148),
        "STREAM opcode 148 should be sent"
    );
}

#[test]
fn set_scheduling_leds_sends_correct_bytes() {
    let transport = MockTransport::new();
    let mut robot = Create::new(transport, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    robot.set_scheduling_leds(0b0101010, 0b0011).unwrap();
    let written = robot.transport().written_bytes();
    // Expect: [162, 0b0101010, 0b0011]
    assert!(
        written.ends_with(&[162, 0b0101010, 0b0011]),
        "expected scheduling LEDs command bytes; got {written:?}"
    );
}

#[test]
fn set_scheduling_leds_rejects_create1() {
    let transport = MockTransport::new();
    let mut robot = Create::new(transport, RobotModel::Create1)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    let result = robot.set_scheduling_leds(0x7f, 0x0f);
    assert!(
        result.is_err(),
        "set_scheduling_leds should fail on Create 1"
    );
}

#[test]
fn set_scheduling_leds_rejects_roomba400() {
    let transport = MockTransport::new();
    let mut robot = Create::new(transport, RobotModel::Roomba400)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    let result = robot.set_scheduling_leds(0x7f, 0x0f);
    assert!(
        result.is_err(),
        "set_scheduling_leds should fail on Roomba 400"
    );
}

// ---------------------------------------------------------------------------
// Round 12: clean()/seek_dock() available from Safe and Full modes
// ---------------------------------------------------------------------------

#[test]
fn clean_available_in_safe_mode() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    let passive = create.clean(CleanMode::Default).unwrap();
    let written = passive.transport().written_bytes();
    // SAFE=131 (start→safe), CLEAN=135
    assert!(
        written.contains(&135),
        "expected CLEAN opcode 135 in payload"
    );
}

#[test]
fn clean_available_in_full_mode() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap()
        .to_full()
        .unwrap();

    let passive = create.clean(CleanMode::Spot).unwrap();
    let written = passive.transport().written_bytes();
    // SPOT=134
    assert!(
        written.contains(&134),
        "expected SPOT opcode 134 in payload"
    );
}

#[test]
fn seek_dock_available_in_safe_mode() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    let passive = create.seek_dock().unwrap();
    let written = passive.transport().written_bytes();
    // DOCK=143
    assert!(
        written.contains(&143),
        "expected DOCK opcode 143 in payload"
    );
}

#[test]
fn seek_dock_available_in_full_mode() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap()
        .to_full()
        .unwrap();

    let passive = create.seek_dock().unwrap();
    let written = passive.transport().written_bytes();
    assert!(
        written.contains(&143),
        "expected DOCK opcode 143 in payload"
    );
}

// ---------------------------------------------------------------------------
// Round 12: query_sensor_raw accepts group packet IDs (0-6, 100)
// ---------------------------------------------------------------------------

#[test]
fn query_sensor_raw_with_group_id_zero() {
    // Group 0 spans packets 7-26; get expected byte count from protocol
    let group_len = create_oi::protocol::opcode::group_data_len(0).expect("group 0 must be known");
    let read_data = vec![0u8; group_len];
    let mock = MockTransport::with_read_data(&read_data);
    let mut create = Create::new(mock, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    let result = create.query_sensor_raw(0);
    assert!(
        result.is_ok(),
        "group ID 0 should be accepted, got {:?}",
        result
    );
    assert_eq!(result.unwrap().len(), group_len);
}

#[test]
fn query_sensor_raw_into_with_group_id_100() {
    // Group 100 = all individual packets (52 packets); largest group
    let group_len =
        create_oi::protocol::opcode::group_data_len(100).expect("group 100 must be known");
    let read_data = vec![0u8; group_len];
    let mock = MockTransport::with_read_data(&read_data);
    let mut create = Create::new(mock, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    let mut buf = vec![0u8; group_len];
    let result = create.query_sensor_raw_into(100, &mut buf);
    assert!(
        result.is_ok(),
        "group ID 100 should be accepted, got {:?}",
        result
    );
    assert_eq!(result.unwrap(), group_len);
}

#[test]
fn query_sensor_raw_still_rejects_truly_unknown_id() {
    let mock = MockTransport::new();
    let mut create = Create::new(mock, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    // 101 is not a valid individual or group packet ID
    let result = create.query_sensor_raw(101);
    assert!(
        matches!(result, Err(create_oi::error::Error::Protocol(_))),
        "unknown ID 101 should return ProtocolError"
    );
}

// ---------------------------------------------------------------------------
// Baud rate command tests
// ---------------------------------------------------------------------------

#[test]
fn baud_sends_correct_bytes_and_calls_set_baud() {
    let mock = MockTransport::new();
    let mut create = Create::new(mock, RobotModel::Create2).start().unwrap();

    create.baud(BaudRate::Baud57600).unwrap();

    let written = create.transport().written_bytes();
    // START (128) + BAUD (129) + baud_code (10)
    assert_eq!(&written[written.len() - 2..], &[129, 10]);
    assert_eq!(create.transport().last_set_baud, Some(BaudRate::Baud57600));
}

#[test]
fn baud_available_from_passive_mode() {
    let mock = MockTransport::new();
    let mut create = Create::new(mock, RobotModel::Create2).start().unwrap();

    // Passive mode: baud() should compile and succeed
    assert!(create.baud(BaudRate::Baud115200).is_ok());
}

#[test]
fn baud_available_from_safe_mode() {
    let mock = MockTransport::new();
    let mut create = Create::new(mock, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    assert!(create.baud(BaudRate::Baud115200).is_ok());
    assert_eq!(create.transport().last_set_baud, Some(BaudRate::Baud115200));
}

#[test]
fn baud_available_from_full_mode() {
    let mock = MockTransport::new();
    let mut create = Create::new(mock, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap()
        .to_full()
        .unwrap();

    assert!(create.baud(BaudRate::Baud9600).is_ok());
    assert_eq!(create.transport().last_set_baud, Some(BaudRate::Baud9600));
}

#[test]
fn baud_rate_baud_u32_all_codes() {
    assert_eq!(BaudRate::Baud300.baud_u32(), 300);
    assert_eq!(BaudRate::Baud600.baud_u32(), 600);
    assert_eq!(BaudRate::Baud1200.baud_u32(), 1200);
    assert_eq!(BaudRate::Baud2400.baud_u32(), 2400);
    assert_eq!(BaudRate::Baud4800.baud_u32(), 4800);
    assert_eq!(BaudRate::Baud9600.baud_u32(), 9600);
    assert_eq!(BaudRate::Baud14400.baud_u32(), 14400);
    assert_eq!(BaudRate::Baud19200.baud_u32(), 19200);
    assert_eq!(BaudRate::Baud28800.baud_u32(), 28800);
    assert_eq!(BaudRate::Baud38400.baud_u32(), 38400);
    assert_eq!(BaudRate::Baud57600.baud_u32(), 57600);
    assert_eq!(BaudRate::Baud115200.baud_u32(), 115200);
}

#[test]
fn baud_rate_from_code_round_trip() {
    for code in 0u8..=11 {
        let rate = BaudRate::from_code(code).expect("valid code");
        assert_eq!(rate as u8, code);
    }
    assert!(BaudRate::from_code(12).is_none());
    assert!(BaudRate::from_code(255).is_none());
}

// ---------------------------------------------------------------------------
// Round 14: duplicate IDs, poll_stream guard, scheduling_leds reserved bits
// ---------------------------------------------------------------------------

#[test]
fn start_stream_rejects_duplicate_ids() {
    let transport = MockTransport::new();
    let mut robot = Create::new(transport, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    let written_before = robot.transport().written_bytes().to_vec();
    let result = robot.start_stream(&[8, 22, 8]); // duplicate packet 8
    assert!(result.is_err(), "should reject duplicate packet IDs");
    let written_after = robot.transport().written_bytes().to_vec();
    assert_eq!(
        written_before, written_after,
        "no bytes sent when duplicates detected"
    );
}

#[test]
fn query_list_rejects_duplicate_ids() {
    let transport = MockTransport::new();
    let mut robot = Create::new(transport, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    let written_before = robot.transport().written_bytes().to_vec();
    let result = robot.query_list(&[7, 8, 7]); // duplicate packet 7
    assert!(
        result.is_err(),
        "should reject duplicate packet IDs in query_list"
    );
    let written_after = robot.transport().written_bytes().to_vec();
    assert_eq!(
        written_before, written_after,
        "no bytes sent when duplicates detected"
    );
}

#[test]
fn poll_stream_rejects_when_not_streaming() {
    let transport = MockTransport::new();
    let mut robot = Create::new(transport, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    let result = robot.poll_stream();
    assert!(
        result.is_err(),
        "poll_stream should fail when not streaming"
    );
    let err_msg = format!("{:?}", result.unwrap_err());
    assert!(
        err_msg.contains("start_stream"),
        "error message should mention start_stream(); got {err_msg}"
    );
}

#[test]
fn poll_stream_with_rejects_when_not_streaming() {
    let transport = MockTransport::new();
    let mut robot = Create::new(transport, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    let result = robot.poll_stream_with(|_| {});
    assert!(
        result.is_err(),
        "poll_stream_with should fail when not streaming"
    );
}

#[test]
fn set_scheduling_leds_rejects_reserved_day_leds_bit7() {
    let transport = MockTransport::new();
    let mut robot = Create::new(transport, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    let result = robot.set_scheduling_leds(0x80, 0x00); // bit 7 of day_leds set
    assert!(result.is_err(), "should reject reserved bit 7 in day_leds");
    let err_msg = format!("{:?}", result.unwrap_err());
    assert!(
        err_msg.contains("day_leds"),
        "error should name the day_leds field"
    );
}

#[test]
fn set_scheduling_leds_rejects_reserved_schedule_leds_upper_nibble() {
    let transport = MockTransport::new();
    let mut robot = Create::new(transport, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    let result = robot.set_scheduling_leds(0x7F, 0xF0); // upper 4 bits of schedule_leds set
    assert!(
        result.is_err(),
        "should reject reserved upper bits in schedule_leds"
    );
    let err_msg = format!("{:?}", result.unwrap_err());
    assert!(
        err_msg.contains("schedule_leds"),
        "error should name the schedule_leds field"
    );
}

#[test]
fn set_scheduling_leds_accepts_valid_bits() {
    let transport = MockTransport::new();
    let mut robot = Create::new(transport, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    let result = robot.set_scheduling_leds(0x7F, 0x0F); // all valid bits
    assert!(
        result.is_ok(),
        "should accept fully-set valid bits; got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Model-guard tests (OI spec compliance)
// ---------------------------------------------------------------------------

#[test]
fn roomba400_passive_to_safe_uses_control_opcode() {
    // Roomba 400 SCI uses CONTROL (130) for Passive→Safe, not SAFE (131).
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Roomba400);
    let create = create.start().unwrap().to_safe().unwrap();
    let written = create.transport().written_bytes();
    // START(128) + CONTROL(130)
    assert_eq!(written, &[128, 130]);
}

#[test]
fn create1_passive_to_safe_uses_safe_opcode() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create1);
    let create = create.start().unwrap().to_safe().unwrap();
    let written = create.transport().written_bytes();
    assert_eq!(written, &[128, 131]);
}

#[test]
fn drive_direct_rejected_on_roomba400() {
    let mock = MockTransport::new();
    let mut robot = Create::new(mock, RobotModel::Roomba400)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();
    let result = robot.drive_direct(Velocity::new(0.1).unwrap(), Velocity::new(0.1).unwrap());
    assert!(
        result.is_err(),
        "drive_direct must be rejected on Roomba 400"
    );
    let err_msg = format!("{:?}", result.unwrap_err());
    assert!(
        err_msg.contains("model"),
        "error should name the model field"
    );
}

#[test]
fn drive_twist_rejected_on_roomba400() {
    let mock = MockTransport::new();
    let mut robot = Create::new(mock, RobotModel::Roomba400)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();
    let result = robot.drive_twist(
        Velocity::new(0.1).unwrap(),
        AngularVelocity::new(0.0).unwrap(),
    );
    assert!(
        result.is_err(),
        "drive_twist must be rejected on Roomba 400"
    );
}

#[test]
fn stop_on_roomba400_uses_drive_opcode() {
    // stop() on Roomba 400 must use Drive (opcode 137), not Drive Direct (opcode 145).
    let mock = MockTransport::new();
    let mut robot = Create::new(mock, RobotModel::Roomba400)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();
    robot.stop().unwrap();
    let written = robot.transport().written_bytes();
    // START(128) + CONTROL(130) + DRIVE(137, velocity=0x0000, radius=0x8000)
    assert_eq!(written, &[128, 130, 137, 0x00, 0x00, 0x80, 0x00]);
}

#[test]
fn stop_on_create2_uses_drive_direct_opcode() {
    let mock = MockTransport::new();
    let mut robot = Create::new(mock, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();
    robot.stop().unwrap();
    let written = robot.transport().written_bytes();
    // START(128) + SAFE(131) + DRIVE_DIRECT(145, right=0x0000, left=0x0000)
    assert_eq!(written, &[128, 131, 145, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn clean_max_rejected_on_create1_from_passive() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create1).start().unwrap();
    let result = create.clean(CleanMode::Max);
    assert!(
        result.is_err(),
        "CleanMode::Max must be rejected on Create 1"
    );
    let err_msg = format!("{:?}", result.unwrap_err());
    assert!(err_msg.contains("mode"), "error should name the mode field");
}

#[test]
fn clean_max_rejected_on_create1_from_safe() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create1)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();
    let result = create.clean(CleanMode::Max);
    assert!(
        result.is_err(),
        "CleanMode::Max must be rejected on Create 1"
    );
}

#[test]
fn clean_max_accepted_on_create2() {
    let mock = MockTransport::new();
    let create = Create::new(mock, RobotModel::Create2).start().unwrap();
    let result = create.clean(CleanMode::Max);
    assert!(
        result.is_ok(),
        "CleanMode::Max must be accepted on Create 2"
    );
}

#[test]
fn query_list_rejected_on_roomba400() {
    let mock = MockTransport::with_read_data(&[]);
    let mut robot = Create::new(mock, RobotModel::Roomba400)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();
    let result = robot.query_list(&[0]);
    assert!(result.is_err(), "query_list must be rejected on Roomba 400");
    let err_msg = format!("{:?}", result.unwrap_err());
    assert!(
        err_msg.contains("model"),
        "error should name the model field"
    );
}

#[test]
fn query_sensor_individual_packet_rejected_on_roomba400() {
    // Roomba 400 does not support individual sensor packet IDs (7+).
    let mock = MockTransport::with_read_data(&[0; 2]);
    let mut robot = Create::new(mock, RobotModel::Roomba400)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();
    // Packet 7 is the first individual packet (bump/wheel drop, 1 byte).
    let result = robot.query_sensor_raw(7);
    assert!(
        result.is_err(),
        "individual sensor packets must be rejected on Roomba 400"
    );
}

#[test]
fn query_sensor_packet_43_rejected_on_create1() {
    // Packet 43+ are Create 2 only.
    let mock = MockTransport::with_read_data(&[0; 2]);
    let mut robot = Create::new(mock, RobotModel::Create1)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();
    let result = robot.query_sensor_raw(43);
    assert!(result.is_err(), "packet 43 must be rejected on Create 1");
}

#[test]
fn query_sensor_group_100_rejected_on_create1() {
    // Group 100 is Create 2 only.
    let mock = MockTransport::with_read_data(&[0; 100]);
    let mut robot = Create::new(mock, RobotModel::Create1)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();
    let result = robot.query_sensor_raw(100);
    assert!(result.is_err(), "group 100 must be rejected on Create 1");
}

#[test]
fn query_sensor_group_0_accepted_on_roomba400() {
    // Group 0 is supported on all models.
    // Group 0 data = 26 bytes per spec.
    let mock = MockTransport::with_read_data(&[0; 26]);
    let mut robot = Create::new(mock, RobotModel::Roomba400)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();
    let result = robot.query_sensor_raw(0);
    assert!(
        result.is_ok(),
        "group 0 must be accepted on Roomba 400; got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Round 15: BaudRate codes, straight sentinel, set_leds Roomba 400 bit layout
// ---------------------------------------------------------------------------

#[test]
fn baud_38400_sends_code_9() {
    let mock = MockTransport::new();
    let mut create = Create::new(mock, RobotModel::Create2).start().unwrap();

    create.baud(BaudRate::Baud38400).unwrap();

    let written = create.transport().written_bytes();
    assert_eq!(&written[written.len() - 2..], &[129, 9]);
    assert_eq!(create.transport().last_set_baud, Some(BaudRate::Baud38400));
}

#[test]
fn baud_57600_sends_code_10() {
    let mock = MockTransport::new();
    let mut create = Create::new(mock, RobotModel::Create2).start().unwrap();

    create.baud(BaudRate::Baud57600).unwrap();

    let written = create.transport().written_bytes();
    assert_eq!(&written[written.len() - 2..], &[129, 10]);
}

#[test]
fn baud_115200_sends_code_11() {
    let mock = MockTransport::new();
    let mut create = Create::new(mock, RobotModel::Create2).start().unwrap();

    create.baud(BaudRate::Baud115200).unwrap();

    let written = create.transport().written_bytes();
    assert_eq!(&written[written.len() - 2..], &[129, 11]);
}

#[test]
fn set_leds_create2_uses_low_bits() {
    let mock = MockTransport::new();
    let mut create = Create::new(mock, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    // debris=true(bit0), spot=true(bit1), dock=false, check_robot=false → 0b0000_0011 = 3
    create
        .set_leds(
            true,
            true,
            false,
            false,
            PowerLedColor::GREEN,
            LedIntensity::new(128),
        )
        .unwrap();

    let written = create.transport().written_bytes();
    // LEDS (139) + led_bits (3) + color (0) + intensity (128)
    let last4 = &written[written.len() - 4..];
    assert_eq!(last4[0], 139);
    assert_eq!(last4[1], 0b0000_0011, "Create2: debris=bit0, spot=bit1");
}

#[test]
fn set_leds_roomba400_uses_high_bits() {
    let mock = MockTransport::new();
    let mut create = Create::new(mock, RobotModel::Roomba400)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    // spot=true → bit6; check_robot=true → bit4
    // → 0b0101_0000 = 0x50
    create
        .set_leds(
            false,
            true,
            false,
            true,
            PowerLedColor::GREEN,
            LedIntensity::new(0),
        )
        .unwrap();

    let written = create.transport().written_bytes();
    let last4 = &written[written.len() - 4..];
    assert_eq!(last4[0], 139);
    assert_eq!(
        last4[1], 0b0101_0000,
        "Roomba400: spot=bit6, check_robot=bit4"
    );
}

#[test]
fn set_leds_roomba400_debris_dock_bits() {
    let mock = MockTransport::new();
    let mut create = Create::new(mock, RobotModel::Roomba400)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();

    // debris=true → bit3; dock=true → bit5
    // → 0b0010_1000 = 0x28
    create
        .set_leds(
            true,
            false,
            true,
            false,
            PowerLedColor::GREEN,
            LedIntensity::new(0),
        )
        .unwrap();

    let written = create.transport().written_bytes();
    let last4 = &written[written.len() - 4..];
    assert_eq!(last4[0], 139);
    assert_eq!(last4[1], 0b0010_1000, "Roomba400: debris=bit3, dock=bit5");
}

// ---------------------------------------------------------------------------
// write_bytes / send_cmd flush discipline tests
// ---------------------------------------------------------------------------

/// Verify that query_sensor (request-response path) does not call flush().
/// This documents the `write_bytes()` vs `send_cmd()` split: sensor queries
/// must not incur the tcdrain latency of a flush.
#[test]
fn query_sensor_does_not_flush() {
    // Packet 35 = OI mode = 1 byte.
    let mock = MockTransport::with_read_data(&[3u8]);
    let mut create = Create::new(mock, RobotModel::Create2).start().unwrap();
    let flush_before = create.transport().flush_count();
    let _ = create.query_sensor(35).unwrap();
    assert_eq!(
        create.transport().flush_count(),
        flush_before,
        "query_sensor must not call flush()"
    );
}

/// Verify that query_list (request-response path) does not call flush().
#[test]
fn query_list_does_not_flush() {
    // Packets 7 (bumps, 1 byte) + 35 (OI mode, 1 byte) = 2 bytes.
    let mock = MockTransport::with_read_data(&[0u8, 3u8]);
    let mut create = Create::new(mock, RobotModel::Create2).start().unwrap();
    let flush_before = create.transport().flush_count();
    let _ = create.query_list(&[7, 35]).unwrap();
    assert_eq!(
        create.transport().flush_count(),
        flush_before,
        "query_list must not call flush()"
    );
}

/// Verify that a fire-and-forget command (drive) calls flush() via send_cmd().
#[test]
fn send_cmd_drive_calls_flush() {
    let mock = MockTransport::new();
    let mut create = Create::new(mock, RobotModel::Create2)
        .start()
        .unwrap()
        .to_safe()
        .unwrap();
    let flush_before = create.transport().flush_count();
    create
        .drive(Velocity::new(0.1).unwrap(), Radius::STRAIGHT)
        .unwrap();
    assert!(
        create.transport().flush_count() > flush_before,
        "drive (send_cmd) must call flush()"
    );
}
