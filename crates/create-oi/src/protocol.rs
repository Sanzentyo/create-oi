//! Sans-IO protocol layer for the iRobot Open Interface.
//!
//! This module contains pure protocol logic with zero I/O dependencies:
//! command encoding, sensor parsing, and stream framing.

pub mod command;
pub mod opcode;
pub mod sensor;
pub mod stream;
