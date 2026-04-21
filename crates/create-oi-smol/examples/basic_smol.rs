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
        let transport = SmolTransport::open(&port, RobotModel::Create2)?;

        let create = AsyncCreate::new(transport, RobotModel::Create2);
        let create = create.start().await.map_err(|e| e.source)?;

        let mut create = create.to_safe().await.map_err(|e| e.source)?;

        // Query battery voltage (packet 22)
        let sd = create.query_list(&[22]).await?;
        println!("Battery voltage: {:?} mV", sd.voltage);

        // Drive forward at 200 mm/s straight
        create.drive(Velocity::new(0.2)?, Radius::STRAIGHT).await?;
        smol::Timer::after(std::time::Duration::from_secs(2)).await;

        // Stop motors
        create.stop().await?;

        // Return to passive mode
        let _create = create.to_passive().await.map_err(|e| e.source)?;

        println!("Done!");
        Ok(())
    })
}
