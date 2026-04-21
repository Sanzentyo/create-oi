//! Clean and dock example using `SerialTransport`.
//!
//! Demonstrates:
//! - `clean(CleanMode::Spot)` — start a spot-clean cycle, then reclaim control
//! - `seek_dock` — send the robot back to its charging dock
//!
//! Both commands transition the robot to Passive mode (consuming the handle).
//! Transitioning back to Safe reclaims control and stops any ongoing
//! autonomous operation.
//!
//! # Usage
//!
//! ```text
//! cargo run --example dock_sync -- /dev/ttyUSB0
//! ```

use std::time::Duration;

use create_oi::prelude::*;
use create_oi_serial::SerialTransport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/ttyUSB0".into());

    println!("Opening {port}...");
    let transport = SerialTransport::open(&port, RobotModel::Create2)?;

    let create = Create::new(transport, RobotModel::Create2);
    let create = create.start().map_err(|e| e.source)?;
    let create = create.to_safe().map_err(|e| e.source)?;

    // --- Spot clean for a few seconds, then reclaim control ---
    // `clean()` consumes the Safe handle and returns a Passive handle —
    // the robot drives autonomously in spot-clean mode.
    println!("Starting spot clean (5 seconds)...");
    let create = create.clean(CleanMode::Spot).map_err(|e| e.source)?;
    std::thread::sleep(Duration::from_secs(5));

    // Transitioning back to Safe reclaims control and aborts any ongoing
    // autonomous operation. We then query sensors before heading to dock.
    let mut create = create.to_safe().map_err(|e| e.source)?;
    let sd = create.query_list(&[22])?;
    println!("Battery voltage: {:?} mV", sd.voltage);

    create.stop()?;

    // --- Seek dock: robot navigates back to the charging dock ---
    // seek_dock() also transitions to Passive and returns a Create<Passive, T>.
    println!("Sending robot to dock...");
    let _create = create.seek_dock().map_err(|e| e.source)?;

    // In a real application you would poll sensors to detect the charging
    // state (packet 21 = ChargingState) and wait for docking to complete.
    std::thread::sleep(Duration::from_secs(3));

    println!("Done! (Robot is now seeking dock autonomously.)");
    Ok(())
}
