//! Scheduling example using `SerialTransport` (Create 2 only).
//!
//! Demonstrates:
//! - `set_date` — set the robot's internal clock
//! - `set_schedule` — program weekly cleaning times
//!
//! Both commands are only supported on Create 2.
//!
//! # Usage
//!
//! ```text
//! cargo run --example schedule_sync -- /dev/ttyUSB0
//! ```

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

    // --- set_date: sync the robot's internal clock ---
    // Args: day-of-week, hour (0-23), minute (0-59)
    println!("Setting robot clock to Monday 09:30...");
    create.set_date(DayOfWeek::Monday, 9, 30)?;

    // --- set_schedule: program a weekly cleaning schedule ---
    // `days` bitmask: bit 0 = Sunday, bit 1 = Monday, ..., bit 6 = Saturday
    // `times`: array of 7 (hour, minute) pairs, one per day (Sunday first).
    //          Use (0, 0) to disable a day.
    //
    // Example: clean Monday at 09:30 and Thursday at 18:00.
    let days = 0b001_0010_u8; // Monday (bit 1) + Thursday (bit 4)
    let times: [(u8, u8); 7] = [
        (0, 0),  // Sunday    — disabled
        (9, 30), // Monday    — 09:30
        (0, 0),  // Tuesday   — disabled
        (0, 0),  // Wednesday — disabled
        (18, 0), // Thursday  — 18:00
        (0, 0),  // Friday    — disabled
        (0, 0),  // Saturday  — disabled
    ];
    println!("Setting schedule: Monday 09:30, Thursday 18:00...");
    create.set_schedule(days, times)?;

    // Clear the schedule (all days disabled)
    println!("Clearing schedule...");
    create.set_schedule(0, [(0, 0); 7])?;

    let _create = create.to_passive().map_err(|e| e.source)?;
    println!("Done!");
    Ok(())
}
