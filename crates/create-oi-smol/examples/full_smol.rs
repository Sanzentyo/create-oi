//! Async full-mode and advanced drive example using `SmolTransport`.
//!
//! Demonstrates features available in Full mode (no safety cutoffs):
//! - `to_full` — transition from Safe to Full mode
//! - `drive_direct` — independent per-wheel velocity (Create 1/2 only)
//! - `drive_twist` — cmd_vel–style linear + angular velocity (Create 1/2 only)
//! - `drive_pwm` — per-wheel PWM drive (Create 2 only)
//! - `set_motors` — enable/disable cleaning brushes
//! - `set_motors_pwm` — fine-grained brush motor PWM (Create 2 only)
//! - `simulate_buttons` — programmatically press robot buttons (Full mode only)
//!
//! **Note:** In Full mode the robot ignores cliff/wheel-drop safety cutoffs.
//! Always return to Safe mode before leaving the robot unattended.
//!
//! # Usage
//!
//! ```text
//! cargo run --example full_smol -- /dev/ttyUSB0
//! ```

use std::time::Duration;

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
        let create = create.to_safe().await.map_err(|e| e.source)?;

        // --- Transition to Full mode ---
        println!("Entering Full mode...");
        let mut create = create.to_full().await.map_err(|e| e.source)?;

        // --- drive_direct: per-wheel velocity (left, right independently) ---
        // Positive = forward, negative = reverse. Range: -0.5 m/s to +0.5 m/s.
        println!("drive_direct: gentle left arc (right faster than left)...");
        create
            .drive_direct(Velocity::new(0.15)?, Velocity::new(0.08)?)
            .await?;
        smol::Timer::after(Duration::from_secs(2)).await;
        create.stop().await?;
        smol::Timer::after(Duration::from_millis(300)).await;

        // Spin in place: right wheel forward, left wheel reverse
        println!("drive_direct: spin in place...");
        create
            .drive_direct(Velocity::new(0.1)?, Velocity::new(-0.1)?)
            .await?;
        smol::Timer::after(Duration::from_secs(1)).await;
        create.stop().await?;
        smol::Timer::after(Duration::from_millis(300)).await;

        // --- drive_twist: cmd_vel-style (linear velocity + angular velocity) ---
        // velocity: forward speed in m/s; omega: angular rate in rad/s (positive = left/CCW)
        println!("drive_twist: forward 0.2 m/s, curving left at 0.5 rad/s...");
        create
            .drive_twist(Velocity::new(0.2)?, AngularVelocity::new(0.5)?)
            .await?;
        smol::Timer::after(Duration::from_secs(2)).await;
        create.stop().await?;
        smol::Timer::after(Duration::from_millis(300)).await;

        // --- drive_pwm: direct PWM [-1.0, 1.0] (Create 2 only) ---
        println!("drive_pwm: right 60% forward, left 40% forward...");
        create
            .drive_pwm(MotorPower::new(0.6)?, MotorPower::new(0.4)?)
            .await?;
        smol::Timer::after(Duration::from_secs(2)).await;
        create.stop().await?;
        smol::Timer::after(Duration::from_millis(300)).await;

        // --- set_motors: enable cleaning brushes ---
        println!("set_motors: main brush + vacuum + side brush on...");
        create
            .set_motors(MotorBits {
                side_brush: true,
                vacuum: true,
                main_brush: true,
                side_brush_backward: false,
                main_brush_backward: false,
            })
            .await?;
        smol::Timer::after(Duration::from_secs(2)).await;

        // Reverse main brush direction
        println!("set_motors: main brush reversed...");
        create
            .set_motors(MotorBits {
                side_brush: false,
                vacuum: false,
                main_brush: true,
                side_brush_backward: false,
                main_brush_backward: true,
            })
            .await?;
        smol::Timer::after(Duration::from_secs(1)).await;

        // All motors off
        create.set_motors(MotorBits::default()).await?;
        smol::Timer::after(Duration::from_millis(300)).await;

        // --- set_motors_pwm: fine-grained brush PWM (Create 2 only) ---
        // Raw values: main_brush: i8 (-127..=127), side_brush: i8, vacuum: u8 (0..=127)
        println!("set_motors_pwm: brushes at ~50% power...");
        create.set_motors_pwm(64, 64, 64).await?;
        smol::Timer::after(Duration::from_secs(2)).await;
        create.set_motors_pwm(0, 0, 0).await?;

        // --- simulate_buttons (Full mode only) ---
        // Simulate pressing the Spot button programmatically
        println!("simulate_buttons: pressing Spot button...");
        create
            .simulate_buttons(ButtonBits {
                spot: true,
                ..ButtonBits::default()
            })
            .await?;
        smol::Timer::after(Duration::from_millis(200)).await;
        // Release (all buttons up)
        create.simulate_buttons(ButtonBits::default()).await?;

        // Return to Safe mode, then Passive
        println!("Returning to Safe, then Passive...");
        let create = create.to_safe().await.map_err(|e| e.source)?;
        let _create = create.to_passive().await.map_err(|e| e.source)?;

        println!("Done!");
        Ok(())
    })
}
