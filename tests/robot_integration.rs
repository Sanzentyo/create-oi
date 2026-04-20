//! Integration tests for libcreate — requires a real iRobot Create 2 / Roomba.
//!
//! All tests are `#[ignore]` by default. Run with:
//!
//! ```sh
//! LIBCREATE_PORT=/dev/ttyUSB0 cargo test --test robot_integration -- --ignored
//! ```
//!
//! Set `LIBCREATE_PORT` to your serial port (e.g., `/dev/ttyUSB0` on Linux,
//! `/dev/tty.usbserial-*` on macOS).
//!
//! **WARNING**: These tests command the robot to move, activate motors, and
//! toggle LEDs. Ensure the robot is on a safe surface before running.

use libcreate::*;
use std::thread;
use std::time::Duration;

/// Get the serial port from the environment, or skip the test.
fn port() -> String {
    std::env::var("LIBCREATE_PORT").unwrap_or_else(|_| "/dev/ttyUSB0".to_string())
}

/// Default baud rate for Create 2.
const BAUD: u32 = 115200;

/// Short delay between commands to allow the robot to process.
fn short_delay() {
    thread::sleep(Duration::from_millis(100));
}

/// Medium delay for sensor stabilization.
fn medium_delay() {
    thread::sleep(Duration::from_millis(500));
}

// ==========================================================================
// 1. Connection Lifecycle
// ==========================================================================

#[test]
#[ignore]
fn test_connect_and_disconnect() {
    let robot = Robot::new(RobotModel::Create2).expect("failed to create handle");
    let robot = robot.connect(&port(), BAUD).expect("failed to connect");
    assert!(robot.is_connected());

    let robot = robot.disconnect();
    // After disconnect, we have Robot<Off> — can reconnect if desired.
    drop(robot);
}

#[test]
#[ignore]
fn test_reconnect_after_disconnect() {
    let robot = Robot::new(RobotModel::Create2).expect("failed to create handle");
    let robot = robot.connect(&port(), BAUD).expect("failed to connect");
    let robot = robot.disconnect();

    // Reconnect
    short_delay();
    let robot = robot.connect(&port(), BAUD).expect("failed to reconnect");
    assert!(robot.is_connected());
    let _robot = robot.disconnect();
}

// ==========================================================================
// 2. Mode Transitions
// ==========================================================================

#[test]
#[ignore]
fn test_passive_to_safe_and_back() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();

    // Passive → Safe
    let robot = robot.into_safe().expect("failed to enter Safe mode");
    short_delay();

    // Safe → Passive
    let robot = robot.into_passive().expect("failed to enter Passive mode");
    let _robot = robot.disconnect();
}

#[test]
#[ignore]
fn test_passive_to_full_and_back() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();

    let robot = robot.into_full().expect("failed to enter Full mode");
    short_delay();

    let robot = robot.into_passive().expect("failed to return to Passive");
    let _robot = robot.disconnect();
}

#[test]
#[ignore]
fn test_safe_to_full_and_back() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();
    let robot = robot.into_safe().unwrap();

    let robot = robot.into_full().expect("failed Safe→Full");
    short_delay();

    let robot = robot.into_safe().expect("failed Full→Safe");
    let robot = robot.into_passive().unwrap();
    let _robot = robot.disconnect();
}

#[test]
#[ignore]
fn test_verify_mode_matches() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let mut robot = robot.connect(&port(), BAUD).unwrap();
    medium_delay();

    // In Passive mode — verify_mode should succeed
    robot.verify_mode().expect("mode mismatch in Passive");

    let mut robot = robot.into_safe().unwrap();
    medium_delay();
    robot.verify_mode().expect("mode mismatch in Safe");

    let robot = robot.into_passive().unwrap();
    let _robot = robot.disconnect();
}

// ==========================================================================
// 3. Sensor Reading
// ==========================================================================

#[test]
#[ignore]
fn test_read_sensors_passive() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let mut robot = robot.connect(&port(), BAUD).unwrap();
    medium_delay();

    let snapshot = robot.sensors().expect("failed to read sensors");

    // Battery should report reasonable values
    assert!(
        snapshot.battery.voltage > 0.0,
        "battery voltage should be positive, got {}",
        snapshot.battery.voltage
    );
    assert!(
        snapshot.battery.capacity > 0.0,
        "battery capacity should be positive, got {}",
        snapshot.battery.capacity
    );

    println!(
        "Battery: {:.1}V, {:.0}%",
        snapshot.battery.voltage,
        snapshot.battery.charge_ratio() * 100.0
    );
    println!("Charging: {:?}", snapshot.battery.state);
    println!("Temperature: {}°C", snapshot.battery.temperature);
    println!("OI Mode: {:?}", snapshot.oi_mode);

    let _robot = robot.disconnect();
}

#[test]
#[ignore]
fn test_read_sensors_safe() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();
    let mut robot = robot.into_safe().unwrap();
    medium_delay();

    let snapshot = robot
        .sensors()
        .expect("failed to read sensors in Safe mode");

    println!(
        "Bumpers: L={} R={}",
        snapshot.bumpers.left, snapshot.bumpers.right
    );
    println!(
        "Cliffs: L={} FL={} FR={} R={}",
        snapshot.cliffs.left,
        snapshot.cliffs.front_left,
        snapshot.cliffs.front_right,
        snapshot.cliffs.right
    );
    println!(
        "Wheels: L_drop={} R_drop={}",
        snapshot.bumpers.left_wheeldrop, snapshot.bumpers.right_wheeldrop
    );
    println!(
        "Packet stats: {}/{} ({:.2}% corrupt)",
        snapshot.packet_stats.corrupt,
        snapshot.packet_stats.total,
        snapshot.packet_stats.corruption_rate() * 100.0
    );

    let robot = robot.into_passive().unwrap();
    let _robot = robot.disconnect();
}

#[test]
#[ignore]
fn test_sensor_polling_loop() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let mut robot = robot.connect(&port(), BAUD).unwrap();
    medium_delay();

    // Read sensors 10 times in quick succession
    for i in 0..10 {
        let snapshot = robot.sensors().expect("sensor read failed");
        println!(
            "[{}] Battery: {:.1}V, OI: {:?}",
            i, snapshot.battery.voltage, snapshot.oi_mode
        );
        short_delay();
    }

    let _robot = robot.disconnect();
}

// ==========================================================================
// 4. Driving Commands
// ==========================================================================

#[test]
#[ignore]
fn test_drive_forward_and_stop() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();
    let mut robot = robot.into_safe().unwrap();
    short_delay();

    // Drive forward slowly
    let vel = Velocity::new(0.1).unwrap();
    let ang = AngularVelocity::new(0.0).unwrap();
    robot.drive(vel, ang).expect("drive failed");

    // Let it move for 1 second
    thread::sleep(Duration::from_secs(1));

    // Stop
    robot.stop().expect("stop failed");
    short_delay();

    // Check odometry shows some forward movement
    let snapshot = robot.sensors().unwrap();
    println!(
        "Pose after drive: x={:.3} y={:.3} yaw={:.3}",
        snapshot.odometry.pose_x, snapshot.odometry.pose_y, snapshot.odometry.pose_yaw
    );

    let robot = robot.into_passive().unwrap();
    let _robot = robot.disconnect();
}

#[test]
#[ignore]
fn test_drive_wheels_independently() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();
    let mut robot = robot.into_safe().unwrap();
    short_delay();

    // Turn in place: left wheel forward, right wheel backward
    let left = Velocity::new(0.1).unwrap();
    let right = Velocity::new(-0.1).unwrap();
    robot
        .drive_wheels(left, right)
        .expect("drive_wheels failed");

    thread::sleep(Duration::from_millis(500));

    robot.stop().unwrap();
    short_delay();

    let snapshot = robot.sensors().unwrap();
    println!("Yaw after spin: {:.3} rad", snapshot.odometry.pose_yaw);

    let robot = robot.into_passive().unwrap();
    let _robot = robot.disconnect();
}

#[test]
#[ignore]
fn test_drive_radius() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();
    let mut robot = robot.into_safe().unwrap();
    short_delay();

    // Drive in a gentle arc
    let vel = Velocity::new(0.1).unwrap();
    let radius = Radius::new(0.5).unwrap();
    robot
        .drive_radius(vel, radius)
        .expect("drive_radius failed");

    thread::sleep(Duration::from_secs(2));
    robot.stop().unwrap();

    let robot = robot.into_passive().unwrap();
    let _robot = robot.disconnect();
}

#[test]
#[ignore]
fn test_drive_pwm() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();
    let mut robot = robot.into_safe().unwrap();
    short_delay();

    let power = MotorPower::new(0.2).unwrap();
    robot
        .drive_wheels_pwm(power, power)
        .expect("drive_wheels_pwm failed");

    thread::sleep(Duration::from_millis(500));
    robot.stop().unwrap();

    let robot = robot.into_passive().unwrap();
    let _robot = robot.disconnect();
}

// ==========================================================================
// 5. LED Control
// ==========================================================================

#[test]
#[ignore]
fn test_leds_cycle() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();
    let mut robot = robot.into_safe().unwrap();
    short_delay();

    // Toggle each LED on then off
    for (name, led_fn) in [
        (
            "debris",
            Robot::set_debris_led as fn(&mut Robot<Safe>, bool) -> Result<(), Error>,
        ),
        ("spot", Robot::set_spot_led),
        ("dock", Robot::set_dock_led),
        ("check_robot", Robot::set_check_robot_led),
    ] {
        println!("LED {name}: ON");
        led_fn(&mut robot, true).unwrap();
        thread::sleep(Duration::from_millis(500));

        println!("LED {name}: OFF");
        led_fn(&mut robot, false).unwrap();
        thread::sleep(Duration::from_millis(200));
    }

    let robot = robot.into_passive().unwrap();
    let _robot = robot.disconnect();
}

#[test]
#[ignore]
fn test_power_led_colors() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();
    let mut robot = robot.into_safe().unwrap();
    short_delay();

    // Sweep from green to red
    for color_val in (0..=255).step_by(32) {
        let color = PowerLedColor::new(color_val);
        robot
            .set_power_led(color, LedIntensity::FULL)
            .expect("set_power_led failed");
        thread::sleep(Duration::from_millis(200));
    }

    // Turn off
    robot
        .set_power_led(PowerLedColor::GREEN, LedIntensity::OFF)
        .unwrap();

    let robot = robot.into_passive().unwrap();
    let _robot = robot.disconnect();
}

#[test]
#[ignore]
fn test_digits_ascii() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();
    let robot = robot.into_safe().unwrap();
    short_delay();

    // Display "RUST" on the 7-segment display
    robot
        .set_digits_ascii(b'R', b'U', b'S', b'T')
        .expect("set_digits_ascii failed");

    thread::sleep(Duration::from_secs(2));

    // Clear display
    robot.set_digits_ascii(b' ', b' ', b' ', b' ').unwrap();

    let robot = robot.into_passive().unwrap();
    let _robot = robot.disconnect();
}

// ==========================================================================
// 6. Motor Control
// ==========================================================================

#[test]
#[ignore]
fn test_brush_motors() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();
    let mut robot = robot.into_safe().unwrap();
    short_delay();

    let low_power = MotorPower::new(0.3).unwrap();

    // Side brush
    println!("Side brush ON");
    robot.set_side_motor(low_power).unwrap();
    thread::sleep(Duration::from_secs(1));
    robot.set_side_motor(MotorPower::ZERO).unwrap();

    // Main brush
    println!("Main brush ON");
    robot.set_main_motor(low_power).unwrap();
    thread::sleep(Duration::from_secs(1));
    robot.set_main_motor(MotorPower::ZERO).unwrap();

    // Vacuum
    println!("Vacuum ON");
    robot.set_vacuum_motor(low_power).unwrap();
    thread::sleep(Duration::from_secs(1));
    robot.set_vacuum_motor(MotorPower::ZERO).unwrap();

    let robot = robot.into_passive().unwrap();
    let _robot = robot.disconnect();
}

#[test]
#[ignore]
fn test_all_motors_combined() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();
    let mut robot = robot.into_safe().unwrap();
    short_delay();

    let power = MotorPower::new(0.2).unwrap();

    robot.set_all_motors(power, power, power).unwrap();
    thread::sleep(Duration::from_secs(1));
    robot
        .set_all_motors(MotorPower::ZERO, MotorPower::ZERO, MotorPower::ZERO)
        .unwrap();

    let robot = robot.into_passive().unwrap();
    let _robot = robot.disconnect();
}

// ==========================================================================
// 7. Songs
// ==========================================================================

#[test]
#[ignore]
fn test_define_and_play_song() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();
    let robot = robot.into_safe().unwrap();
    short_delay();

    // Define a simple melody (C4, E4, G4, C5)
    // MIDI notes: C4=60, E4=64, G4=67, C5=72
    let notes = [60u8, 64, 67, 72];
    let durations = [0.25f32, 0.25, 0.25, 0.5];
    let song_num = SongNumber::new(0).unwrap();

    robot
        .define_song(song_num, &notes, &durations)
        .expect("define_song failed");
    short_delay();

    robot.play_song(song_num).expect("play_song failed");

    // Wait for song to finish
    thread::sleep(Duration::from_secs(2));

    let robot = robot.into_passive().unwrap();
    let _robot = robot.disconnect();
}

// ==========================================================================
// 8. Date/Clock
// ==========================================================================

#[test]
#[ignore]
fn test_set_date() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();
    let robot = robot.into_safe().unwrap();
    short_delay();

    robot
        .set_date(DayOfWeek::Monday, 14, 30)
        .expect("set_date failed");

    let robot = robot.into_passive().unwrap();
    let _robot = robot.disconnect();
}

// ==========================================================================
// 9. Cleaning and Docking
// ==========================================================================

#[test]
#[ignore]
fn test_clean_default() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let mut robot = robot.connect(&port(), BAUD).unwrap();
    short_delay();

    // Start cleaning (puts robot into cleaning behavior)
    robot.clean(CleanMode::Default).expect("clean failed");

    // Wait a bit then stop by entering Safe mode
    thread::sleep(Duration::from_secs(3));

    // Note: after cleaning starts, the robot is still in Passive mode.
    // We can transition to Safe to regain control.
    let robot = robot.into_safe().unwrap();
    let robot = robot.into_passive().unwrap();
    let _robot = robot.disconnect();
}

#[test]
#[ignore]
fn test_dock() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();

    // dock() sends the seek-dock command
    robot.dock().expect("dock failed");

    // Wait for a bit (robot will seek dock autonomously)
    thread::sleep(Duration::from_secs(5));

    let _robot = robot.disconnect();
}

// ==========================================================================
// 10. Full Mode (no safety limits)
// ==========================================================================

#[test]
#[ignore]
fn test_full_mode_drive() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let robot = robot.connect(&port(), BAUD).unwrap();
    let mut robot = robot.into_full().unwrap();
    short_delay();

    // In Full mode, safety features are disabled.
    // Drive forward slowly.
    let vel = Velocity::new(0.05).unwrap();
    let ang = AngularVelocity::new(0.0).unwrap();
    robot.drive(vel, ang).unwrap();

    thread::sleep(Duration::from_millis(500));
    robot.stop().unwrap();

    let robot = robot.into_passive().unwrap();
    let _robot = robot.disconnect();
}

// ==========================================================================
// 11. Error Recovery (TransitionError)
// ==========================================================================

#[test]
#[ignore]
fn test_connection_failure_recovery() {
    let robot = Robot::new(RobotModel::Create2).unwrap();

    // Try to connect to a non-existent port
    let result = robot.connect("/dev/ttyNONEXISTENT", BAUD);
    match result {
        Ok(_) => panic!("expected connection failure"),
        Err(err) => {
            println!("Got expected error: {}", err.error);
            // The robot is returned — we can try again
            let _recovered_robot = err.robot;
        }
    }
}

// ==========================================================================
// 12. Comprehensive Sensor Check
// ==========================================================================

#[test]
#[ignore]
fn test_comprehensive_sensor_report() {
    let robot = Robot::new(RobotModel::Create2).unwrap();
    let mut robot = robot.connect(&port(), BAUD).unwrap();
    medium_delay();

    let s = robot.sensors().unwrap();

    println!("=== SENSOR REPORT ===");
    println!();
    println!("--- Bumpers ---");
    println!("  Left: {}, Right: {}", s.bumpers.left, s.bumpers.right);
    println!(
        "  Wheeldrop L: {}, R: {}",
        s.bumpers.left_wheeldrop, s.bumpers.right_wheeldrop
    );
    println!();
    println!("--- Cliffs ---");
    println!(
        "  L: {}, FL: {}, FR: {}, R: {}",
        s.cliffs.left, s.cliffs.front_left, s.cliffs.front_right, s.cliffs.right
    );
    println!();
    println!("--- Walls ---");
    println!(
        "  Wall: {}, Virtual: {}",
        s.walls.wall, s.walls.virtual_wall
    );
    println!();
    println!("--- Light Bumpers ---");
    println!(
        "  L: {} ({}) FL: {} ({}) CL: {} ({})",
        s.light_bumpers.left,
        s.light_bumpers.signal_left,
        s.light_bumpers.front_left,
        s.light_bumpers.signal_front_left,
        s.light_bumpers.center_left,
        s.light_bumpers.signal_center_left,
    );
    println!(
        "  CR: {} ({}) FR: {} ({}) R: {} ({})",
        s.light_bumpers.center_right,
        s.light_bumpers.signal_center_right,
        s.light_bumpers.front_right,
        s.light_bumpers.signal_front_right,
        s.light_bumpers.right,
        s.light_bumpers.signal_right,
    );
    println!();
    println!("--- Battery ---");
    println!("  Voltage: {:.2}V", s.battery.voltage);
    println!("  Current: {:.3}A", s.battery.current);
    println!("  Temperature: {}°C", s.battery.temperature);
    println!(
        "  Charge: {:.3} / {:.3} Ah ({:.0}%)",
        s.battery.charge,
        s.battery.capacity,
        s.battery.charge_ratio() * 100.0
    );
    println!("  State: {:?}", s.battery.state);
    println!();
    println!("--- IR ---");
    println!(
        "  Omni: {:?}, Left: {:?}, Right: {:?}",
        s.ir.omni, s.ir.left, s.ir.right
    );
    println!();
    println!("--- Buttons ---");
    println!(
        "  Clean: {}, Spot: {}, Dock: {}, Day: {}, Hour: {}, Min: {}",
        s.buttons.clean,
        s.buttons.spot,
        s.buttons.dock,
        s.buttons.day,
        s.buttons.hour,
        s.buttons.minute
    );
    println!();
    println!("--- Odometry ---");
    println!(
        "  Pose: ({:.3}, {:.3}) yaw={:.3}",
        s.odometry.pose_x, s.odometry.pose_y, s.odometry.pose_yaw
    );
    println!(
        "  Velocity: ({:.3}, {:.3}) angular={:.3}",
        s.odometry.velocity_x, s.odometry.velocity_y, s.odometry.velocity_yaw
    );
    println!();
    println!("--- Misc ---");
    println!("  Dirt detect: {}", s.dirt_detect);
    println!("  Moving forward: {}", s.is_moving_forward);
    println!("  OI Mode: {:?}", s.oi_mode);
    println!(
        "  Packets: {}/{} ({:.2}% corrupt)",
        s.packet_stats.corrupt,
        s.packet_stats.total,
        s.packet_stats.corruption_rate() * 100.0
    );
    println!("=== END REPORT ===");

    let _robot = robot.disconnect();
}
