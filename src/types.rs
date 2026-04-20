//! Domain types modeled as proper Rust ADTs.
//!
//! All enums use exhaustive variants matching the OI protocol, plus an
//! `Unknown` variant for forward-compatibility with unrecognized raw values.

use crate::error::Error;
use libcreate_sys as ffi;

// ---------------------------------------------------------------------------
// Robot model
// ---------------------------------------------------------------------------

/// iRobot model / hardware generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RobotModel {
    /// Roomba 400 series and earlier.
    Roomba400,
    /// iRobot Create 1 / Roomba 500 series.
    Create1,
    /// iRobot Create 2 / Roomba 600+ series.
    Create2,
}

impl RobotModel {
    pub(crate) fn to_raw(self) -> i32 {
        match self {
            Self::Roomba400 => ffi::CREATE_MODEL_ROOMBA_400,
            Self::Create1 => ffi::CREATE_MODEL_CREATE_1,
            Self::Create2 => ffi::CREATE_MODEL_CREATE_2,
        }
    }
}

// ---------------------------------------------------------------------------
// OI Mode (runtime-observed, not the typestate marker)
// ---------------------------------------------------------------------------

/// The Open Interface mode as reported by the hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OiMode {
    Off,
    Passive,
    Safe,
    Full,
    /// An unrecognized mode value from the hardware.
    Unknown(i32),
}

impl OiMode {
    pub(crate) fn from_raw(raw: i32) -> Self {
        match raw {
            ffi::CREATE_MODE_OFF => Self::Off,
            ffi::CREATE_MODE_PASSIVE => Self::Passive,
            ffi::CREATE_MODE_SAFE => Self::Safe,
            ffi::CREATE_MODE_FULL => Self::Full,
            other => Self::Unknown(other),
        }
    }

    /// Human-readable name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Passive => "Passive",
            Self::Safe => "Safe",
            Self::Full => "Full",
            Self::Unknown(_) => "Unknown",
        }
    }
}

// ---------------------------------------------------------------------------
// Charging state
// ---------------------------------------------------------------------------

/// Battery charging state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChargingState {
    NotCharging,
    Reconditioning,
    FullCharging,
    TrickleCharging,
    Waiting,
    Fault,
    /// An unrecognized charging state.
    Unknown(i32),
}

impl ChargingState {
    pub(crate) fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::NotCharging,
            1 => Self::Reconditioning,
            2 => Self::FullCharging,
            3 => Self::TrickleCharging,
            4 => Self::Waiting,
            5 => Self::Fault,
            other => Self::Unknown(other),
        }
    }
}

// ---------------------------------------------------------------------------
// Clean mode
// ---------------------------------------------------------------------------

/// Cleaning behavior to start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CleanMode {
    Default,
    Max,
    Spot,
}

impl CleanMode {
    pub(crate) fn to_raw(self) -> i32 {
        match self {
            Self::Default => ffi::CREATE_CLEAN_DEFAULT,
            Self::Max => ffi::CREATE_CLEAN_MAX,
            Self::Spot => ffi::CREATE_CLEAN_SPOT,
        }
    }
}

// ---------------------------------------------------------------------------
// Day of week
// ---------------------------------------------------------------------------

/// Day of the week for the robot's internal clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DayOfWeek {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl DayOfWeek {
    pub(crate) fn to_raw(self) -> i32 {
        match self {
            Self::Sunday => 0,
            Self::Monday => 1,
            Self::Tuesday => 2,
            Self::Wednesday => 3,
            Self::Thursday => 4,
            Self::Friday => 5,
            Self::Saturday => 6,
        }
    }
}

// ---------------------------------------------------------------------------
// IR character
// ---------------------------------------------------------------------------

/// Infrared character received by the robot's IR sensors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IrChar {
    None,
    Left,
    Forward,
    Right,
    Spot,
    Max,
    Small,
    Medium,
    LargeClean,
    Pause,
    Power,
    ArcLeft,
    ArcRight,
    Stop,
    Download,
    SeekDock,
    RedBuoy,
    GreenBuoy,
    ForceField,
    RedGreenBuoy,
    RedForceField,
    GreenForceField,
    RedGreenForceField,
    VirtualWall,
    /// An unrecognized IR character value.
    Unknown(u8),
}

impl IrChar {
    pub(crate) fn from_raw(raw: u8) -> Self {
        match raw {
            0 => Self::None,
            129 => Self::Left,
            130 => Self::Forward,
            131 => Self::Right,
            132 => Self::Spot,
            133 => Self::Max,
            134 => Self::Small,
            135 => Self::Medium,
            136 => Self::LargeClean,
            137 => Self::Pause,
            138 => Self::Power,
            139 => Self::ArcLeft,
            140 => Self::ArcRight,
            141 => Self::Stop,
            142 => Self::Download,
            143 => Self::SeekDock,
            248 => Self::RedBuoy,
            244 => Self::GreenBuoy,
            242 => Self::ForceField,
            252 => Self::RedGreenBuoy,
            250 => Self::RedForceField,
            246 => Self::GreenForceField,
            254 => Self::RedGreenForceField,
            162 => Self::VirtualWall,
            other => Self::Unknown(other),
        }
    }
}

// ---------------------------------------------------------------------------
// Newtypes with validated ranges
// ---------------------------------------------------------------------------

/// Helper: validate that a value is finite and within [min, max].
#[inline(always)]
fn validate_range(value: f32, min: f32, max: f32) -> Result<f32, Error> {
    if !value.is_finite() {
        return Err(Error::NotFinite(value));
    }
    if value < min || value > max {
        return Err(Error::OutOfRange { value, min, max });
    }
    Ok(value)
}

macro_rules! newtype_f32 {
    (
        $(#[$meta:meta])*
        $name:ident, $min:expr, $max:expr
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        pub struct $name(f32);

        impl $name {
            /// Minimum allowed value.
            pub const MIN: Self = Self($min);
            /// Maximum allowed value.
            pub const MAX: Self = Self($max);
            /// Zero.
            pub const ZERO: Self = Self(0.0);

            /// Create a new value, validating the range.
            pub fn new(value: f32) -> Result<Self, Error> {
                validate_range(value, $min, $max).map(Self)
            }

            /// Get the inner `f32` value.
            #[inline]
            pub fn get(self) -> f32 {
                self.0
            }
        }

        impl TryFrom<f32> for $name {
            type Error = Error;
            fn try_from(value: f32) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
    };
}

newtype_f32!(
    /// Linear velocity in m/s. Range: [-0.5, 0.5].
    Velocity, -0.5, 0.5
);

newtype_f32!(
    /// Angular velocity in rad/s. Range: [-4.25, 4.25] (approximate).
    AngularVelocity, -4.25, 4.25
);

newtype_f32!(
    /// Turning radius in meters. Range: [-2.0, 2.0].
    Radius, -2.0, 2.0
);

newtype_f32!(
    /// Motor power as a fraction. Range: [-1.0, 1.0].
    MotorPower, -1.0, 1.0
);

/// Power LED color (0 = green, 255 = red).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PowerLedColor(u8);

impl PowerLedColor {
    pub const GREEN: Self = Self(0);
    pub const RED: Self = Self(255);

    pub fn new(value: u8) -> Self {
        Self(value)
    }

    #[inline]
    pub fn get(self) -> u8 {
        self.0
    }
}

impl From<u8> for PowerLedColor {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

/// LED intensity (0 = off, 255 = full brightness).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LedIntensity(u8);

impl LedIntensity {
    pub const OFF: Self = Self(0);
    pub const FULL: Self = Self(255);

    pub fn new(value: u8) -> Self {
        Self(value)
    }

    #[inline]
    pub fn get(self) -> u8 {
        self.0
    }
}

impl From<u8> for LedIntensity {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

/// Song slot number (0–3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SongNumber(u8);

impl SongNumber {
    pub fn new(value: u8) -> Result<Self, Error> {
        if value > 3 {
            return Err(Error::OutOfRange {
                value: value as f32,
                min: 0.0,
                max: 3.0,
            });
        }
        Ok(Self(value))
    }

    #[inline]
    pub fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for SongNumber {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Velocity
    // -----------------------------------------------------------------------

    #[test]
    fn velocity_valid_range() {
        assert!(Velocity::new(0.0).is_ok());
        assert!(Velocity::new(0.5).is_ok());
        assert!(Velocity::new(-0.5).is_ok());
        assert!(Velocity::new(0.25).is_ok());
    }

    #[test]
    fn velocity_out_of_range() {
        assert!(Velocity::new(0.6).is_err());
        assert!(Velocity::new(-0.6).is_err());
        assert!(Velocity::new(100.0).is_err());
    }

    #[test]
    fn velocity_nan_rejected() {
        assert!(Velocity::new(f32::NAN).is_err());
    }

    #[test]
    fn velocity_infinity_rejected() {
        assert!(Velocity::new(f32::INFINITY).is_err());
        assert!(Velocity::new(f32::NEG_INFINITY).is_err());
    }

    #[test]
    fn velocity_try_from() {
        assert!(Velocity::try_from(0.3).is_ok());
        assert!(Velocity::try_from(1.0).is_err());
    }

    #[test]
    fn velocity_constants() {
        assert_eq!(Velocity::MAX.get(), 0.5);
        assert_eq!(Velocity::MIN.get(), -0.5);
        assert_eq!(Velocity::ZERO.get(), 0.0);
    }

    // -----------------------------------------------------------------------
    // AngularVelocity
    // -----------------------------------------------------------------------

    #[test]
    fn angular_velocity_valid_range() {
        assert!(AngularVelocity::new(0.0).is_ok());
        assert!(AngularVelocity::new(4.25).is_ok());
        assert!(AngularVelocity::new(-4.25).is_ok());
    }

    #[test]
    fn angular_velocity_out_of_range() {
        assert!(AngularVelocity::new(5.0).is_err());
    }

    // -----------------------------------------------------------------------
    // Radius
    // -----------------------------------------------------------------------

    #[test]
    fn radius_valid_range() {
        assert!(Radius::new(0.0).is_ok());
        assert!(Radius::new(2.0).is_ok());
        assert!(Radius::new(-2.0).is_ok());
    }

    #[test]
    fn radius_out_of_range() {
        assert!(Radius::new(3.0).is_err());
    }

    // -----------------------------------------------------------------------
    // MotorPower
    // -----------------------------------------------------------------------

    #[test]
    fn motor_power_valid_range() {
        assert!(MotorPower::new(0.0).is_ok());
        assert!(MotorPower::new(1.0).is_ok());
        assert!(MotorPower::new(-1.0).is_ok());
        assert!(MotorPower::new(0.5).is_ok());
    }

    #[test]
    fn motor_power_out_of_range() {
        assert!(MotorPower::new(1.1).is_err());
        assert!(MotorPower::new(-1.1).is_err());
    }

    #[test]
    fn motor_power_nan_rejected() {
        assert!(MotorPower::new(f32::NAN).is_err());
    }

    // -----------------------------------------------------------------------
    // SongNumber
    // -----------------------------------------------------------------------

    #[test]
    fn song_number_valid() {
        for i in 0..=3 {
            assert!(SongNumber::new(i).is_ok());
            assert_eq!(SongNumber::new(i).unwrap().get(), i);
        }
    }

    #[test]
    fn song_number_invalid() {
        assert!(SongNumber::new(4).is_err());
        assert!(SongNumber::new(255).is_err());
    }

    #[test]
    fn song_number_try_from() {
        assert!(SongNumber::try_from(2u8).is_ok());
        assert!(SongNumber::try_from(5u8).is_err());
    }

    // -----------------------------------------------------------------------
    // PowerLedColor & LedIntensity
    // -----------------------------------------------------------------------

    #[test]
    fn power_led_color_constants() {
        assert_eq!(PowerLedColor::GREEN.get(), 0);
        assert_eq!(PowerLedColor::RED.get(), 255);
    }

    #[test]
    fn led_intensity_constants() {
        assert_eq!(LedIntensity::OFF.get(), 0);
        assert_eq!(LedIntensity::FULL.get(), 255);
    }

    #[test]
    fn power_led_color_from_u8() {
        let color: PowerLedColor = 128u8.into();
        assert_eq!(color.get(), 128);
    }

    // -----------------------------------------------------------------------
    // OiMode
    // -----------------------------------------------------------------------

    #[test]
    fn oi_mode_from_raw_known() {
        assert_eq!(OiMode::from_raw(0), OiMode::Off);
        assert_eq!(OiMode::from_raw(1), OiMode::Passive);
        assert_eq!(OiMode::from_raw(2), OiMode::Safe);
        assert_eq!(OiMode::from_raw(3), OiMode::Full);
    }

    #[test]
    fn oi_mode_from_raw_unknown() {
        assert_eq!(OiMode::from_raw(99), OiMode::Unknown(99));
        assert_eq!(OiMode::from_raw(-1), OiMode::Unknown(-1));
    }

    #[test]
    fn oi_mode_name() {
        assert_eq!(OiMode::Off.name(), "Off");
        assert_eq!(OiMode::Passive.name(), "Passive");
        assert_eq!(OiMode::Safe.name(), "Safe");
        assert_eq!(OiMode::Full.name(), "Full");
        assert_eq!(OiMode::Unknown(42).name(), "Unknown");
    }

    // -----------------------------------------------------------------------
    // ChargingState
    // -----------------------------------------------------------------------

    #[test]
    fn charging_state_from_raw_known() {
        assert_eq!(ChargingState::from_raw(0), ChargingState::NotCharging);
        assert_eq!(ChargingState::from_raw(1), ChargingState::Reconditioning);
        assert_eq!(ChargingState::from_raw(2), ChargingState::FullCharging);
        assert_eq!(ChargingState::from_raw(3), ChargingState::TrickleCharging);
        assert_eq!(ChargingState::from_raw(4), ChargingState::Waiting);
        assert_eq!(ChargingState::from_raw(5), ChargingState::Fault);
    }

    #[test]
    fn charging_state_from_raw_unknown() {
        assert_eq!(ChargingState::from_raw(99), ChargingState::Unknown(99));
    }

    // -----------------------------------------------------------------------
    // IrChar
    // -----------------------------------------------------------------------

    #[test]
    fn ir_char_from_raw_known() {
        assert_eq!(IrChar::from_raw(0), IrChar::None);
        assert_eq!(IrChar::from_raw(129), IrChar::Left);
        assert_eq!(IrChar::from_raw(143), IrChar::SeekDock);
        assert_eq!(IrChar::from_raw(162), IrChar::VirtualWall);
    }

    #[test]
    fn ir_char_from_raw_unknown() {
        assert_eq!(IrChar::from_raw(1), IrChar::Unknown(1));
        assert_eq!(IrChar::from_raw(200), IrChar::Unknown(200));
    }

    // -----------------------------------------------------------------------
    // CleanMode
    // -----------------------------------------------------------------------

    #[test]
    fn clean_mode_to_raw() {
        assert_eq!(CleanMode::Default.to_raw(), 0);
        assert_eq!(CleanMode::Max.to_raw(), 1);
        assert_eq!(CleanMode::Spot.to_raw(), 2);
    }

    // -----------------------------------------------------------------------
    // DayOfWeek
    // -----------------------------------------------------------------------

    #[test]
    fn day_of_week_to_raw() {
        assert_eq!(DayOfWeek::Sunday.to_raw(), 0);
        assert_eq!(DayOfWeek::Saturday.to_raw(), 6);
    }

    // -----------------------------------------------------------------------
    // RobotModel
    // -----------------------------------------------------------------------

    #[test]
    fn robot_model_to_raw() {
        assert_eq!(RobotModel::Roomba400.to_raw(), 0);
        assert_eq!(RobotModel::Create1.to_raw(), 1);
        assert_eq!(RobotModel::Create2.to_raw(), 2);
    }
}
