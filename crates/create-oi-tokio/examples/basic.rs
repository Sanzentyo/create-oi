//! Basic async example using `TokioTransport`.
//!
//! Connects to a robot asynchronously, enters Safe mode, queries sensors,
//! and drives briefly.

use create_oi::prelude::*;
use create_oi_tokio::TokioTransport;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/ttyUSB0".into());

    println!("Opening {port} (async)...");
    let transport = TokioTransport::open(&port, RobotModel::Create2)?;

    let create = AsyncCreate::new(transport, RobotModel::Create2);
    let create = create.start().await.map_err(|e| e.source)?;

    let mut create = create.to_safe().await.map_err(|e| e.source)?;

    // Query battery
    let sd = create.query_list(&[PacketId::VOLTAGE]).await?;
    println!("Battery voltage: {:?} mV", sd.voltage);

    // Drive forward
    create.drive(Velocity::new(0.2)?, Radius::STRAIGHT).await?;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Stop
    create.stop().await?;

    // Return to passive
    let _create = create.to_passive().await.map_err(|e| e.source)?;

    println!("Done!");
    Ok(())
}
