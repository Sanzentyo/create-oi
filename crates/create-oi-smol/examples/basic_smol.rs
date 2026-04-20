//! Basic async example using `SmolTransport`.
//!
//! Connects to a robot asynchronously via the smol runtime, enters Safe mode,
//! queries sensors, drives briefly, then returns to Passive mode.
//!
//! # Usage
//!
//! ```text
//! cargo run --example basic_smol -- /dev/ttyUSB0
//! ```

use create_oi_smol::SmolTransport;
use create_oi_smol::create_oi::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/ttyUSB0".into());

    smol::block_on(async {
        println!("Opening {port} (smol async)...");
        let transport = SmolTransport::open(&port, CreateRobotModel::Create2)?;

        let robot = AsyncCreate::new(transport, CreateRobotModel::Create2);
        let robot = robot.start().await.map_err(|e| e.source)?;

        let mut robot = robot.to_safe().await.map_err(|e| e.source)?;

        // Query battery voltage (packet 22)
        let sd = robot.query_list(&[22]).await?;
        println!("Battery voltage: {:?} mV", sd.voltage);

        // Drive forward at 200 mm/s straight
        robot.drive(Velocity::new(0.2)?, Radius::STRAIGHT).await?;
        smol::Timer::after(std::time::Duration::from_secs(2)).await;

        // Stop motors
        robot.stop().await?;

        // Return to passive mode
        let _robot = robot.to_passive().await.map_err(|e| e.source)?;

        println!("Done!");
        Ok(())
    })
}
