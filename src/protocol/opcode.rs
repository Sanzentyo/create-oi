//! iRobot Open Interface opcodes.

/// All opcodes defined by the iRobot OI protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    Start = 128,
    Reset = 7,
    Stop = 173,
    Baud = 129,
    Control = 130,
    Safe = 131,
    Full = 132,
    Power = 133,
    Spot = 134,
    Clean = 135,
    Max = 136,
    Drive = 137,
    Motors = 138,
    Leds = 139,
    Song = 140,
    Play = 141,
    Sensors = 142,
    Dock = 143,
    MotorsPwm = 144,
    DriveDirect = 145,
    DrivePwm = 146,
    Stream = 148,
    QueryList = 149,
    ToggleStream = 150,
    SchedulingLeds = 162,
    DigitLedsRaw = 163,
    DigitLedsAscii = 164,
    Buttons = 165,
    Schedule = 167,
    Date = 168,
}

impl Opcode {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

// ---------------------------------------------------------------------------
// Sensor packet IDs and metadata
// ---------------------------------------------------------------------------

/// Metadata for a single sensor packet.
#[derive(Debug, Clone, Copy)]
pub struct PacketInfo {
    pub id: u8,
    /// Number of data bytes for this packet (1 or 2).
    pub len: u8,
    /// Whether the value should be interpreted as signed.
    pub signed: bool,
}

/// All individual sensor packet definitions (IDs 7–58).
/// Group packets (0–6, 100+) are composite and not listed here.
pub const SENSOR_PACKETS: &[PacketInfo] = &[
    PacketInfo {
        id: 7,
        len: 1,
        signed: false,
    }, // bumps & wheeldrops
    PacketInfo {
        id: 8,
        len: 1,
        signed: false,
    }, // wall
    PacketInfo {
        id: 9,
        len: 1,
        signed: false,
    }, // cliff left
    PacketInfo {
        id: 10,
        len: 1,
        signed: false,
    }, // cliff front left
    PacketInfo {
        id: 11,
        len: 1,
        signed: false,
    }, // cliff front right
    PacketInfo {
        id: 12,
        len: 1,
        signed: false,
    }, // cliff right
    PacketInfo {
        id: 13,
        len: 1,
        signed: false,
    }, // virtual wall
    PacketInfo {
        id: 14,
        len: 1,
        signed: false,
    }, // overcurrents
    PacketInfo {
        id: 15,
        len: 1,
        signed: false,
    }, // dirt detect (left)
    PacketInfo {
        id: 16,
        len: 1,
        signed: false,
    }, // dirt detect (right)
    PacketInfo {
        id: 17,
        len: 1,
        signed: false,
    }, // IR omni
    PacketInfo {
        id: 18,
        len: 1,
        signed: false,
    }, // buttons
    PacketInfo {
        id: 19,
        len: 2,
        signed: true,
    }, // distance
    PacketInfo {
        id: 20,
        len: 2,
        signed: true,
    }, // angle
    PacketInfo {
        id: 21,
        len: 1,
        signed: false,
    }, // charging state
    PacketInfo {
        id: 22,
        len: 2,
        signed: false,
    }, // voltage (mV)
    PacketInfo {
        id: 23,
        len: 2,
        signed: true,
    }, // current (mA)
    PacketInfo {
        id: 24,
        len: 1,
        signed: true,
    }, // temperature (°C)
    PacketInfo {
        id: 25,
        len: 2,
        signed: false,
    }, // battery charge (mAh)
    PacketInfo {
        id: 26,
        len: 2,
        signed: false,
    }, // battery capacity (mAh)
    PacketInfo {
        id: 27,
        len: 2,
        signed: false,
    }, // wall signal
    PacketInfo {
        id: 28,
        len: 2,
        signed: false,
    }, // cliff left signal
    PacketInfo {
        id: 29,
        len: 2,
        signed: false,
    }, // cliff front left signal
    PacketInfo {
        id: 30,
        len: 2,
        signed: false,
    }, // cliff front right signal
    PacketInfo {
        id: 31,
        len: 2,
        signed: false,
    }, // cliff right signal
    PacketInfo {
        id: 32,
        len: 1,
        signed: false,
    }, // cargo bay digital inputs
    PacketInfo {
        id: 33,
        len: 2,
        signed: false,
    }, // cargo bay analog signal
    PacketInfo {
        id: 34,
        len: 1,
        signed: false,
    }, // charging sources available
    PacketInfo {
        id: 35,
        len: 1,
        signed: false,
    }, // OI mode
    PacketInfo {
        id: 36,
        len: 1,
        signed: false,
    }, // song number
    PacketInfo {
        id: 37,
        len: 1,
        signed: false,
    }, // song playing
    PacketInfo {
        id: 38,
        len: 1,
        signed: false,
    }, // number of stream packets
    PacketInfo {
        id: 39,
        len: 2,
        signed: true,
    }, // requested velocity
    PacketInfo {
        id: 40,
        len: 2,
        signed: true,
    }, // requested radius
    PacketInfo {
        id: 41,
        len: 2,
        signed: true,
    }, // requested right velocity
    PacketInfo {
        id: 42,
        len: 2,
        signed: true,
    }, // requested left velocity
    PacketInfo {
        id: 43,
        len: 2,
        signed: false,
    }, // left encoder counts
    PacketInfo {
        id: 44,
        len: 2,
        signed: false,
    }, // right encoder counts
    PacketInfo {
        id: 45,
        len: 1,
        signed: false,
    }, // light bumper
    PacketInfo {
        id: 46,
        len: 2,
        signed: false,
    }, // light bump left signal
    PacketInfo {
        id: 47,
        len: 2,
        signed: false,
    }, // light bump front left signal
    PacketInfo {
        id: 48,
        len: 2,
        signed: false,
    }, // light bump center left signal
    PacketInfo {
        id: 49,
        len: 2,
        signed: false,
    }, // light bump center right signal
    PacketInfo {
        id: 50,
        len: 2,
        signed: false,
    }, // light bump front right signal
    PacketInfo {
        id: 51,
        len: 2,
        signed: false,
    }, // light bump right signal
    PacketInfo {
        id: 52,
        len: 1,
        signed: false,
    }, // IR left
    PacketInfo {
        id: 53,
        len: 1,
        signed: false,
    }, // IR right
    PacketInfo {
        id: 54,
        len: 2,
        signed: true,
    }, // left motor current (mA)
    PacketInfo {
        id: 55,
        len: 2,
        signed: true,
    }, // right motor current (mA)
    PacketInfo {
        id: 56,
        len: 2,
        signed: true,
    }, // main brush motor current (mA)
    PacketInfo {
        id: 57,
        len: 2,
        signed: true,
    }, // side brush motor current (mA)
    PacketInfo {
        id: 58,
        len: 1,
        signed: false,
    }, // stasis
];

/// IDs 7–42 (group 6).
const GROUP_6_IDS: [u8; 36] = [
    7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
    31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42,
];

/// IDs 7–58 (group 100).
const GROUP_100_IDS: [u8; 52] = [
    7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
    31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54,
    55, 56, 57, 58,
];

/// Sensor packet group definitions. Each group returns a contiguous range of packets.
pub fn group_packet_ids(group: u8) -> Option<&'static [u8]> {
    match group {
        0 => Some(&[
            7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
        ]),
        1 => Some(&[7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
        2 => Some(&[17, 18, 19, 20]),
        3 => Some(&[21, 22, 23, 24, 25, 26]),
        4 => Some(&[27, 28, 29, 30, 31, 32, 33, 34]),
        5 => Some(&[35, 36, 37, 38, 39, 40, 41, 42]),
        6 => Some(&GROUP_6_IDS),
        100 => Some(&GROUP_100_IDS),
        _ => None,
    }
}

/// Total byte length for a group of sensor packets.
pub fn group_data_len(group: u8) -> Option<usize> {
    let ids = group_packet_ids(group)?;
    let total = ids
        .iter()
        .map(|id| packet_info(*id).map_or(0, |p| p.len as usize))
        .sum();
    Some(total)
}

/// Look up packet info by ID.
pub fn packet_info(id: u8) -> Option<&'static PacketInfo> {
    SENSOR_PACKETS.iter().find(|p| p.id == id)
}

/// Total data bytes for group 100 (all sensors).
pub fn all_sensors_data_len() -> usize {
    SENSOR_PACKETS.iter().map(|p| p.len as usize).sum()
}
