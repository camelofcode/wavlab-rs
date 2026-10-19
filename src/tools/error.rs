use std::fmt::{self, Display, Formatter};

use crate::WavError;

/// An error encountered while combining, splitting, or processing audio.
#[derive(Debug)]
pub enum ProcessingError {
    /// An underlying WAV sample-container operation failed.
    Wav(WavError),
    /// An operation expected a mono file.
    ExpectedMono(u16),
    /// An operation expected a stereo file.
    ExpectedStereo(u16),
    /// A requested split time does not identify a valid boundary.
    InvalidSplitTime(u64),
    /// An operation received fewer input files than it requires.
    TooFewInputs {
        /// The minimum number of inputs required.
        minimum: usize,
        /// The number of inputs supplied.
        actual: usize,
    },
    /// Input files have different sample rates.
    SampleRateMismatch {
        /// The sample rate established by the first input.
        expected: u32,
        /// The incompatible sample rate encountered later.
        actual: u32,
    },
    /// Input files have different channel counts.
    ChannelCountMismatch {
        /// The channel count established by the first input.
        expected: u16,
        /// The incompatible channel count encountered later.
        actual: u16,
    },
    /// Processed sample storage would exceed the platform's limits.
    SizeOverflow(&'static str),
}

impl Display for ProcessingError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wav(error) => Display::fmt(error, formatter),
            Self::ExpectedMono(count) => {
                write!(formatter, "expected a mono file, found {count} channels")
            }
            Self::ExpectedStereo(count) => {
                write!(formatter, "expected a stereo file, found {count} channel")?;
                if *count != 1 {
                    formatter.write_str("s")?;
                }
                Ok(())
            }
            Self::InvalidSplitTime(milliseconds) => write!(
                formatter,
                "split time {milliseconds} ms must resolve to a distinct, increasing frame boundary inside the file"
            ),
            Self::TooFewInputs { minimum, actual } => write!(
                formatter,
                "expected at least {minimum} input files, found {actual}"
            ),
            Self::SampleRateMismatch { expected, actual } => write!(
                formatter,
                "sample rate mismatch: expected {expected} Hz, found {actual} Hz"
            ),
            Self::ChannelCountMismatch { expected, actual } => write!(
                formatter,
                "channel count mismatch: expected {expected}, found {actual}"
            ),
            Self::SizeOverflow(item) => {
                write!(formatter, "{item} is too large to process on this platform")
            }
        }
    }
}

impl std::error::Error for ProcessingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Wav(error) => Some(error),
            _ => None,
        }
    }
}

impl From<WavError> for ProcessingError {
    fn from(error: WavError) -> Self {
        Self::Wav(error)
    }
}
