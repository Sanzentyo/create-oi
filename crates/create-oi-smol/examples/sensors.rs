//! Async sensor query example using `SmolTransport`.
//!
//! Demonstrates the different ways to read sensor data:
//! - `query_sensor` — query a single sensor packet by ID
//! - `query_list` — query multiple sensor packets at once
//! - `read_oi_mode` — read the robot's current OI mode
//!
//! Common sensor packet IDs:
//! - 7  = Bumps and wheel drops
//! - 10 = Left bump signal
//! - 11 = Right bump signal
//! - 22 = Battery voltage (mV)
//! - 23 = Battery current (mA)
//! - 25 = Battery charge (mAh)
//! - 26 = Battery capacity (mAh)
//! - 35 = OI mode
//!
//! # Usage
//!
//! ```text
//! cargo run --example sensors_smol -- /dev/ttyUSB0
//! ```

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
        let mut create = create.to_safe().await.map_err(|e| e.source)?;

        // --- read_oi_mode: verify current mode ---
        let mode = create.read_oi_mode().await?;
        println!("Current OI mode: {mode:?}");

        // --- query_sensor: single packet ---
        let sd = create.query_sensor(22).await?; // voltage
        println!("Battery voltage (packet 22): {:?} mV", sd.voltage);

        let sd = create.query_sensor(7).await?; // bumps
        println!(
            "Bumps (packet 7): left={:?}  right={:?}  left_wheel_drop={:?}  right_wheel_drop={:?}",
            sd.is_left_bump(),
            sd.is_right_bump(),
            sd.is_left_wheeldrop(),
            sd.is_right_wheeldrop(),
        );

        // --- query_list: multiple packets in one round-trip ---
        let sd = create.query_list(&[22, 23, 25, 26, 35]).await?;
        println!("\n--- Battery status ---");
        println!("  Voltage:  {:?} mV", sd.voltage);
        println!("  Current:  {:?} mA", sd.current);
        println!("  Charge:   {:?} mAh", sd.battery_charge);
        println!("  Capacity: {:?} mAh", sd.battery_capacity);
        println!("  OI mode:  {:?}", sd.oi_mode);

        let _create = create.to_passive().await.map_err(|e| e.source)?;
        println!("\nDone!");
        Ok(())
    })
}
