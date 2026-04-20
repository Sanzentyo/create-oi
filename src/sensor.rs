//! Sensor snapshot types — structured representation of all sensor data.

use crate::types::{ChargingState, IrChar, OiMode};

/// Complete sensor snapshot captured atomically from the robot.
///
/// All fields are populated in a single locked read through the FFI layer,
/// ensuring consistency across values.
#[derive(Debug, Clone)]
pub struct SensorSnapshot {
    pub bumpers: Bumpers,
    pub cliffs: Cliffs,
    pub walls: Walls,
    pub light_bumpers: LightBumpers,
    pub battery: Battery,
    pub ir: IrSensors,
    pub buttons: Buttons,
    pub overcurrent: Overcurrent,
    pub odometry: Odometry,
    pub dirt_detect: u8,
    pub is_moving_forward: bool,
    pub oi_mode: OiMode,
    pub packet_stats: PacketStats,
}

/// Physical bump and wheeldrop sensors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bumpers {
    pub left: bool,
    pub right: bool,
    pub left_wheeldrop: bool,
    pub right_wheeldrop: bool,
}

/// Cliff detection sensors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cliffs {
    pub left: bool,
    pub front_left: bool,
    pub front_right: bool,
    pub right: bool,
}

/// Wall detection sensors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Walls {
    pub wall: bool,
    pub virtual_wall: bool,
}

/// Light bumper sensors (Create 2 only).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightBumpers {
    pub left: bool,
    pub front_left: bool,
    pub center_left: bool,
    pub center_right: bool,
    pub front_right: bool,
    pub right: bool,
    pub signal_left: u16,
    pub signal_front_left: u16,
    pub signal_center_left: u16,
    pub signal_center_right: u16,
    pub signal_front_right: u16,
    pub signal_right: u16,
}

/// Battery and power information.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Battery {
    /// Battery voltage in volts.
    pub voltage: f32,
    /// Current draw in amps (negative = charging).
    pub current: f32,
    /// Battery temperature in degrees Celsius.
    pub temperature: i8,
    /// Current battery charge in amp-hours.
    pub charge: f32,
    /// Battery capacity in amp-hours.
    pub capacity: f32,
    /// Current charging state.
    pub state: ChargingState,
}

impl Battery {
    /// Battery charge as a percentage (0.0–1.0).
    pub fn charge_ratio(&self) -> f32 {
        if self.capacity > 0.0 {
            (self.charge / self.capacity).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// Infrared sensor readings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IrSensors {
    pub omni: IrChar,
    pub left: IrChar,
    pub right: IrChar,
}

/// Button press states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Buttons {
    pub clean: bool,
    pub clock: bool,
    pub schedule: bool,
    pub day: bool,
    pub hour: bool,
    pub minute: bool,
    pub dock: bool,
    pub spot: bool,
}

/// Motor overcurrent flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overcurrent {
    pub wheels: bool,
    pub main_brush: bool,
    pub side_brush: bool,
}

/// Pose and velocity information from wheel encoders.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Odometry {
    pub pose_x: f32,
    pub pose_y: f32,
    pub pose_yaw: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub velocity_yaw: f32,
    pub left_wheel_distance: f32,
    pub right_wheel_distance: f32,
    pub measured_left_wheel_vel: f32,
    pub measured_right_wheel_vel: f32,
    pub requested_left_wheel_vel: f32,
    pub requested_right_wheel_vel: f32,
}

/// Packet reception statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketStats {
    pub corrupt: u64,
    pub total: u64,
}

impl PacketStats {
    /// Fraction of packets that were corrupt (0.0–1.0).
    pub fn corruption_rate(&self) -> f64 {
        if self.total > 0 {
            self.corrupt as f64 / self.total as f64
        } else {
            0.0
        }
    }
}

// ---------------------------------------------------------------------------
// Conversion from FFI snapshot
// ---------------------------------------------------------------------------

fn b(val: u8) -> bool {
    val != 0
}

impl From<libcreate_sys::create_sensor_snapshot_t> for SensorSnapshot {
    fn from(raw: libcreate_sys::create_sensor_snapshot_t) -> Self {
        Self {
            bumpers: Bumpers {
                left: b(raw.is_left_bumper),
                right: b(raw.is_right_bumper),
                left_wheeldrop: b(raw.is_left_wheeldrop),
                right_wheeldrop: b(raw.is_right_wheeldrop),
            },
            cliffs: Cliffs {
                left: b(raw.is_cliff_left),
                front_left: b(raw.is_cliff_front_left),
                front_right: b(raw.is_cliff_front_right),
                right: b(raw.is_cliff_right),
            },
            walls: Walls {
                wall: b(raw.is_wall),
                virtual_wall: b(raw.is_virtual_wall),
            },
            light_bumpers: LightBumpers {
                left: b(raw.is_light_bumper_left),
                front_left: b(raw.is_light_bumper_front_left),
                center_left: b(raw.is_light_bumper_center_left),
                center_right: b(raw.is_light_bumper_center_right),
                front_right: b(raw.is_light_bumper_front_right),
                right: b(raw.is_light_bumper_right),
                signal_left: raw.light_signal_left,
                signal_front_left: raw.light_signal_front_left,
                signal_center_left: raw.light_signal_center_left,
                signal_center_right: raw.light_signal_center_right,
                signal_front_right: raw.light_signal_front_right,
                signal_right: raw.light_signal_right,
            },
            battery: Battery {
                voltage: raw.voltage,
                current: raw.current,
                temperature: raw.temperature,
                charge: raw.battery_charge,
                capacity: raw.battery_capacity,
                state: ChargingState::from_raw(raw.charging_state),
            },
            ir: IrSensors {
                omni: IrChar::from_raw(raw.ir_omni),
                left: IrChar::from_raw(raw.ir_left),
                right: IrChar::from_raw(raw.ir_right),
            },
            buttons: Buttons {
                clean: b(raw.is_clean_button),
                clock: b(raw.is_clock_button),
                schedule: b(raw.is_schedule_button),
                day: b(raw.is_day_button),
                hour: b(raw.is_hour_button),
                minute: b(raw.is_min_button),
                dock: b(raw.is_dock_button),
                spot: b(raw.is_spot_button),
            },
            overcurrent: Overcurrent {
                wheels: b(raw.is_wheel_overcurrent),
                main_brush: b(raw.is_main_brush_overcurrent),
                side_brush: b(raw.is_side_brush_overcurrent),
            },
            odometry: Odometry {
                pose_x: raw.pose_x,
                pose_y: raw.pose_y,
                pose_yaw: raw.pose_yaw,
                velocity_x: raw.vel_x,
                velocity_y: raw.vel_y,
                velocity_yaw: raw.vel_yaw,
                left_wheel_distance: raw.left_wheel_distance,
                right_wheel_distance: raw.right_wheel_distance,
                measured_left_wheel_vel: raw.measured_left_wheel_vel,
                measured_right_wheel_vel: raw.measured_right_wheel_vel,
                requested_left_wheel_vel: raw.requested_left_wheel_vel,
                requested_right_wheel_vel: raw.requested_right_wheel_vel,
            },
            dirt_detect: raw.dirt_detect,
            is_moving_forward: b(raw.is_moving_forward),
            oi_mode: OiMode::from_raw(raw.oi_mode),
            packet_stats: PacketStats {
                corrupt: raw.num_corrupt_packets,
                total: raw.total_packets,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libcreate_sys::create_sensor_snapshot_t;

    #[test]
    fn battery_charge_ratio_normal() {
        let battery = Battery {
            voltage: 14.5,
            current: -0.5,
            temperature: 25,
            charge: 1.5,
            capacity: 3.0,
            state: ChargingState::NotCharging,
        };
        assert!((battery.charge_ratio() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn battery_charge_ratio_full() {
        let battery = Battery {
            voltage: 16.0,
            current: 0.0,
            temperature: 25,
            charge: 3.0,
            capacity: 3.0,
            state: ChargingState::FullCharging,
        };
        assert!((battery.charge_ratio() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn battery_charge_ratio_zero_capacity() {
        let battery = Battery {
            voltage: 0.0,
            current: 0.0,
            temperature: 0,
            charge: 0.0,
            capacity: 0.0,
            state: ChargingState::NotCharging,
        };
        assert_eq!(battery.charge_ratio(), 0.0);
    }

    #[test]
    fn battery_charge_ratio_clamped() {
        let battery = Battery {
            voltage: 16.0,
            current: 0.0,
            temperature: 25,
            charge: 5.0,
            capacity: 3.0,
            state: ChargingState::NotCharging,
        };
        assert_eq!(battery.charge_ratio(), 1.0);
    }

    #[test]
    fn packet_stats_corruption_rate() {
        let stats = PacketStats {
            corrupt: 5,
            total: 100,
        };
        assert!((stats.corruption_rate() - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn packet_stats_corruption_rate_zero_total() {
        let stats = PacketStats {
            corrupt: 0,
            total: 0,
        };
        assert_eq!(stats.corruption_rate(), 0.0);
    }

    #[test]
    fn sensor_snapshot_from_raw_defaults() {
        let raw = create_sensor_snapshot_t::default();
        let snapshot = SensorSnapshot::from(raw);

        assert!(!snapshot.bumpers.left);
        assert!(!snapshot.bumpers.right);
        assert!(!snapshot.cliffs.left);
        assert!(!snapshot.walls.wall);
        assert_eq!(snapshot.battery.voltage, 0.0);
        assert_eq!(snapshot.oi_mode, OiMode::Off);
        assert!(!snapshot.is_moving_forward);
        assert_eq!(snapshot.packet_stats.total, 0);
    }

    #[test]
    fn sensor_snapshot_from_raw_with_values() {
        let mut raw = create_sensor_snapshot_t::default();
        raw.is_left_bumper = 1;
        raw.is_cliff_right = 1;
        raw.voltage = 14.8;
        raw.battery_charge = 2.0;
        raw.battery_capacity = 3.0;
        raw.charging_state = 3; // TrickleCharging
        raw.oi_mode = 2; // Safe
        raw.is_moving_forward = 1;
        raw.num_corrupt_packets = 2;
        raw.total_packets = 1000;

        let snapshot = SensorSnapshot::from(raw);

        assert!(snapshot.bumpers.left);
        assert!(!snapshot.bumpers.right);
        assert!(snapshot.cliffs.right);
        assert!(!snapshot.cliffs.left);
        assert!((snapshot.battery.voltage - 14.8).abs() < f32::EPSILON);
        assert_eq!(snapshot.battery.state, ChargingState::TrickleCharging);
        assert_eq!(snapshot.oi_mode, OiMode::Safe);
        assert!(snapshot.is_moving_forward);
        assert_eq!(snapshot.packet_stats.corrupt, 2);
        assert_eq!(snapshot.packet_stats.total, 1000);
    }

    #[test]
    fn sensor_snapshot_boolean_conversion() {
        let mut raw = create_sensor_snapshot_t::default();
        // Any non-zero value should be true
        raw.is_wall = 255;
        raw.is_virtual_wall = 42;
        raw.is_clean_button = 1;
        raw.is_dock_button = 0;

        let snapshot = SensorSnapshot::from(raw);
        assert!(snapshot.walls.wall);
        assert!(snapshot.walls.virtual_wall);
        assert!(snapshot.buttons.clean);
        assert!(!snapshot.buttons.dock);
    }

    #[test]
    fn sensor_snapshot_ir_chars() {
        let mut raw = create_sensor_snapshot_t::default();
        raw.ir_omni = 143; // SeekDock
        raw.ir_left = 0; // None
        raw.ir_right = 99; // Unknown

        let snapshot = SensorSnapshot::from(raw);
        assert_eq!(snapshot.ir.omni, IrChar::SeekDock);
        assert_eq!(snapshot.ir.left, IrChar::None);
        assert_eq!(snapshot.ir.right, IrChar::Unknown(99));
    }

    #[test]
    fn sensor_snapshot_light_bumper_signals() {
        let mut raw = create_sensor_snapshot_t::default();
        raw.is_light_bumper_left = 1;
        raw.light_signal_left = 2048;
        raw.light_signal_right = 100;

        let snapshot = SensorSnapshot::from(raw);
        assert!(snapshot.light_bumpers.left);
        assert_eq!(snapshot.light_bumpers.signal_left, 2048);
        assert_eq!(snapshot.light_bumpers.signal_right, 100);
    }
}
