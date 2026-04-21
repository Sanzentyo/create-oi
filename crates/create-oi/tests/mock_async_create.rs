//! Async mock transport and integration-level AsyncCreate tests.

use std::io;
use std::time::Duration;

use create_oi::prelude::*;
use create_oi::transport::AsyncTransport;

// ---------------------------------------------------------------------------
// Async mock transport
// ---------------------------------------------------------------------------

/// An async mock transport that records writes and replays pre-loaded read data.
#[derive(Debug)]
struct MockAsyncTransport {
    written: Vec<u8>,
    read_buf: Vec<u8>,
    read_pos: usize,
    closed: bool,
    /// When true, `read()` returns `Ok(0)` to simulate EOF/disconnect.
    eof_on_read: bool,
}

impl MockAsyncTransport {
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
}

impl AsyncTransport for MockAsyncTransport {
    type Error = io::Error;

    async fn write_all(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        if self.closed {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "closed"));
        }
        self.written.extend_from_slice(data);
        Ok(())
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
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

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn delay(&self, _duration: Duration) {
        // No-op in tests — no real delay needed.
    }
}

// ---------------------------------------------------------------------------
// Mode transition tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn async_create_start_sends_start_opcode() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);

    let create = create.start().await.unwrap();
    let written = create.transport().written_bytes();
    assert_eq!(written, &[128]); // START opcode
}

#[tokio::test]
async fn async_passive_to_safe() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let create = create.start().await.unwrap();

    let create = create.to_safe().await.unwrap();
    let written = create.transport().written_bytes();
    assert_eq!(written, &[128, 131]); // START + SAFE
}

#[tokio::test]
async fn async_passive_to_full() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let create = create.start().await.unwrap();

    let create = create.to_full().await.unwrap();
    let written = create.transport().written_bytes();
    assert_eq!(written, &[128, 132]); // START + FULL
}

#[tokio::test]
async fn async_safe_to_full() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let create = create.start().await.unwrap().to_safe().await.unwrap();

    let create = create.to_full().await.unwrap();
    let written = create.transport().written_bytes();
    assert_eq!(written, &[128, 131, 132]);
}

#[tokio::test]
async fn async_full_to_safe() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let create = create.start().await.unwrap().to_full().await.unwrap();

    let create = create.to_safe().await.unwrap();
    let written = create.transport().written_bytes();
    assert_eq!(written, &[128, 132, 131]);
}

#[tokio::test]
async fn async_passive_to_off() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let create = create.start().await.unwrap();

    let off = create.to_off().await.unwrap();
    let transport = off.into_transport();
    assert_eq!(transport.written_bytes(), &[128, 173]); // START + STOP
}

#[tokio::test]
async fn async_safe_to_off() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let create = create.start().await.unwrap().to_safe().await.unwrap();

    let off = create.to_off().await.unwrap();
    let transport = off.into_transport();
    assert_eq!(transport.written_bytes(), &[128, 131, 173]); // START + SAFE + STOP
}

#[tokio::test]
async fn async_full_to_off() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let create = create.start().await.unwrap().to_full().await.unwrap();

    let off = create.to_off().await.unwrap();
    let transport = off.into_transport();
    assert_eq!(transport.written_bytes(), &[128, 132, 173]); // START + FULL + STOP
}

#[tokio::test]
async fn async_full_to_passive() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let create = create.start().await.unwrap().to_full().await.unwrap();

    let create = create.to_passive().await.unwrap();
    let written = create.transport().written_bytes();
    assert_eq!(written, &[128, 132, 128]); // START + FULL + START
}

// ---------------------------------------------------------------------------
// Sensor query tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn async_query_single_sensor() {
    // OI mode (packet 35) = 2 (Safe), 1 byte response
    let mock = MockAsyncTransport::with_read_data(&[2]);
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap();

    let sd = create.query_sensor(35).await.unwrap();
    assert_eq!(sd.oi_mode, Some(OiMode::Safe));

    // Verify: START(128) + SENSORS(142) + packet_id(35)
    let written = create.transport().written_bytes();
    assert_eq!(written, &[128, 142, 35]);
}

#[tokio::test]
async fn async_query_list_multiple_sensors() {
    // wall(id=8, 1 byte) = 1, voltage(id=22, 2 bytes) = 12500 (0x30D4)
    let mock = MockAsyncTransport::with_read_data(&[1, 0x30, 0xD4]);
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap();

    let sd = create.query_list(&[8, 22]).await.unwrap();
    assert_eq!(sd.wall, Some(true));
    assert_eq!(sd.voltage, Some(12500));
}

// ---------------------------------------------------------------------------
// Drive command tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn async_drive_sends_correct_bytes() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap().to_safe().await.unwrap();

    let v = Velocity::new(0.2).unwrap();
    let r = Radius::new(0.5).unwrap();
    create.drive(v, r).await.unwrap();

    let written = create.transport().written_bytes();
    assert_eq!(written[0], 128); // START
    assert_eq!(written[1], 131); // SAFE
    assert_eq!(written[2], 137); // DRIVE opcode
    let vel = i16::from_be_bytes([written[3], written[4]]);
    let rad = i16::from_be_bytes([written[5], written[6]]);
    assert_eq!(vel, 200); // 0.2 * 1000
    assert_eq!(rad, 500); // 0.5 * 1000
}

#[tokio::test]
async fn async_stop_sends_zero_drive() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap().to_safe().await.unwrap();

    create.stop().await.unwrap();

    let written = create.transport().written_bytes();
    let drive_cmd = &written[written.len() - 5..];
    assert_eq!(drive_cmd, &[137, 0, 0, 0, 0]);
}

// ---------------------------------------------------------------------------
// LED tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn async_set_leds() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap().to_safe().await.unwrap();

    create
        .set_leds(
            true,
            false,
            true,
            false,
            PowerLedColor::RED,
            LedIntensity::FULL,
        )
        .await
        .unwrap();

    let written = create.transport().written_bytes();
    let led_cmd = &written[written.len() - 4..];
    assert_eq!(led_cmd[0], 139);
    assert_eq!(led_cmd[1], 0b0101); // debris=1, spot=0, dock=1, check=0
    assert_eq!(led_cmd[2], 255); // RED
    assert_eq!(led_cmd[3], 255); // FULL
}

// ---------------------------------------------------------------------------
// ConnectError preserves transport
// ---------------------------------------------------------------------------

#[tokio::test]
async fn async_connect_error_returns_transport() {
    let mock = MockAsyncTransport {
        closed: true,
        ..MockAsyncTransport::new()
    };
    let create = AsyncCreate::new(mock, RobotModel::Create2);

    let err = create.start().await.unwrap_err();
    assert!(err.transport.closed);
}

// ---------------------------------------------------------------------------
// into_transport recovers the transport
// ---------------------------------------------------------------------------

#[tokio::test]
async fn async_into_transport_recovers() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let create = create.start().await.unwrap();
    let transport = create.into_transport();
    assert_eq!(transport.written_bytes(), &[128]); // START was written
}

// ---------------------------------------------------------------------------
// Validation error path tests (validate-before-send)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn async_set_date_invalid_hour_rejects_before_send() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap().to_full().await.unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create.set_date(DayOfWeek::Monday, 24, 0).await.unwrap_err();
    assert!(
        matches!(err, create_oi::error::Error::Validation(_)),
        "expected Validation error, got {err:?}"
    );
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[tokio::test]
async fn async_set_date_invalid_minute_rejects_before_send() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap().to_full().await.unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create.set_date(DayOfWeek::Monday, 0, 60).await.unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[tokio::test]
async fn async_set_schedule_invalid_days_mask_rejects() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap().to_full().await.unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create.set_schedule(0x80, [(0, 0); 7]).await.unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[tokio::test]
async fn async_set_schedule_invalid_time_rejects() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap().to_full().await.unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create
        .set_schedule(
            0x7F,
            [(0, 0), (0, 0), (0, 0), (25, 0), (0, 0), (0, 0), (0, 0)],
        )
        .await
        .unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[tokio::test]
async fn async_query_list_too_many_ids_rejects_before_send() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap();
    let bytes_before = create.transport().written_bytes().len();

    // 53 IDs exceeds the async stack buffer limit of 52.
    let ids: Vec<u8> = (7..60).collect(); // 53 IDs
    let err = create.query_list(&ids).await.unwrap_err();
    assert!(
        matches!(err, create_oi::error::Error::Validation(_)),
        "expected ValidationError, got {err:?}"
    );
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[tokio::test]
async fn async_start_stream_too_many_ids_rejects_before_send() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap().to_safe().await.unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let ids: Vec<u8> = (7..60).collect(); // 53 IDs
    let err = create.start_stream(&ids).await.unwrap_err();
    assert!(
        matches!(err, create_oi::error::Error::Validation(_)),
        "expected ValidationError, got {err:?}"
    );
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[tokio::test]
async fn async_start_stream_unsupported_model_rejects_before_send() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Roomba400);
    let mut create = create.start().await.unwrap().to_safe().await.unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let err = create.start_stream(&[8, 22]).await.unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[tokio::test]
async fn async_query_sensor_raw_into_unknown_packet_id_rejects_before_send() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let mut buf = [0u8; 32];
    let err = create
        .query_sensor_raw_into(0xFF, &mut buf)
        .await
        .unwrap_err();
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

#[tokio::test]
async fn async_poll_stream_eof_returns_protocol_error() {
    let mock = MockAsyncTransport::with_eof_on_read();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap();

    let err = create.poll_stream().await.unwrap_err();
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
// toggle_stream model guard (async, unsupported model)
// already tested above for Roomba400; this tests the *supported* model passes
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// set_motors_pwm validation guards
// ---------------------------------------------------------------------------

#[tokio::test]
async fn async_set_motors_pwm_invalid_values_reject_before_send() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap().to_safe().await.unwrap();
    let bytes_before = create.transport().written_bytes().len();

    // i8::MIN (-128) is invalid for main_brush and side_brush
    let err = create.set_motors_pwm(i8::MIN, 0, 0).await.unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);

    let err = create.set_motors_pwm(0, i8::MIN, 0).await.unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);

    // Negative vacuum is invalid per OI spec (vacuum is 0..=127 only)
    let err = create.set_motors_pwm(0, 0, -1).await.unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);

    let err = create.set_motors_pwm(0, 0, i8::MIN).await.unwrap_err();
    assert!(matches!(err, create_oi::error::Error::Validation(_)));
    assert_eq!(create.transport().written_bytes().len(), bytes_before);

    // Valid boundary values should succeed
    create.set_motors_pwm(0, 0, 0).await.unwrap();
    create.set_motors_pwm(0, 0, 127).await.unwrap();
    create.set_motors_pwm(-127, -127, 0).await.unwrap();
}

// ---------------------------------------------------------------------------
// define_song available in Passive mode
// ---------------------------------------------------------------------------

#[tokio::test]
async fn async_define_song_available_in_passive() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap();
    let notes = [
        SongNote::new(69, 32).unwrap(),
        SongNote::new(71, 32).unwrap(),
    ];
    create
        .define_song(SongNumber::new(0).unwrap(), &notes)
        .await
        .unwrap();
    // Song opcode = 140
    let written = create.transport().written_bytes();
    assert_eq!(written[1], 140);
}

// ---------------------------------------------------------------------------
// define_song / play_song model-specific slot validation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn async_define_song_rejects_out_of_range_slot_for_create2() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let song = SongNumber::new(5).unwrap();
    let err = create
        .define_song(song, &[SongNote::new(69, 32).unwrap()])
        .await
        .unwrap_err();
    assert!(
        matches!(err, create_oi::error::Error::Validation(_)),
        "expected ValidationError for slot 5 on Create2, got {err:?}"
    );
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

#[tokio::test]
async fn async_define_song_accepts_slot_15_for_create1() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create1);
    let mut create = create.start().await.unwrap();

    let song = SongNumber::new(15).unwrap();
    create
        .define_song(song, &[SongNote::new(69, 32).unwrap()])
        .await
        .unwrap();
    let written = create.transport().written_bytes();
    let pos = written
        .iter()
        .position(|&b| b == 140)
        .expect("opcode 140 not written");
    assert_eq!(written[pos + 1], 15, "expected song slot 15 in payload");
}

#[tokio::test]
async fn async_play_song_rejects_out_of_range_slot_for_create2() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap();
    let bytes_before = create.transport().written_bytes().len();

    let song = SongNumber::new(5).unwrap();
    let err = create.play_song(song).await.unwrap_err();
    assert!(
        matches!(err, create_oi::error::Error::Validation(_)),
        "expected ValidationError for slot 5 on Create2, got {err:?}"
    );
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

// ---------------------------------------------------------------------------
// start_stream payload byte validation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn async_start_stream_payload_overflow_rejects_before_send() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap();
    let bytes_before = create.transport().written_bytes().len();

    // Packet 8 (wall sensor) has 1 data byte → each entry costs 2 bytes.
    // 128 × 2 = 256 > 255, should be rejected.
    let ids: Vec<u8> = vec![8u8; 128];
    let err = create.start_stream(&ids).await.unwrap_err();
    assert!(
        matches!(err, create_oi::error::Error::Validation(_)),
        "expected ValidationError for oversized stream payload, got {err:?}"
    );
    assert_eq!(create.transport().written_bytes().len(), bytes_before);
}

// ---------------------------------------------------------------------------
// Round 5: streaming / query exclusion guard
// ---------------------------------------------------------------------------

#[tokio::test]
async fn async_query_sensor_raw_rejects_while_streaming() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap().to_safe().await.unwrap();

    create.start_stream(&[8u8]).await.unwrap();

    let err = create.query_sensor_raw(8).await.unwrap_err();
    assert!(
        matches!(err, create_oi::error::Error::Validation(_)),
        "expected ValidationError while streaming, got {err:?}"
    );
}

#[tokio::test]
async fn async_query_resumes_after_toggle_stream_false() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap().to_safe().await.unwrap();

    create.start_stream(&[8u8]).await.unwrap();
    create.toggle_stream(false).await.unwrap();

    // After disabling the stream, sensor queries should not raise ValidationError.
    let result = create.query_sensor_raw(8).await;
    assert!(
        !matches!(result, Err(create_oi::error::Error::Validation(_))),
        "should not get ValidationError after disabling stream"
    );
}

// ---------------------------------------------------------------------------
// Round 5: set_digit_leds ASCII validation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn async_set_digit_leds_rejects_non_printable_ascii() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap().to_safe().await.unwrap();

    let err = create
        .set_digit_leds(b'0', b'0', b'0', 0x01)
        .await
        .unwrap_err();
    assert!(
        matches!(err, create_oi::error::Error::Validation(_)),
        "expected ValidationError for non-printable ASCII, got {err:?}"
    );
}

#[tokio::test]
async fn async_set_digit_leds_accepts_printable_ascii() {
    let mock = MockAsyncTransport::new();
    let create = AsyncCreate::new(mock, RobotModel::Create2);
    let mut create = create.start().await.unwrap().to_safe().await.unwrap();

    create.set_digit_leds(b'1', b'2', b'3', b'4').await.unwrap();
}
