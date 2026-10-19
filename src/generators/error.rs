use std::fmt::{self, Display, Formatter};

use crate::WavError;

/// An error encountered while generating audio samples.
#[derive(Debug)]
pub enum GeneratorError {
    /// An underlying WAV sample-container operation failed.
    Wav(WavError),
    /// A waveform frequency is outside the usable sampled range.
    InvalidFrequency(f64),
    /// An additive generator was asked to use no harmonics.
    InvalidHarmonicCount(u32),
    /// Generated sample storage would exceed the platform's limits.
    SizeOverflow(&'static str),
}

impl Display for GeneratorError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wav(error) => Display::fmt(error, formatter),
            Self::InvalidFrequency(frequency) => write!(
                formatter,
                "invalid frequency {frequency}; it must be finite, greater than zero, and below the Nyquist frequency"
            ),
            Self::InvalidHarmonicCount(harmonics) => write!(
                formatter,
                "invalid harmonic count {harmonics}; it must be greater than zero"
            ),
            Self::SizeOverflow(item) => {
                write!(
                    formatter,
                    "{item} is too large to generate on this platform"
                )
            }
        }
    }
}

impl std::error::Error for GeneratorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Wav(error) => Some(error),
            _ => None,
        }
    }
}

impl From<WavError> for GeneratorError {
    fn from(error: WavError) -> Self {
        Self::Wav(error)
    }
}
