//! Async music example using `TokioTransport`.
//!
//! Demonstrates:
//! - `define_song` — upload a melody to one of the robot's song slots
//! - `play_song` — play a previously defined song
//!
//! Songs are stored in slots 0–4 (Roomba 400/Create 1) or 0–15 (Create 2).
//! Each song can hold up to 16 notes. Notes use MIDI numbering: 60 = C4.
//! Duration is in units of 1/64 second (e.g. 16 = 0.25 s at 64 Hz).
//!
//! # Usage
//!
//! ```text
//! cargo run --example songs_tokio -- /dev/ttyUSB0
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

    // --- Song 0: C major scale (C4 D4 E4 F4 G4 A4 B4 C5) ---
    // MIDI: 60=C4, 62=D4, 64=E4, 65=F4, 67=G4, 69=A4, 71=B4, 72=C5
    // Duration 32 = 0.5 s per note
    let scale = [
        SongNote::new(60, 32)?, // C4
        SongNote::new(62, 32)?, // D4
        SongNote::new(64, 32)?, // E4
        SongNote::new(65, 32)?, // F4
        SongNote::new(67, 32)?, // G4
        SongNote::new(69, 32)?, // A4
        SongNote::new(71, 32)?, // B4
        SongNote::new(72, 32)?, // C5
    ];
    println!("Defining song 0: C major scale...");
    create.define_song(SongNumber::new(0)?, &scale).await?;

    println!("Playing song 0...");
    create.play_song(SongNumber::new(0)?).await?;
    // Wait for scale to finish (8 notes × 0.5 s = 4 s)
    tokio::time::sleep(Duration::from_secs(5)).await;

    // --- Song 1: Simple two-note fanfare ---
    let fanfare = [
        SongNote::new(72, 16)?, // C5 (0.25 s)
        SongNote::new(79, 64)?, // G5 (1.0 s)
    ];
    println!("Defining song 1: fanfare...");
    create.define_song(SongNumber::new(1)?, &fanfare).await?;

    println!("Playing song 1...");
    create.play_song(SongNumber::new(1)?).await?;
    tokio::time::sleep(Duration::from_millis(1500)).await;

    let _create = create.to_passive().await.map_err(|e| e.source)?;
    println!("Done!");
    Ok(())
}
