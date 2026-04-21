//! Wire-level types parsed from or encoded into OI packets.
//!
//! These are raw protocol values, not validated physical quantities.

use core::fmt;

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
        }
        assert!(DayOfWeek::from_raw(7).is_none());
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
