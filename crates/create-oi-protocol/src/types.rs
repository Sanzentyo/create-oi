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
}

impl fmt::Display for IrChar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(x) => write!(f, "Unknown({x})"),
            _ => f.write_str(self.name()),
        }
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
}
