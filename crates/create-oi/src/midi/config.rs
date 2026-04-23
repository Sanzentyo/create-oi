use core::num::NonZeroUsize;

/// Configuration for MIDI parsing.
#[derive(Debug, Clone)]
pub struct MidiConfig {
    /// Track index to use (0-based). `None` = auto-detect the first track
    /// that contains at least one `NoteOn` event with nonzero velocity.
    ///
    /// Ignored when [`merge_all_tracks`](Self::merge_all_tracks) is `true`.
    pub track: Option<usize>,
    /// Override the tempo (µs per beat). `None` = read from the MIDI file;
    /// defaults to 500 000 (120 BPM) if no tempo event is present.
    pub tempo_micros_per_beat: Option<u32>,
    /// When `true`, notes from **all** tracks are merged into a single
    /// monophonic voice using the sweep-line algorithm.
    ///
    /// [`voice_selection`](Self::voice_selection) determines which note wins
    /// when multiple are active simultaneously. Useful for complex multi-track
    /// MIDI files where a simple single-track extraction produces many very
    /// short notes.
    ///
    /// Default: `false` (single-track extraction, same behaviour as before).
    pub merge_all_tracks: bool,
    /// Voice selection policy used when [`merge_all_tracks`](Self::merge_all_tracks)
    /// is `true`. Default: [`VoiceSelection::HighestPitch`].
    pub voice_selection: VoiceSelection,
    /// Maximum simultaneous voices before polyphony reduction is applied.
    ///
    /// When set to `Some(n)`, at most `n` notes are allowed to sound
    /// simultaneously. If more than `n` notes are active, the least-important
    /// ones are dropped immediately according to
    /// [`voice_selection`](Self::voice_selection). `None` (the default) means
    /// no limit.
    ///
    /// Only effective when [`merge_all_tracks`](Self::merge_all_tracks) is
    /// `true`. This is useful for dense orchestral files where reducing from
    /// 10+ voices to 2–4 before final monophonization can produce better
    /// melodic output than going directly from 10+ to 1.
    pub max_voices: Option<NonZeroUsize>,
    /// When `true` (default), MIDI channel 10 (0-indexed: 9) is excluded from
    /// the multi-track merge. Channel 10 is conventionally reserved for
    /// percussion in General MIDI; including drums in a melodic monophonization
    /// usually produces poor results.
    ///
    /// Ignored when [`channel`](Self::channel) is explicitly set (the explicit
    /// channel selection takes precedence).
    ///
    /// Only used when [`merge_all_tracks`](Self::merge_all_tracks) is `true`.
    pub filter_percussion: bool,
    /// Restrict note extraction to a single MIDI channel (0-indexed, 0–15).
    /// `None` (default) includes all channels.
    ///
    /// When set, this overrides [`filter_percussion`](Self::filter_percussion):
    /// even `channel = Some(9)` is allowed so that percussion can be extracted
    /// deliberately. Valid values are `0..=15`; [`MidiError::InvalidChannel`]
    /// is returned for values outside this range.
    ///
    /// Works in both single-track and multi-track merge modes.
    pub channel: Option<u8>,
    /// When `true` (default), silence gaps between notes are encoded as rest
    /// notes (MIDI pitch 0) in the output. Set to `false` to drop gaps so that
    /// notes play back-to-back.
    ///
    /// Note spans with out-of-range pitches (those that would be dropped by
    /// the converter) are treated as silence for this purpose.
    pub include_rests: bool,
    /// Trim the leading silence before the first audible note.
    ///
    /// Only effective when [`include_rests`](Self::include_rests) is `true`.
    /// Default: `true`.
    pub trim_start: bool,
    /// Trim the trailing silence after the last audible note.
    ///
    /// Only effective when [`include_rests`](Self::include_rests) is `true`.
    /// Default: `true`.
    pub trim_end: bool,
}

impl Default for MidiConfig {
    fn default() -> Self {
        Self {
            track: None,
            tempo_micros_per_beat: None,
            merge_all_tracks: false,
            voice_selection: VoiceSelection::default(),
            max_voices: None,
            filter_percussion: true,
            channel: None,
            include_rests: true,
            trim_start: true,
            trim_end: true,
        }
    }
}

/// Policy for selecting the active note when multiple are sounding
/// simultaneously during multi-track monophonization.
///
/// Only used when [`MidiConfig::merge_all_tracks`] is `true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VoiceSelection {
    /// The sounding note with the highest MIDI pitch wins (soprano voice).
    ///
    /// This matches the melody in most pop and game music, where the melody
    /// sits above the harmony and bass.
    #[default]
    HighestPitch,
    /// The sounding note with the lowest MIDI pitch wins (bass voice).
    LowestPitch,
    /// Follow the melodic contour: among all sounding notes, prefer the pitch
    /// closest to the previously played note.
    ///
    /// This creates smoother melodic lines by avoiding large pitch jumps.
    /// Ties (equal distance from previous) are broken by higher pitch.
    /// Before any note has been played (no prior context), falls back to
    /// [`HighestPitch`].
    ///
    /// When used with [`MidiConfig::max_voices`] polyphony reduction, this
    /// behaves like `HighestPitch` within that step (contour tracking is
    /// reserved for the final monophonization pass).
    NearestPitch,
    /// Prefer the note with the highest MIDI velocity (musical emphasis).
    ///
    /// When multiple notes sound simultaneously, the loudest one is selected.
    /// Ties are broken by higher pitch. Velocity is tracked per
    /// `(channel, pitch)` pair so overlapping notes on different channels
    /// are handled independently.
    HighestVelocity,
}

/// Error type for MIDI parsing.
///
/// This enum is intentionally **exhaustive**: callers can write exhaustive
/// `match` arms.  If a new variant is added, the compiler will flag incomplete
/// match statements.
#[derive(Debug)]
pub enum MidiError {
    /// The MIDI byte stream could not be parsed.
    Parse(midly::Error),
    /// The parsed MIDI file contains no usable notes (all pitches were out of
    /// the robot's range 31–127, or the file is empty).
    NoNotes,
    /// The file uses SMPTE timecode, which cannot be converted to robot song
    /// units. Use metrical (tempo-based) timing instead.
    UnsupportedTiming,
    /// The file is MIDI Format 2 (sequential multi-song), which this crate
    /// does not support.
    UnsupportedFormat,
    /// The timing header has `ticks_per_beat == 0`, which would cause a
    /// division by zero in duration conversion.
    InvalidTiming,
    /// The requested MIDI channel number is out of range. Valid channels are
    /// 0–15 (0-indexed); the robot's OI has no concept of MIDI channels.
    InvalidChannel(u8),
}

impl core::fmt::Display for MidiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MidiError::Parse(e) => write!(f, "MIDI parse error: {e}"),
            MidiError::NoNotes => write!(f, "no playable notes in MIDI file"),
            MidiError::UnsupportedTiming => {
                write!(f, "SMPTE timecode is not supported; use metrical timing")
            }
            MidiError::UnsupportedFormat => {
                write!(f, "MIDI Format 2 (sequential) is not supported")
            }
            MidiError::InvalidTiming => {
                write!(f, "ticks_per_beat is zero; invalid MIDI file")
            }
            MidiError::InvalidChannel(ch) => {
                write!(f, "channel {ch} is out of range; valid channels are 0–15")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MidiError {}
