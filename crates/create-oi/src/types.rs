//! Domain types: robot models and validated newtypes for the control layer.
//!
//! Wire-level types (`OiMode`, `ChargingState`, `IrChar`, etc.) live in
//! [`create_oi_protocol::types`] and are re-exported from `crate::protocol::types`.

use std::time::Duration;

use crate::error::Error;

// Re-export wire-level types for convenience
pub use create_oi_protocol::types::{ChargingState, CleanMode, DayOfWeek, IrChar, OiMode};

// ---------------------------------------------------------------------------
// Robot model
// ---------------------------------------------------------------------------

/// Physical robot model, determining protocol version and physical parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotModel {
    /// Roomba 400 series and earlier (protocol V1).
    Roomba400,
    /// iRobot Create 1 / Roomba 500 series (protocol V2).
    Create1,
    /// iRobot Create 2 / Roomba 600+ series (protocol V3).
    Create2,
}

impl RobotModel {
    /// Default baud rate for this model.
    pub fn baud(self) -> u32 {
        match self {
            Self::Roomba400 | Self::Create1 => 57600,
            Self::Create2 => 115200,
        }
    }

    /// Axle length in meters (distance between wheels).
    pub fn axle_length(self) -> f32 {
        match self {
            Self::Roomba400 | Self::Create1 => 0.258,
            Self::Create2 => 0.235,
        }
    }

    /// Maximum forward velocity in m/s.
    pub fn max_velocity(self) -> f32 {
        0.5
    }

    /// Wheel diameter in meters.
    pub fn wheel_diameter(self) -> f32 {
        0.078
    }

    /// Encoder ticks per revolution (Create 2 / V3 only).
    pub fn ticks_per_rev(self) -> Option<f32> {
        match self {
            Self::Create2 => Some(508.8),
            _ => None,
        }
    }

    /// Whether this model supports the sensor stream protocol.
    pub fn supports_stream(self) -> bool {
        matches!(self, Self::Create1 | Self::Create2)
    }

    /// Recommended delay after sending a mode-change command.
    pub fn mode_change_delay(self) -> Duration {
        Duration::from_millis(20)
    }
}

// ---------------------------------------------------------------------------
// Validated newtypes
// ---------------------------------------------------------------------------

/// Linear velocity in m/s. Valid range: [-0.5, 0.5].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Velocity(f32);

impl Velocity {
    pub const MAX: f32 = 0.5;
    pub const MIN: f32 = -0.5;
    pub const ZERO: Self = Self(0.0);

    pub fn new(value: f32) -> Result<Self, Error> {
        validate_finite("Velocity", value)?;
        validate_range("Velocity", value, Self::MIN, Self::MAX)?;
        Ok(Self(value))
    }

    pub fn get(self) -> f32 {
        self.0
    }

    /// Convert to mm/s as i16 for the OI protocol.
    pub fn to_mm_per_sec(self) -> i16 {
        (self.0 * 1000.0) as i16
    }
}

impl TryFrom<f32> for Velocity {
    type Error = Error;
    fn try_from(v: f32) -> Result<Self, Self::Error> {
        Self::new(v)
    }
}

/// Angular velocity in rad/s. Valid range: [-π, π].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct AngularVelocity(f32);

impl AngularVelocity {
    pub const MAX: f32 = std::f32::consts::PI;
    pub const MIN: f32 = -std::f32::consts::PI;

    pub fn new(value: f32) -> Result<Self, Error> {
        validate_finite("AngularVelocity", value)?;
        validate_range("AngularVelocity", value, Self::MIN, Self::MAX)?;
        Ok(Self(value))
    }

    pub fn get(self) -> f32 {
        self.0
    }
}

/// Turning radius in meters. Valid range: [-2.0, 2.0].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Radius(f32);

impl Radius {
    pub const MAX: f32 = 2.0;
    pub const MIN: f32 = -2.0;
    pub const STRAIGHT: Self = Self(32.768);

    pub fn new(value: f32) -> Result<Self, Error> {
        validate_finite("Radius", value)?;
        validate_range("Radius", value, Self::MIN, Self::MAX)?;
        Ok(Self(value))
    }

    pub fn get(self) -> f32 {
        self.0
    }

    /// Convert to mm as i16 for the OI protocol.
    pub fn to_mm(self) -> i16 {
        (self.0 * 1000.0) as i16
    }
}

/// Motor power level. Valid range: [-1.0, 1.0].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MotorPower(f32);

impl MotorPower {
    pub const MAX: f32 = 1.0;
    pub const MIN: f32 = -1.0;
    pub const OFF: Self = Self(0.0);

    pub fn new(value: f32) -> Result<Self, Error> {
        validate_finite("MotorPower", value)?;
        validate_range("MotorPower", value, Self::MIN, Self::MAX)?;
        Ok(Self(value))
    }

    pub fn get(self) -> f32 {
        self.0
    }

    /// Convert to PWM value (-255..255) for the OI protocol.
    pub fn to_pwm(self) -> i16 {
        (self.0 * 255.0) as i16
    }
}

/// Power LED color (0 = green, 255 = red).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerLedColor(u8);

impl PowerLedColor {
    pub const GREEN: Self = Self(0);
    pub const RED: Self = Self(255);

    pub fn new(value: u8) -> Self {
        Self(value)
    }

    pub fn get(self) -> u8 {
        self.0
    }
}

impl From<u8> for PowerLedColor {
    fn from(v: u8) -> Self {
        Self(v)
    }
}

/// LED intensity (0 = off, 255 = full brightness).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedIntensity(u8);

impl LedIntensity {
    pub const OFF: Self = Self(0);
    pub const FULL: Self = Self(255);

    pub fn new(value: u8) -> Self {
        Self(value)
    }

    pub fn get(self) -> u8 {
        self.0
    }
}

impl From<u8> for LedIntensity {
    fn from(v: u8) -> Self {
        Self(v)
    }
}

/// Song number (0..3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SongNumber(u8);

impl SongNumber {
    pub fn new(value: u8) -> Result<Self, Error> {
        if value > 3 {
            return Err(Error::InvalidValue {
                field: "SongNumber",
                reason: format!("{value} > 3"),
            });
        }
        Ok(Self(value))
    }

    pub fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for SongNumber {
    type Error = Error;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        Self::new(v)
    }
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_finite(field: &'static str, value: f32) -> Result<(), Error> {
    if !value.is_finite() {
        return Err(Error::InvalidValue {
            field,
            reason: format!("must be finite, got {value}"),
        });
    }
    Ok(())
}

fn validate_range(field: &'static str, value: f32, min: f32, max: f32) -> Result<(), Error> {
    if value < min || value > max {
        return Err(Error::InvalidValue {
            field,
            reason: format!("{value} not in [{min}, {max}]"),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn velocity_valid() {
        assert!(Velocity::new(0.0).is_ok());
        assert!(Velocity::new(0.5).is_ok());
        assert!(Velocity::new(-0.5).is_ok());
    }

    #[test]
    fn velocity_out_of_range() {
        assert!(Velocity::new(0.6).is_err());
        assert!(Velocity::new(-0.6).is_err());
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
    fn velocity_to_mm_per_sec() {
        let v = Velocity::new(0.5).unwrap();
        assert_eq!(v.to_mm_per_sec(), 500);
        let v = Velocity::new(-0.3).unwrap();
        assert_eq!(v.to_mm_per_sec(), -300);
    }

    #[test]
    fn radius_valid() {
        assert!(Radius::new(0.0).is_ok());
        assert!(Radius::new(2.0).is_ok());
        assert!(Radius::new(-2.0).is_ok());
    }

    #[test]
    fn radius_out_of_range() {
        assert!(Radius::new(2.1).is_err());
    }

    #[test]
    fn motor_power_valid() {
        assert!(MotorPower::new(0.0).is_ok());
        assert!(MotorPower::new(1.0).is_ok());
        assert!(MotorPower::new(-1.0).is_ok());
    }

    #[test]
    fn motor_power_to_pwm() {
        let p = MotorPower::new(1.0).unwrap();
        assert_eq!(p.to_pwm(), 255);
        let p = MotorPower::new(-1.0).unwrap();
        assert_eq!(p.to_pwm(), -255);
    }

    #[test]
    fn song_number_valid() {
        assert!(SongNumber::new(0).is_ok());
        assert!(SongNumber::new(3).is_ok());
    }

    #[test]
    fn song_number_invalid() {
        assert!(SongNumber::new(4).is_err());
    }

    #[test]
    fn oi_mode_from_raw() {
        assert_eq!(OiMode::from_raw(0), OiMode::Off);
        assert_eq!(OiMode::from_raw(1), OiMode::Passive);
        assert_eq!(OiMode::from_raw(2), OiMode::Safe);
        assert_eq!(OiMode::from_raw(3), OiMode::Full);
        assert_eq!(OiMode::from_raw(99), OiMode::Unknown(99));
    }

    #[test]
    fn charging_state_from_raw() {
        assert_eq!(ChargingState::from_raw(0), ChargingState::NotCharging);
        assert_eq!(
            ChargingState::from_raw(5),
            ChargingState::ChargingFaultCondition
        );
        assert_eq!(ChargingState::from_raw(42), ChargingState::Unknown(42));
    }

    #[test]
    fn ir_char_from_raw() {
        assert_eq!(IrChar::from_raw(0), IrChar::None);
        assert_eq!(IrChar::from_raw(143), IrChar::SeekDock);
        assert_eq!(IrChar::from_raw(200), IrChar::Unknown(200));
    }

    #[test]
    fn robot_model_baud() {
        assert_eq!(RobotModel::Create2.baud(), 115200);
        assert_eq!(RobotModel::Create1.baud(), 57600);
    }
}
