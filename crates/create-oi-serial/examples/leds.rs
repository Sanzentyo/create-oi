//! LED control example using `SerialTransport`.
//!
//! Demonstrates all LED-related commands:
//! - `set_leds` — main status LEDs (debris, spot, dock, check robot) + power LED color/intensity
//! - `set_scheduling_leds` — day-of-week and schedule icon LEDs (Create 2 only)
//! - `set_digit_leds` — 4-digit ASCII display (Create 2 only)
//! - `set_digit_leds_raw` — 4-digit raw segment-bit display (Create 2 only)
//!
//! # Usage
//!
//! ```text
//! cargo run --example leds_sync -- /dev/ttyUSB0
//! ```

use std::thread::sleep;
use std::time::Duration;

use create_oi::prelude::*;
use create_oi_serial::SerialTransport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/ttyUSB0".into());

    println!("Opening {port}...");
    let transport = SerialTransport::open(&port, RobotModel::Create2)?;

    let create = Create::new(transport, RobotModel::Create2);
    let create = create.start().map_err(|e| e.source)?;
    let mut create = create.to_safe().map_err(|e| e.source)?;

    // --- set_leds: main status LEDs ---
    // Args: debris, spot, dock, check_robot, power_color, power_intensity
    println!("All LEDs on, power LED red at full brightness...");
    create.set_leds(
        true,
        true,
        true,
        true,
        PowerLedColor::RED,
        LedIntensity::new(255),
    )?;
    sleep(Duration::from_secs(2));

    println!("Debris + dock only, power LED green at half brightness...");
    create.set_leds(
        true,
        false,
        true,
        false,
        PowerLedColor::GREEN,
        LedIntensity::new(128),
    )?;
    sleep(Duration::from_secs(2));

    // Power LED sweep: green → amber → red
    println!("Power LED colour sweep (green → red)...");
    for level in (0u8..=255).step_by(8) {
        create.set_leds(
            false,
            false,
            false,
            false,
            PowerLedColor::new(level),
            LedIntensity::new(200),
        )?;
        sleep(Duration::from_millis(40));
    }

    // All status LEDs off
    create.set_leds(
        false,
        false,
        false,
        false,
        PowerLedColor::GREEN,
        LedIntensity::new(0),
    )?;
    sleep(Duration::from_millis(500));

    // --- set_scheduling_leds (Create 2 only) ---
    // day_leds: bits 0-6 = Sun-Sat; schedule_leds: bit0=colon, bit1=AM/PM, bit2=clock, bit3=schedule
    println!("Scheduling LEDs: Monday + Wednesday + colon + AM/PM...");
    create.set_scheduling_leds(
        0b000_1010,  // Monday (bit 1) + Wednesday (bit 3)
        0b0000_0011, // colon (bit 0) + AM/PM (bit 1)
    )?;
    sleep(Duration::from_secs(2));
    create.set_scheduling_leds(0, 0)?; // clear

    // --- set_digit_leds: ASCII characters (Create 2 only) ---
    println!("Digit LEDs: ASCII 'OI  '...");
    create.set_digit_leds(b'O', b'I', b' ', b' ')?;
    sleep(Duration::from_secs(2));

    println!("Digit LEDs: '1234'...");
    create.set_digit_leds(b'1', b'2', b'3', b'4')?;
    sleep(Duration::from_secs(2));

    // --- set_digit_leds_raw: direct segment bits (Create 2 only) ---
    // Each byte encodes 7 segments: bits 0-6 = A B C D E F G
    println!("Digit LEDs raw: all segments on (0x7F)...");
    create.set_digit_leds_raw(0x7F, 0x7F, 0x7F, 0x7F)?;
    sleep(Duration::from_secs(2));

    // Clear digit display
    create.set_digit_leds_raw(0, 0, 0, 0)?;

    println!("Restoring defaults and returning to passive...");
    create.set_leds(
        false,
        false,
        false,
        false,
        PowerLedColor::GREEN,
        LedIntensity::new(0),
    )?;
    let _create = create.to_passive().map_err(|e| e.source)?;

    println!("Done!");
    Ok(())
}
