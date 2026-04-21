use alloc::vec::Vec;

use midly::{MetaMessage, MidiMessage, Smf, TrackEventKind};

use crate::types::SongNote;

/// A tempo change: the tempo becomes `micros_per_beat` at `abs_tick`.
#[derive(Clone, Copy, Debug)]
pub(super) struct TempoChange {
    pub(super) abs_tick: u64,
    pub(super) micros_per_beat: u32,
}

/// Collect all tempo change events from every track in the file.
///
/// Events at the same tick are de-duplicated: only the last one (in source
/// order) is kept, matching the "last writer wins" semantics of MIDI.
pub(super) fn build_tempo_map(smf: &Smf<'_>) -> Vec<TempoChange> {
    let mut changes: Vec<TempoChange> = Vec::new();

    for track in &smf.tracks {
        let mut abs_tick: u64 = 0;
        for event in track {
            abs_tick += u64::from(event.delta.as_int());
            if let TrackEventKind::Meta(MetaMessage::Tempo(t)) = event.kind {
                changes.retain(|c| c.abs_tick != abs_tick);
                changes.push(TempoChange {
                    abs_tick,
                    micros_per_beat: t.as_int(),
                });
            }
        }
    }

    changes.sort_by_key(|c| c.abs_tick);
    changes
}

/// Return the tempo (µs/beat) that is in effect at `abs_tick`.
pub(super) fn tempo_at(tempo_map: &[TempoChange], abs_tick: u64) -> u32 {
    tempo_map
        .iter()
        .rev()
        .find(|c| c.abs_tick <= abs_tick)
        .map(|c| c.micros_per_beat)
        .unwrap_or(500_000) // 120 BPM default
}

/// Convert a note's span `[start_tick, start_tick + dur_ticks)` to robot song
/// duration units (1/64 s = 15 625 µs).
///
/// Uses u128 arithmetic throughout to handle very long notes without overflow.
/// Piecewise-integrates across tempo-change boundaries so mid-note tempo
/// changes are handled correctly.
///
/// Returns a value clamped to `1..=255`.
pub(super) fn ticks_to_robot_units(
    start_tick: u64,
    dur_ticks: u64,
    tempo_map: &[TempoChange],
    ticks_per_beat: u32,
) -> u8 {
    debug_assert!(ticks_per_beat > 0);

    if dur_ticks == 0 {
        return 1;
    }

    let tpb = u128::from(ticks_per_beat);
    let end_tick = start_tick + dur_ticks;
    let mut total_micros: u128 = 0;
    let mut cursor = start_tick;

    for change in tempo_map {
        if change.abs_tick <= cursor {
            continue;
        }
        if change.abs_tick >= end_tick {
            break;
        }
        let segment_ticks = u128::from(change.abs_tick - cursor);
        let current_tempo = u128::from(tempo_at(tempo_map, cursor));
        total_micros =
            total_micros.saturating_add(segment_ticks.saturating_mul(current_tempo) / tpb);
        cursor = change.abs_tick;
    }

    let remaining = u128::from(end_tick - cursor);
    let final_tempo = u128::from(tempo_at(tempo_map, cursor));
    total_micros = total_micros.saturating_add(remaining.saturating_mul(final_tempo) / tpb);

    // 1 robot unit = 1/64 s = 15 625 µs.
    let units = total_micros / 15_625;
    units.clamp(1, 255) as u8
}

/// An internal note event produced during multi-track sweep-line collection.
///
/// Derived `Ord` sorts by `(abs_tick, is_on, pitch, channel)`. Because `false
/// < true`, `NoteOff` events (is_on = false) sort before `NoteOn` events at
/// the same tick, which ensures a clean handoff when one note ends exactly as
/// another begins.
///
/// `velocity` does **not** participate in the sort order; see the manual
/// `Ord` impl.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NoteEvent {
    pub(super) abs_tick: u64,
    /// `false` = NoteOff (sorts first at same tick); `true` = NoteOn.
    pub(super) is_on: bool,
    pub(super) pitch: u8,
    pub(super) channel: u8,
    /// NoteOn velocity (1–127). Zero for NoteOff events.
    pub(super) velocity: u8,
}

impl PartialOrd for NoteEvent {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NoteEvent {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.abs_tick
            .cmp(&other.abs_tick)
            .then(self.is_on.cmp(&other.is_on))
            .then(self.pitch.cmp(&other.pitch))
            .then(self.channel.cmp(&other.channel))
    }
}

/// Collect all NoteOn/NoteOff events from every track, optionally skipping
/// MIDI channel 10 (0-indexed: 9, percussion) or filtering to a single channel.
pub(super) fn collect_note_events(
    smf: &Smf<'_>,
    filter_percussion: bool,
    channel_filter: Option<u8>,
) -> Vec<NoteEvent> {
    let mut events = Vec::new();
    for track in &smf.tracks {
        let mut abs_tick: u64 = 0;
        for event in track {
            abs_tick += u64::from(event.delta.as_int());
            if let TrackEventKind::Midi { channel, message } = event.kind {
                let ch = channel.as_int();
                match channel_filter {
                    Some(only_ch) => {
                        if ch != only_ch {
                            continue;
                        }
                    }
                    None => {
                        if filter_percussion && ch == 9 {
                            continue;
                        }
                    }
                }
                let (is_on, pitch, velocity) = match message {
                    MidiMessage::NoteOn { key, vel } => {
                        let v = vel.as_int();
                        (v > 0, key.as_int(), v)
                    }
                    MidiMessage::NoteOff { key, .. } => (false, key.as_int(), 0),
                    _ => continue,
                };
                events.push(NoteEvent {
                    abs_tick,
                    is_on,
                    pitch,
                    channel: ch,
                    velocity,
                });
            }
        }
    }
    events
}

/// Attempt to build a [`SongNote`] from a MIDI pitch and timing data.
///
/// Returns `None` if the pitch is outside the robot's range (31–127).
pub(super) fn make_note(
    pitch: u8,
    start_tick: u64,
    dur_ticks: u64,
    tempo_map: &[TempoChange],
    ticks_per_beat: u32,
) -> Option<SongNote> {
    let duration = ticks_to_robot_units(start_tick, dur_ticks, tempo_map, ticks_per_beat);
    SongNote::new(pitch, duration).ok()
}

/// Build a rest [`SongNote`] (pitch = 0) from a silence span.
///
/// Returns `None` if `dur_ticks` is zero (explicit check before calling
/// `ticks_to_robot_units`, which clamps zero to 1).
pub(super) fn make_rest(
    start_tick: u64,
    dur_ticks: u64,
    tempo_map: &[TempoChange],
    ticks_per_beat: u32,
) -> Option<SongNote> {
    if dur_ticks == 0 {
        return None;
    }
    let duration = ticks_to_robot_units(start_tick, dur_ticks, tempo_map, ticks_per_beat);
    Some(SongNote::rest(duration))
}

/// Find the maximum track end tick across all tracks.
///
/// Uses the `EndOfTrack` meta event if present; falls back to the tick of the
/// last event in the track if there is no explicit `EndOfTrack`.
pub(super) fn find_max_track_end_tick(smf: &midly::Smf<'_>) -> u64 {
    smf.tracks
        .iter()
        .map(|track| {
            let mut abs_tick: u64 = 0;
            let mut end_tick: u64 = 0;
            for event in track {
                abs_tick += u64::from(event.delta.as_int());
                if let TrackEventKind::Meta(MetaMessage::EndOfTrack) = event.kind {
                    end_tick = abs_tick;
                }
            }
            end_tick.max(abs_tick)
        })
        .max()
        .unwrap_or(0)
}
