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

// ---------------------------------------------------------------------------
// Sensor packet IDs and metadata
// ---------------------------------------------------------------------------

/// A sensor packet identifier.
///
/// Use the named constants for all OI-defined packet IDs, or construct via
/// [`PacketId::new`] for protocol-extension or dynamically-determined IDs.
///
/// Group IDs (0–6, 100, 101, 106, 107) request a predefined bundle of packets
/// and are accepted by [`query_sensor_raw`](crate::command::encode_sensors),
/// but **not** by `query_sensor` or `decode_packet` (which only accept
/// individual packet IDs 7–58).  Named constants for both kinds are provided
/// on this type for convenience.
///
/// # Examples
/// ```
/// use create_oi_protocol::PacketId;
/// assert_eq!(PacketId::VOLTAGE.get(), 22);
/// let id = PacketId::new(22);
/// assert_eq!(id, PacketId::VOLTAGE);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PacketId(u8);

impl PacketId {
    /// Construct a `PacketId` from a raw byte value.
    ///
    /// No range validation is performed here; unknown IDs are rejected at the
    /// call site (e.g., `query_sensor_raw`).
    #[inline]
    pub const fn new(id: u8) -> Self {
        Self(id)
    }

    /// Return the raw byte value of this packet ID.
    #[inline]
    pub const fn get(self) -> u8 {
        self.0
    }

    // -----------------------------------------------------------------------
    // Group IDs (return a predefined bundle of sensor packets)
    // -----------------------------------------------------------------------
    /// Group 0 — packets 7–26.
    pub const GROUP_0: PacketId = PacketId(0);
    /// Group 1 — packets 7–16.
    pub const GROUP_1: PacketId = PacketId(1);
    /// Group 2 — packets 17–20.
    pub const GROUP_2: PacketId = PacketId(2);
    /// Group 3 — packets 21–26.
    pub const GROUP_3: PacketId = PacketId(3);
    /// Group 4 — packets 27–34.
    pub const GROUP_4: PacketId = PacketId(4);
    /// Group 5 — packets 35–42.
    pub const GROUP_5: PacketId = PacketId(5);
    /// Group 6 — packets 7–42.
    pub const GROUP_6: PacketId = PacketId(6);
    /// Group 100 — packets 7–58 (Create 2 / Roomba 500+).
    pub const GROUP_100: PacketId = PacketId(100);
    /// Group 101 — packets 43–58 (Create 2 / Roomba 500+).
    pub const GROUP_101: PacketId = PacketId(101);
    /// Group 106 — light bump signals 46–51 (Create 2 / Roomba 500+).
    pub const GROUP_106: PacketId = PacketId(106);
    /// Group 107 — motor currents 54–58 (Create 2 / Roomba 500+).
    pub const GROUP_107: PacketId = PacketId(107);

    // -----------------------------------------------------------------------
    // Individual sensor packet IDs (7–58)
    // -----------------------------------------------------------------------
    /// Bumps and wheel drops (1 byte, unsigned).
    pub const BUMPS_AND_WHEEL_DROPS: PacketId = PacketId(7);
    /// Wall sensor (1 byte, unsigned).
    pub const WALL: PacketId = PacketId(8);
    /// Cliff left (1 byte, unsigned).
    pub const CLIFF_LEFT: PacketId = PacketId(9);
    /// Cliff front left (1 byte, unsigned).
    pub const CLIFF_FRONT_LEFT: PacketId = PacketId(10);
    /// Cliff front right (1 byte, unsigned).
    pub const CLIFF_FRONT_RIGHT: PacketId = PacketId(11);
    /// Cliff right (1 byte, unsigned).
    pub const CLIFF_RIGHT: PacketId = PacketId(12);
    /// Virtual wall (1 byte, unsigned).
    pub const VIRTUAL_WALL: PacketId = PacketId(13);
    /// Wheel overcurrents (1 byte, unsigned).
    pub const WHEEL_OVERCURRENTS: PacketId = PacketId(14);
    /// Dirt detect left (1 byte, unsigned).
    pub const DIRT_DETECT_LEFT: PacketId = PacketId(15);
    /// Dirt detect right / unused (1 byte, unsigned).
    pub const DIRT_DETECT_RIGHT: PacketId = PacketId(16);
    /// Infrared character omni (1 byte, unsigned).
    pub const IR_OPCODE_OMNI: PacketId = PacketId(17);
    /// Buttons (1 byte, unsigned).
    pub const BUTTONS: PacketId = PacketId(18);
    /// Distance (2 bytes, signed, mm).
    pub const DISTANCE: PacketId = PacketId(19);
    /// Angle (2 bytes, signed, degrees).
    pub const ANGLE: PacketId = PacketId(20);
    /// Charging state (1 byte, unsigned).
    pub const CHARGING_STATE: PacketId = PacketId(21);
    /// Battery voltage (2 bytes, unsigned, mV).
    pub const VOLTAGE: PacketId = PacketId(22);
    /// Battery current (2 bytes, signed, mA).
    pub const CURRENT: PacketId = PacketId(23);
    /// Battery temperature (1 byte, signed, °C).
    pub const TEMPERATURE: PacketId = PacketId(24);
    /// Battery charge (2 bytes, unsigned, mAh).
    pub const BATTERY_CHARGE: PacketId = PacketId(25);
    /// Battery capacity (2 bytes, unsigned, mAh).
    pub const BATTERY_CAPACITY: PacketId = PacketId(26);
    /// Wall signal strength (2 bytes, unsigned).
    pub const WALL_SIGNAL: PacketId = PacketId(27);
    /// Cliff left signal strength (2 bytes, unsigned).
    pub const CLIFF_LEFT_SIGNAL: PacketId = PacketId(28);
    /// Cliff front left signal strength (2 bytes, unsigned).
    pub const CLIFF_FRONT_LEFT_SIGNAL: PacketId = PacketId(29);
    /// Cliff front right signal strength (2 bytes, unsigned).
    pub const CLIFF_FRONT_RIGHT_SIGNAL: PacketId = PacketId(30);
    /// Cliff right signal strength (2 bytes, unsigned).
    pub const CLIFF_RIGHT_SIGNAL: PacketId = PacketId(31);
    /// Cargo bay digital inputs (1 byte, unsigned).
    pub const CARGO_BAY_DIGITAL_INPUTS: PacketId = PacketId(32);
    /// Cargo bay analog signal (2 bytes, unsigned).
    pub const CARGO_BAY_ANALOG_SIGNAL: PacketId = PacketId(33);
    /// Charging sources available (1 byte, unsigned).
    pub const CHARGING_SOURCES_AVAILABLE: PacketId = PacketId(34);
    /// Current OI mode (1 byte, unsigned).
    pub const OI_MODE: PacketId = PacketId(35);
    /// Song number (1 byte, unsigned).
    pub const SONG_NUMBER: PacketId = PacketId(36);
    /// Song playing flag (1 byte, unsigned).
    pub const SONG_PLAYING: PacketId = PacketId(37);
    /// Number of stream packets (1 byte, unsigned).
    pub const NUMBER_OF_STREAM_PACKETS: PacketId = PacketId(38);
    /// Requested velocity (2 bytes, signed, mm/s).
    pub const REQUESTED_VELOCITY: PacketId = PacketId(39);
    /// Requested radius (2 bytes, signed, mm).
    pub const REQUESTED_RADIUS: PacketId = PacketId(40);
    /// Requested right velocity (2 bytes, signed, mm/s).
    pub const REQUESTED_RIGHT_VELOCITY: PacketId = PacketId(41);
    /// Requested left velocity (2 bytes, signed, mm/s).
    pub const REQUESTED_LEFT_VELOCITY: PacketId = PacketId(42);
    /// Left encoder counts (2 bytes, unsigned; Create 2 only).
    pub const LEFT_ENCODER_COUNTS: PacketId = PacketId(43);
    /// Right encoder counts (2 bytes, unsigned; Create 2 only).
    pub const RIGHT_ENCODER_COUNTS: PacketId = PacketId(44);
    /// Light bumper state (1 byte, unsigned; Create 2 only).
    pub const LIGHT_BUMPER: PacketId = PacketId(45);
    /// Light bump left signal (2 bytes, unsigned; Create 2 only).
    pub const LIGHT_BUMP_LEFT_SIGNAL: PacketId = PacketId(46);
    /// Light bump front left signal (2 bytes, unsigned; Create 2 only).
    pub const LIGHT_BUMP_FRONT_LEFT_SIGNAL: PacketId = PacketId(47);
    /// Light bump center left signal (2 bytes, unsigned; Create 2 only).
    pub const LIGHT_BUMP_CENTER_LEFT_SIGNAL: PacketId = PacketId(48);
    /// Light bump center right signal (2 bytes, unsigned; Create 2 only).
    pub const LIGHT_BUMP_CENTER_RIGHT_SIGNAL: PacketId = PacketId(49);
    /// Light bump front right signal (2 bytes, unsigned; Create 2 only).
    pub const LIGHT_BUMP_FRONT_RIGHT_SIGNAL: PacketId = PacketId(50);
    /// Light bump right signal (2 bytes, unsigned; Create 2 only).
    pub const LIGHT_BUMP_RIGHT_SIGNAL: PacketId = PacketId(51);
    /// Infrared character left (1 byte, unsigned; Create 2 only).
    pub const IR_OPCODE_LEFT: PacketId = PacketId(52);
    /// Infrared character right (1 byte, unsigned; Create 2 only).
    pub const IR_OPCODE_RIGHT: PacketId = PacketId(53);
    /// Left motor current (2 bytes, signed, mA; Create 2 only).
    pub const LEFT_MOTOR_CURRENT: PacketId = PacketId(54);
    /// Right motor current (2 bytes, signed, mA; Create 2 only).
    pub const RIGHT_MOTOR_CURRENT: PacketId = PacketId(55);
    /// Main brush motor current (2 bytes, signed, mA; Create 2 only).
    pub const MAIN_BRUSH_MOTOR_CURRENT: PacketId = PacketId(56);
    /// Side brush motor current (2 bytes, signed, mA; Create 2 only).
    pub const SIDE_BRUSH_MOTOR_CURRENT: PacketId = PacketId(57);
    /// Stasis sensor (1 byte, unsigned; Create 2 only).
    pub const STASIS: PacketId = PacketId(58);
}

impl From<u8> for PacketId {
    /// Wraps a raw byte as a `PacketId` without validation.
    ///
    /// Prefer the named constants (e.g., [`PacketId::VOLTAGE`]) for OI-defined
    /// IDs.  Use this conversion only when the ID is dynamic or unknown at
    /// compile time.
    #[inline]
    fn from(id: u8) -> Self {
        Self(id)
    }
}

impl From<PacketId> for u8 {
    #[inline]
    fn from(id: PacketId) -> Self {
        id.0
    }
}

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

pub const SENSOR_PACKETS_ID_MIN: u8 = SENSOR_PACKETS[0].id;
pub const SENSOR_PACKETS_ID_MAX: u8 = SENSOR_PACKETS[SENSOR_PACKETS.len() - 1].id;

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

/// Returns the constituent individual packet IDs for a group packet.
///
/// Returns `None` for non-group IDs (i.e., IDs outside 0–6 and 100–107).
/// The returned slice contains raw `u8` values for internal protocol use.
pub const fn group_packet_ids(group: PacketId) -> Option<&'static [u8]> {
    match group.0 {
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

/// Total byte length for the data returned by a group packet request.
///
/// Returns `None` for non-group IDs.
pub const fn group_data_len(group: PacketId) -> Option<usize> {
    let ids = match group_packet_ids(group) {
        Some(ids) => ids,
        None => return None,
    };
    let mut total = 0usize;
    let mut i = 0;
    while i < ids.len() {
        if let Some(p) = packet_info_raw(ids[i]) {
            total += p.len as usize;
        }
        i += 1;
    }
    Some(total)
}

/// Look up metadata for an individual sensor packet by its ID.
///
/// Returns `None` for group IDs (0–6, 100+) and any ID outside the
/// individual range 7–58.
#[inline(always)]
pub const fn packet_info(id: PacketId) -> Option<&'static PacketInfo> {
    packet_info_raw(id.0)
}

/// Internal: look up packet info by raw `u8` ID (used in const contexts).
#[inline(always)]
pub(crate) const fn packet_info_raw(id: u8) -> Option<&'static PacketInfo> {
    if SENSOR_PACKETS_ID_MIN <= id && id <= SENSOR_PACKETS_ID_MAX {
        Some(&SENSOR_PACKETS[(id - SENSOR_PACKETS_ID_MIN) as usize])
    } else {
        None
    }
}

/// Total data bytes for group 100 (all sensors).
pub const fn all_sensors_data_len() -> usize {
    let mut total = 0usize;
    let mut i = 0;
    while i < SENSOR_PACKETS.len() {
        total += SENSOR_PACKETS[i].len as usize;
        i += 1;
    }
    total
}
