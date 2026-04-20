//! Sans-IO stream framing state machine.
//!
//! The iRobot OI stream format:
//!
//! ```text
//! [19] [nbytes] [id1] [data1...] [id2] [data2...] ... [checksum]
//! ```
//!
//! Where `checksum` = low byte such that `(19 + nbytes + all_data_bytes + checksum) & 0xFF == 0`.
//!
//! This module provides a [`StreamParser`] that consumes raw bytes via [`feed()`](StreamParser::feed)
//! and emits parsed frames.

use crate::error::Error;
use crate::protocol::sensor::SensorData;

use super::opcode::packet_info;

/// Header byte that starts every stream frame.
const STREAM_HEADER: u8 = 19;

/// Maximum plausible frame size to guard against corrupt data.
const MAX_FRAME_LEN: usize = 256;

/// State machine for parsing OI stream frames from a byte stream.
///
/// This is a sans-IO component: it does not read from any I/O source.
/// Feed it bytes, and it produces parsed [`SensorData`] frames.
#[derive(Debug)]
pub struct StreamParser {
    /// Internal byte buffer.
    buf: Vec<u8>,
    /// Current parser state.
    state: State,
    /// Expected packet IDs in the stream (set by the user via `set_packet_ids`).
    packet_ids: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Looking for the 19 header byte.
    WaitingHeader,
    /// Have header, waiting for the nbytes length byte.
    WaitingLength,
    /// Collecting `expected` data bytes (including IDs interspersed with data, plus checksum).
    /// `expected` = nbytes + 1 (for the checksum byte).
    Collecting { expected: usize },
}

impl StreamParser {
    /// Create a new parser. Call [`set_packet_ids`](Self::set_packet_ids) before feeding data.
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(128),
            state: State::WaitingHeader,
            packet_ids: Vec::new(),
        }
    }

    /// Set the packet IDs that this stream is expected to contain.
    /// This must match the IDs passed to the `encode_stream()` command.
    pub fn set_packet_ids(&mut self, ids: &[u8]) {
        self.packet_ids = ids.to_vec();
    }

    /// Reset the parser state, discarding any partial frame.
    pub fn reset(&mut self) {
        self.buf.clear();
        self.state = State::WaitingHeader;
    }

    /// Feed raw bytes from the transport into the parser.
    ///
    /// Returns a `Vec` of successfully parsed sensor frames.
    /// In normal operation 0 or 1 frames are returned per call.
    pub fn feed(&mut self, data: &[u8]) -> Vec<Result<SensorData, Error>> {
        let mut frames = Vec::new();
        for &byte in data {
            match self.state {
                State::WaitingHeader => {
                    if byte == STREAM_HEADER {
                        self.buf.clear();
                        self.buf.push(byte);
                        self.state = State::WaitingLength;
                    }
                    // else: discard byte, keep looking
                }
                State::WaitingLength => {
                    self.buf.push(byte);
                    let nbytes = byte as usize;
                    if nbytes == 0 || nbytes > MAX_FRAME_LEN {
                        // Invalid length; go back to scanning.
                        self.state = State::WaitingHeader;
                    } else {
                        // We need nbytes more data bytes + 1 checksum.
                        self.state = State::Collecting {
                            expected: nbytes + 1,
                        };
                    }
                }
                State::Collecting { expected } => {
                    self.buf.push(byte);
                    if self.buf.len() - 2 >= expected {
                        // Full frame collected. Validate checksum.
                        frames.push(self.parse_frame());
                        self.state = State::WaitingHeader;
                    }
                }
            }
        }
        frames
    }

    /// Parse the complete frame in `self.buf`.
    fn parse_frame(&mut self) -> Result<SensorData, Error> {
        // Verify checksum: sum of all bytes (including header) mod 256 == 0
        let checksum: u8 = self.buf.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        if checksum != 0 {
            let expected = self.buf.last().copied().unwrap_or(0);
            let actual = expected.wrapping_sub(checksum).wrapping_add(expected);
            return Err(Error::Checksum { expected, actual });
        }

        // Parse the payload: [id1][data1...][id2][data2...] ...
        // Payload starts at index 2 (after header + nbytes), ends before last byte (checksum).
        let payload = &self.buf[2..self.buf.len() - 1];
        let mut sd = SensorData::default();
        let mut offset = 0;

        while offset < payload.len() {
            let pkt_id = payload[offset];
            offset += 1;

            let info = match packet_info(pkt_id) {
                Some(i) => i,
                None => {
                    return Err(Error::Protocol(format!(
                        "unknown packet id {pkt_id} in stream"
                    )));
                }
            };
            let len = info.len as usize;
            if offset + len > payload.len() {
                return Err(Error::InsufficientData {
                    need: len,
                    got: payload.len() - offset,
                });
            }
            sd.decode_packet(pkt_id, &payload[offset..offset + len])?;
            offset += len;
        }

        Ok(sd)
    }
}

impl Default for StreamParser {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a stream frame: [19][nbytes][payload...][checksum]
    fn make_frame(payload: &[u8]) -> Vec<u8> {
        let nbytes = payload.len() as u8;
        let mut frame = vec![STREAM_HEADER, nbytes];
        frame.extend_from_slice(payload);
        // Checksum: sum all bytes so far, then append byte that makes total % 256 == 0
        let sum: u8 = frame.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        frame.push(0u8.wrapping_sub(sum));
        frame
    }

    #[test]
    fn parse_single_frame_wall() {
        let mut parser = StreamParser::new();
        // Stream with packet 8 (wall), value=1
        let payload = [8, 1]; // id=8, data=1
        let frame = make_frame(&payload);

        let results = parser.feed(&frame);
        assert_eq!(results.len(), 1);
        let sd = results[0].as_ref().unwrap();
        assert_eq!(sd.wall, Some(true));
    }

    #[test]
    fn parse_single_frame_voltage() {
        let mut parser = StreamParser::new();
        // Stream with packet 22 (voltage, 2 bytes), value=12500 (0x30D4)
        let payload = [22, 0x30, 0xD4];
        let frame = make_frame(&payload);

        let results = parser.feed(&frame);
        assert_eq!(results.len(), 1);
        let sd = results[0].as_ref().unwrap();
        assert_eq!(sd.voltage, Some(12500));
    }

    #[test]
    fn parse_two_packets_in_one_frame() {
        let mut parser = StreamParser::new();
        // wall (id=8, 1 byte) + voltage (id=22, 2 bytes)
        let payload = [8, 1, 22, 0x30, 0xD4];
        let frame = make_frame(&payload);

        let results = parser.feed(&frame);
        assert_eq!(results.len(), 1);
        let sd = results[0].as_ref().unwrap();
        assert_eq!(sd.wall, Some(true));
        assert_eq!(sd.voltage, Some(12500));
    }

    #[test]
    fn parse_split_across_feeds() {
        let mut parser = StreamParser::new();
        let payload = [8, 1];
        let frame = make_frame(&payload);

        // Feed byte by byte
        for &byte in &frame[..frame.len() - 1] {
            let results = parser.feed(&[byte]);
            assert!(results.is_empty(), "no frame should be emitted yet");
        }
        let results = parser.feed(&[*frame.last().unwrap()]);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }

    #[test]
    fn bad_checksum_returns_error() {
        let mut parser = StreamParser::new();
        let payload = [8, 1];
        let mut frame = make_frame(&payload);
        // Corrupt the checksum
        *frame.last_mut().unwrap() = frame.last().unwrap().wrapping_add(1);

        let results = parser.feed(&frame);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
    }

    #[test]
    fn garbage_before_header_is_skipped() {
        let mut parser = StreamParser::new();
        let payload = [8, 0]; // wall = false
        let frame = make_frame(&payload);

        // Prepend garbage
        let mut data = vec![0xFF, 0xAA, 0x00, 0x55];
        data.extend_from_slice(&frame);

        let results = parser.feed(&data);
        assert_eq!(results.len(), 1);
        let sd = results[0].as_ref().unwrap();
        assert_eq!(sd.wall, Some(false));
    }

    #[test]
    fn two_frames_in_one_feed() {
        let mut parser = StreamParser::new();
        let f1 = make_frame(&[8, 1]);
        let f2 = make_frame(&[35, 2]); // OI mode = Safe

        let mut data = f1;
        data.extend_from_slice(&f2);

        let results = parser.feed(&data);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].as_ref().unwrap().wall, Some(true));
        assert_eq!(
            results[1].as_ref().unwrap().oi_mode,
            Some(crate::types::OiMode::Safe)
        );
    }

    #[test]
    fn reset_discards_partial() {
        let mut parser = StreamParser::new();
        let frame = make_frame(&[8, 1]);
        // Feed partial
        parser.feed(&frame[..2]);
        parser.reset();
        // Feed the same frame fully — should parse ok
        let results = parser.feed(&frame);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }
}
