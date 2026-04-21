//! Async LED control example using `TokioTransport`.
//!
//! Demonstrates all LED commands asynchronously:
//! - `set_leds` — status LEDs + power LED
//! - `set_scheduling_leds` — day-of-week and schedule icons (Create 2 only)
//! - `set_digit_leds` — ASCII digit display (Create 2 only)
//! - `set_digit_leds_raw` — raw segment bits (Create 2 only)
//!
//! # Usage
//!
//! ```text
//! cargo run --example leds_tokio -- /dev/ttyUSB0
//! ```

use std::time::Duration;

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

    // All LEDs on, power LED red at full brightness
    println!("All LEDs on, power LED red...");
    create
        .set_leds(
            true,
            true,
            true,
            true,
            PowerLedColor::RED,
            LedIntensity::new(255),
        )
        .await?;
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Power LED colour sweep: green → amber → red
    println!("Power LED sweep...");
    for level in (0u8..=255).step_by(8) {
        create
            .set_leds(
                false,
                false,
                false,
                false,
                PowerLedColor::new(level),
                LedIntensity::new(200),
            )
            .await?;
        tokio::time::sleep(Duration::from_millis(40)).await;
    }

    create
        .set_leds(
            false,
            false,
            false,
            false,
            PowerLedColor::GREEN,
            LedIntensity::new(0),
        )
        .await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Scheduling LEDs: Monday + Wednesday
    println!("Scheduling LEDs...");
    create.set_scheduling_leds(0b000_1010, 0b0000_0011).await?;
    tokio::time::sleep(Duration::from_secs(2)).await;
    create.set_scheduling_leds(0, 0).await?;

    // Digit display: "OI  "
    println!("Digit LEDs: 'OI  '...");
    create.set_digit_leds(b'O', b'I', b' ', b' ').await?;
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Raw segment bits: all segments on
    println!("Digit LEDs raw: all segments...");
    create.set_digit_leds_raw(0x7F, 0x7F, 0x7F, 0x7F).await?;
    tokio::time::sleep(Duration::from_secs(2)).await;
    create.set_digit_leds_raw(0, 0, 0, 0).await?;

    create
        .set_leds(
            false,
            false,
            false,
            false,
            PowerLedColor::GREEN,
            LedIntensity::new(0),
        )
        .await?;

    let _create = create.to_passive().await.map_err(|e| e.source)?;
    println!("Done!");
    Ok(())
}
