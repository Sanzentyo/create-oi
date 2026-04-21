use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::num::NonZeroUsize;

use super::config::VoiceSelection;
use super::events::{NoteEvent, TempoChange, make_note, make_rest};
use crate::types::SongNote;

/// Sweep-line monophonization over a sorted slice of note events.
///
/// Maintains a count-map of currently-sounding pitches. At each tick where the
/// active set changes, the current winner (highest or lowest pitch) is checked;
/// if it changed, the previous segment is emitted as a [`SongNote`].
///
/// Zero-duration segments (two NoteOn events at the exact same tick) are
/// discarded. Dangling notes with no NoteOff are emitted at the end with their
/// accumulated duration (or clamped to 1 robot unit if duration is zero).
///
/// When `include_rests` is `true`, silence gaps (including spans where the
/// winner is out of the 31–127 range) are emitted as rest notes (pitch 0).
#[allow(clippy::too_many_arguments)]
pub(super) fn monophonize_events(
    events: &[NoteEvent],
    voice_selection: VoiceSelection,
    tempo_map: &[TempoChange],
    ticks_per_beat: u32,
    include_rests: bool,
    trim_start: bool,
    trim_end: bool,
    track_end_tick: u64,
) -> Vec<SongNote> {
    let mut active: BTreeMap<u8, u32> = BTreeMap::new(); // pitch → active count
    // (channel, pitch) → NoteOn velocity; used by HighestVelocity.
    let mut active_vel: BTreeMap<(u8, u8), u8> = BTreeMap::new();
    let mut current_winner: Option<u8> = None;
    let mut segment_start: u64 = 0;
    let mut last_tick: u64 = 0;
    let mut notes: Vec<SongNote> = Vec::new();

    // Start of the current silence gap; `Some(0)` tracks leading silence.
    let mut rest_start: Option<u64> = if include_rests { Some(0) } else { None };
    // Whether any audible note has been started yet.
    let mut first_note_started = false;
    // For NearestPitch: the last emitted audible pitch.
    let mut prev_pitch: Option<u8> = None;
    // Pre-tick reference for NearestPitch: the winner before the current tick's
    // events, so all events within the same tick use a consistent reference.
    let mut tick_ref: Option<u8> = None;
    // Tick of the previous event; used to detect tick advances.
    let mut prev_event_tick: u64 = u64::MAX;

    for event in events {
        // When the tick advances, record the current winner as the reference
        // for NearestPitch, so every event at the same tick uses the same
        // pre-tick reference rather than intermediate intra-tick states.
        if event.abs_tick != prev_event_tick {
            tick_ref = current_winner;
            prev_event_tick = event.abs_tick;
        }
        last_tick = event.abs_tick;

        if event.is_on {
            *active.entry(event.pitch).or_insert(0) += 1;
            active_vel.insert((event.channel, event.pitch), event.velocity);
        } else {
            match active.get_mut(&event.pitch) {
                Some(count) if *count > 1 => *count -= 1,
                Some(_) => {
                    active.remove(&event.pitch);
                }
                None => {} // Spurious NoteOff — ignore.
            }
            active_vel.remove(&(event.channel, event.pitch));
        }

        let new_winner = match voice_selection {
            VoiceSelection::HighestPitch => active.keys().next_back().copied(),
            VoiceSelection::LowestPitch => active.keys().next().copied(),
            VoiceSelection::NearestPitch => {
                let reference = tick_ref.or(prev_pitch);
                match reference {
                    None => active.keys().next_back().copied(),
                    Some(p) => active
                        .keys()
                        .min_by_key(|&&k| {
                            let dist = (k as i16 - p as i16).unsigned_abs();
                            (dist, u8::MAX - k) // tiebreak: higher pitch wins
                        })
                        .copied(),
                }
            }
            VoiceSelection::HighestVelocity => active_vel
                .iter()
                .max_by(|(k1, v1): &(&(u8, u8), &u8), (k2, v2): &(&(u8, u8), &u8)| {
                    v1.cmp(v2).then_with(|| k1.1.cmp(&k2.1))
                })
                .map(|((_, p), _)| *p),
        };

        if new_winner != current_winner {
            // Flush the previous segment.
            if let Some(pitch) = current_winner {
                let dur = event.abs_tick.saturating_sub(segment_start);
                if dur > 0 {
                    if let Some(note) =
                        make_note(pitch, segment_start, dur, tempo_map, ticks_per_beat)
                    {
                        notes.push(note);
                        prev_pitch = Some(pitch); // for NearestPitch contour tracking
                        // Audible note ended; silence starts here.
                        if include_rests {
                            rest_start = Some(event.abs_tick);
                        }
                    }
                    // Out-of-range segment: rest_start unchanged (silence continues).
                }
            }
            // previous was None (silence) — rest_start already set; no flush needed.

            // Start the new segment.
            match new_winner {
                Some(new_pitch) if (31u8..=127).contains(&new_pitch) => {
                    // Audible note starting: emit any pending rest.
                    if include_rests {
                        if let Some(rs) = rest_start.take() {
                            let is_leading = !first_note_started;
                            if !(trim_start && is_leading) {
                                if let Some(rest) =
                                    make_rest(rs, event.abs_tick - rs, tempo_map, ticks_per_beat)
                                {
                                    notes.push(rest);
                                }
                            }
                        }
                    }
                    first_note_started = true;
                }
                Some(_) => {
                    // Out-of-range winner: treat as silence.
                    // Open rest_start if not already tracking silence.
                    if include_rests && rest_start.is_none() {
                        rest_start = Some(event.abs_tick);
                    }
                }
                None => {
                    // No active notes: silence.
                    // rest_start was already set when the last note ended (above).
                }
            }

            segment_start = event.abs_tick;
            current_winner = new_winner;
        }
    }

    // Flush final dangling segment (always emit, even if duration is zero).
    if let Some(pitch) = current_winner {
        let dur = last_tick.saturating_sub(segment_start);
        if let Some(note) = make_note(pitch, segment_start, dur, tempo_map, ticks_per_beat) {
            notes.push(note);
            if include_rests {
                rest_start = Some(last_tick);
            }
        }
        // Out-of-range dangling note: rest_start unchanged.
    }

    // Trailing rest (after last audible note, up to track end).
    if include_rests && !trim_end {
        let effective_end = track_end_tick.max(last_tick);
        if let Some(rs) = rest_start {
            if let Some(rest) = make_rest(rs, effective_end - rs, tempo_map, ticks_per_beat) {
                notes.push(rest);
            }
        }
    }

    notes
}

/// Reduce the polyphony of a sorted NoteEvent stream to at most `max_voices`
/// simultaneous notes.
///
/// Processes events in tick-groups. Within each tick:
/// 1. All natural NoteOff events are applied (always kept in the output).
/// 2. All NoteOn events are tentatively added to the active set.
/// 3. If the active set exceeds `max_voices`, the least-important notes are
///    evicted according to `selection`:
///    - **Sustained** notes (already active before this tick) receive a
///      synthetic NoteOff at the current tick so downstream processors see a
///      clean end.
///    - **New** NoteOn events that are evicted are simply suppressed (no
///      NoteOn is emitted for them).
///
/// The returned stream has at most `max_voices` simultaneous notes at any
/// tick and preserves the original event ordering invariant
/// (NoteOff < NoteOn within a tick).
///
/// # Selection policy
///
/// | `VoiceSelection`  | Which voices are kept |
/// |-------------------|-----------------------|
/// | `HighestPitch`    | highest `max_voices` pitches |
/// | `LowestPitch`     | lowest `max_voices` pitches |
/// | `HighestVelocity` | loudest `max_voices` notes (tie: higher pitch) |
/// | `NearestPitch`    | same as `HighestPitch` (contour tracking is left to the final monophonization pass) |
pub(super) fn limit_voices(
    events: &[NoteEvent],
    max_voices: NonZeroUsize,
    selection: VoiceSelection,
) -> Vec<NoteEvent> {
    let max = max_voices.get();
    // (channel, pitch) → most recent NoteOn velocity.
    let mut active: BTreeMap<(u8, u8), u8> = BTreeMap::new();
    let mut output: Vec<NoteEvent> = Vec::with_capacity(events.len());

    let mut i = 0;
    while i < events.len() {
        let tick = events[i].abs_tick;

        // Determine how many events belong to this tick.
        let group_len = events[i..].partition_point(|e| e.abs_tick == tick);
        let group = &events[i..i + group_len];

        // Step 1: Apply NoteOffs — emit only for notes that are currently active.
        // If a note was previously evicted (synthetic NoteOff already emitted),
        // its original NoteOff is silently dropped to avoid spurious events.
        for ev in group.iter().filter(|e| !e.is_on) {
            if active.remove(&(ev.channel, ev.pitch)).is_some() {
                output.push(*ev);
            }
        }

        // Step 2: Snapshot which notes were active before this tick's NoteOns.
        // (Used to distinguish "sustained" from "new" when evicting.)
        let pre_keys: Vec<(u8, u8)> = active.keys().cloned().collect();

        // Step 3: Register all NoteOns in the active set.
        let new_ons: Vec<NoteEvent> = group.iter().filter(|e| e.is_on).copied().collect();
        for ev in &new_ons {
            active.insert((ev.channel, ev.pitch), ev.velocity);
        }

        // Step 4: Evict excess voices if over the limit.
        if active.len() > max {
            let kept = rank_and_keep(&active, max, selection);

            let dropped: Vec<(u8, u8)> = active
                .keys()
                .filter(|k| !kept.contains(*k))
                .cloned()
                .collect();

            for key in &dropped {
                active.remove(key);
                if pre_keys.contains(key) {
                    // Sustained note being evicted: emit synthetic NoteOff.
                    output.push(NoteEvent {
                        abs_tick: tick,
                        is_on: false,
                        pitch: key.1,
                        channel: key.0,
                        velocity: 0,
                    });
                }
                // New NoteOn being evicted: suppressed — no NoteOn emitted.
            }

            // Emit kept NoteOns only.
            for ev in &new_ons {
                if kept.contains(&(ev.channel, ev.pitch)) {
                    output.push(*ev);
                }
            }
        } else {
            // All voices fit: emit all NoteOns.
            for ev in &new_ons {
                output.push(*ev);
            }
        }

        i += group_len;
    }

    output
}

/// Return the `max` highest-priority `(channel, pitch)` keys from `active`
/// according to the selection policy.
fn rank_and_keep(
    active: &BTreeMap<(u8, u8), u8>,
    max: usize,
    selection: VoiceSelection,
) -> Vec<(u8, u8)> {
    let mut ranked: Vec<(u8, u8)> = active.keys().cloned().collect();

    match selection {
        // NearestPitch without a monophonic reference defaults to HighestPitch.
        VoiceSelection::HighestPitch | VoiceSelection::NearestPitch => {
            ranked.sort_unstable_by(|a, b| b.1.cmp(&a.1)); // descending pitch
        }
        VoiceSelection::LowestPitch => {
            ranked.sort_unstable_by(|a, b| a.1.cmp(&b.1)); // ascending pitch
        }
        VoiceSelection::HighestVelocity => {
            ranked.sort_unstable_by(|a, b| {
                let va = active.get(a).copied().unwrap_or(0);
                let vb = active.get(b).copied().unwrap_or(0);
                vb.cmp(&va).then_with(|| b.1.cmp(&a.1)) // velocity desc, pitch desc
            });
        }
    }

    ranked.truncate(max);
    ranked
}
