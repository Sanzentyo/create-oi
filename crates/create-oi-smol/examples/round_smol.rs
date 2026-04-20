//! Sensor-stream round-trip example using `SmolTransport`.
//!
//! Subscribes to the robot's sensor stream, prints bumper and battery readings
//! for a fixed number of frames, then stops cleanly.
//!
//! # Usage
//!
//! ```text
//! cargo run --example round_smol -- /dev/ttyUSB0
//! ```

use create_oi_smol::SmolTransport;
use create_oi_smol::create_oi::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/ttyUSB0".into());

    smol::block_on(async {
        println!("Opening {port} (smol stream)...");
        let transport = SmolTransport::open(&port, CreateRobotModel::Create2)?;

        let robot = AsyncCreate::new(transport, CreateRobotModel::Create2);
        let robot = robot.start().await.map_err(|e| e.source)?;
        let mut robot = robot.to_safe().await.map_err(|e| e.source)?;

        // Start sensor stream: bump/wheeldrop (7), voltage (22), OI mode (35)
        robot.start_stream(&[7, 22, 35]).await?;

        const MAX_FRAMES: u32 = 20;
        let mut frames = 0u32;

        while frames < MAX_FRAMES {
            robot
                .poll_stream_with(|result| match result {
                    Ok(sd) => {
                        frames += 1;
                        println!(
                            "Frame {frames:2}: voltage={:?} mV  right_bump={:?}  left_bump={:?}  mode={:?}",
                            sd.voltage,
                            sd.is_right_bump(),
                            sd.is_left_bump(),
                            sd.oi_mode,
                        );
                    }
                    Err(e) => eprintln!("Parse error: {e}"),
                })
                .await?;
        }

        // Pause stream, stop motors, return to passive
        robot.toggle_stream(false).await?;
        robot.stop().await?;
        let _robot = robot.to_passive().await.map_err(|e| e.source)?;

        println!("Stream complete ({frames} frames received).");
        Ok(())
    })
}
