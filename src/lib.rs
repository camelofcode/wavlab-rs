#![warn(missing_docs)]
//! Read, inspect, create, process, and save integer PCM WAV files.
//!
//! `wavlab` supports mono and stereo WAV files with 8-, 16-, 24-, or 32-bit
//! integer PCM samples. [`WavFile::open`] loads samples as interleaved `f64`
//! values, while [`WavFile::open_metadata`] reads the file structure without
//! loading sample data.
//!
//! Processing methods operate on the samples held in memory. PCM input is
//! decoded into the range `-1.0..=1.0`, but processing can produce values
//! outside that range; those values are clipped when [`WavFile::save`] writes
//! them back to integer PCM.
//!
//! # Example
//!
//! ```no_run
//! use wavlab::{WavError, WavFile};
//!
//! fn main() -> Result<(), WavError> {
//!     let mut wav_file = WavFile::open("input.wav")?;
//!     wav_file.normalize_to_db(-1.0)?;
//!     wav_file.save("output.wav")?;
//!     Ok(())
//! }
//! ```

#[doc(hidden)]
pub mod cli;

/// Mathematical constants used by audio-processing operations.
pub mod constants;
mod eq;
mod fx;
mod generators;
mod tools;
mod wav_file;

pub use eq::Eq;
pub use fx::{
    AutoPanSettings, CHORUS_CENTER_DELAY_MILLISECONDS, ChorusChannelSettings,
    DEFAULT_DELAY_CUTOFF_HZ, DEFAULT_PHASER_STAGES, DelayChannelSettings, EffectError,
    FlangerChannelSettings, MAX_CHORUS_DEPTH_MILLISECONDS, MAX_FLANGER_DELAY_MILLISECONDS,
    MAX_FLANGER_FEEDBACK, MAX_FLANGER_FREQUENCY_HZ, MAX_PHASER_STAGES, MAX_PHASER_SWEEP_OCTAVES,
    MIN_FLANGER_FREQUENCY_HZ, PhaserChannelSettings, TremoloSettings, VibratoChannelSettings,
};
pub use generators::GeneratorError;
pub use tools::{Delay, DelayError, Lfo, LfoWaveform, ProcessingError};
pub use wav_file::{WavError, WavFile};
