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

    let robot = AsyncCreate::new(transport, RobotModel::Create2);
    let robot = robot.start().await.map_err(|e| e.source)?;

    let mut robot = robot.to_safe().await.map_err(|e| e.source)?;

    // Query battery
    let sd = robot.query_list(&[22]).await?;
    println!("Battery voltage: {:?} mV", sd.voltage);

    // Drive forward
    robot.drive(Velocity::new(0.2)?, Radius::STRAIGHT).await?;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Stop
    robot.stop().await?;

    // Return to passive
    let _robot = robot.to_passive().await.map_err(|e| e.source)?;

    println!("Done!");
    Ok(())
}
