//! Basic synchronous usage example.
//!
//! ```bash
//! cargo run --example basic_sync --features serial -- /dev/ttyUSB0
//! ```

use libcreate::io::serial::SerialTransport;
use libcreate::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/ttyUSB0".to_string());

    println!("Opening {port} for Create 2...");
    let transport = SerialTransport::open(&port, RobotModel::Create2)?;

    let robot = Robot::new(transport, RobotModel::Create2);
    println!("Sending START...");
    let robot = robot.start().map_err(|e| e.source)?;
    println!("In Passive mode. Querying OI mode...");

    let mut robot_passive = robot;
    let mode = robot_passive.read_oi_mode()?;
    println!("OI mode: {}", mode.name());

    println!("Transitioning to Safe mode...");
    let mut robot_safe = robot_passive.to_safe().map_err(|e| e.source)?;

    println!("Setting LEDs...");
    robot_safe.set_leds(
        true,
        false,
        false,
        false,
        PowerLedColor::GREEN,
        LedIntensity::FULL,
    )?;

    println!("Driving forward slowly for 2 seconds...");
    robot_safe.drive(Velocity::new(0.1)?, Radius::STRAIGHT)?;
    std::thread::sleep(std::time::Duration::from_secs(2));

    println!("Stopping...");
    robot_safe.stop()?;

    println!("Returning to Passive...");
    let _robot = robot_safe.to_passive().map_err(|e| e.source)?;
    println!("Done!");

    Ok(())
}
