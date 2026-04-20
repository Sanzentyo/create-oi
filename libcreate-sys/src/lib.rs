//! Raw FFI bindings to libcreate via our C wrapper.
//!
//! This crate is not intended to be used directly. Use the `libcreate` crate instead.

#![allow(non_camel_case_types)]

use std::os::raw::{c_char, c_float, c_int};

/// Opaque handle to the wrapped C++ `create::Create` instance.
pub enum create_robot_handle_t {}

// Robot model IDs
pub const CREATE_MODEL_ROOMBA_400: c_int = 0;
pub const CREATE_MODEL_CREATE_1: c_int = 1;
pub const CREATE_MODEL_CREATE_2: c_int = 2;

// OI modes
pub const CREATE_MODE_OFF: c_int = 0;
pub const CREATE_MODE_PASSIVE: c_int = 1;
pub const CREATE_MODE_SAFE: c_int = 2;
pub const CREATE_MODE_FULL: c_int = 3;

// Clean modes
pub const CREATE_CLEAN_DEFAULT: c_int = 0;
pub const CREATE_CLEAN_MAX: c_int = 1;
pub const CREATE_CLEAN_SPOT: c_int = 2;

// Return codes
pub const CREATE_OK: c_int = 0;
pub const CREATE_ERROR: c_int = -1;

/// Sensor snapshot — all values captured in a single locked read.
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct create_sensor_snapshot_t {
    // Bumpers & wheeldrops
    pub is_left_bumper: u8,
    pub is_right_bumper: u8,
    pub is_left_wheeldrop: u8,
    pub is_right_wheeldrop: u8,

    // Cliffs
    pub is_cliff_left: u8,
    pub is_cliff_front_left: u8,
    pub is_cliff_front_right: u8,
    pub is_cliff_right: u8,

    // Walls
    pub is_wall: u8,
    pub is_virtual_wall: u8,

    // Light bumpers
    pub is_light_bumper_left: u8,
    pub is_light_bumper_front_left: u8,
    pub is_light_bumper_center_left: u8,
    pub is_light_bumper_center_right: u8,
    pub is_light_bumper_front_right: u8,
    pub is_light_bumper_right: u8,
    pub light_signal_left: u16,
    pub light_signal_front_left: u16,
    pub light_signal_center_left: u16,
    pub light_signal_center_right: u16,
    pub light_signal_front_right: u16,
    pub light_signal_right: u16,

    // Battery
    pub voltage: c_float,
    pub current: c_float,
    pub temperature: i8,
    pub battery_charge: c_float,
    pub battery_capacity: c_float,
    pub charging_state: i32,

    // IR
    pub ir_omni: u8,
    pub ir_left: u8,
    pub ir_right: u8,

    // Dirt
    pub dirt_detect: u8,

    // Buttons
    pub is_clean_button: u8,
    pub is_clock_button: u8,
    pub is_schedule_button: u8,
    pub is_day_button: u8,
    pub is_hour_button: u8,
    pub is_min_button: u8,
    pub is_dock_button: u8,
    pub is_spot_button: u8,

    // Overcurrent
    pub is_wheel_overcurrent: u8,
    pub is_main_brush_overcurrent: u8,
    pub is_side_brush_overcurrent: u8,

    // Odometry
    pub pose_x: c_float,
    pub pose_y: c_float,
    pub pose_yaw: c_float,
    pub vel_x: c_float,
    pub vel_y: c_float,
    pub vel_yaw: c_float,
    pub left_wheel_distance: c_float,
    pub right_wheel_distance: c_float,
    pub measured_left_wheel_vel: c_float,
    pub measured_right_wheel_vel: c_float,
    pub requested_left_wheel_vel: c_float,
    pub requested_right_wheel_vel: c_float,

    // Stasis
    pub is_moving_forward: u8,

    // OI mode
    pub oi_mode: i32,

    // Packet stats
    pub num_corrupt_packets: u64,
    pub total_packets: u64,
}

unsafe extern "C" {
    // Lifecycle
    pub fn create_robot_new(model_id: c_int) -> *mut create_robot_handle_t;
    pub fn create_robot_destroy(handle: *mut create_robot_handle_t);

    // Connection
    pub fn create_robot_connect(
        handle: *mut create_robot_handle_t,
        port: *const c_char,
        baud: c_int,
    ) -> c_int;
    pub fn create_robot_disconnect(handle: *mut create_robot_handle_t);
    pub fn create_robot_connected(handle: *const create_robot_handle_t) -> c_int;

    // Mode
    pub fn create_robot_set_mode(handle: *mut create_robot_handle_t, mode: c_int) -> c_int;
    pub fn create_robot_get_mode(handle: *mut create_robot_handle_t) -> c_int;

    // Cleaning
    pub fn create_robot_clean(handle: *mut create_robot_handle_t, clean_mode: c_int) -> c_int;
    pub fn create_robot_dock(handle: *const create_robot_handle_t) -> c_int;

    // Driving
    pub fn create_robot_drive(
        handle: *mut create_robot_handle_t,
        x_vel: c_float,
        angular_vel: c_float,
    ) -> c_int;
    pub fn create_robot_drive_radius(
        handle: *mut create_robot_handle_t,
        velocity: c_float,
        radius: c_float,
    ) -> c_int;
    pub fn create_robot_drive_wheels(
        handle: *mut create_robot_handle_t,
        left: c_float,
        right: c_float,
    ) -> c_int;
    pub fn create_robot_drive_wheels_pwm(
        handle: *mut create_robot_handle_t,
        left: c_float,
        right: c_float,
    ) -> c_int;

    // Motors
    pub fn create_robot_set_side_motor(handle: *mut create_robot_handle_t, power: c_float)
    -> c_int;
    pub fn create_robot_set_main_motor(handle: *mut create_robot_handle_t, power: c_float)
    -> c_int;
    pub fn create_robot_set_vacuum_motor(
        handle: *mut create_robot_handle_t,
        power: c_float,
    ) -> c_int;
    pub fn create_robot_set_all_motors(
        handle: *mut create_robot_handle_t,
        main_power: c_float,
        side_power: c_float,
        vacuum_power: c_float,
    ) -> c_int;

    // LEDs
    pub fn create_robot_enable_debris_led(handle: *mut create_robot_handle_t, enable: u8) -> c_int;
    pub fn create_robot_enable_spot_led(handle: *mut create_robot_handle_t, enable: u8) -> c_int;
    pub fn create_robot_enable_dock_led(handle: *mut create_robot_handle_t, enable: u8) -> c_int;
    pub fn create_robot_enable_check_robot_led(
        handle: *mut create_robot_handle_t,
        enable: u8,
    ) -> c_int;
    pub fn create_robot_set_power_led(
        handle: *mut create_robot_handle_t,
        power: u8,
        intensity: u8,
    ) -> c_int;

    // Display
    pub fn create_robot_set_digits_ascii(
        handle: *const create_robot_handle_t,
        d1: u8,
        d2: u8,
        d3: u8,
        d4: u8,
    ) -> c_int;

    // Songs
    pub fn create_robot_define_song(
        handle: *const create_robot_handle_t,
        song_number: u8,
        song_length: u8,
        notes: *const u8,
        durations: *const c_float,
    ) -> c_int;
    pub fn create_robot_play_song(handle: *const create_robot_handle_t, song_number: u8) -> c_int;

    // Date
    pub fn create_robot_set_date(
        handle: *const create_robot_handle_t,
        day_of_week: c_int,
        hour: u8,
        minute: u8,
    ) -> c_int;

    // Sensors
    pub fn create_robot_get_sensors(
        handle: *mut create_robot_handle_t,
        out: *mut create_sensor_snapshot_t,
    ) -> c_int;

    // Mode report workaround
    pub fn create_robot_set_mode_report_workaround(handle: *mut create_robot_handle_t, enable: u8);
    pub fn create_robot_get_mode_report_workaround(handle: *const create_robot_handle_t) -> c_int;
}
