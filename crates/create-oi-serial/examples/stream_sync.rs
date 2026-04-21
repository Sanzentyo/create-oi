//! Sensor streaming example using `SerialTransport`.
//!
//! Demonstrates the continuous sensor stream API:
//! - `start_stream` — subscribe to a set of sensor packets
//! - `poll_stream_with` — receive and process one frame (no-alloc callback style)
//! - `toggle_stream` — pause and resume the stream
//!
//! The robot sends a new frame of requested sensor data at 15 Hz once streaming
//! is started. This is more efficient than polling with `query_list` in a loop.
//!
//! # Usage
//!
//! ```text
//! cargo run --example stream_sync -- /dev/ttyUSB0
//! ```

use create_oi::prelude::*;
use create_oi_serial::SerialTransport;

const TOTAL_FRAMES: u32 = 30;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/ttyUSB0".into());

    println!("Opening {port}...");
    let transport = SerialTransport::open(&port, RobotModel::Create2)?;

    let create = Create::new(transport, RobotModel::Create2);
    let create = create.start().map_err(|e| e.source)?;
    let mut create = create.to_safe().map_err(|e| e.source)?;

    // Subscribe to three sensor packets:
    //   7  = Bumps and wheel drops
    //  22  = Battery voltage (mV)
    //  35  = OI mode
    println!("Starting sensor stream (packets 7, 22, 35)...");
    create.start_stream(&[7, 22, 35])?;

    let mut frames = 0u32;
    let mut paused = false;
    while frames < TOTAL_FRAMES {
        create.poll_stream_with(|result| match result {
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
        })?;

        // After the 15th frame pause the stream briefly, then resume.
        // Use a flag so we pause exactly once even if a single read yields
        // multiple frames pushing us past 15.
        if frames >= 15 && !paused {
            paused = true;
            println!("--- Pausing stream ---");
            create.toggle_stream(false)?;
            std::thread::sleep(std::time::Duration::from_millis(500));
            println!("--- Resuming stream ---");
            create.toggle_stream(true)?;
        }
    }

    // Cleanly stop the stream before leaving
    create.toggle_stream(false)?;
    create.stop()?;
    let _create = create.to_passive().map_err(|e| e.source)?;

    println!("Stream complete ({frames} frames).");
    Ok(())
}
