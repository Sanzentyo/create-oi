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
//! This module provides a [`StreamParser`] that consumes raw bytes via
//! [`feed_with()`](StreamParser::feed_with) (no-alloc callback) or
//! [`feed()`](StreamParser::feed) (alloc, returns `Vec`).

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::error::ProtocolError;
use crate::opcode::packet_info;
use crate::sensor::SensorData;

/// Header byte that starts every stream frame.
const STREAM_HEADER: u8 = 19;

/// Default buffer capacity — sufficient for any valid OI stream frame.
///
/// An OI frame is: header(1) + N byte(1) + payload(N, max 255) + checksum(1) = N + 3 bytes.
/// With N_max = 255: 255 + 3 = 258 bytes total.
const DEFAULT_BUF_CAP: usize = 258;

/// State machine for parsing OI stream frames from a byte stream.
///
/// This is a sans-IO component: it does not read from any I/O source.
/// Feed it bytes, and it produces parsed [`SensorData`] frames.
///
/// The const generic `N` controls the internal buffer capacity.
/// Default is 258, which is the exact maximum for any valid OI stream frame
/// (header + length byte + 255-byte payload + checksum).
#[derive(Debug)]
pub struct StreamParser<const N: usize = DEFAULT_BUF_CAP> {
    /// Internal fixed-size byte buffer.
    buf: [u8; N],
    /// Current number of valid bytes in the buffer.
    len: usize,
    /// Current parser state.
    state: State,
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

impl<const N: usize> StreamParser<N> {
    /// Create a new parser with the given buffer capacity.
    pub fn new() -> Self {
        Self {
            buf: [0u8; N],
            len: 0,
            state: State::WaitingHeader,
        }
    }

    /// Reset the parser state, discarding any partial frame.
    pub fn reset(&mut self) {
        self.len = 0;
        self.state = State::WaitingHeader;
    }

    /// Feed raw bytes and invoke `on_frame` for each complete frame parsed.
    ///
    /// This is the primary no-alloc API. Each successfully parsed frame is
    /// delivered to the callback as it is decoded.
    pub fn feed_with<F>(&mut self, data: &[u8], mut on_frame: F)
    where
        F: FnMut(Result<SensorData, ProtocolError>),
    {
        for &byte in data {
            match self.state {
                State::WaitingHeader => {
                    if byte == STREAM_HEADER {
                        self.len = 0;
                        self.push_byte(byte);
                        self.state = State::WaitingLength;
                    }
                }
                State::WaitingLength => {
                    let nbytes = byte as usize;
                    // A full frame is: header(1) + nbytes_byte(1) + nbytes payload + checksum(1)
                    // = nbytes + 3 bytes total, so we need nbytes + 3 <= N.
                    if nbytes == 0 || nbytes > N.saturating_sub(3) {
                        // Oversized or empty frame — discard and resync.
                        self.state = State::WaitingHeader;
                    } else {
                        self.push_byte(byte);
                        self.state = State::Collecting {
                            expected: nbytes + 1,
                        };
                    }
                }
                State::Collecting { expected } => {
                    self.push_byte(byte);
                    if self.len - 2 >= expected {
                        on_frame(self.parse_frame());
                        self.state = State::WaitingHeader;
                    }
                }
            }
        }
    }

    /// Feed raw bytes from the transport into the parser.
    ///
    /// Returns a `Vec` of successfully parsed sensor frames.
    /// In normal operation 0 or 1 frames are returned per call.
    #[cfg(feature = "alloc")]
    pub fn feed(&mut self, data: &[u8]) -> Vec<Result<SensorData, ProtocolError>> {
        let mut frames = Vec::new();
        self.feed_with(data, |result| frames.push(result));
        frames
    }

    fn push_byte(&mut self, byte: u8) {
        if self.len < N {
            self.buf[self.len] = byte;
            self.len += 1;
        }
    }

    /// Parse the complete frame in `self.buf[..self.len]`.
    fn parse_frame(&self) -> Result<SensorData, ProtocolError> {
        let frame = &self.buf[..self.len];

        // Verify checksum: sum of all bytes (including header) mod 256 == 0.
        let fold: u8 = frame.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        if fold != 0 {
            // The checksum byte that was received.
            let actual = frame[self.len - 1];
            // The checksum byte that would make the sum zero.
            let expected = actual.wrapping_sub(fold);
            return Err(ProtocolError::Checksum { expected, actual });
        }

        // Parse the payload: [id1][data1...][id2][data2...] ...
        // Payload starts at index 2 (after header + nbytes), ends before last byte (checksum).
        let payload = &frame[2..self.len - 1];
        let mut sd = SensorData::default();
        let mut offset = 0;

        while offset < payload.len() {
            let pkt_id = payload[offset];
            offset += 1;

            let info = match packet_info(pkt_id) {
                Some(i) => i,
                None => {
                    return Err(ProtocolError::UnknownPacketId(pkt_id));
                }
            };
            let len = info.len as usize;
            if offset + len > payload.len() {
                return Err(ProtocolError::InsufficientData {
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

impl<const N: usize> Default for StreamParser<N> {
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
    use alloc::vec;
    use alloc::vec::Vec;

    /// Build a stream frame: [19][nbytes][payload...][checksum]
    fn make_frame(payload: &[u8]) -> Vec<u8> {
        let nbytes = payload.len() as u8;
        let mut frame = vec![STREAM_HEADER, nbytes];
        frame.extend_from_slice(payload);
        let sum: u8 = frame.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        frame.push(0u8.wrapping_sub(sum));
        frame
    }

    #[test]
    fn parse_single_frame_wall() {
        let mut parser = StreamParser::<256>::new();
        let payload = [8, 1];
        let frame = make_frame(&payload);

        let results = parser.feed(&frame);
        assert_eq!(results.len(), 1);
        let sd = results[0].as_ref().unwrap();
        assert_eq!(sd.wall, Some(true));
    }

    #[test]
    fn parse_single_frame_voltage() {
        let mut parser = StreamParser::<256>::new();
        let payload = [22, 0x30, 0xD4];
        let frame = make_frame(&payload);

        let results = parser.feed(&frame);
        assert_eq!(results.len(), 1);
        let sd = results[0].as_ref().unwrap();
        assert_eq!(sd.voltage, Some(12500));
    }

    #[test]
    fn parse_two_packets_in_one_frame() {
        let mut parser = StreamParser::<256>::new();
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
        let mut parser = StreamParser::<256>::new();
        let payload = [8, 1];
        let frame = make_frame(&payload);

        for &byte in &frame[..frame.len() - 1] {
            let results = parser.feed(&[byte]);
            assert!(results.is_empty(), "no frame should be emitted yet");
        }
        let results = parser.feed(&[*frame.last().unwrap()]);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }

    #[test]
    fn bad_checksum_returns_error_with_correct_bytes() {
        let mut parser = StreamParser::<256>::new();
        let payload = [8, 1];
        let mut frame = make_frame(&payload);
        // Corrupt checksum by +1
        let orig_cs = *frame.last().unwrap();
        *frame.last_mut().unwrap() = orig_cs.wrapping_add(1);

        let results = parser.feed(&frame);
        assert_eq!(results.len(), 1);
        let err = results[0].as_ref().unwrap_err();
        if let crate::error::ProtocolError::Checksum { expected, actual } = *err {
            // actual = received (corrupted) byte
            assert_eq!(actual, orig_cs.wrapping_add(1));
            // expected = the byte that would have made the sum zero
            assert_eq!(expected, orig_cs);
        } else {
            panic!("expected Checksum error, got: {err:?}");
        }
    }

    #[test]
    fn oversized_frame_is_rejected_and_next_frame_parses() {
        // Use a tiny buffer (size 8) to exercise the overflow guard.
        // Buffer holds 8 bytes; a frame needs nbytes+3, so max nbytes = 5.
        // Feed nbytes=6 (too large) followed by a valid frame.
        let mut parser = StreamParser::<8>::new();
        let valid_frame = make_frame(&[8, 1]); // nbytes=2, fits in 8

        // An oversized "frame": [19][6][...] — 6 > 8-3=5, should be rejected.
        let mut data = vec![STREAM_HEADER, 6, 0, 0, 0, 0, 0, 0, 0];
        data.extend_from_slice(&valid_frame);

        let results = parser.feed(&data);
        assert_eq!(results.len(), 1, "only the valid frame should be parsed");
        assert!(results[0].is_ok());
    }

    #[test]
    fn garbage_before_header_is_skipped() {
        let mut parser = StreamParser::<256>::new();
        let payload = [8, 0];
        let frame = make_frame(&payload);

        let mut data = vec![0xFF, 0xAA, 0x00, 0x55];
        data.extend_from_slice(&frame);

        let results = parser.feed(&data);
        assert_eq!(results.len(), 1);
        let sd = results[0].as_ref().unwrap();
        assert_eq!(sd.wall, Some(false));
    }

    #[test]
    fn two_frames_in_one_feed() {
        let mut parser = StreamParser::<256>::new();
        let f1 = make_frame(&[8, 1]);
        let f2 = make_frame(&[35, 2]);

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
        let mut parser = StreamParser::<256>::new();
        let frame = make_frame(&[8, 1]);
        parser.feed(&frame[..2]);
        parser.reset();
        let results = parser.feed(&frame);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }

    #[test]
    fn feed_with_callback() {
        let mut parser = StreamParser::<256>::new();
        let frame = make_frame(&[8, 1]);
        let mut count = 0;
        parser.feed_with(&frame, |result| {
            assert!(result.is_ok());
            assert_eq!(result.unwrap().wall, Some(true));
            count += 1;
        });
        assert_eq!(count, 1);
    }
}
