//! dora-rs dataflow node for iRobot Create / Roomba robots.
//!
//! This crate provides a ready-to-use [dora-rs](https://dora-rs.ai/) node
//! that wraps `create-oi` to expose sensor data as Arrow outputs and
//! accept motor commands as Arrow inputs.
//!
//! ## Dataflow integration
//!
//! ```yaml
//! nodes:
//!   - id: create_driver
//!     path: target/release/create-oi-dora
//!     inputs:
//!       tick: dora/timer/millis/50
//!       motor_cmd: controller/command
//!     outputs:
//!       - sensors
//!       - bumpers
//!       - battery
//! ```

pub mod node;
