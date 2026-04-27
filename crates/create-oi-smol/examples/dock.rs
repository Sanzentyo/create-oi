//! Async clean and dock example using `SmolTransport`.
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
//! cargo run --example dock_smol -- /dev/ttyUSB0
//! ```

use std::time::Duration;

use create_oi_smol::SmolTransport;
use create_oi_smol::create_oi::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/ttyUSB0".into());

    smol::block_on(async {
        println!("Opening {port} (smol)...");
        let transport = SmolTransport::open(&port, RobotModel::Create2)?;

        let create = AsyncCreate::new(transport, RobotModel::Create2);
        let create = create.start().await.map_err(|e| e.source)?;
        let create = create.to_safe().await.map_err(|e| e.source)?;

        // --- Spot clean for a few seconds, then reclaim control ---
        // `clean()` consumes the Safe handle and returns a Passive handle —
        // the robot drives autonomously in spot-clean mode.
        println!("Starting spot clean (5 seconds)...");
        let create = create.clean(CleanMode::Spot).await.map_err(|e| e.source)?;
        smol::Timer::after(Duration::from_secs(5)).await;

        // Transitioning back to Safe reclaims control and aborts any ongoing
        // autonomous operation. We then query sensors before heading to dock.
        let mut create = create.to_safe().await.map_err(|e| e.source)?;
        let sd = create.query_list(&[PacketId::VOLTAGE]).await?;
        println!("Battery voltage: {:?} mV", sd.voltage);

        create.stop().await?;

        // --- Seek dock: robot navigates back to the charging dock ---
        // seek_dock() also transitions to Passive and returns a Create<Passive, T>.
        println!("Sending robot to dock...");
        let _create = create.seek_dock().await.map_err(|e| e.source)?;

        // In a real application you would poll sensors to detect the charging
        // state (packet 21 = ChargingState) and wait for docking to complete.
        smol::Timer::after(Duration::from_secs(3)).await;

        println!("Done! (Robot is now seeking dock autonomously.)");
        Ok(())
    })
}
