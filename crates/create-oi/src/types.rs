//! Domain types: robot models and validated newtypes for the control layer.
//!
//! Wire-level types (`OiMode`, `ChargingState`, `IrChar`, etc.) live in
//! [`create_oi_protocol::types`] and are re-exported from `crate::protocol::types`.

use std::time::Duration;

use crate::error::Error;

// Re-export wire-level types for convenience
pub use create_oi_protocol::types::{ChargingState, CleanMode, DayOfWeek, IrChar, OiMode};

// ---------------------------------------------------------------------------
// OI protocol constants (from the iRobot Create 2 Open Interface Spec)
// ---------------------------------------------------------------------------

/// Maximum linear velocity: ±500 mm/s (OI spec §5.5).
const OI_MAX_VELOCITY_MM_S: i16 = 500;

/// Maximum turn radius: ±2000 mm (OI spec §5.5).
const OI_MAX_RADIUS_MM: i16 = 2000;

/// OI special radius value: drive straight (0x7FFF, OI spec §5.5).
const OI_RADIUS_STRAIGHT_RAW: i16 = 0x7FFF;

/// OI special radius value: turn in place clockwise (-1, OI spec §5.5).
const OI_RADIUS_TURN_CW_RAW: i16 = -1;

/// OI special radius value: turn in place counter-clockwise (1, OI spec §5.5).
const OI_RADIUS_TURN_CCW_RAW: i16 = 1;

/// Maximum PWM magnitude for motor power: 255 (OI spec §5.8).
const OI_MAX_PWM: i16 = 255;

/// Maximum song slot index (0–3, OI spec §5.13).
const OI_MAX_SONG_NUMBER: u8 = 3;

/// Conversion factor: meters → millimeters.
const M_TO_MM: f32 = 1000.0;

// ---------------------------------------------------------------------------
// Robot physical constants (from iRobot documentation)
// ---------------------------------------------------------------------------

/// Baud rate for Create 1 / Roomba 400/500 series (protocol V1/V2).
const BAUD_RATE_V1_V2: u32 = 57_600;

/// Baud rate for Create 2 / Roomba 600+ series (protocol V3).
const BAUD_RATE_V3: u32 = 115_200;

/// Axle length (wheel-to-wheel) for Roomba 400 / Create 1, in meters.
const AXLE_LENGTH_CREATE1_M: f32 = 0.258;

/// Axle length (wheel-to-wheel) for Create 2, in meters.
const AXLE_LENGTH_CREATE2_M: f32 = 0.235;

/// Maximum forward velocity in m/s (same across all models).
const MAX_VELOCITY_M_S: f32 = 0.5;

/// Wheel diameter in meters (same across all models).
const WHEEL_DIAMETER_M: f32 = 0.078;

/// Encoder ticks per wheel revolution for Create 2.
const TICKS_PER_REV_CREATE2: f32 = 508.8;

/// Recommended delay after mode-change commands, in milliseconds.
const MODE_CHANGE_DELAY_MS: u64 = 20;

// ---------------------------------------------------------------------------
// Robot model
// ---------------------------------------------------------------------------

/// Physical robot model, determining protocol version and physical parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateRobotModel {
    /// Roomba 400 series and earlier (protocol V1).
    Roomba400,
    /// iRobot Create 1 / Roomba 500 series (protocol V2).
    Create1,
    /// iRobot Create 2 / Roomba 600+ series (protocol V3).
    Create2,
}

impl CreateRobotModel {
    /// Default baud rate for this model.
    pub fn baud(self) -> u32 {
        match self {
            Self::Roomba400 | Self::Create1 => BAUD_RATE_V1_V2,
            Self::Create2 => BAUD_RATE_V3,
        }
    }

    /// Axle length in meters (distance between wheels).
    pub fn axle_length(self) -> f32 {
        match self {
            Self::Roomba400 | Self::Create1 => AXLE_LENGTH_CREATE1_M,
            Self::Create2 => AXLE_LENGTH_CREATE2_M,
        }
    }

    /// Maximum forward velocity in m/s.
    pub fn max_velocity(self) -> f32 {
        MAX_VELOCITY_M_S
    }

    /// Wheel diameter in meters.
    pub fn wheel_diameter(self) -> f32 {
        WHEEL_DIAMETER_M
    }

    /// Encoder ticks per revolution (Create 2 / V3 only).
    pub fn ticks_per_rev(self) -> Option<f32> {
        match self {
            Self::Create2 => Some(TICKS_PER_REV_CREATE2),
            _ => None,
        }
    }

    /// Whether this model supports the sensor stream protocol.
    pub fn supports_stream(self) -> bool {
        matches!(self, Self::Create1 | Self::Create2)
    }

    /// Recommended delay after sending a mode-change command.
    pub fn mode_change_delay(self) -> Duration {
        Duration::from_millis(MODE_CHANGE_DELAY_MS)
    }
}

// ---------------------------------------------------------------------------
// Validated newtypes
// ---------------------------------------------------------------------------

/// Linear velocity in m/s. Valid range: [`-MAX_VELOCITY_M_S`, `MAX_VELOCITY_M_S`].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Velocity(f32);

impl Velocity {
    /// Maximum forward velocity (0.5 m/s per OI spec).
    pub const MAX: f32 = MAX_VELOCITY_M_S;
    /// Maximum reverse velocity (-0.5 m/s).
    pub const MIN: f32 = -MAX_VELOCITY_M_S;
    /// Zero velocity.
    pub const ZERO: Self = Self(0.0);
    /// Maximum raw OI velocity in mm/s (for reference).
    pub const MAX_MM_S: i16 = OI_MAX_VELOCITY_MM_S;

    pub fn new(value: f32) -> Result<Self, Error> {
        validate_finite("Velocity", value)?;
        validate_range("Velocity", value, Self::MIN, Self::MAX)?;
        Ok(Self(value))
    }

    pub fn get(self) -> f32 {
        self.0
    }

    /// Convert to mm/s as i16 for the OI protocol (rounds to nearest).
    pub fn to_mm_per_sec(self) -> i16 {
        (self.0 * M_TO_MM).round() as i16
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

/// Turn radius for the OI `drive` command.
///
/// The OI protocol uses special i16 values for straight and in-place turns,
/// so this type is modeled as an enum rather than a simple newtype.
///
/// # Protocol details (OI spec §5.5)
///
/// - `0x7FFF` (32767): drive straight
/// - `-1`: turn in place clockwise
/// - `1`: turn in place counter-clockwise
/// - `-2000..2000` mm: arc radius (positive = left, negative = right)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Radius {
    /// Drive in a straight line.
    Straight,
    /// Turn in place clockwise (spin right).
    TurnInPlaceCw,
    /// Turn in place counter-clockwise (spin left).
    TurnInPlaceCcw,
    /// Drive in an arc with the given radius in meters.
    /// Valid range: [-2.0, 2.0] m, excluding 0 (use in-place turns instead).
    /// Positive = turn left, negative = turn right.
    Curve(f32),
}

impl Radius {
    /// Legacy constant alias for `Radius::Straight`.
    pub const STRAIGHT: Self = Self::Straight;

    /// Maximum physical arc radius (2.0 m = 2000 mm).
    pub const MAX_CURVE_M: f32 = OI_MAX_RADIUS_MM as f32 / M_TO_MM;
    /// Minimum physical arc radius (-2.0 m = -2000 mm).
    pub const MIN_CURVE_M: f32 = -(OI_MAX_RADIUS_MM as f32) / M_TO_MM;

    /// Create a curve radius from a value in meters.
    ///
    /// Valid range: [-2.0, 2.0] m. Values of exactly `0.001` (1 mm) or
    /// `-0.001` (-1 mm) are rejected because they collide with OI special
    /// values; use [`Radius::TurnInPlaceCcw`] or [`Radius::TurnInPlaceCw`].
    pub fn new(value: f32) -> Result<Self, Error> {
        validate_finite("Radius", value)?;
        validate_range("Radius", value, Self::MIN_CURVE_M, Self::MAX_CURVE_M)?;
        let raw_mm = (value * M_TO_MM).round() as i16;
        // Reject raw values that collide with protocol specials
        if raw_mm == OI_RADIUS_STRAIGHT_RAW
            || raw_mm == OI_RADIUS_TURN_CW_RAW
            || raw_mm == OI_RADIUS_TURN_CCW_RAW
        {
            return Err(Error::InvalidValue {
                field: "Radius",
                reason: format!(
                    "{value} m maps to reserved OI value {raw_mm}; use Radius::Straight/TurnInPlaceCw/TurnInPlaceCcw"
                ),
            });
        }
        Ok(Self::Curve(value))
    }

    /// Get the physical radius in meters, if this is a curve.
    /// Returns `None` for special values (Straight, TurnInPlaceCw/Ccw).
    pub fn as_meters(self) -> Option<f32> {
        match self {
            Self::Curve(v) => Some(v),
            _ => None,
        }
    }

    /// Convert to the raw i16 millimeter value for the OI protocol.
    pub fn to_mm(self) -> i16 {
        match self {
            Self::Straight => OI_RADIUS_STRAIGHT_RAW,
            Self::TurnInPlaceCw => OI_RADIUS_TURN_CW_RAW,
            Self::TurnInPlaceCcw => OI_RADIUS_TURN_CCW_RAW,
            Self::Curve(m) => (m * M_TO_MM).round() as i16,
        }
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

    /// Convert to PWM value (-255..255) for the OI protocol (rounds to nearest).
    pub fn to_pwm(self) -> i16 {
        (self.0 * OI_MAX_PWM as f32).round() as i16
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

/// Song number (0..3, per OI spec §5.13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SongNumber(u8);

impl SongNumber {
    pub fn new(value: u8) -> Result<Self, Error> {
        if value > OI_MAX_SONG_NUMBER {
            return Err(Error::InvalidValue {
                field: "SongNumber",
                reason: format!("{value} > {OI_MAX_SONG_NUMBER}"),
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
    fn velocity_to_mm_per_sec_rounds() {
        let v = Velocity::new(0.5).unwrap();
        assert_eq!(v.to_mm_per_sec(), 500);
        let v = Velocity::new(-0.3).unwrap();
        assert_eq!(v.to_mm_per_sec(), -300);
        // Test rounding: 0.1234 m/s → 123.4 → rounds to 123
        let v = Velocity::new(0.1234).unwrap();
        assert_eq!(v.to_mm_per_sec(), 123);
        // 0.1235 → 123.5 → rounds to 124
        let v = Velocity::new(0.1235).unwrap();
        assert_eq!(v.to_mm_per_sec(), 124);
    }

    #[test]
    fn radius_straight_encodes_correctly() {
        assert_eq!(Radius::Straight.to_mm(), 0x7FFF);
        assert_eq!(Radius::STRAIGHT.to_mm(), 32767);
    }

    #[test]
    fn radius_turn_in_place_encodes_correctly() {
        assert_eq!(Radius::TurnInPlaceCw.to_mm(), -1);
        assert_eq!(Radius::TurnInPlaceCcw.to_mm(), 1);
    }

    #[test]
    fn radius_curve_valid() {
        let r = Radius::new(0.5).unwrap();
        assert_eq!(r.to_mm(), 500);
        assert_eq!(r.as_meters(), Some(0.5));

        let r = Radius::new(-2.0).unwrap();
        assert_eq!(r.to_mm(), -2000);

        let r = Radius::new(2.0).unwrap();
        assert_eq!(r.to_mm(), 2000);
    }

    #[test]
    fn radius_curve_out_of_range() {
        assert!(Radius::new(2.1).is_err());
        assert!(Radius::new(-2.1).is_err());
    }

    #[test]
    fn radius_special_values_not_constructible_via_new() {
        // 0.001 m = 1 mm = TurnInPlaceCcw special
        assert!(Radius::new(0.001).is_err());
        // -0.001 m = -1 mm = TurnInPlaceCw special
        assert!(Radius::new(-0.001).is_err());
    }

    #[test]
    fn radius_straight_has_no_meters() {
        assert_eq!(Radius::Straight.as_meters(), None);
        assert_eq!(Radius::TurnInPlaceCw.as_meters(), None);
    }

    #[test]
    fn motor_power_valid() {
        assert!(MotorPower::new(0.0).is_ok());
        assert!(MotorPower::new(1.0).is_ok());
        assert!(MotorPower::new(-1.0).is_ok());
    }

    #[test]
    fn motor_power_to_pwm_rounds() {
        let p = MotorPower::new(1.0).unwrap();
        assert_eq!(p.to_pwm(), 255);
        let p = MotorPower::new(-1.0).unwrap();
        assert_eq!(p.to_pwm(), -255);
        // 0.5 * 255 = 127.5 → rounds to 128
        let p = MotorPower::new(0.5).unwrap();
        assert_eq!(p.to_pwm(), 128);
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
        assert_eq!(CreateRobotModel::Create2.baud(), 115_200);
        assert_eq!(CreateRobotModel::Create1.baud(), 57_600);
    }
}
