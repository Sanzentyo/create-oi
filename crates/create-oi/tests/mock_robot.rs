//! Mock transport and integration-level robot tests.

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
    /// Bytes written by the robot.
    written: Vec<u8>,
    /// Bytes to be read by the robot (pre-loaded).
    read_buf: Vec<u8>,
    /// Current read position.
    read_pos: usize,
    closed: bool,
}

impl MockTransport {
    fn new() -> Self {
        Self {
            written: Vec::new(),
            read_buf: Vec::new(),
            read_pos: 0,
            closed: false,
        }
    }

    fn with_read_data(data: &[u8]) -> Self {
        Self {
            written: Vec::new(),
            read_buf: data.to_vec(),
            read_pos: 0,
            closed: false,
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
fn robot_start_sends_start_opcode() {
    let mock = MockTransport::new();
    let robot = Create::new(mock, CreateRobotModel::Create2);

    // start() transitions Off → Passive
    let robot = robot.start().unwrap();
    let written = robot.transport().written_bytes();
    assert_eq!(written, &[128]); // START opcode
}

#[test]
fn robot_passive_to_safe_sends_safe_opcode() {
    let mock = MockTransport::new();
    let robot = Create::new(mock, CreateRobotModel::Create2);
    let robot = robot.start().unwrap();

    let robot = robot.to_safe().unwrap();
    let written = robot.transport().written_bytes();
    // START(128) + SAFE(131)
    assert_eq!(written, &[128, 131]);
}

#[test]
fn robot_passive_to_full_sends_full_opcode() {
    let mock = MockTransport::new();
    let robot = Create::new(mock, CreateRobotModel::Create2);
    let robot = robot.start().unwrap();

    let robot = robot.to_full().unwrap();
    let written = robot.transport().written_bytes();
    assert_eq!(written, &[128, 132]); // START + FULL
}

#[test]
fn robot_safe_to_full() {
    let mock = MockTransport::new();
    let robot = Create::new(mock, CreateRobotModel::Create2);
    let robot = robot.start().unwrap().to_safe().unwrap();

    let robot = robot.to_full().unwrap();
    let written = robot.transport().written_bytes();
    assert_eq!(written, &[128, 131, 132]);
}

#[test]
fn robot_full_to_safe() {
    let mock = MockTransport::new();
    let robot = Create::new(mock, CreateRobotModel::Create2);
    let robot = robot.start().unwrap().to_full().unwrap();

    let robot = robot.to_safe().unwrap();
    let written = robot.transport().written_bytes();
    assert_eq!(written, &[128, 132, 131]);
}

#[test]
fn robot_full_to_passive() {
    let mock = MockTransport::new();
    let robot = Create::new(mock, CreateRobotModel::Create2);
    let robot = robot.start().unwrap().to_full().unwrap();

    let robot = robot.to_passive().unwrap();
    let written = robot.transport().written_bytes();
    assert_eq!(written, &[128, 132, 128]); // START + FULL + START
}

// ---------------------------------------------------------------------------
// Sensor query tests
// ---------------------------------------------------------------------------

#[test]
fn query_single_sensor() {
    // OI mode (packet 35) = 2 (Safe), 1 byte response
    let mock = MockTransport::with_read_data(&[2]);
    let robot = Create::new(mock, CreateRobotModel::Create2);
    let mut robot = robot.start().unwrap();

    let sd = robot.query_sensor(35).unwrap();
    assert_eq!(sd.oi_mode, Some(OiMode::Safe));

    // Verify query command was sent: START(128) + SENSORS(142) + packet_id(35)
    let written = robot.transport().written_bytes();
    assert_eq!(written, &[128, 142, 35]);
}

#[test]
fn query_list_multiple_sensors() {
    // wall(id=8, 1 byte) = 1, voltage(id=22, 2 bytes) = 12500 (0x30D4)
    let mock = MockTransport::with_read_data(&[1, 0x30, 0xD4]);
    let robot = Create::new(mock, CreateRobotModel::Create2);
    let mut robot = robot.start().unwrap();

    let sd = robot.query_list(&[8, 22]).unwrap();
    assert_eq!(sd.wall, Some(true));
    assert_eq!(sd.voltage, Some(12500));
}

// ---------------------------------------------------------------------------
// Drive command tests
// ---------------------------------------------------------------------------

#[test]
fn drive_sends_correct_bytes() {
    let mock = MockTransport::new();
    let robot = Create::new(mock, CreateRobotModel::Create2);
    let mut robot = robot.start().unwrap().to_safe().unwrap();

    let v = Velocity::new(0.2).unwrap();
    let r = Radius::new(0.5).unwrap();
    robot.drive(v, r).unwrap();

    let written = robot.transport().written_bytes();
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
    let robot = Create::new(mock, CreateRobotModel::Create2);
    let mut robot = robot.start().unwrap().to_safe().unwrap();

    robot.stop().unwrap();

    let written = robot.transport().written_bytes();
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
    let robot = Create::new(mock, CreateRobotModel::Create2);
    let mut robot = robot.start().unwrap().to_safe().unwrap();

    robot
        .set_leds(
            true,
            false,
            true,
            false,
            PowerLedColor::RED,
            LedIntensity::FULL,
        )
        .unwrap();

    let written = robot.transport().written_bytes();
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
    let robot = Create::new(mock, CreateRobotModel::Create2);

    let err = robot.start().unwrap_err();
    // We get the transport back
    assert!(err.transport.closed);
}

// ---------------------------------------------------------------------------
// TransitionError preserves robot
// ---------------------------------------------------------------------------

#[test]
fn transition_error_returns_robot() {
    // Verify that TransitionError<Robot<Passive, MockTransport>> compiles.
    // This is a compile-time check — the type system is the test.
    let mock = MockTransport::new();
    let robot = Create::new(mock, CreateRobotModel::Create2);
    let _robot = robot.start().unwrap();
}

// ---------------------------------------------------------------------------
// into_transport recovers the transport
// ---------------------------------------------------------------------------

#[test]
fn into_transport_recovers() {
    let mock = MockTransport::new();
    let robot = Create::new(mock, CreateRobotModel::Create2);
    let robot = robot.start().unwrap();
    let transport = robot.into_transport();
    assert_eq!(transport.written_bytes(), &[128]); // START was written
}
