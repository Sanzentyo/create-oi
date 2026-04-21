//! Domain types: robot models and validated newtypes for the control layer.
//!
//! Wire-level types (`OiMode`, `ChargingState`, `IrChar`, etc.) live in
//! [`create_oi_protocol::types`] and are re-exported from `crate::protocol::types`.

use core::time::Duration;

use crate::error::ValidationError;

// Re-export wire-level types for convenience
pub use create_oi_protocol::types::{ChargingState, CleanMode, DayOfWeek, IrChar, OiMode};

// ---------------------------------------------------------------------------
// OI protocol constants (from the iRobot Create 2 Open Interface Spec)
// ---------------------------------------------------------------------------

/// Maximum linear velocity: ±500 mm/s (OI spec §5.5).
const OI_MAX_VELOCITY_MM_S: i16 = 500;

/// Maximum turn radius: ±2000 mm (OI spec §5.5).
const OI_MAX_RADIUS_MM: i16 = 2000;

/// OI special radius value: drive straight (0x8000 = i16::MIN, OI spec §5.5).
///
/// The OI spec lists 32768 (bytes `[0x80, 0x00]`) as the primary sentinel for
/// "straight"; 32767 (`0x7FFF`) is listed as an accepted alias. Using `i16::MIN`
/// (-32768) produces the canonical wire encoding `[0x80, 0x00]`.
const OI_RADIUS_STRAIGHT_RAW: i16 = i16::MIN;

/// OI special radius value: turn in place clockwise (-1, OI spec §5.5).
const OI_RADIUS_TURN_CW_RAW: i16 = -1;

/// OI special radius value: turn in place counter-clockwise (1, OI spec §5.5).
const OI_RADIUS_TURN_CCW_RAW: i16 = 1;

/// Maximum PWM magnitude for motor power: 255 (OI spec §5.8).
const OI_MAX_PWM: i16 = 255;

/// Maximum song slot index for Create 2 (0–4, OI spec §5.13).
///
/// Create 2 supports 5 song slots (0–4). Create 1 / Roomba 400–500 supports
/// 16 slots (0–15). This constant represents the maximum across all models;
/// the control layer enforces the per-model limit via [`RobotModel::max_song_number`].
const OI_MAX_SONG_NUMBER: u8 = 15;

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
    #[inline(always)]
    pub const fn baud(self) -> u32 {
        match self {
            Self::Roomba400 | Self::Create1 => BAUD_RATE_V1_V2,
            Self::Create2 => BAUD_RATE_V3,
        }
    }

    /// Axle length in meters (distance between wheels).
    #[inline(always)]
    pub const fn axle_length(self) -> f32 {
        match self {
            Self::Roomba400 | Self::Create1 => AXLE_LENGTH_CREATE1_M,
            Self::Create2 => AXLE_LENGTH_CREATE2_M,
        }
    }

    /// Maximum forward velocity in m/s.
    #[inline(always)]
    pub const fn max_velocity(self) -> f32 {
        MAX_VELOCITY_M_S
    }

    /// Wheel diameter in meters.
    #[inline(always)]
    pub const fn wheel_diameter(self) -> f32 {
        WHEEL_DIAMETER_M
    }

    /// Encoder ticks per revolution (Create 2 / V3 only).
    #[inline(always)]
    pub const fn ticks_per_rev(self) -> Option<f32> {
        match self {
            Self::Create2 => Some(TICKS_PER_REV_CREATE2),
            _ => None,
        }
    }

    /// Whether this model supports the sensor stream protocol.
    #[allow(clippy::match_like_matches_macro)]
    #[inline(always)]
    pub const fn supports_stream(self) -> bool {
        match self {
            Self::Create1 | Self::Create2 => true,
            _ => false,
        }
    }

    /// Whether this model supports Create 2–specific opcodes.
    ///
    /// Returns `true` only for [`RobotModel::Create2`]. Several OI opcodes
    /// (DRIVE_PWM 146, MOTORS_PWM 144, DIGIT_LEDS_RAW 163, DIGIT_LEDS_ASCII 164,
    /// BUTTONS 165, SCHEDULE 167, SET_DAY_TIME 168) were introduced with the
    /// Create 2 / Roomba 600+ firmware and are not available on Create 1 or
    /// Roomba 400 series hardware.
    #[inline(always)]
    pub const fn is_create2(self) -> bool {
        matches!(self, Self::Create2)
    }

    /// Maximum song slot index (0..=max, inclusive) for this model.
    ///
    /// - Create 2: 5 slots (0–4)
    /// - Create 1 / Roomba 400: 16 slots (0–15)
    #[inline(always)]
    pub const fn max_song_number(self) -> u8 {
        match self {
            Self::Create2 => 4,
            Self::Roomba400 | Self::Create1 => 15,
        }
    }

    /// Whether this model supports the Drive Direct command (opcode 145).
    ///
    /// Drive Direct was introduced with OI V2 (Create 1 / Roomba 500 series).
    /// Roomba 400 only supports the Drive command (opcode 137).
    #[inline(always)]
    pub const fn supports_drive_direct(self) -> bool {
        !matches!(self, Self::Roomba400)
    }

    /// Whether this model supports individual sensor packet IDs (7+).
    ///
    /// Roomba 400 only supports group sensor requests (groups 0–3). Create 1
    /// and Create 2 both support individual packet IDs 7–42 and 7–58 respectively.
    #[inline(always)]
    pub const fn supports_individual_sensor_packets(self) -> bool {
        !matches!(self, Self::Roomba400)
    }

    /// Maximum individual sensor packet ID (inclusive) for this model.
    ///
    /// - Create 2: 58 (packets 7–58, OI V3)
    /// - Create 1: 42 (packets 7–42; packets 43–58 are Create 2-only)
    /// - Roomba 400: 0 (no individual packet IDs; use group queries 0–3 only)
    #[inline(always)]
    pub const fn max_individual_sensor_packet_id(self) -> u8 {
        match self {
            Self::Create2 => 58,
            Self::Create1 => 42,
            Self::Roomba400 => 0,
        }
    }

    /// Whether this model supports the Query List command (opcode 149).
    ///
    /// Query List was introduced in OI V2. Roomba 400 does not support it.
    #[inline(always)]
    pub const fn supports_query_list(self) -> bool {
        !matches!(self, Self::Roomba400)
    }

    /// Whether this model supports the given group sensor packet ID.
    ///
    /// - Groups 0–3: all models (Roomba 400, Create 1, Create 2)
    /// - Groups 4–6: Create 1 and Create 2 only
    /// - Group 100: Create 2 only
    #[inline(always)]
    pub const fn supports_group_packet(self, group: u8) -> bool {
        match group {
            0..=3 => true,
            4..=6 => !matches!(self, Self::Roomba400),
            100 => matches!(self, Self::Create2),
            _ => false,
        }
    }

    /// Recommended delay after sending a mode-change command.
    #[inline(always)]
    pub const fn mode_change_delay(self) -> Duration {
        Duration::from_millis(MODE_CHANGE_DELAY_MS)
    }
}

impl core::fmt::Display for RobotModel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Roomba400 => f.write_str("Roomba400"),
            Self::Create1 => f.write_str("Create1"),
            Self::Create2 => f.write_str("Create2"),
        }
    }
}

// ---------------------------------------------------------------------------
// LED bit-packing helper
// ---------------------------------------------------------------------------

/// Compute the LED-bits byte for the OI `LEDS` command (opcode 139).
///
/// The bit layout differs between Roomba 400 (SCI V1) and Create 1/2 (OI V2/V3):
///
/// | Parameter    | Create 1/2 bit | Roomba 400 bit |
/// |---|---|---|
/// | `debris`     | 0              | 3              |
/// | `spot`       | 1              | 6              |
/// | `dock`       | 2              | 5              |
/// | `check_robot`| 3              | 4              |
#[inline(always)]
pub(crate) const fn led_bits(
    model: RobotModel,
    debris: bool,
    spot: bool,
    dock: bool,
    check_robot: bool,
) -> u8 {
    if matches!(model, RobotModel::Roomba400) {
        // Roomba 400 SCI: dirt_detect=3, check_robot=4, dock=5, spot=6
        ((debris as u8) << 3)
            | ((check_robot as u8) << 4)
            | ((dock as u8) << 5)
            | ((spot as u8) << 6)
    } else {
        // Create 1/2 OI: debris=0, spot=1, dock=2, check_robot=3
        (debris as u8) | ((spot as u8) << 1) | ((dock as u8) << 2) | ((check_robot as u8) << 3)
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

    pub fn new(value: f32) -> Result<Self, ValidationError> {
        validate_finite("Velocity", value)?;
        validate_range("Velocity", value, Self::MIN, Self::MAX)?;
        Ok(Self(value))
    }

    #[inline(always)]
    pub const fn get(self) -> f32 {
        self.0
    }

    /// Convert to mm/s as i16 for the OI protocol (rounds to nearest).
    pub fn to_mm_per_sec(self) -> i16 {
        libm::roundf(self.0 * M_TO_MM) as i16
    }
}

impl TryFrom<f32> for Velocity {
    type Error = ValidationError;
    fn try_from(v: f32) -> Result<Self, Self::Error> {
        Self::new(v)
    }
}

impl core::fmt::Display for Velocity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.3} m/s", self.0)
    }
}

/// Angular velocity in rad/s.
///
/// The maximum achievable value is derived from the Create 2 geometry:
/// `ω_max = 2 × MAX_VELOCITY_M_S / AXLE_LENGTH_CREATE2_M ≈ 4.26 rad/s`.
/// `drive_twist()` clamps wheel speeds regardless, so values near this limit
/// are allowed even if the robot cannot physically achieve them precisely.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct AngularVelocity(f32);

impl AngularVelocity {
    /// Maximum angular velocity (rad/s): in-place spin at full wheel speed on Create 2.
    pub const MAX: f32 = 2.0 * MAX_VELOCITY_M_S / AXLE_LENGTH_CREATE2_M;
    pub const MIN: f32 = -(2.0 * MAX_VELOCITY_M_S / AXLE_LENGTH_CREATE2_M);

    pub fn new(value: f32) -> Result<Self, ValidationError> {
        validate_finite("AngularVelocity", value)?;
        validate_range("AngularVelocity", value, Self::MIN, Self::MAX)?;
        Ok(Self(value))
    }

    #[inline(always)]
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl core::fmt::Display for AngularVelocity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.3} rad/s", self.0)
    }
}

impl TryFrom<f32> for AngularVelocity {
    type Error = ValidationError;
    fn try_from(v: f32) -> Result<Self, Self::Error> {
        Self::new(v)
    }
}

/// Opaque arc radius value in meters.
///
/// This type can only be constructed via [`Radius::new`]; direct construction
/// is intentionally prevented to enforce OI protocol validation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveRadius(f32);

impl CurveRadius {
    /// Returns the radius value in meters.
    #[inline(always)]
    pub const fn as_meters(self) -> f32 {
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
/// - `i16::MIN` (bytes `[0x80, 0x00]`): drive straight (primary sentinel; `32767` is also accepted)
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
    /// Drive in an arc with the given radius.
    ///
    /// Use [`Radius::new`] to construct — direct `Radius::Curve(CurveRadius(...))`
    /// construction is not possible because [`CurveRadius`] has a private inner field.
    Curve(CurveRadius),
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
    /// Valid range: `[-2.0, 2.0]` m. Any value whose rounded millimeter
    /// encoding falls on a reserved OI special (`-1`, `0`, or `1` mm) is
    /// rejected. Use [`Radius::TurnInPlaceCcw`] / [`Radius::TurnInPlaceCw`]
    /// for in-place spins, and choose a magnitude of at least `0.002` m for
    /// genuine arc radii.
    pub fn new(value: f32) -> Result<Self, ValidationError> {
        validate_finite("Radius", value)?;
        validate_range("Radius", value, Self::MIN_CURVE_M, Self::MAX_CURVE_M)?;
        let raw_mm = libm::roundf(value * M_TO_MM) as i16;
        if raw_mm == 0 {
            return Err(ValidationError {
                field: "Radius",
                reason: "value rounds to 0 mm which is not a valid OI arc radius; choose a value whose absolute magnitude is at least 0.002 m",
            });
        }
        // Reject raw values that collide with turn-in-place specials (-1, +1).
        // The straight sentinel (i16::MIN = -32768) is already outside ±2000 mm
        // and is rejected by validate_range above.
        if raw_mm == OI_RADIUS_TURN_CW_RAW || raw_mm == OI_RADIUS_TURN_CCW_RAW {
            return Err(ValidationError {
                field: "Radius",
                reason: "value rounds to a reserved OI special encoding; use Radius::TurnInPlaceCw/TurnInPlaceCcw",
            });
        }
        Ok(Self::Curve(CurveRadius(value)))
    }

    /// Get the physical radius in meters, if this is a curve.
    /// Returns `None` for special values (Straight, TurnInPlaceCw/Ccw).
    #[inline(always)]
    pub const fn as_meters(self) -> Option<f32> {
        match self {
            Self::Curve(r) => Some(r.as_meters()),
            _ => None,
        }
    }

    /// Convert to the raw i16 millimeter value for the OI protocol.
    pub fn to_mm(self) -> i16 {
        match self {
            Self::Straight => OI_RADIUS_STRAIGHT_RAW,
            Self::TurnInPlaceCw => OI_RADIUS_TURN_CW_RAW,
            Self::TurnInPlaceCcw => OI_RADIUS_TURN_CCW_RAW,
            Self::Curve(r) => libm::roundf(r.as_meters() * M_TO_MM) as i16,
        }
    }
}

impl core::fmt::Display for Radius {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Straight => f.write_str("straight"),
            Self::TurnInPlaceCw => f.write_str("turn-cw"),
            Self::TurnInPlaceCcw => f.write_str("turn-ccw"),
            Self::Curve(r) => write!(f, "{:.3} m", r.as_meters()),
        }
    }
}

impl TryFrom<f32> for Radius {
    type Error = ValidationError;
    /// Construct a `Radius::Curve` from a value in meters.
    ///
    /// Use `Radius::Straight`, `Radius::TurnInPlaceCw`, or `Radius::TurnInPlaceCcw`
    /// for the special OI values; those cannot be represented as a float.
    fn try_from(v: f32) -> Result<Self, Self::Error> {
        Self::new(v)
    }
}

/// Motor power level. Valid range: [-1.0, 1.0].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MotorPower(f32);

impl MotorPower {
    pub const MAX: f32 = 1.0;
    pub const MIN: f32 = -1.0;
    pub const OFF: Self = Self(0.0);

    pub fn new(value: f32) -> Result<Self, ValidationError> {
        validate_finite("MotorPower", value)?;
        validate_range("MotorPower", value, Self::MIN, Self::MAX)?;
        Ok(Self(value))
    }

    #[inline(always)]
    pub const fn get(self) -> f32 {
        self.0
    }

    /// Convert to PWM value (-255..255) for the OI protocol (rounds to nearest).
    pub fn to_pwm(self) -> i16 {
        libm::roundf(self.0 * OI_MAX_PWM as f32) as i16
    }
}

impl core::fmt::Display for MotorPower {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

impl TryFrom<f32> for MotorPower {
    type Error = ValidationError;
    fn try_from(v: f32) -> Result<Self, Self::Error> {
        Self::new(v)
    }
}

/// Power LED color (0 = green, 255 = red).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerLedColor(u8);

impl PowerLedColor {
    pub const GREEN: Self = Self(0);
    pub const RED: Self = Self(255);

    #[inline(always)]
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    #[inline(always)]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl From<u8> for PowerLedColor {
    #[inline(always)]
    fn from(v: u8) -> Self {
        Self(v)
    }
}

impl From<PowerLedColor> for u8 {
    #[inline(always)]
    fn from(c: PowerLedColor) -> u8 {
        c.0
    }
}

impl core::fmt::Display for PowerLedColor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// LED intensity (0 = off, 255 = full brightness).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedIntensity(u8);

impl LedIntensity {
    pub const OFF: Self = Self(0);
    pub const FULL: Self = Self(255);

    #[inline(always)]
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    #[inline(always)]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl From<u8> for LedIntensity {
    #[inline(always)]
    fn from(v: u8) -> Self {
        Self(v)
    }
}

impl From<LedIntensity> for u8 {
    #[inline(always)]
    fn from(i: LedIntensity) -> u8 {
        i.0
    }
}

impl core::fmt::Display for LedIntensity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Song number (0–15 for Create 1 / Roomba 400, 0–4 for Create 2).
///
/// `SongNumber::new()` accepts the widest valid range (0–15). The control
/// layer (`define_song`, `play_song`) further restricts this to the
/// per-model maximum via [`RobotModel::max_song_number`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SongNumber(u8);

impl SongNumber {
    pub const fn new(value: u8) -> Result<Self, ValidationError> {
        if value > OI_MAX_SONG_NUMBER {
            return Err(ValidationError {
                field: "SongNumber",
                reason: "song number exceeds OI maximum of 15",
            });
        }
        Ok(Self(value))
    }

    #[inline(always)]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for SongNumber {
    type Error = ValidationError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        Self::new(v)
    }
}

impl From<SongNumber> for u8 {
    #[inline(always)]
    fn from(s: SongNumber) -> u8 {
        s.0
    }
}

impl core::fmt::Display for SongNumber {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Motor and button bitfield types
// ---------------------------------------------------------------------------

/// A validated song note: MIDI pitch number and duration.
///
/// OI spec §5.13: note numbers must be in 31..=127. Duration is in 1/64-second
/// increments (0–255, where 0 means "play for 0 frames" and 255 ≈ 3.98 s).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SongNote {
    /// MIDI note number (31–127).
    midi_note: u8,
    /// Duration in units of 1/64 second (0–255).
    duration_64ths: u8,
}

impl SongNote {
    /// Create a new `SongNote`, validating that `midi_note` is in 31..=127.
    ///
    /// Duration is unconstrained (0–255 covers the full spec range).
    pub const fn new(midi_note: u8, duration_64ths: u8) -> Result<Self, ValidationError> {
        if midi_note < 31 || midi_note > 127 {
            return Err(ValidationError {
                field: "midi_note",
                reason: "MIDI note number must be in range 31..=127 (OI spec §5.13)",
            });
        }
        Ok(Self {
            midi_note,
            duration_64ths,
        })
    }

    /// Returns the MIDI note number (31–127).
    #[inline(always)]
    pub const fn midi_note(self) -> u8 {
        self.midi_note
    }

    /// Returns the duration in 1/64-second units (0–255).
    #[inline(always)]
    pub const fn duration_64ths(self) -> u8 {
        self.duration_64ths
    }
}

/// Motor enable and direction bits for the MOTORS command (opcode 138).
///
/// Bit layout per OI spec §5.6:
/// - Bit 0 = side_brush (1 = on)
/// - Bit 1 = vacuum (1 = on)
/// - Bit 2 = main_brush (1 = on)
/// - Bit 3 = side_brush_backward (1 = clockwise direction)
/// - Bit 4 = main_brush_backward (1 = outward direction)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MotorBits {
    /// Enable the side brush motor.
    pub side_brush: bool,
    /// Enable the vacuum motor.
    pub vacuum: bool,
    /// Enable the main brush motor.
    pub main_brush: bool,
    /// Reverse the side brush direction (`false` = counterclockwise, `true` = clockwise).
    pub side_brush_backward: bool,
    /// Reverse the main brush direction (`false` = inward, `true` = outward).
    pub main_brush_backward: bool,
}

impl MotorBits {
    /// Encode to the raw OI byte.
    ///
    /// Per OI spec §5.6: bit 3 = side brush direction, bit 4 = main brush direction.
    #[inline(always)]
    pub const fn to_raw(self) -> u8 {
        (self.side_brush as u8)
            | ((self.vacuum as u8) << 1)
            | ((self.main_brush as u8) << 2)
            | ((self.side_brush_backward as u8) << 3)
            | ((self.main_brush_backward as u8) << 4)
    }
}

/// Button simulation bits for the BUTTONS command (opcode 165, Full mode only).
///
/// Setting a field to `true` simulates pressing the corresponding physical button.
/// Bit layout: 0=clean, 1=spot, 2=dock, 3=minute, 4=hour, 5=day, 6=schedule, 7=clock.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ButtonBits {
    /// Simulate pressing the Clean button.
    pub clean: bool,
    /// Simulate pressing the Spot button.
    pub spot: bool,
    /// Simulate pressing the Dock button.
    pub dock: bool,
    /// Simulate pressing the Minute button.
    pub minute: bool,
    /// Simulate pressing the Hour button.
    pub hour: bool,
    /// Simulate pressing the Day button.
    pub day: bool,
    /// Simulate pressing the Schedule button.
    pub schedule: bool,
    /// Simulate pressing the Clock button.
    pub clock: bool,
}

impl ButtonBits {
    /// Encode to the raw OI byte.
    #[inline(always)]
    pub const fn to_raw(self) -> u8 {
        (self.clean as u8)
            | ((self.spot as u8) << 1)
            | ((self.dock as u8) << 2)
            | ((self.minute as u8) << 3)
            | ((self.hour as u8) << 4)
            | ((self.day as u8) << 5)
            | ((self.schedule as u8) << 6)
            | ((self.clock as u8) << 7)
    }
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_finite(field: &'static str, value: f32) -> Result<(), ValidationError> {
    if !value.is_finite() {
        return Err(ValidationError {
            field,
            reason: "must be finite",
        });
    }
    Ok(())
}

fn validate_range(
    field: &'static str,
    value: f32,
    min: f32,
    max: f32,
) -> Result<(), ValidationError> {
    if value < min || value > max {
        return Err(ValidationError {
            field,
            reason: "out of valid range",
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
        assert_eq!(Radius::Straight.to_mm(), i16::MIN);
        assert_eq!(Radius::STRAIGHT.to_mm(), i16::MIN);
        // Wire encoding must be [0x80, 0x00] — the canonical OI straight sentinel.
        let bytes = Radius::Straight.to_mm().to_be_bytes();
        assert_eq!(bytes, [0x80, 0x00]);
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
    fn radius_zero_rejected() {
        // Exact zero
        assert!(Radius::new(0.0).is_err());
        // Sub-millimetre values that round to 0 mm
        assert!(Radius::new(0.0004).is_err());
        assert!(Radius::new(-0.0004).is_err());
    }

    #[test]
    fn radius_smallest_valid_curve() {
        // 0.002 m = 2 mm — first valid positive arc radius
        let r = Radius::new(0.002).unwrap();
        assert_eq!(r.to_mm(), 2);
        // -0.002 m = -2 mm — first valid negative arc radius
        let r = Radius::new(-0.002).unwrap();
        assert_eq!(r.to_mm(), -2);
    }

    #[test]
    fn song_note_accessors() {
        let note = SongNote::new(69, 32).unwrap();
        assert_eq!(note.midi_note(), 69);
        assert_eq!(note.duration_64ths(), 32);
    }

    #[test]
    fn song_note_invalid_midi() {
        assert!(SongNote::new(30, 32).is_err());
        assert!(SongNote::new(128, 32).is_err());
        // Boundary values
        assert!(SongNote::new(31, 0).is_ok());
        assert!(SongNote::new(127, 255).is_ok());
    }

    #[test]
    fn curve_radius_as_meters() {
        let r = Radius::new(0.5).unwrap();
        if let Radius::Curve(c) = r {
            assert!((c.as_meters() - 0.5).abs() < 1e-6);
        } else {
            panic!("expected Curve variant");
        }
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
        assert!(SongNumber::new(4).is_ok());
        assert!(SongNumber::new(15).is_ok());
    }

    #[test]
    fn song_number_invalid() {
        assert!(SongNumber::new(16).is_err());
    }

    #[test]
    fn model_max_song_number() {
        assert_eq!(RobotModel::Create2.max_song_number(), 4);
        assert_eq!(RobotModel::Create1.max_song_number(), 15);
        assert_eq!(RobotModel::Roomba400.max_song_number(), 15);
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
    fn model_baud() {
        assert_eq!(RobotModel::Create2.baud(), 115_200);
        assert_eq!(RobotModel::Create1.baud(), 57_600);
    }

    #[test]
    fn motor_bits_all_off() {
        assert_eq!(MotorBits::default().to_raw(), 0);
    }

    #[test]
    fn motor_bits_all_on() {
        let bits = MotorBits {
            side_brush: true,
            vacuum: true,
            main_brush: true,
            main_brush_backward: false,
            side_brush_backward: false,
        };
        assert_eq!(bits.to_raw(), 0b00000111);
    }

    #[test]
    fn motor_bits_reverse() {
        let bits = MotorBits {
            side_brush: true,
            vacuum: false,
            main_brush: true,
            main_brush_backward: true,
            side_brush_backward: true,
        };
        // bits: 0=side_brush(1), 1=vacuum(0), 2=main_brush(1),
        //       3=side_brush_backward(1), 4=main_brush_backward(1) → 0b11101 = 29
        assert_eq!(bits.to_raw(), 0b11101);
    }

    #[test]
    fn motor_bits_side_brush_backward_is_bit3() {
        // OI spec §5.6: bit 3 = side brush direction
        let bits = MotorBits {
            side_brush_backward: true,
            ..Default::default()
        };
        assert_eq!(bits.to_raw(), 0b01000); // only bit 3 set = 8
    }

    #[test]
    fn motor_bits_main_brush_backward_is_bit4() {
        // OI spec §5.6: bit 4 = main brush direction
        let bits = MotorBits {
            main_brush_backward: true,
            ..Default::default()
        };
        assert_eq!(bits.to_raw(), 0b10000); // only bit 4 set = 16
    }

    #[test]
    fn button_bits_all_off() {
        assert_eq!(ButtonBits::default().to_raw(), 0);
    }

    #[test]
    fn button_bits_clean_and_dock() {
        let bits = ButtonBits {
            clean: true,
            dock: true,
            ..Default::default()
        };
        assert_eq!(bits.to_raw(), 0b00000101);
    }

    #[test]
    fn button_bits_all_on() {
        let bits = ButtonBits {
            clean: true,
            spot: true,
            dock: true,
            minute: true,
            hour: true,
            day: true,
            schedule: true,
            clock: true,
        };
        assert_eq!(bits.to_raw(), 0xFF);
    }

    // -----------------------------------------------------------------------
    // TryFrom<f32> tests
    // -----------------------------------------------------------------------

    #[test]
    fn angular_velocity_try_from() {
        assert!(AngularVelocity::try_from(0.0_f32).is_ok());
        assert!(AngularVelocity::try_from(core::f32::consts::PI).is_ok());
        assert!(AngularVelocity::try_from(-core::f32::consts::PI).is_ok());
        // MAX ≈ 4.255 rad/s (2 × 0.5 / 0.235); 4.0 is now valid, 5.0 is not
        assert!(AngularVelocity::try_from(4.0_f32).is_ok());
        assert!(AngularVelocity::try_from(5.0_f32).is_err());
        assert!(AngularVelocity::try_from(-5.0_f32).is_err());
        assert!(AngularVelocity::try_from(f32::NAN).is_err());
    }

    #[test]
    fn radius_try_from_builds_curve() {
        let r = Radius::try_from(0.5_f32).unwrap();
        assert_eq!(r.to_mm(), 500);
        assert!(Radius::try_from(2.1_f32).is_err()); // > 2.0
        assert!(Radius::try_from(f32::NAN).is_err());
    }

    #[test]
    fn motor_power_try_from() {
        assert!(MotorPower::try_from(0.0_f32).is_ok());
        assert!(MotorPower::try_from(1.0_f32).is_ok());
        assert!(MotorPower::try_from(-1.0_f32).is_ok());
        assert!(MotorPower::try_from(1.1_f32).is_err());
        assert!(MotorPower::try_from(f32::NAN).is_err());
    }

    // -----------------------------------------------------------------------
    // Display format tests
    // -----------------------------------------------------------------------

    #[test]
    fn velocity_display() {
        assert_eq!(Velocity::new(0.5).unwrap().to_string(), "0.500 m/s");
        assert_eq!(Velocity::new(-0.5).unwrap().to_string(), "-0.500 m/s");
        assert_eq!(Velocity::new(0.0).unwrap().to_string(), "0.000 m/s");
    }

    #[test]
    fn angular_velocity_display() {
        let av = AngularVelocity::new(0.0).unwrap();
        assert_eq!(av.to_string(), "0.000 rad/s");
    }

    #[test]
    fn radius_display() {
        assert_eq!(Radius::Straight.to_string(), "straight");
        assert_eq!(Radius::TurnInPlaceCw.to_string(), "turn-cw");
        assert_eq!(Radius::TurnInPlaceCcw.to_string(), "turn-ccw");
        assert_eq!(Radius::new(0.5).unwrap().to_string(), "0.500 m");
    }

    #[test]
    fn model_display() {
        assert_eq!(RobotModel::Create2.to_string(), "Create2");
        assert_eq!(RobotModel::Create1.to_string(), "Create1");
        assert_eq!(RobotModel::Roomba400.to_string(), "Roomba400");
    }

    #[test]
    fn model_supports_drive_direct() {
        assert!(RobotModel::Create1.supports_drive_direct());
        assert!(RobotModel::Create2.supports_drive_direct());
        assert!(!RobotModel::Roomba400.supports_drive_direct());
    }

    #[test]
    fn model_supports_individual_sensor_packets() {
        assert!(RobotModel::Create1.supports_individual_sensor_packets());
        assert!(RobotModel::Create2.supports_individual_sensor_packets());
        assert!(!RobotModel::Roomba400.supports_individual_sensor_packets());
    }

    #[test]
    fn model_max_individual_sensor_packet_id() {
        assert_eq!(RobotModel::Create2.max_individual_sensor_packet_id(), 58);
        assert_eq!(RobotModel::Create1.max_individual_sensor_packet_id(), 42);
        assert_eq!(RobotModel::Roomba400.max_individual_sensor_packet_id(), 0);
    }

    #[test]
    fn model_supports_query_list() {
        assert!(RobotModel::Create1.supports_query_list());
        assert!(RobotModel::Create2.supports_query_list());
        assert!(!RobotModel::Roomba400.supports_query_list());
    }

    #[test]
    fn model_supports_group_packet() {
        // Groups 0-3: all models
        for g in 0..=3u8 {
            assert!(RobotModel::Roomba400.supports_group_packet(g), "group {g}");
            assert!(RobotModel::Create1.supports_group_packet(g), "group {g}");
            assert!(RobotModel::Create2.supports_group_packet(g), "group {g}");
        }
        // Groups 4-6: Create 1 and Create 2 only
        for g in 4..=6u8 {
            assert!(!RobotModel::Roomba400.supports_group_packet(g), "group {g}");
            assert!(RobotModel::Create1.supports_group_packet(g), "group {g}");
            assert!(RobotModel::Create2.supports_group_packet(g), "group {g}");
        }
        // Group 100: Create 2 only
        assert!(!RobotModel::Roomba400.supports_group_packet(100));
        assert!(!RobotModel::Create1.supports_group_packet(100));
        assert!(RobotModel::Create2.supports_group_packet(100));
        // Unknown group
        assert!(!RobotModel::Create2.supports_group_packet(7));
        assert!(!RobotModel::Create2.supports_group_packet(99));
    }

    #[test]
    fn motor_power_display() {
        assert_eq!(MotorPower::new(1.0).unwrap().to_string(), "1.000");
    }
}
