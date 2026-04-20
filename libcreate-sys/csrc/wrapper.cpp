/**
 * C wrapper implementation for libcreate C++ API.
 *
 * Every function is wrapped in try/catch to prevent C++ exceptions
 * from crossing the FFI boundary.
 */

#include "wrapper.h"
#include "create/create.h"

#include <mutex>
#include <memory>
#include <cstring>

struct create_robot_handle {
    std::unique_ptr<create::Create> robot;
    std::mutex mtx;

    explicit create_robot_handle(create::RobotModel model)
        : robot(std::make_unique<create::Create>(model, false)) {}
};

static create::RobotModel model_from_id(int id) {
    switch (id) {
        case CREATE_MODEL_ROOMBA_400: return create::RobotModel::ROOMBA_400;
        case CREATE_MODEL_CREATE_1:   return create::RobotModel::CREATE_1;
        case CREATE_MODEL_CREATE_2:
        default:                      return create::RobotModel::CREATE_2;
    }
}

static create::CleanMode clean_mode_from_id(int id) {
    switch (id) {
        case CREATE_CLEAN_MAX:  return create::CLEAN_MAX;
        case CREATE_CLEAN_SPOT: return create::CLEAN_SPOT;
        default:                return create::CLEAN_DEFAULT;
    }
}

static create::CreateMode create_mode_from_id(int id) {
    switch (id) {
        case CREATE_MODE_OFF:     return create::MODE_OFF;
        case CREATE_MODE_PASSIVE: return create::MODE_PASSIVE;
        case CREATE_MODE_SAFE:    return create::MODE_SAFE;
        case CREATE_MODE_FULL:    return create::MODE_FULL;
        default:                  return create::MODE_OFF;
    }
}

static create::DayOfWeek day_from_id(int id) {
    switch (id) {
        case 0: return create::SUN;
        case 1: return create::MON;
        case 2: return create::TUE;
        case 3: return create::WED;
        case 4: return create::THU;
        case 5: return create::FRI;
        case 6: return create::SAT;
        default: return create::SUN;
    }
}

/* Lifecycle */

create_robot_handle_t* create_robot_new(int model_id) {
    try {
        return new create_robot_handle(model_from_id(model_id));
    } catch (...) {
        return nullptr;
    }
}

void create_robot_destroy(create_robot_handle_t* handle) {
    try {
        delete handle;
    } catch (...) {
        // Suppress
    }
}

/* Connection */

int create_robot_connect(create_robot_handle_t* handle, const char* port, int baud) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        return handle->robot->connect(std::string(port), baud) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

void create_robot_disconnect(create_robot_handle_t* handle) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        handle->robot->disconnect();
    } catch (...) {
        // Suppress
    }
}

int create_robot_connected(const create_robot_handle_t* handle) {
    try {
        return handle->robot->connected() ? 1 : 0;
    } catch (...) {
        return 0;
    }
}

/* Mode */

int create_robot_set_mode(create_robot_handle_t* handle, int mode) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        return handle->robot->setMode(create_mode_from_id(mode)) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

int create_robot_get_mode(create_robot_handle_t* handle) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        auto mode = handle->robot->getMode();
        switch (mode) {
            case create::MODE_OFF:     return CREATE_MODE_OFF;
            case create::MODE_PASSIVE: return CREATE_MODE_PASSIVE;
            case create::MODE_SAFE:    return CREATE_MODE_SAFE;
            case create::MODE_FULL:    return CREATE_MODE_FULL;
            default:                   return -1;
        }
    } catch (...) {
        return -1;
    }
}

/* Cleaning */

int create_robot_clean(create_robot_handle_t* handle, int clean_mode) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        return handle->robot->clean(clean_mode_from_id(clean_mode)) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

int create_robot_dock(const create_robot_handle_t* handle) {
    try {
        return handle->robot->dock() ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

/* Driving */

int create_robot_drive(create_robot_handle_t* handle, float x_vel, float angular_vel) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        return handle->robot->drive(x_vel, angular_vel) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

int create_robot_drive_radius(create_robot_handle_t* handle, float velocity, float radius) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        return handle->robot->driveRadius(velocity, radius) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

int create_robot_drive_wheels(create_robot_handle_t* handle, float left, float right) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        return handle->robot->driveWheels(left, right) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

int create_robot_drive_wheels_pwm(create_robot_handle_t* handle, float left, float right) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        return handle->robot->driveWheelsPwm(left, right) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

/* Motors */

int create_robot_set_side_motor(create_robot_handle_t* handle, float power) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        return handle->robot->setSideMotor(power) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

int create_robot_set_main_motor(create_robot_handle_t* handle, float power) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        return handle->robot->setMainMotor(power) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

int create_robot_set_vacuum_motor(create_robot_handle_t* handle, float power) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        return handle->robot->setVacuumMotor(power) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

int create_robot_set_all_motors(create_robot_handle_t* handle, float main_power, float side_power, float vacuum_power) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        return handle->robot->setAllMotors(main_power, side_power, vacuum_power) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

/* LEDs */

int create_robot_enable_debris_led(create_robot_handle_t* handle, uint8_t enable) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        return handle->robot->enableDebrisLED(enable != 0) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

int create_robot_enable_spot_led(create_robot_handle_t* handle, uint8_t enable) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        return handle->robot->enableSpotLED(enable != 0) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

int create_robot_enable_dock_led(create_robot_handle_t* handle, uint8_t enable) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        return handle->robot->enableDockLED(enable != 0) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

int create_robot_enable_check_robot_led(create_robot_handle_t* handle, uint8_t enable) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        return handle->robot->enableCheckRobotLED(enable != 0) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

int create_robot_set_power_led(create_robot_handle_t* handle, uint8_t power, uint8_t intensity) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        return handle->robot->setPowerLED(power, intensity) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

/* Display */

int create_robot_set_digits_ascii(const create_robot_handle_t* handle,
                                  uint8_t d1, uint8_t d2, uint8_t d3, uint8_t d4) {
    try {
        return handle->robot->setDigitsASCII(d1, d2, d3, d4) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

/* Songs */

int create_robot_define_song(const create_robot_handle_t* handle,
                             uint8_t song_number, uint8_t song_length,
                             const uint8_t* notes, const float* durations) {
    try {
        return handle->robot->defineSong(song_number, song_length, notes, durations) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

int create_robot_play_song(const create_robot_handle_t* handle, uint8_t song_number) {
    try {
        return handle->robot->playSong(song_number) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

/* Date */

int create_robot_set_date(const create_robot_handle_t* handle,
                          int day_of_week, uint8_t hour, uint8_t minute) {
    try {
        return handle->robot->setDate(day_from_id(day_of_week), hour, minute) ? CREATE_OK : CREATE_ERROR;
    } catch (...) {
        return CREATE_ERROR;
    }
}

/* Sensor snapshot */

int create_robot_get_sensors(create_robot_handle_t* handle, create_sensor_snapshot_t* out) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        std::memset(out, 0, sizeof(create_sensor_snapshot_t));

        auto& r = *handle->robot;

        out->is_left_bumper = r.isLeftBumper() ? 1 : 0;
        out->is_right_bumper = r.isRightBumper() ? 1 : 0;
        out->is_left_wheeldrop = r.isLeftWheeldrop() ? 1 : 0;
        out->is_right_wheeldrop = r.isRightWheeldrop() ? 1 : 0;

        out->is_cliff_left = r.isCliffLeft() ? 1 : 0;
        out->is_cliff_front_left = r.isCliffFrontLeft() ? 1 : 0;
        out->is_cliff_front_right = r.isCliffFrontRight() ? 1 : 0;
        out->is_cliff_right = r.isCliffRight() ? 1 : 0;

        out->is_wall = r.isWall() ? 1 : 0;
        out->is_virtual_wall = r.isVirtualWall() ? 1 : 0;

        out->is_light_bumper_left = r.isLightBumperLeft() ? 1 : 0;
        out->is_light_bumper_front_left = r.isLightBumperFrontLeft() ? 1 : 0;
        out->is_light_bumper_center_left = r.isLightBumperCenterLeft() ? 1 : 0;
        out->is_light_bumper_center_right = r.isLightBumperCenterRight() ? 1 : 0;
        out->is_light_bumper_front_right = r.isLightBumperFrontRight() ? 1 : 0;
        out->is_light_bumper_right = r.isLightBumperRight() ? 1 : 0;

        out->light_signal_left = r.getLightSignalLeft();
        out->light_signal_front_left = r.getLightSignalFrontLeft();
        out->light_signal_center_left = r.getLightSignalCenterLeft();
        out->light_signal_center_right = r.getLightSignalCenterRight();
        out->light_signal_front_right = r.getLightSignalFrontRight();
        out->light_signal_right = r.getLightSignalRight();

        out->voltage = r.getVoltage();
        out->current = r.getCurrent();
        out->temperature = r.getTemperature();
        out->battery_charge = r.getBatteryCharge();
        out->battery_capacity = r.getBatteryCapacity();
        out->charging_state = static_cast<int32_t>(r.getChargingState());

        out->ir_omni = r.getIROmni();
        out->ir_left = r.getIRLeft();
        out->ir_right = r.getIRRight();

        out->dirt_detect = r.getDirtDetect();

        out->is_clean_button = r.isCleanButtonPressed() ? 1 : 0;
        out->is_clock_button = r.isClockButtonPressed() ? 1 : 0;
        out->is_schedule_button = r.isScheduleButtonPressed() ? 1 : 0;
        out->is_day_button = r.isDayButtonPressed() ? 1 : 0;
        out->is_hour_button = r.isHourButtonPressed() ? 1 : 0;
        out->is_min_button = r.isMinButtonPressed() ? 1 : 0;
        out->is_dock_button = r.isDockButtonPressed() ? 1 : 0;
        out->is_spot_button = r.isSpotButtonPressed() ? 1 : 0;

        out->is_wheel_overcurrent = r.isWheelOvercurrent() ? 1 : 0;
        out->is_main_brush_overcurrent = r.isMainBrushOvercurrent() ? 1 : 0;
        out->is_side_brush_overcurrent = r.isSideBrushOvercurrent() ? 1 : 0;

        auto pose = r.getPose();
        out->pose_x = pose.x;
        out->pose_y = pose.y;
        out->pose_yaw = pose.yaw;

        auto vel = r.getVel();
        out->vel_x = vel.x;
        out->vel_y = vel.y;
        out->vel_yaw = vel.yaw;

        out->left_wheel_distance = r.getLeftWheelDistance();
        out->right_wheel_distance = r.getRightWheelDistance();
        out->measured_left_wheel_vel = r.getMeasuredLeftWheelVel();
        out->measured_right_wheel_vel = r.getMeasuredRightWheelVel();
        out->requested_left_wheel_vel = r.getRequestedLeftWheelVel();
        out->requested_right_wheel_vel = r.getRequestedRightWheelVel();

        out->is_moving_forward = r.isMovingForward() ? 1 : 0;

        out->oi_mode = static_cast<int32_t>(r.getMode());

        out->num_corrupt_packets = r.getNumCorruptPackets();
        out->total_packets = r.getTotalPackets();

        return CREATE_OK;
    } catch (...) {
        return CREATE_ERROR;
    }
}

/* Mode report workaround */

void create_robot_set_mode_report_workaround(create_robot_handle_t* handle, uint8_t enable) {
    try {
        std::lock_guard<std::mutex> lock(handle->mtx);
        handle->robot->setModeReportWorkaround(enable != 0);
    } catch (...) {
        // Suppress
    }
}

int create_robot_get_mode_report_workaround(const create_robot_handle_t* handle) {
    try {
        return handle->robot->getModeReportWorkaround() ? 1 : 0;
    } catch (...) {
        return 0;
    }
}
