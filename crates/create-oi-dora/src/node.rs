//! Core node logic for the dora-rs Create driver.
//!
//! The driver follows dora-rs conventions:
//! - Timer-driven sensor polling (input: `tick`)
//! - Command-driven actuation (input: `motor_cmd`)
//! - Sensor outputs as Apache Arrow arrays

use create_oi::prelude::*;
use create_oi::transport::Transport;
use std::marker::PhantomData;

/// Configuration for the Create dora-rs driver node.
#[derive(Debug, Clone)]
pub struct CreateNodeConfig {
    /// Serial port path (e.g. `/dev/ttyUSB0`).
    pub port: String,
    /// Robot model.
    pub model: RobotModel,
    /// Sensor packet IDs to poll on each tick.
    pub sensor_ids: Vec<u8>,
}

impl Default for CreateNodeConfig {
    fn default() -> Self {
        Self {
            port: "/dev/ttyUSB0".into(),
            model: RobotModel::Create2,
            // Default: bumps+wheeldrops(7), wall(8), cliff sensors(9-12),
            // voltage(22), current(23), temperature(24), charge(25), capacity(26)
            sensor_ids: vec![7, 8, 9, 10, 11, 12, 22, 23, 24, 25, 26],
        }
    }
}

/// The Create driver node state machine.
///
/// Manages the Create lifecycle (connect → start → safe mode → poll sensors).
#[derive(Debug)]
pub struct CreateNode<T: Transport> {
    config: CreateNodeConfig,
    _transport: PhantomData<T>,
}

impl<T: Transport> CreateNode<T> {
    /// Create a new node with the given configuration.
    pub fn new(config: CreateNodeConfig) -> Self {
        Self {
            config,
            _transport: PhantomData,
        }
    }

    /// Get the node configuration.
    pub fn config(&self) -> &CreateNodeConfig {
        &self.config
    }
}
