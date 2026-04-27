//! Async sensor streaming example using `TokioTransport`.
//!
//! Demonstrates the continuous sensor stream API with tokio:
//! - `start_stream` — subscribe to sensor packets
//! - `poll_stream_with` — receive and process one frame
//! - `toggle_stream` — pause and resume
//!
//! # Usage
//!
//! ```text
//! cargo run --example stream_tokio -- /dev/ttyUSB0
//! ```

use create_oi::prelude::*;
use create_oi_tokio::TokioTransport;

const TOTAL_FRAMES: u32 = 30;

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

    // Subscribe to sensor packets: BUMPS_AND_WHEEL_DROPS, VOLTAGE, OI_MODE
    println!("Starting sensor stream (BUMPS_AND_WHEEL_DROPS, VOLTAGE, OI_MODE)...");
    create
        .start_stream(&[
            PacketId::BUMPS_AND_WHEEL_DROPS,
            PacketId::VOLTAGE,
            PacketId::OI_MODE,
        ])
        .await?;

    let mut frames = 0u32;
    let mut paused = false;
    while frames < TOTAL_FRAMES {
        create
            .poll_stream_with(|result| match result {
                Ok(sd) => {
                    frames += 1;
                    println!(
                        "Frame {:2}: voltage={:?} mV  \
                         right_bump={:?}  left_bump={:?}  mode={:?}",
                        frames,
                        sd.voltage,
                        sd.is_right_bump(),
                        sd.is_left_bump(),
                        sd.oi_mode,
                    );
                }
                Err(e) => eprintln!("Parse error: {e}"),
            })
            .await?;

        // After the 15th frame, pause briefly then resume once.
        // Use a flag so we pause exactly once even if a single read delivers
        // multiple frames at once.
        if frames >= 15 && !paused {
            paused = true;
            println!("--- Pausing stream ---");
            create.toggle_stream(false).await?;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            println!("--- Resuming stream ---");
            create.toggle_stream(true).await?;
        }
    }

    create.toggle_stream(false).await?;
    create.stop().await?;
    let _create = create.to_passive().await.map_err(|e| e.source)?;

    println!("Stream complete ({frames} frames).");
    Ok(())
}
