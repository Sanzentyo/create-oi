//! TypeState mode markers and capability traits.
//!
//! The iRobot OI defines four modes. We encode them as zero-sized marker
//! types so the compiler prevents invalid operations at each mode.

use core::marker::PhantomData;

// ---------------------------------------------------------------------------
// Sealed trait
// ---------------------------------------------------------------------------

mod sealed {
    pub trait Sealed {}
}

// ---------------------------------------------------------------------------
// Capability-sealed traits (prevent external impl for wrong modes)
// ---------------------------------------------------------------------------

/// Private supertraits that gate capability implementations.
/// External code cannot name these traits, so cannot add new impls.
mod cap_sealed {
    pub trait SensorCapable {}
    pub trait ActuateCapable {}
    pub trait FullCapable {}
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
pub trait Mode: sealed::Sealed + core::fmt::Debug + Copy + Send + Sync + 'static {
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
///
/// This trait is sealed: only the pre-defined mode markers implement it.
pub trait SensorReadable: Mode + cap_sealed::SensorCapable {}

impl cap_sealed::SensorCapable for Passive {}
impl cap_sealed::SensorCapable for Safe {}
impl cap_sealed::SensorCapable for Full {}

impl SensorReadable for Passive {}
impl SensorReadable for Safe {}
impl SensorReadable for Full {}

/// Modes where actuator commands are available: Safe, Full.
///
/// This trait is sealed: only the pre-defined mode markers implement it.
pub trait Actuatable: SensorReadable + cap_sealed::ActuateCapable {}

impl cap_sealed::ActuateCapable for Safe {}
impl cap_sealed::ActuateCapable for Full {}

impl Actuatable for Safe {}
impl Actuatable for Full {}

/// Modes where Full-control commands are available: Full only.
///
/// Commands in this category can override all safety checks and affect
/// robot scheduling or simulated inputs. Only [`Full`] implements this trait.
///
/// This trait is sealed: only the pre-defined mode markers implement it.
pub trait FullControl: Actuatable + cap_sealed::FullCapable {}

impl cap_sealed::FullCapable for Full {}

impl FullControl for Full {}

// ---------------------------------------------------------------------------
// ModePhantom helper
// ---------------------------------------------------------------------------

/// Convenience alias for a zero-sized, `Send`+`Sync` mode phantom.
pub type ModePhantom<M> = PhantomData<M>;
