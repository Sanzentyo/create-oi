//! TypeState mode markers and capability traits.
//!
//! The iRobot OI defines four modes. We encode them as zero-sized marker
//! types so the compiler prevents invalid operations at each mode.

use std::marker::PhantomData;

// ---------------------------------------------------------------------------
// Sealed trait
// ---------------------------------------------------------------------------

mod sealed {
    pub trait Sealed {}
}

// ---------------------------------------------------------------------------
// Mode markers
// ---------------------------------------------------------------------------

/// Zero-sized marker: robot is not connected.
#[derive(Debug, Clone, Copy, Default)]
pub struct Off;

/// Zero-sized marker: robot is in Passive mode (sensors only).
#[derive(Debug, Clone, Copy, Default)]
pub struct Passive;

/// Zero-sized marker: robot is in Safe mode (sensors + actuators, with safety).
#[derive(Debug, Clone, Copy, Default)]
pub struct Safe;

/// Zero-sized marker: robot is in Full mode (sensors + actuators, no safety).
#[derive(Debug, Clone, Copy, Default)]
pub struct Full;

// ---------------------------------------------------------------------------
// Mode trait
// ---------------------------------------------------------------------------

/// Trait implemented by all OI mode markers.
///
/// This is sealed — external code cannot implement additional modes.
pub trait Mode: sealed::Sealed + std::fmt::Debug + Copy + Send + Sync + 'static {
    /// Human-readable name of this mode.
    fn name() -> &'static str;
}

impl sealed::Sealed for Off {}
impl sealed::Sealed for Passive {}
impl sealed::Sealed for Safe {}
impl sealed::Sealed for Full {}

impl Mode for Off {
    fn name() -> &'static str {
        "Off"
    }
}
impl Mode for Passive {
    fn name() -> &'static str {
        "Passive"
    }
}
impl Mode for Safe {
    fn name() -> &'static str {
        "Safe"
    }
}
impl Mode for Full {
    fn name() -> &'static str {
        "Full"
    }
}

// ---------------------------------------------------------------------------
// Capability traits (sealed)
// ---------------------------------------------------------------------------

/// Modes where sensor reading is available: Passive, Safe, Full.
pub trait SensorReadable: Mode {}

impl SensorReadable for Passive {}
impl SensorReadable for Safe {}
impl SensorReadable for Full {}

/// Modes where actuator commands are available: Safe, Full.
pub trait Actuatable: SensorReadable {}

impl Actuatable for Safe {}
impl Actuatable for Full {}

// ---------------------------------------------------------------------------
// ModePhantom helper
// ---------------------------------------------------------------------------

/// Convenience alias for a zero-sized, `Send`+`Sync` mode phantom.
pub type ModePhantom<M> = PhantomData<M>;
