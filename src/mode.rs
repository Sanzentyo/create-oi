//! TypeState marker types for the iRobot Open Interface modes.
//!
//! The OI protocol defines four modes:
//! - **Off**: No serial communication active.
//! - **Passive**: Connected; sensor data available, no actuator control.
//! - **Safe**: Full actuator control; safety features (cliff, bump) active.
//! - **Full**: Full actuator control; all safety features disabled.

use crate::types::OiMode;

mod sealed {
    pub trait Sealed {}
}

/// Trait implemented by all OI mode marker types.
///
/// Sealed so that only the four canonical modes can exist.
pub trait Mode: sealed::Sealed + std::fmt::Debug {
    /// The raw FFI constant for this mode.
    const RAW: i32;
    /// Human-readable mode name.
    const NAME: &'static str;
    /// The corresponding runtime [`OiMode`] variant.
    const OI_MODE: OiMode;
}

/// Not connected / power off.
#[derive(Debug, Clone, Copy)]
pub struct Off;

/// Connected in passive mode (sensors only).
#[derive(Debug, Clone, Copy)]
pub struct Passive;

/// Safe mode — actuators enabled with safety limits.
#[derive(Debug, Clone, Copy)]
pub struct Safe;

/// Full mode — actuators enabled, all safety features disabled.
#[derive(Debug, Clone, Copy)]
pub struct Full;

// Seal all mode types.
impl sealed::Sealed for Off {}
impl sealed::Sealed for Passive {}
impl sealed::Sealed for Safe {}
impl sealed::Sealed for Full {}

impl Mode for Off {
    const RAW: i32 = libcreate_sys::CREATE_MODE_OFF;
    const NAME: &'static str = "Off";
    const OI_MODE: OiMode = OiMode::Off;
}

impl Mode for Passive {
    const RAW: i32 = libcreate_sys::CREATE_MODE_PASSIVE;
    const NAME: &'static str = "Passive";
    const OI_MODE: OiMode = OiMode::Passive;
}

impl Mode for Safe {
    const RAW: i32 = libcreate_sys::CREATE_MODE_SAFE;
    const NAME: &'static str = "Safe";
    const OI_MODE: OiMode = OiMode::Safe;
}

impl Mode for Full {
    const RAW: i32 = libcreate_sys::CREATE_MODE_FULL;
    const NAME: &'static str = "Full";
    const OI_MODE: OiMode = OiMode::Full;
}

// ---------------------------------------------------------------------------
// Capability traits — bound what operations are available in which modes.
// ---------------------------------------------------------------------------

/// Modes where sensor data can be read (Passive, Safe, Full).
pub trait SensorReadable: Mode {}
impl SensorReadable for Passive {}
impl SensorReadable for Safe {}
impl SensorReadable for Full {}

/// Modes where actuator commands (drive, motors, LEDs) are available (Safe, Full).
pub trait Actuatable: Mode {}
impl Actuatable for Safe {}
impl Actuatable for Full {}
