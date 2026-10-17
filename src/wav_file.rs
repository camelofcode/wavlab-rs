mod binary_io;
mod file_io;
mod sample_conversion;

use std::fmt::{self, Display, Formatter};

const PCM_FORMAT_CHUNK_SIZE: u32 = 16;
const EMPTY_PCM_FILE_LENGTH: u64 = 44;
const EMPTY_PCM_RIFF_CHUNK_SIZE: u32 = 36;

pub struct WavFile {
    length_in_bytes: u64,
    file_chunk: FileChunk,
    format_chunk: FormatChunk,
    bext_chunk: Option<BextChunk>,
    data_chunk: DataChunk,
    number_of_samples: usize,
    samples: Option<Vec<f64>>,
}

struct FileChunk {
    id: [u8; 4],
    size: u32,
    format: [u8; 4],
}

struct FormatChunk {
    id: [u8; 4],
    size: u32,
    audio_format: u16,
    number_of_channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BextChunkLocation {
    Header,
    Footer,
}

struct BextChunk {
    location: BextChunkLocation,
    id: [u8; 4],
    size: u32,
    description: String,
    originator: String,
    originator_reference: String,
    origination_date: String,
    origination_time: String,
    time_reference_low: u32,
    time_reference_high: u32,
    version: u16,
    extra_data: Vec<u8>,
}

struct DataChunk {
    id: [u8; 4],
    size: u32,
    start_offset: u64,
}

#[derive(Debug)]
pub enum WavError {
    Io(std::io::Error),
    InvalidFile(String),
    UnsupportedChannelCount(u16),
    ExpectedMono(u16),
    ExpectedStereo(u16),
    InvalidSplitTime(u64),
    TooFewInputs {
        minimum: usize,
        actual: usize,
    },
    SampleRateMismatch {
        expected: u32,
        actual: u32,
    },
    ChannelCountMismatch {
        expected: u16,
        actual: u16,
    },
    InvalidSampleRate(u32),
    UnsupportedBitsPerSample(u16),
    ByteRateOverflow {
        sample_rate: u32,
        block_align: u16,
    },
    SamplesNotLoaded,
    IncompleteSampleFrame {
        sample_count: usize,
        number_of_channels: u16,
    },
    SizeOverflow(&'static str),
    TextFieldTooLong {
        field: &'static str,
        maximum: usize,
        actual: usize,
    },
}

#[derive(Clone, Copy)]
struct PcmLayout {
    bytes_per_sample: u16,
    block_align: u16,
    byte_rate: u32,
}

impl WavFile {
    pub fn new(
        number_of_channels: u16,
        sample_rate: u32,
        bits_per_sample: u16,
    ) -> Result<Self, WavError> {
        let layout = pcm_layout(number_of_channels, sample_rate, bits_per_sample)?;

        Ok(Self {
            length_in_bytes: EMPTY_PCM_FILE_LENGTH,
            file_chunk: FileChunk {
                id: *b"RIFF",
                size: EMPTY_PCM_RIFF_CHUNK_SIZE,
                format: *b"WAVE",
            },
            format_chunk: FormatChunk {
                id: *b"fmt ",
                size: PCM_FORMAT_CHUNK_SIZE,
                audio_format: 1,
                number_of_channels,
                sample_rate,
                byte_rate: layout.byte_rate,
                block_align: layout.block_align,
                bits_per_sample,
            },
            bext_chunk: None,
            data_chunk: DataChunk {
                id: *b"data",
                size: 0,
                start_offset: EMPTY_PCM_FILE_LENGTH,
            },
            number_of_samples: 0,
            samples: Some(Vec::new()),
        })
    }

    /// Returns the number of interleaved audio channels.
    pub fn number_of_channels(&self) -> u16 {
        self.format_chunk.number_of_channels
    }

    /// Returns the sample rate in Hz.
    pub fn sample_rate(&self) -> u32 {
        self.format_chunk.sample_rate
    }

    /// Returns the PCM bit depth used when saving the file.
    pub fn bits_per_sample(&self) -> u16 {
        self.format_chunk.bits_per_sample
    }

    /// Returns the total number of interleaved samples in the file.
    pub fn number_of_samples(&self) -> usize {
        self.number_of_samples
    }

    /// Returns the loaded interleaved samples.
    pub fn samples(&self) -> Result<&[f64], WavError> {
        self.samples.as_deref().ok_or(WavError::SamplesNotLoaded)
    }

    /// Returns the loaded interleaved samples for in-place processing.
    ///
    /// The slice has a fixed length, so callers cannot accidentally leave a
    /// partial multichannel sample frame or make the stored sample count stale.
    pub fn samples_mut(&mut self) -> Result<&mut [f64], WavError> {
        self.samples
            .as_deref_mut()
            .ok_or(WavError::SamplesNotLoaded)
    }

    /// Replaces the loaded interleaved samples.
    pub fn set_samples(&mut self, samples: Vec<f64>) -> Result<(), WavError> {
        if !samples
            .len()
            .is_multiple_of(usize::from(self.number_of_channels()))
        {
            return Err(WavError::IncompleteSampleFrame {
                sample_count: samples.len(),
                number_of_channels: self.number_of_channels(),
            });
        }

        self.number_of_samples = samples.len();
        self.samples = Some(samples);
        Ok(())
    }
}

fn pcm_layout(
    number_of_channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
) -> Result<PcmLayout, WavError> {
    if !matches!(number_of_channels, 1 | 2) {
        return Err(WavError::UnsupportedChannelCount(number_of_channels));
    }

    if sample_rate == 0 {
        return Err(WavError::InvalidSampleRate(sample_rate));
    }

    let bytes_per_sample = match bits_per_sample {
        8 => 1,
        16 => 2,
        24 => 3,
        32 => 4,
        _ => return Err(WavError::UnsupportedBitsPerSample(bits_per_sample)),
    };

    let block_align = number_of_channels * bytes_per_sample;
    let byte_rate =
        sample_rate
            .checked_mul(u32::from(block_align))
            .ok_or(WavError::ByteRateOverflow {
                sample_rate,
                block_align,
            })?;

    Ok(PcmLayout {
        bytes_per_sample,
        block_align,
        byte_rate,
    })
}

impl Display for WavError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => Display::fmt(error, formatter),
            Self::InvalidFile(message) => write!(formatter, "invalid WAV file: {message}"),
            Self::UnsupportedChannelCount(count) => write!(
                formatter,
                "unsupported channel count {count}; only mono and stereo PCM are supported"
            ),
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
            Self::InvalidSampleRate(sample_rate) => {
                write!(
                    formatter,
                    "invalid sample rate {sample_rate}; it must be greater than zero"
                )
            }
            Self::UnsupportedBitsPerSample(bits) => write!(
                formatter,
                "unsupported PCM bit depth {bits}; expected 8, 16, 24, or 32 bits"
            ),
            Self::ByteRateOverflow {
                sample_rate,
                block_align,
            } => write!(
                formatter,
                "byte rate exceeds the WAV limit for sample rate {sample_rate} and block alignment {block_align}"
            ),
            Self::SamplesNotLoaded => formatter.write_str("sample data was not loaded"),
            Self::IncompleteSampleFrame {
                sample_count,
                number_of_channels,
            } => write!(
                formatter,
                "{sample_count} samples do not form complete frames for {number_of_channels} channels"
            ),
            Self::SizeOverflow(item) => write!(formatter, "{item} is too large for a WAV file"),
            Self::TextFieldTooLong {
                field,
                maximum,
                actual,
            } => write!(
                formatter,
                "BEXT {field} is {actual} bytes; the maximum is {maximum} bytes"
            ),
        }
    }
}

impl std::error::Error for WavError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for WavError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl Display for WavFile {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "WAV file")?;
        writeln!(formatter, "Length: {} bytes", self.length_in_bytes)?;
        writeln!(formatter)?;
        writeln!(formatter, "{}", self.file_chunk)?;
        writeln!(formatter)?;
        writeln!(formatter, "{}", self.format_chunk)?;
        writeln!(formatter)?;

        match &self.bext_chunk {
            Some(bext_chunk) => {
                writeln!(formatter, "{bext_chunk}")?;
                writeln!(formatter)?;
            }
            None => {
                writeln!(formatter, "BEXT chunk: not present")?;
                writeln!(formatter)?;
            }
        }

        writeln!(formatter, "{}", self.data_chunk)?;
        writeln!(formatter)?;

        match &self.samples {
            Some(samples) => write!(formatter, "Samples: {} loaded", samples.len()),
            None => write!(
                formatter,
                "Samples: not loaded ({} in file)",
                self.number_of_samples
            ),
        }
    }
}

impl Display for FileChunk {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "File chunk")?;
        writeln!(formatter, "  ID: {}", String::from_utf8_lossy(&self.id))?;
        writeln!(formatter, "  Size: {} bytes", self.size)?;
        write!(
            formatter,
            "  Format: {}",
            String::from_utf8_lossy(&self.format)
        )
    }
}

impl Display for FormatChunk {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "Format chunk")?;
        writeln!(formatter, "  ID: {}", String::from_utf8_lossy(&self.id))?;
        writeln!(formatter, "  Size: {} bytes", self.size)?;
        writeln!(formatter, "  Audio format: {}", self.audio_format)?;
        writeln!(formatter, "  Channels: {}", self.number_of_channels)?;
        writeln!(formatter, "  Sample rate: {} Hz", self.sample_rate)?;
        writeln!(formatter, "  Byte rate: {} bytes/second", self.byte_rate)?;
        writeln!(formatter, "  Block alignment: {} bytes", self.block_align)?;
        write!(formatter, "  Bits per sample: {}", self.bits_per_sample)
    }
}

impl Display for BextChunkLocation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Header => write!(formatter, "header"),
            Self::Footer => write!(formatter, "footer"),
        }
    }
}

impl Display for BextChunk {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "BEXT chunk")?;
        writeln!(formatter, "  Location: {}", self.location)?;
        writeln!(formatter, "  ID: {}", String::from_utf8_lossy(&self.id))?;
        writeln!(formatter, "  Size: {} bytes", self.size)?;
        writeln!(formatter, "  Description: {}", self.description)?;
        writeln!(formatter, "  Originator: {}", self.originator)?;
        writeln!(
            formatter,
            "  Originator reference: {}",
            self.originator_reference
        )?;
        writeln!(formatter, "  Origination date: {}", self.origination_date)?;
        writeln!(formatter, "  Origination time: {}", self.origination_time)?;
        writeln!(
            formatter,
            "  Time reference (low): {}",
            self.time_reference_low
        )?;
        writeln!(
            formatter,
            "  Time reference (high): {}",
            self.time_reference_high
        )?;
        write!(formatter, "  Version: {}", self.version)
    }
}

impl Display for DataChunk {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "Data chunk")?;
        writeln!(formatter, "  ID: {}", String::from_utf8_lossy(&self.id))?;
        writeln!(formatter, "  Size: {} bytes", self.size)?;
        write!(formatter, "  Start offset: {} bytes", self.start_offset)
    }
}

#[cfg(test)]
mod tests {
    use super::{WavError, WavFile};

    #[test]
    fn creates_empty_stereo_pcm_file() {
        let wav_file = WavFile::new(2, 48_000, 24).expect("valid PCM settings");

        assert_eq!(wav_file.length_in_bytes, 44);
        assert_eq!(wav_file.file_chunk.id, *b"RIFF");
        assert_eq!(wav_file.file_chunk.size, 36);
        assert_eq!(wav_file.file_chunk.format, *b"WAVE");
        assert_eq!(wav_file.format_chunk.id, *b"fmt ");
        assert_eq!(wav_file.format_chunk.size, 16);
        assert_eq!(wav_file.format_chunk.audio_format, 1);
        assert_eq!(wav_file.format_chunk.number_of_channels, 2);
        assert_eq!(wav_file.format_chunk.sample_rate, 48_000);
        assert_eq!(wav_file.format_chunk.byte_rate, 288_000);
        assert_eq!(wav_file.format_chunk.block_align, 6);
        assert_eq!(wav_file.format_chunk.bits_per_sample, 24);
        assert!(wav_file.bext_chunk.is_none());
        assert_eq!(wav_file.data_chunk.id, *b"data");
        assert_eq!(wav_file.data_chunk.size, 0);
        assert_eq!(wav_file.data_chunk.start_offset, 44);
        assert_eq!(wav_file.number_of_samples, 0);
        assert_eq!(wav_file.samples, Some(Vec::new()));
    }

    #[test]
    fn rejects_unsupported_pcm_settings() {
        assert!(matches!(
            WavFile::new(3, 48_000, 24),
            Err(WavError::UnsupportedChannelCount(3))
        ));
        assert!(matches!(
            WavFile::new(2, 0, 24),
            Err(WavError::InvalidSampleRate(0))
        ));
        assert!(matches!(
            WavFile::new(2, 48_000, 12),
            Err(WavError::UnsupportedBitsPerSample(12))
        ));
    }

    #[test]
    fn rejects_byte_rates_that_do_not_fit_in_the_wav_header() {
        assert!(matches!(
            WavFile::new(2, u32::MAX, 32),
            Err(WavError::ByteRateOverflow { .. })
        ));
    }
}
