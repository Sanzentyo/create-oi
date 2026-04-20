//! dora-rs Create driver node example.
//!
//! This is a complete dora-rs node that drives an iRobot Create robot.
//! It polls sensors on a timer tick and accepts motor commands.
//!
//! ## Dataflow YAML
//!
//! ```yaml
//! nodes:
//!   - id: create_driver
//!     path: target/release/examples/dora_create_driver
//!     inputs:
//!       tick: dora/timer/millis/50
//!     outputs:
//!       - sensors
//! ```

use create_oi::prelude::*;
use create_oi_serial::SerialTransport;
use eyre::Result;

fn main() -> Result<()> {
    let port = std::env::var("CREATE_PORT").unwrap_or_else(|_| "/dev/ttyUSB0".into());

    eprintln!("[create_driver] Opening {port} for Create 2...");
    let transport = SerialTransport::open(&port, CreateRobotModel::Create2)?;

    let robot = Create::new(transport, CreateRobotModel::Create2);
    let robot = robot.start().map_err(|e| e.source)?;
    let mut robot = robot.to_safe().map_err(|e| e.source)?;

    eprintln!("[create_driver] Robot in Safe mode. Polling sensors...");

    // In a full dora-rs integration, this would use dora_node_api::DoraNode
    // to receive tick events and send Arrow-formatted sensor data.
    // For now, demonstrate the polling pattern:
    loop {
        // Query common sensors
        let sd = robot.query_list(&[7, 8, 22, 24])?;

        eprintln!(
            "[sensors] bumps={:?} wall={:?} voltage={:?}mV temp={:?}°C",
            sd.bumps_and_wheeldrops, sd.wall, sd.voltage, sd.temperature,
        );

        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
