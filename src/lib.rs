//! # libcreate — Safe Rust wrapper for the iRobot Create / Roomba
//!
//! This crate provides an idiomatic, type-safe Rust API for controlling
//! iRobot Create 1, Create 2, and compatible Roomba robots over a serial
//! connection.
//!
//! ## TypeState Pattern
//!
//! The robot's Open Interface (OI) modes are encoded at the type level:
//!
//! - [`Robot<Off>`](robot::Robot) — not connected
//! - [`Robot<Passive>`](robot::Robot) — connected, sensors only
//! - [`Robot<Safe>`](robot::Robot) — actuator control with safety limits
//! - [`Robot<Full>`](robot::Robot) — actuator control, no safety limits
//!
//! Mode transitions consume `self`, making it impossible to use commands
//! that are invalid for the current mode.
//!
//! ## Quick Start
//!
//! ```no_run
//! use libcreate::{Robot, RobotModel};
//!
//! // Create a handle and connect
//! let robot = Robot::new(RobotModel::Create2)?;
//! let robot = robot.connect("/dev/ttyUSB0", 115200)?;
//!
//! // Enter Safe mode for actuator control
//! let mut robot = robot.into_safe()?;
//!
//! // Read sensors
//! let snapshot = robot.sensors()?;
//! println!("Battery: {:.0}%", snapshot.battery.charge_ratio() * 100.0);
//!
//! // Drive forward
//! robot.drive(0.2.try_into()?, 0.0.try_into()?)?;
//!
//! // Stop and disconnect
//! robot.stop()?;
//! let robot = robot.into_passive()?;
//! let _robot = robot.disconnect();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod error;
pub mod mode;
pub mod robot;
pub mod sensor;
pub mod types;

// Re-export the most commonly used items at crate root.
pub use error::{Error, TransitionError};
pub use mode::{Actuatable, Full, Mode, Off, Passive, Safe, SensorReadable};
pub use robot::Robot;
pub use sensor::SensorSnapshot;
pub use types::{
    AngularVelocity, ChargingState, CleanMode, DayOfWeek, IrChar, LedIntensity, MotorPower, OiMode,
    PowerLedColor, Radius, RobotModel, SongNumber, Velocity,
};
