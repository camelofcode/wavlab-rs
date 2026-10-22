use std::fmt::{self, Display, Formatter};

use crate::{DelayError, WavError};

/// An error encountered while applying an audio effect.
#[derive(Debug)]
pub enum EffectError {
    /// An underlying WAV sample-container operation failed.
    Wav(WavError),
    /// A delay line could not be configured.
    Delay(DelayError),
    /// Feedback must be finite and have a magnitude no greater than one.
    InvalidFeedback(f64),
    /// Wet/dry mix must be finite and between zero and one.
    InvalidMix(f64),
    /// Chorus depth must be finite and between zero and one.
    InvalidChorusDepth(f64),
    /// An LFO frequency must be finite and non-negative.
    InvalidLfoFrequency(f64),
    /// Flanger depth must be finite and between zero and one.
    InvalidFlangerDepth(f64),
    /// Flanger LFO frequency must be between 0.1 and 10 Hz.
    InvalidFlangerFrequency(f64),
    /// Flanger feedback must be finite and between -0.95 and 0.95.
    InvalidFlangerFeedback(f64),
    /// Tremolo depth must be finite and between zero and one.
    InvalidTremoloDepth(f64),
    /// Automatic-panning depth must be finite and between zero and one.
    InvalidAutoPanDepth(f64),
    /// Phaser depth must be finite and between zero and one.
    InvalidPhaserDepth(f64),
    /// Phaser center frequency must be finite and lie below Nyquist.
    InvalidPhaserCenterFrequency {
        /// The requested center frequency in Hz.
        center_frequency_hz: f64,
        /// The sample rate used to calculate the Nyquist frequency.
        sample_rate: u32,
    },
    /// A phaser must contain a supported number of all-pass stages.
    InvalidPhaserStages(usize),
    /// Reverb room size must be finite and between zero and one.
    InvalidRoomSize(f64),
    /// Reverb decay time must be finite and greater than zero.
    InvalidDecayTime(f64),
    /// A low-pass cutoff must be finite and lie below the Nyquist frequency.
    InvalidCutoffFrequency {
        /// The requested cutoff frequency in Hz.
        cutoff_hz: f64,
        /// The sample rate used to calculate the Nyquist frequency.
        sample_rate: u32,
    },
    /// Tape saturation drive must be finite.
    InvalidDrive(f64),
    /// Effect output storage would exceed the platform's limits.
    SizeOverflow(&'static str),
}

impl Display for EffectError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wav(error) => Display::fmt(error, formatter),
            Self::Delay(error) => Display::fmt(error, formatter),
            Self::InvalidFeedback(feedback) => write!(
                formatter,
                "invalid delay feedback {feedback}; it must be finite and between -1 and 1"
            ),
            Self::InvalidMix(mix) => write!(
                formatter,
                "invalid effect mix {mix}; it must be finite and between 0 and 1"
            ),
            Self::InvalidChorusDepth(depth) => write!(
                formatter,
                "invalid chorus depth {depth}; it must be finite and between 0 and 1"
            ),
            Self::InvalidLfoFrequency(frequency_hz) => write!(
                formatter,
                "invalid LFO frequency {frequency_hz} Hz; it must be finite and non-negative"
            ),
            Self::InvalidFlangerDepth(depth) => write!(
                formatter,
                "invalid flanger depth {depth}; it must be finite and between 0 and 1"
            ),
            Self::InvalidFlangerFrequency(frequency_hz) => write!(
                formatter,
                "invalid flanger LFO frequency {frequency_hz} Hz; it must be finite and between 0.1 and 10 Hz"
            ),
            Self::InvalidFlangerFeedback(feedback) => write!(
                formatter,
                "invalid flanger feedback {feedback}; it must be finite and between -0.95 and 0.95"
            ),
            Self::InvalidTremoloDepth(depth) => write!(
                formatter,
                "invalid tremolo depth {depth}; it must be finite and between 0 and 1"
            ),
            Self::InvalidAutoPanDepth(depth) => write!(
                formatter,
                "invalid autopan depth {depth}; it must be finite and between 0 and 1"
            ),
            Self::InvalidPhaserDepth(depth) => write!(
                formatter,
                "invalid phaser depth {depth}; it must be finite and between 0 and 1"
            ),
            Self::InvalidPhaserCenterFrequency {
                center_frequency_hz,
                sample_rate,
            } => write!(
                formatter,
                "invalid phaser center frequency {center_frequency_hz} Hz; it must be finite, greater than zero, and below {} Hz",
                f64::from(*sample_rate) / 2.0
            ),
            Self::InvalidPhaserStages(stages) => write!(
                formatter,
                "invalid phaser stage count {stages}; it must be between 1 and {}",
                super::phaser::MAX_PHASER_STAGES
            ),
            Self::InvalidRoomSize(room_size) => write!(
                formatter,
                "invalid reverb room size {room_size}; it must be finite and between 0 and 1"
            ),
            Self::InvalidDecayTime(decay_seconds) => write!(
                formatter,
                "invalid reverb decay time {decay_seconds} seconds; it must be finite and greater than zero"
            ),
            Self::InvalidCutoffFrequency {
                cutoff_hz,
                sample_rate,
            } => write!(
                formatter,
                "invalid delay cutoff frequency {cutoff_hz} Hz; it must be finite, greater than zero, and below {} Hz",
                f64::from(*sample_rate) / 2.0
            ),
            Self::InvalidDrive(drive_db) => write!(
                formatter,
                "invalid tape delay drive {drive_db} dB; it must be finite"
            ),
            Self::SizeOverflow(item) => {
                write!(formatter, "{item} is too large to process on this platform")
            }
        }
    }
}

impl std::error::Error for EffectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Wav(error) => Some(error),
            Self::Delay(error) => Some(error),
            _ => None,
        }
    }
}

impl From<WavError> for EffectError {
    fn from(error: WavError) -> Self {
        Self::Wav(error)
    }
}

impl From<DelayError> for EffectError {
    fn from(error: DelayError) -> Self {
        Self::Delay(error)
    }
}
