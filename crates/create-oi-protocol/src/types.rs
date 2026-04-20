//! Wire-level types parsed from or encoded into OI packets.
//!
//! These are raw protocol values, not validated physical quantities.

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
}
