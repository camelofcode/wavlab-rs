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
