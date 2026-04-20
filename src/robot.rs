//! TypeState-based robot API.
//!
//! `Robot<M>` represents a connection to an iRobot Create / Roomba in OI mode
//! `M`. Mode transitions consume `self` and return the robot in the new mode,
//! preventing use of commands that are invalid for the current mode at compile
//! time.
//!
//! # Example
//!
//! ```no_run
//! use libcreate::{Robot, RobotModel, mode};
//!
//! let robot = Robot::new(RobotModel::Create2)?;
//! let robot = robot.connect("/dev/ttyUSB0", 115200)?;
//! let mut robot = robot.into_safe()?;
//! robot.drive(0.2.try_into()?, 0.0.try_into()?)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::ffi::CString;
use std::marker::PhantomData;
use std::ptr::NonNull;

use libcreate_sys as ffi;

use crate::error::{Error, TransitionError};
use crate::mode::{Actuatable, Full, Mode, Off, Passive, Safe, SensorReadable};
use crate::sensor::SensorSnapshot;
use crate::types::{
    AngularVelocity, CleanMode, DayOfWeek, LedIntensity, MotorPower, OiMode, PowerLedColor, Radius,
    RobotModel, SongNumber, Velocity,
};

/// An iRobot Create / Roomba robot connection parameterized by its OI mode.
///
/// The type parameter `M` is one of [`Off`], [`Passive`], [`Safe`], or
/// [`Full`], encoding the current Open Interface mode at the type level.
///
/// `Robot` is intentionally `!Send` and `!Sync` because the underlying C++
/// library uses internal threads that are not safe to access from multiple
/// Rust threads.
pub struct Robot<M: Mode> {
    handle: NonNull<ffi::create_robot_handle_t>,
    model: RobotModel,
    _mode: PhantomData<M>,
    /// Make `Robot` !Send + !Sync.
    _not_send_sync: PhantomData<*const ()>,
}

// Safety: Robot must NOT be Send or Sync. The PhantomData<*const ()> ensures
// this, but let's be explicit:
static_assertions::assert_not_impl_any!(Robot<Off>: Send, Sync);
static_assertions::assert_not_impl_any!(Robot<Passive>: Send, Sync);
static_assertions::assert_not_impl_any!(Robot<Safe>: Send, Sync);
static_assertions::assert_not_impl_any!(Robot<Full>: Send, Sync);

impl<M: Mode> Drop for Robot<M> {
    fn drop(&mut self) {
        unsafe {
            // Disconnect if connected, then destroy the handle.
            ffi::create_robot_disconnect(self.handle.as_ptr());
            ffi::create_robot_destroy(self.handle.as_ptr());
        }
    }
}

impl<M: Mode> std::fmt::Debug for Robot<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Robot")
            .field("mode", &M::NAME)
            .field("model", &self.model)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Off mode — construction and connection
// ---------------------------------------------------------------------------

impl Robot<Off> {
    /// Create a new robot handle (not yet connected).
    ///
    /// This allocates the internal C++ object but does not open a serial
    /// connection. Call [`connect`](Robot::connect) to establish communication.
    pub fn new(model: RobotModel) -> Result<Self, Error> {
        let ptr = unsafe { ffi::create_robot_new(model.to_raw()) };
        let handle = NonNull::new(ptr).ok_or(Error::HandleCreationFailed)?;
        Ok(Self {
            handle,
            model,
            _mode: PhantomData,
            _not_send_sync: PhantomData,
        })
    }

    /// Connect to the robot over a serial port and enter Passive mode.
    ///
    /// The OI specification requires that connection always starts in Passive
    /// mode. On failure, the robot is returned in Off mode for reuse.
    pub fn connect(self, port: &str, baud: u32) -> Result<Robot<Passive>, TransitionError<Off>> {
        let c_port = match CString::new(port) {
            Ok(s) => s,
            Err(_) => {
                return Err(TransitionError {
                    error: Error::ConnectionFailed {
                        port: port.to_owned(),
                    },
                    robot: self,
                });
            }
        };

        let handle = self.handle;
        let model = self.model;
        std::mem::forget(self);

        let rc =
            unsafe { ffi::create_robot_connect(handle.as_ptr(), c_port.as_ptr(), baud as i32) };

        if rc != ffi::CREATE_OK {
            let robot = Robot::<Off> {
                handle,
                model,
                _mode: PhantomData,
                _not_send_sync: PhantomData,
            };
            return Err(TransitionError {
                error: Error::ConnectionFailed {
                    port: port.to_owned(),
                },
                robot,
            });
        }

        // Enable mode-report workaround for more reliable mode tracking.
        unsafe {
            ffi::create_robot_set_mode_report_workaround(handle.as_ptr(), 1);
        }

        Ok(Robot::<Passive> {
            handle,
            model,
            _mode: PhantomData,
            _not_send_sync: PhantomData,
        })
    }
}

// ---------------------------------------------------------------------------
// Shared methods for all connected modes
// ---------------------------------------------------------------------------

/// Methods available in any connected mode.
impl<M: SensorReadable> Robot<M> {
    /// Read a complete sensor snapshot atomically.
    pub fn sensors(&mut self) -> Result<SensorSnapshot, Error> {
        let mut raw = libcreate_sys::create_sensor_snapshot_t::default();
        let rc = unsafe { ffi::create_robot_get_sensors(self.handle.as_ptr(), &mut raw) };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(SensorSnapshot::from(raw))
    }

    /// Query the actual OI mode reported by the hardware.
    ///
    /// This does a sensor read and returns the hardware-reported mode. Use
    /// this to detect asynchronous mode changes (e.g., bump in Safe mode
    /// causing a transition to Passive).
    pub fn actual_mode(&mut self) -> Result<OiMode, Error> {
        let raw_mode = unsafe { ffi::create_robot_get_mode(self.handle.as_ptr()) };
        if raw_mode < 0 {
            return Err(Error::CommandFailed);
        }
        Ok(OiMode::from_raw(raw_mode))
    }

    /// Verify that the hardware mode matches this robot's typestate.
    ///
    /// Returns `Ok(())` if they match, or `Err(Error::ModeMismatch)` if
    /// the hardware has autonomously changed modes.
    pub fn verify_mode(&mut self) -> Result<(), Error> {
        let actual = self.actual_mode()?;
        if actual != M::OI_MODE {
            return Err(Error::ModeMismatch {
                expected: M::NAME,
                actual: actual.name(),
            });
        }
        Ok(())
    }

    /// Check if the robot is currently connected.
    pub fn is_connected(&self) -> bool {
        unsafe { ffi::create_robot_connected(self.handle.as_ptr()) != 0 }
    }

    /// Get the robot model.
    pub fn model(&self) -> RobotModel {
        self.model
    }
}

// ---------------------------------------------------------------------------
// Passive mode — transitions and cleaning
// ---------------------------------------------------------------------------

impl Robot<Passive> {
    /// Transition to Safe mode.
    pub fn into_safe(self) -> Result<Robot<Safe>, TransitionError<Passive>> {
        self.transition_to::<Safe>()
    }

    /// Transition to Full mode.
    pub fn into_full(self) -> Result<Robot<Full>, TransitionError<Passive>> {
        self.transition_to::<Full>()
    }

    /// Start a cleaning cycle. The robot stays in Passive mode during cleaning.
    pub fn clean(&mut self, mode: CleanMode) -> Result<(), Error> {
        let rc = unsafe { ffi::create_robot_clean(self.handle.as_ptr(), mode.to_raw()) };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Send the robot to seek its dock.
    pub fn dock(&self) -> Result<(), Error> {
        let rc = unsafe { ffi::create_robot_dock(self.handle.as_ptr()) };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Disconnect and return to Off mode.
    pub fn disconnect(self) -> Robot<Off> {
        let handle = self.handle;
        let model = self.model;
        std::mem::forget(self);
        unsafe {
            ffi::create_robot_disconnect(handle.as_ptr());
        }
        Robot::<Off> {
            handle,
            model,
            _mode: PhantomData,
            _not_send_sync: PhantomData,
        }
    }
}

// ---------------------------------------------------------------------------
// Safe mode — transitions
// ---------------------------------------------------------------------------

impl Robot<Safe> {
    /// Transition to Passive mode.
    pub fn into_passive(self) -> Result<Robot<Passive>, TransitionError<Safe>> {
        self.transition_to::<Passive>()
    }

    /// Transition to Full mode.
    pub fn into_full(self) -> Result<Robot<Full>, TransitionError<Safe>> {
        self.transition_to::<Full>()
    }

    /// Disconnect and return to Off mode.
    pub fn disconnect(self) -> Robot<Off> {
        let handle = self.handle;
        let model = self.model;
        std::mem::forget(self);
        unsafe {
            ffi::create_robot_disconnect(handle.as_ptr());
        }
        Robot::<Off> {
            handle,
            model,
            _mode: PhantomData,
            _not_send_sync: PhantomData,
        }
    }
}

// ---------------------------------------------------------------------------
// Full mode — transitions
// ---------------------------------------------------------------------------

impl Robot<Full> {
    /// Transition to Passive mode.
    pub fn into_passive(self) -> Result<Robot<Passive>, TransitionError<Full>> {
        self.transition_to::<Passive>()
    }

    /// Transition to Safe mode.
    pub fn into_safe(self) -> Result<Robot<Safe>, TransitionError<Full>> {
        self.transition_to::<Safe>()
    }

    /// Disconnect and return to Off mode.
    pub fn disconnect(self) -> Robot<Off> {
        let handle = self.handle;
        let model = self.model;
        std::mem::forget(self);
        unsafe {
            ffi::create_robot_disconnect(handle.as_ptr());
        }
        Robot::<Off> {
            handle,
            model,
            _mode: PhantomData,
            _not_send_sync: PhantomData,
        }
    }
}

// ---------------------------------------------------------------------------
// Actuator commands — only available in Safe or Full modes
// ---------------------------------------------------------------------------

impl<M: Actuatable> Robot<M> {
    /// Drive with linear velocity and angular velocity.
    pub fn drive(&mut self, velocity: Velocity, angular: AngularVelocity) -> Result<(), Error> {
        let rc =
            unsafe { ffi::create_robot_drive(self.handle.as_ptr(), velocity.get(), angular.get()) };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Drive along an arc with the given velocity and turning radius.
    pub fn drive_radius(&mut self, velocity: Velocity, radius: Radius) -> Result<(), Error> {
        let rc = unsafe {
            ffi::create_robot_drive_radius(self.handle.as_ptr(), velocity.get(), radius.get())
        };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Set individual wheel velocities.
    pub fn drive_wheels(&mut self, left: Velocity, right: Velocity) -> Result<(), Error> {
        let rc = unsafe {
            ffi::create_robot_drive_wheels(self.handle.as_ptr(), left.get(), right.get())
        };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Set individual wheel velocities using PWM.
    pub fn drive_wheels_pwm(&mut self, left: MotorPower, right: MotorPower) -> Result<(), Error> {
        let rc = unsafe {
            ffi::create_robot_drive_wheels_pwm(self.handle.as_ptr(), left.get(), right.get())
        };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Stop all wheel motion.
    pub fn stop(&mut self) -> Result<(), Error> {
        self.drive(Velocity::ZERO, AngularVelocity::ZERO)
    }

    /// Set the side brush motor power.
    pub fn set_side_motor(&mut self, power: MotorPower) -> Result<(), Error> {
        let rc = unsafe { ffi::create_robot_set_side_motor(self.handle.as_ptr(), power.get()) };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Set the main brush motor power.
    pub fn set_main_motor(&mut self, power: MotorPower) -> Result<(), Error> {
        let rc = unsafe { ffi::create_robot_set_main_motor(self.handle.as_ptr(), power.get()) };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Set the vacuum motor power.
    pub fn set_vacuum_motor(&mut self, power: MotorPower) -> Result<(), Error> {
        let rc = unsafe { ffi::create_robot_set_vacuum_motor(self.handle.as_ptr(), power.get()) };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Set all three motor powers at once.
    pub fn set_all_motors(
        &mut self,
        main: MotorPower,
        side: MotorPower,
        vacuum: MotorPower,
    ) -> Result<(), Error> {
        let rc = unsafe {
            ffi::create_robot_set_all_motors(
                self.handle.as_ptr(),
                main.get(),
                side.get(),
                vacuum.get(),
            )
        };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Enable or disable the debris LED.
    pub fn set_debris_led(&mut self, enable: bool) -> Result<(), Error> {
        let rc =
            unsafe { ffi::create_robot_enable_debris_led(self.handle.as_ptr(), u8::from(enable)) };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Enable or disable the spot LED.
    pub fn set_spot_led(&mut self, enable: bool) -> Result<(), Error> {
        let rc =
            unsafe { ffi::create_robot_enable_spot_led(self.handle.as_ptr(), u8::from(enable)) };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Enable or disable the dock LED.
    pub fn set_dock_led(&mut self, enable: bool) -> Result<(), Error> {
        let rc =
            unsafe { ffi::create_robot_enable_dock_led(self.handle.as_ptr(), u8::from(enable)) };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Enable or disable the "check robot" LED.
    pub fn set_check_robot_led(&mut self, enable: bool) -> Result<(), Error> {
        let rc = unsafe {
            ffi::create_robot_enable_check_robot_led(self.handle.as_ptr(), u8::from(enable))
        };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Set the power LED color and intensity.
    pub fn set_power_led(
        &mut self,
        color: PowerLedColor,
        intensity: LedIntensity,
    ) -> Result<(), Error> {
        let rc = unsafe {
            ffi::create_robot_set_power_led(self.handle.as_ptr(), color.get(), intensity.get())
        };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Display four ASCII characters on the 7-segment display (Create 2).
    pub fn set_digits_ascii(&self, d1: u8, d2: u8, d3: u8, d4: u8) -> Result<(), Error> {
        let rc =
            unsafe { ffi::create_robot_set_digits_ascii(self.handle.as_ptr(), d1, d2, d3, d4) };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Define a song in a song slot.
    ///
    /// `notes` and `durations` must have the same length (max 16).
    pub fn define_song(
        &self,
        number: SongNumber,
        notes: &[u8],
        durations: &[f32],
    ) -> Result<(), Error> {
        if notes.len() != durations.len() || notes.len() > 16 {
            return Err(Error::CommandFailed);
        }
        let rc = unsafe {
            ffi::create_robot_define_song(
                self.handle.as_ptr(),
                number.get(),
                notes.len() as u8,
                notes.as_ptr(),
                durations.as_ptr(),
            )
        };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Play a previously defined song.
    pub fn play_song(&self, number: SongNumber) -> Result<(), Error> {
        let rc = unsafe { ffi::create_robot_play_song(self.handle.as_ptr(), number.get()) };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }

    /// Set the robot's internal clock.
    pub fn set_date(&self, day: DayOfWeek, hour: u8, minute: u8) -> Result<(), Error> {
        if hour > 23 || minute > 59 {
            return Err(Error::OutOfRange {
                value: if hour > 23 {
                    hour as f32
                } else {
                    minute as f32
                },
                min: 0.0,
                max: if hour > 23 { 23.0 } else { 59.0 },
            });
        }
        let rc =
            unsafe { ffi::create_robot_set_date(self.handle.as_ptr(), day.to_raw(), hour, minute) };
        if rc != ffi::CREATE_OK {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

impl<M: Mode> Robot<M> {
    /// Generic mode transition. Sends the FFI set_mode command and transmutes.
    fn transition_to<Target: Mode>(self) -> Result<Robot<Target>, TransitionError<M>> {
        let handle = self.handle;
        let model = self.model;
        std::mem::forget(self);

        let rc = unsafe { ffi::create_robot_set_mode(handle.as_ptr(), Target::RAW) };

        if rc != ffi::CREATE_OK {
            // Reconstruct original mode for recovery.
            let robot = Robot::<M> {
                handle,
                model,
                _mode: PhantomData,
                _not_send_sync: PhantomData,
            };
            return Err(TransitionError {
                error: Error::CommandFailed,
                robot,
            });
        }

        Ok(Robot::<Target> {
            handle,
            model,
            _mode: PhantomData,
            _not_send_sync: PhantomData,
        })
    }
}
