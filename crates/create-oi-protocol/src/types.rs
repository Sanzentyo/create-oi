//! Wire-level types parsed from or encoded into OI packets.
//!
//! These are raw protocol values, not validated physical quantities.

use core::fmt;

// ---------------------------------------------------------------------------
// Baud rate codes (opcode 129 argument)
// ---------------------------------------------------------------------------

/// OI baud rate code (argument to opcode 129 / BAUD).
///
/// After the robot receives the `BAUD` command it switches to the new rate.
/// The host must wait 100 ms and then reconfigure its serial connection
/// to the same rate before sending further commands.
///
/// The default rate is 115200 for Create 2 and 57600 for Create 1 / Roomba 400.
///
/// The baud code table is the same for all robot models (OI spec Table 3):
/// codes 0–11 map to 300–115200 bps inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BaudRate {
    /// 300 bps
    Baud300 = 0,
    /// 600 bps
    Baud600 = 1,
    /// 1200 bps
    Baud1200 = 2,
    /// 2400 bps
    Baud2400 = 3,
    /// 4800 bps
    Baud4800 = 4,
    /// 9600 bps
    Baud9600 = 5,
    /// 14400 bps
    Baud14400 = 6,
    /// 19200 bps
    Baud19200 = 7,
    /// 28800 bps
    Baud28800 = 8,
    /// 38400 bps
    Baud38400 = 9,
    /// 57600 bps — default for Create 1 / Roomba 400
    Baud57600 = 10,
    /// 115200 bps — default for Create 2
    Baud115200 = 11,
}

impl BaudRate {
    /// The baud rate as a `u32`.
    #[inline(always)]
    pub const fn baud_u32(self) -> u32 {
        match self {
            Self::Baud300 => 300,
            Self::Baud600 => 600,
            Self::Baud1200 => 1200,
            Self::Baud2400 => 2400,
            Self::Baud4800 => 4800,
            Self::Baud9600 => 9600,
            Self::Baud14400 => 14400,
            Self::Baud19200 => 19200,
            Self::Baud28800 => 28800,
            Self::Baud38400 => 38400,
            Self::Baud57600 => 57600,
            Self::Baud115200 => 115200,
        }
    }

    /// Decode a baud code byte. Returns `None` for values outside 0–11.
    #[inline(always)]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Baud300),
            1 => Some(Self::Baud600),
            2 => Some(Self::Baud1200),
            3 => Some(Self::Baud2400),
            4 => Some(Self::Baud4800),
            5 => Some(Self::Baud9600),
            6 => Some(Self::Baud14400),
            7 => Some(Self::Baud19200),
            8 => Some(Self::Baud28800),
            9 => Some(Self::Baud38400),
            10 => Some(Self::Baud57600),
            11 => Some(Self::Baud115200),
            _ => None,
        }
    }
}

impl fmt::Display for BaudRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} bps", self.baud_u32())
    }
}

// ---------------------------------------------------------------------------
// OI mode (runtime value from sensor data, packet 35)
// ---------------------------------------------------------------------------

/// The OI mode as reported by the robot's sensor data (packet 35).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OiMode {
    Off,
    Passive,
    Safe,
    Full,
    Unknown(u8),
}

impl OiMode {
    #[inline(always)]
    pub const fn from_raw(v: u8) -> Self {
        match v {
            0 => Self::Off,
            1 => Self::Passive,
            2 => Self::Safe,
            3 => Self::Full,
            x => Self::Unknown(x),
        }
    }

    #[inline(always)]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Passive => "Passive",
            Self::Safe => "Safe",
            Self::Full => "Full",
            Self::Unknown(_) => "Unknown",
        }
    }
}

impl fmt::Display for OiMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(x) => write!(f, "Unknown({x})"),
            _ => f.write_str(self.name()),
        }
    }
}

// ---------------------------------------------------------------------------
// Charging state (packet 21)
// ---------------------------------------------------------------------------

/// Battery charging state (packet 21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargingState {
    NotCharging,
    ReconditioningCharging,
    FullCharging,
    TrickleCharging,
    Waiting,
    ChargingFaultCondition,
    Unknown(u8),
}

impl ChargingState {
    #[inline(always)]
    pub const fn from_raw(v: u8) -> Self {
        match v {
            0 => Self::NotCharging,
            1 => Self::ReconditioningCharging,
            2 => Self::FullCharging,
            3 => Self::TrickleCharging,
            4 => Self::Waiting,
            5 => Self::ChargingFaultCondition,
            x => Self::Unknown(x),
        }
    }

    #[inline(always)]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NotCharging => "NotCharging",
            Self::ReconditioningCharging => "ReconditioningCharging",
            Self::FullCharging => "FullCharging",
            Self::TrickleCharging => "TrickleCharging",
            Self::Waiting => "Waiting",
            Self::ChargingFaultCondition => "ChargingFaultCondition",
            Self::Unknown(_) => "Unknown",
        }
    }
}

impl fmt::Display for ChargingState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(x) => write!(f, "Unknown({x})"),
            _ => f.write_str(self.name()),
        }
    }
}

// ---------------------------------------------------------------------------
// Clean mode
// ---------------------------------------------------------------------------

/// Cleaning mode command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanMode {
    Default,
    Max,
    Spot,
}

impl CleanMode {
    /// Returns the human-readable name of this cleaning mode.
    #[inline(always)]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Max => "Max",
            Self::Spot => "Spot",
        }
    }
}

impl fmt::Display for CleanMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

// ---------------------------------------------------------------------------
// Day of week
// ---------------------------------------------------------------------------

/// Day of week for the robot's internal clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    #[inline(always)]
    pub const fn to_raw(self) -> u8 {
        self as u8
    }

    /// Convert a day index (0 = Sunday … 6 = Saturday) to a `DayOfWeek`.
    ///
    /// Returns `None` for values outside 0–6.
    #[inline(always)]
    pub const fn from_raw(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Sunday),
            1 => Some(Self::Monday),
            2 => Some(Self::Tuesday),
            3 => Some(Self::Wednesday),
            4 => Some(Self::Thursday),
            5 => Some(Self::Friday),
            6 => Some(Self::Saturday),
            _ => None,
        }
    }

    #[inline(always)]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sunday => "Sunday",
            Self::Monday => "Monday",
            Self::Tuesday => "Tuesday",
            Self::Wednesday => "Wednesday",
            Self::Thursday => "Thursday",
            Self::Friday => "Friday",
            Self::Saturday => "Saturday",
        }
    }
}

impl fmt::Display for DayOfWeek {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<u8> for DayOfWeek {
    type Error = u8;

    /// Convert a day index (0 = Sunday … 6 = Saturday) to a `DayOfWeek`.
    ///
    /// Returns `Err(v)` for values outside 0–6.
    #[inline(always)]
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        Self::from_raw(v).ok_or(v)
    }
}

// ---------------------------------------------------------------------------
// IR character
// ---------------------------------------------------------------------------

/// IR character received by the robot's IR sensors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrChar {
    None,
    Left,
    ForwardLeft,
    CenterLeft,
    CenterRight,
    ForwardRight,
    Right,
    SeekDock,
    ReservedGreen,
    ForceField,
    ReservedRed,
    BuoyGreen,
    BuoyRed,
    BuoyGreenAndRed,
    BuoyGreenAndForceField,
    BuoyRedAndForceField,
    BuoyGreenRedAndForceField,
    Unknown(u8),
}

impl IrChar {
    #[inline(always)]
    pub const fn from_raw(v: u8) -> Self {
        match v {
            0 => Self::None,
            129 => Self::Left,
            130 => Self::ForwardLeft,
            131 => Self::CenterLeft,
            132 => Self::CenterRight,
            133 => Self::ForwardRight,
            134 => Self::Right,
            143 => Self::SeekDock,
            160 => Self::ReservedGreen,
            161 => Self::ForceField,
            162 => Self::ReservedRed,
            164 => Self::BuoyGreen,
            168 => Self::BuoyRed,
            172 => Self::BuoyGreenAndRed,
            165 => Self::BuoyGreenAndForceField,
            169 => Self::BuoyRedAndForceField,
            173 => Self::BuoyGreenRedAndForceField,
            x => Self::Unknown(x),
        }
    }

    #[inline(always)]
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Left => "Left",
            Self::ForwardLeft => "ForwardLeft",
            Self::CenterLeft => "CenterLeft",
            Self::CenterRight => "CenterRight",
            Self::ForwardRight => "ForwardRight",
            Self::Right => "Right",
            Self::SeekDock => "SeekDock",
            Self::ReservedGreen => "ReservedGreen",
            Self::ForceField => "ForceField",
            Self::ReservedRed => "ReservedRed",
            Self::BuoyGreen => "BuoyGreen",
            Self::BuoyRed => "BuoyRed",
            Self::BuoyGreenAndRed => "BuoyGreenAndRed",
            Self::BuoyGreenAndForceField => "BuoyGreenAndForceField",
            Self::BuoyRedAndForceField => "BuoyRedAndForceField",
            Self::BuoyGreenRedAndForceField => "BuoyGreenRedAndForceField",
            Self::Unknown(_) => "Unknown",
        }
    }
    /// Returns `true` if this code was emitted by a Create 2 Remote Control.
    ///
    /// The remote control sends directional codes (`Left`, `ForwardLeft`,
    /// `CenterLeft`, `CenterRight`, `ForwardRight`, `Right`) and `SeekDock`
    /// to tell the robot to approach the home base.
    #[inline(always)]
    pub const fn is_remote_control(self) -> bool {
        matches!(
            self,
            Self::Left
                | Self::ForwardLeft
                | Self::CenterLeft
                | Self::CenterRight
                | Self::ForwardRight
                | Self::Right
                | Self::SeekDock
        )
    }

    /// Returns `true` if this is a dock approach beacon signal emitted by the
    /// home base dock or a virtual wall beacon.
    ///
    /// The dock emits buoy and force-field IR codes to guide the robot's
    /// approach. Virtual wall devices also emit `ForceField`.
    #[inline(always)]
    pub const fn is_dock_beacon(self) -> bool {
        matches!(
            self,
            Self::ForceField
                | Self::BuoyGreen
                | Self::BuoyRed
                | Self::BuoyGreenAndRed
                | Self::BuoyGreenAndForceField
                | Self::BuoyRedAndForceField
                | Self::BuoyGreenRedAndForceField
        )
    }

    /// Returns `true` if this code includes a virtual wall / force field
    /// component. The force field is emitted by both the home base dock
    /// (to prevent driving over it) and standalone virtual wall devices.
    #[inline(always)]
    pub const fn includes_force_field(self) -> bool {
        matches!(
            self,
            Self::ForceField
                | Self::BuoyGreenAndForceField
                | Self::BuoyRedAndForceField
                | Self::BuoyGreenRedAndForceField
        )
    }

    /// Returns `true` if no IR signal is present (the [`IrChar::None`] variant).
    ///
    /// Named `is_no_signal` rather than `is_none` to avoid visual confusion
    /// with [`Option::is_none`].
    #[inline(always)]
    pub const fn is_no_signal(self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns `true` if this is an unrecognized IR code not listed in the
    /// Create 2 OI specification.
    #[inline(always)]
    pub const fn is_unknown(self) -> bool {
        matches!(self, Self::Unknown(_))
    }
}

impl fmt::Display for IrChar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(x) => write!(f, "Unknown({x})"),
            _ => f.write_str(self.name()),
        }
    }
}

// ---------------------------------------------------------------------------
// Drive command wire-level integer newtypes
// ---------------------------------------------------------------------------

/// Wheel velocity in mm/s, as used in the OI wire protocol.
///
/// This is a raw protocol integer with OI spec validation available via
/// [`TryFrom<i16>`]: valid range is −500 to +500 mm/s.
///
/// Use [`from_raw`](Self::from_raw) for unchecked construction or
/// [`try_from`](TryFrom::try_from) for validated construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VelocityMmPerSec(i16);

impl VelocityMmPerSec {
    /// Zero velocity (stopped).
    pub const ZERO: Self = Self::from_raw(0);

    /// Construct from a raw mm/s value. No range validation is performed.
    #[inline(always)]
    pub const fn from_raw(v: i16) -> Self {
        Self(v)
    }

    /// Return the raw mm/s value.
    #[inline(always)]
    pub const fn get(self) -> i16 {
        self.0
    }
}

impl TryFrom<i16> for VelocityMmPerSec {
    /// The out-of-range input value.
    type Error = i16;

    /// Construct a `VelocityMmPerSec` with OI spec range validation (−500 to +500 mm/s).
    ///
    /// Returns `Err(v)` if `v` is outside `−500..=500`.
    #[inline(always)]
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        if (-500..=500).contains(&v) {
            Ok(Self::from_raw(v))
        } else {
            Err(v)
        }
    }
}

impl fmt::Display for VelocityMmPerSec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} mm/s", self.0)
    }
}

/// Turning radius in mm, as used in the OI `Drive` command wire protocol.
///
/// Encodes both physical arc radii and the three OI special values:
/// [`STRAIGHT`](Self::STRAIGHT), [`TURN_CW`](Self::TURN_CW), [`TURN_CCW`](Self::TURN_CCW).
///
/// Use [`TryFrom<i16>`] for validated construction (accepts ±2000 mm and the
/// three OI special sentinels; rejects 0 and out-of-range values), or
/// [`from_raw`](Self::from_raw) for unchecked construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RadiusMm(i16);

impl RadiusMm {
    /// Drive straight (OI special: `0x8000` = `i16::MIN`).
    pub const STRAIGHT: Self = Self::from_raw(i16::MIN);
    /// Turn in place clockwise (OI special: `-1`).
    pub const TURN_CW: Self = Self::from_raw(-1);
    /// Turn in place counter-clockwise (OI special: `1`).
    pub const TURN_CCW: Self = Self::from_raw(1);

    /// Construct from a raw mm value. No range validation is performed.
    #[inline(always)]
    pub const fn from_raw(v: i16) -> Self {
        Self(v)
    }

    /// Return the raw mm value.
    #[inline(always)]
    pub const fn get(self) -> i16 {
        self.0
    }
}

impl TryFrom<i16> for RadiusMm {
    /// The out-of-range input value.
    type Error = i16;

    /// Construct a `RadiusMm` with OI spec validation.
    ///
    /// Valid values:
    /// - `i16::MIN` (−32768): drive straight (OI sentinel `0x8000`)
    /// - `−2000..=−1` and `1..=2000`: arc radius in mm
    ///
    /// `0` and values outside `[−2000, 2000] ∪ {i16::MIN}` are rejected.
    ///
    /// Returns `Err(v)` for any invalid value.
    #[inline(always)]
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        if v == i16::MIN || (v >= -2000 && v != 0 && v <= 2000) {
            Ok(Self::from_raw(v))
        } else {
            Err(v)
        }
    }
}

impl fmt::Display for RadiusMm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::STRAIGHT => f.write_str("straight"),
            Self::TURN_CW => f.write_str("turn-cw"),
            Self::TURN_CCW => f.write_str("turn-ccw"),
            Self(v) => write!(f, "{v} mm"),
        }
    }
}

/// Wheel PWM value as used in the OI `DrivePwm` command wire protocol.
///
/// Valid OI range is ±255. Use [`TryFrom<i16>`] for validated construction,
/// or [`from_raw`](Self::from_raw) for unchecked construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WheelPwm(i16);

impl WheelPwm {
    /// Zero PWM (motor off).
    pub const STOP: Self = Self::from_raw(0);

    /// Construct from a raw PWM value. No range validation is performed.
    #[inline(always)]
    pub const fn from_raw(v: i16) -> Self {
        Self(v)
    }

    /// Return the raw PWM value.
    #[inline(always)]
    pub const fn get(self) -> i16 {
        self.0
    }
}

impl TryFrom<i16> for WheelPwm {
    /// The out-of-range input value.
    type Error = i16;

    /// Construct a `WheelPwm` with OI spec range validation (−255 to +255).
    ///
    /// Returns `Err(v)` if `v` is outside `−255..=255`.
    #[inline(always)]
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        if (-255..=255).contains(&v) {
            Ok(Self::from_raw(v))
        } else {
            Err(v)
        }
    }
}

impl fmt::Display for WheelPwm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PWM {}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_of_week_round_trip() {
        for i in 0u8..=6 {
            let day = DayOfWeek::from_raw(i).expect("valid day index");
            assert_eq!(day.to_raw(), i);
            // TryFrom<u8> must agree with from_raw
            let day2 = DayOfWeek::try_from(i).expect("try_from valid");
            assert_eq!(day, day2);
        }
        assert!(DayOfWeek::from_raw(7).is_none());
        assert!(DayOfWeek::try_from(7u8).is_err());
    }

    #[test]
    fn day_of_week_display() {
        assert_eq!(DayOfWeek::Sunday.to_string(), "Sunday");
        assert_eq!(DayOfWeek::Saturday.to_string(), "Saturday");
    }

    #[test]
    fn oi_mode_display() {
        assert_eq!(OiMode::Off.to_string(), "Off");
        assert_eq!(OiMode::Full.to_string(), "Full");
        assert_eq!(OiMode::Unknown(42).to_string(), "Unknown(42)");
    }

    #[test]
    fn charging_state_display() {
        assert_eq!(ChargingState::NotCharging.to_string(), "NotCharging");
        assert_eq!(ChargingState::Unknown(99).to_string(), "Unknown(99)");
    }

    #[test]
    fn velocity_mm_per_sec_round_trip() {
        let v = VelocityMmPerSec::from_raw(300);
        assert_eq!(v.get(), 300);
        assert_eq!(VelocityMmPerSec::ZERO.get(), 0);
        assert_eq!(v.to_string(), "300 mm/s");
    }

    #[test]
    fn radius_mm_special_values() {
        assert_eq!(RadiusMm::STRAIGHT.get(), i16::MIN);
        assert_eq!(RadiusMm::TURN_CW.get(), -1);
        assert_eq!(RadiusMm::TURN_CCW.get(), 1);
        assert_eq!(RadiusMm::STRAIGHT.to_string(), "straight");
        assert_eq!(RadiusMm::TURN_CW.to_string(), "turn-cw");
        assert_eq!(RadiusMm::TURN_CCW.to_string(), "turn-ccw");
        assert_eq!(RadiusMm::from_raw(500).to_string(), "500 mm");
    }

    #[test]
    fn wheel_pwm_round_trip() {
        let p = WheelPwm::from_raw(-128);
        assert_eq!(p.get(), -128);
        assert_eq!(WheelPwm::STOP.get(), 0);
        assert_eq!(p.to_string(), "PWM -128");
    }

    #[test]
    fn velocity_mm_per_sec_try_from() {
        assert!(VelocityMmPerSec::try_from(500_i16).is_ok());
        assert!(VelocityMmPerSec::try_from(-500_i16).is_ok());
        assert!(VelocityMmPerSec::try_from(0_i16).is_ok());
        assert_eq!(VelocityMmPerSec::try_from(501_i16), Err(501));
        assert_eq!(VelocityMmPerSec::try_from(-501_i16), Err(-501));
        assert_eq!(VelocityMmPerSec::try_from(i16::MAX), Err(i16::MAX));
    }

    #[test]
    fn radius_mm_try_from() {
        // Special sentinels are valid
        assert!(RadiusMm::try_from(i16::MIN).is_ok());
        assert!(RadiusMm::try_from(-1_i16).is_ok());
        assert!(RadiusMm::try_from(1_i16).is_ok());
        // Arc radii in range
        assert!(RadiusMm::try_from(500_i16).is_ok());
        assert!(RadiusMm::try_from(-2000_i16).is_ok());
        assert!(RadiusMm::try_from(2000_i16).is_ok());
        // Invalid values
        assert_eq!(RadiusMm::try_from(0_i16), Err(0));
        assert_eq!(RadiusMm::try_from(2001_i16), Err(2001));
        assert_eq!(RadiusMm::try_from(-2001_i16), Err(-2001));
        // i16::MAX is neither a sentinel nor in range
        assert!(RadiusMm::try_from(i16::MAX).is_err());
    }

    #[test]
    fn wheel_pwm_try_from() {
        assert!(WheelPwm::try_from(255_i16).is_ok());
        assert!(WheelPwm::try_from(-255_i16).is_ok());
        assert!(WheelPwm::try_from(0_i16).is_ok());
        assert_eq!(WheelPwm::try_from(256_i16), Err(256));
        assert_eq!(WheelPwm::try_from(-256_i16), Err(-256));
    }

    // Round 34: IrChar predicates
    #[test]
    fn irchar_is_no_signal() {
        assert!(IrChar::None.is_no_signal());
        assert!(!IrChar::Left.is_no_signal());
        assert!(!IrChar::ForceField.is_no_signal());
    }

    #[test]
    fn irchar_is_unknown() {
        assert!(IrChar::Unknown(42).is_unknown());
        assert!(!IrChar::None.is_unknown());
        assert!(!IrChar::SeekDock.is_unknown());
    }

    #[test]
    fn irchar_is_remote_control() {
        // All RC directional codes and SeekDock
        for code in [
            IrChar::Left,
            IrChar::ForwardLeft,
            IrChar::CenterLeft,
            IrChar::CenterRight,
            IrChar::ForwardRight,
            IrChar::Right,
            IrChar::SeekDock,
        ] {
            assert!(
                code.is_remote_control(),
                "{code:?} should be remote control"
            );
        }
        // Dock beacon codes are not RC
        assert!(!IrChar::ForceField.is_remote_control());
        assert!(!IrChar::BuoyGreen.is_remote_control());
        assert!(!IrChar::None.is_remote_control());
    }

    #[test]
    fn irchar_is_dock_beacon() {
        for code in [
            IrChar::ForceField,
            IrChar::BuoyGreen,
            IrChar::BuoyRed,
            IrChar::BuoyGreenAndRed,
            IrChar::BuoyGreenAndForceField,
            IrChar::BuoyRedAndForceField,
            IrChar::BuoyGreenRedAndForceField,
        ] {
            assert!(code.is_dock_beacon(), "{code:?} should be dock beacon");
        }
        // RC codes are not dock beacons
        assert!(!IrChar::Left.is_dock_beacon());
        assert!(!IrChar::SeekDock.is_dock_beacon());
        assert!(!IrChar::None.is_dock_beacon());
    }

    #[test]
    fn irchar_includes_force_field() {
        assert!(IrChar::ForceField.includes_force_field());
        assert!(IrChar::BuoyGreenAndForceField.includes_force_field());
        assert!(IrChar::BuoyRedAndForceField.includes_force_field());
        assert!(IrChar::BuoyGreenRedAndForceField.includes_force_field());
        assert!(!IrChar::BuoyGreen.includes_force_field());
        assert!(!IrChar::BuoyRed.includes_force_field());
        assert!(!IrChar::SeekDock.includes_force_field());
        assert!(!IrChar::None.includes_force_field());
    }

    // Round 34: CleanMode Display
    #[test]
    fn clean_mode_display() {
        assert_eq!(CleanMode::Default.to_string(), "Default");
        assert_eq!(CleanMode::Max.to_string(), "Max");
        assert_eq!(CleanMode::Spot.to_string(), "Spot");
    }

    #[test]
    fn clean_mode_name() {
        assert_eq!(CleanMode::Default.name(), "Default");
        assert_eq!(CleanMode::Max.name(), "Max");
        assert_eq!(CleanMode::Spot.name(), "Spot");
    }
}
