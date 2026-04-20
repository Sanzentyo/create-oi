/**
 * C wrapper for libcreate C++ API.
 *
 * Provides an opaque handle and extern "C" functions for FFI consumption.
 * All functions include exception handling to prevent C++ exceptions from
 * crossing the FFI boundary.
 */

#ifndef LIBCREATE_WRAPPER_H
#define LIBCREATE_WRAPPER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Opaque handle to a wrapped Create robot instance. */
typedef struct create_robot_handle create_robot_handle_t;

/** Sensor snapshot — all values captured atomically. */
typedef struct {
    /* Bumpers & wheeldrops */
    uint8_t is_left_bumper;
    uint8_t is_right_bumper;
    uint8_t is_left_wheeldrop;
    uint8_t is_right_wheeldrop;

    /* Cliffs */
    uint8_t is_cliff_left;
    uint8_t is_cliff_front_left;
    uint8_t is_cliff_front_right;
    uint8_t is_cliff_right;

    /* Walls */
    uint8_t is_wall;
    uint8_t is_virtual_wall;

    /* Light bumpers (Create 2 only) */
    uint8_t is_light_bumper_left;
    uint8_t is_light_bumper_front_left;
    uint8_t is_light_bumper_center_left;
    uint8_t is_light_bumper_center_right;
    uint8_t is_light_bumper_front_right;
    uint8_t is_light_bumper_right;
    uint16_t light_signal_left;
    uint16_t light_signal_front_left;
    uint16_t light_signal_center_left;
    uint16_t light_signal_center_right;
    uint16_t light_signal_front_right;
    uint16_t light_signal_right;

    /* Battery */
    float voltage;
    float current;
    int8_t temperature;
    float battery_charge;
    float battery_capacity;
    int32_t charging_state;

    /* IR */
    uint8_t ir_omni;
    uint8_t ir_left;
    uint8_t ir_right;

    /* Dirt */
    uint8_t dirt_detect;

    /* Buttons */
    uint8_t is_clean_button;
    uint8_t is_clock_button;
    uint8_t is_schedule_button;
    uint8_t is_day_button;
    uint8_t is_hour_button;
    uint8_t is_min_button;
    uint8_t is_dock_button;
    uint8_t is_spot_button;

    /* Overcurrent */
    uint8_t is_wheel_overcurrent;
    uint8_t is_main_brush_overcurrent;
    uint8_t is_side_brush_overcurrent;

    /* Odometry */
    float pose_x;
    float pose_y;
    float pose_yaw;
    float vel_x;
    float vel_y;
    float vel_yaw;
    float left_wheel_distance;
    float right_wheel_distance;
    float measured_left_wheel_vel;
    float measured_right_wheel_vel;
    float requested_left_wheel_vel;
    float requested_right_wheel_vel;

    /* Stasis */
    uint8_t is_moving_forward;

    /* OI Mode */
    int32_t oi_mode;

    /* Packet stats */
    uint64_t num_corrupt_packets;
    uint64_t total_packets;
} create_sensor_snapshot_t;

/* Robot model IDs (must match RobotModel static instances) */
#define CREATE_MODEL_ROOMBA_400 0
#define CREATE_MODEL_CREATE_1   1
#define CREATE_MODEL_CREATE_2   2

/* OI Modes */
#define CREATE_MODE_OFF       0
#define CREATE_MODE_PASSIVE   1
#define CREATE_MODE_SAFE      2
#define CREATE_MODE_FULL      3

/* Clean modes */
#define CREATE_CLEAN_DEFAULT  0
#define CREATE_CLEAN_MAX      1
#define CREATE_CLEAN_SPOT     2

/* Return codes */
#define CREATE_OK       0
#define CREATE_ERROR   -1

/* Lifecycle */
create_robot_handle_t* create_robot_new(int model_id);
void create_robot_destroy(create_robot_handle_t* handle);

/* Connection */
int create_robot_connect(create_robot_handle_t* handle, const char* port, int baud);
void create_robot_disconnect(create_robot_handle_t* handle);
int create_robot_connected(const create_robot_handle_t* handle);

/* Mode */
int create_robot_set_mode(create_robot_handle_t* handle, int mode);
int create_robot_get_mode(create_robot_handle_t* handle);

/* Cleaning */
int create_robot_clean(create_robot_handle_t* handle, int clean_mode);
int create_robot_dock(const create_robot_handle_t* handle);

/* Driving */
int create_robot_drive(create_robot_handle_t* handle, float x_vel, float angular_vel);
int create_robot_drive_radius(create_robot_handle_t* handle, float velocity, float radius);
int create_robot_drive_wheels(create_robot_handle_t* handle, float left, float right);
int create_robot_drive_wheels_pwm(create_robot_handle_t* handle, float left, float right);

/* Motors */
int create_robot_set_side_motor(create_robot_handle_t* handle, float power);
int create_robot_set_main_motor(create_robot_handle_t* handle, float power);
int create_robot_set_vacuum_motor(create_robot_handle_t* handle, float power);
int create_robot_set_all_motors(create_robot_handle_t* handle, float main_power, float side_power, float vacuum_power);

/* LEDs */
int create_robot_enable_debris_led(create_robot_handle_t* handle, uint8_t enable);
int create_robot_enable_spot_led(create_robot_handle_t* handle, uint8_t enable);
int create_robot_enable_dock_led(create_robot_handle_t* handle, uint8_t enable);
int create_robot_enable_check_robot_led(create_robot_handle_t* handle, uint8_t enable);
int create_robot_set_power_led(create_robot_handle_t* handle, uint8_t power, uint8_t intensity);

/* Display */
int create_robot_set_digits_ascii(const create_robot_handle_t* handle,
                                  uint8_t d1, uint8_t d2, uint8_t d3, uint8_t d4);

/* Songs */
int create_robot_define_song(const create_robot_handle_t* handle,
                             uint8_t song_number, uint8_t song_length,
                             const uint8_t* notes, const float* durations);
int create_robot_play_song(const create_robot_handle_t* handle, uint8_t song_number);

/* Date */
int create_robot_set_date(const create_robot_handle_t* handle,
                          int day_of_week, uint8_t hour, uint8_t minute);

/* Sensor snapshot (atomic read of all sensors) */
int create_robot_get_sensors(create_robot_handle_t* handle, create_sensor_snapshot_t* out);

/* Mode report workaround */
void create_robot_set_mode_report_workaround(create_robot_handle_t* handle, uint8_t enable);
int create_robot_get_mode_report_workaround(const create_robot_handle_t* handle);

#ifdef __cplusplus
}
#endif

#endif /* LIBCREATE_WRAPPER_H */
