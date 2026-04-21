//! Basic synchronous example using `SerialTransport`.
//!
//! Connects to a robot, enters Safe mode, queries a few sensors, and
//! drives briefly before stopping and disconnecting.

use create_oi::prelude::*;
use create_oi_serial::SerialTransport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/ttyUSB0".into());

    println!("Opening {port}...");
    let transport = SerialTransport::open(&port, RobotModel::Create2)?;

    // Connect — enters Off → Passive
    let create = Create::new(transport, RobotModel::Create2);
    let create = create.start().map_err(|e| e.source)?;

    // Passive → Safe
    let mut create = create.to_safe().map_err(|e| e.source)?;

    // Query battery voltage
    let sd = create.query_list(&[22])?;
    println!("Battery voltage: {:?} mV", sd.voltage);

    // Drive forward at 200 mm/s straight
    create.drive(Velocity::new(0.2)?, Radius::STRAIGHT)?;
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Stop motors
    create.stop()?;

    // Return to passive mode (stop is already done)
    let _create = create.to_passive().map_err(|e| e.source)?;
    // Robot is dropped here, transport closes automatically.

    println!("Done!");
    Ok(())
}
