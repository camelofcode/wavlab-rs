//! Command-line interface definitions and generated reference documentation.

use std::fmt::Write as _;
use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};

/// The `wavlab` command-line interface.
#[derive(Debug, Parser)]
#[command(version, about = "Inspect and process PCM WAV files")]
pub struct Cli {
    /// The operation to perform.
    #[command(subcommand)]
    pub command: Command,
}

/// An operation supported by the `wavlab` executable.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate a mono sine-wave WAV file.
    SineWave {
        /// Destination WAV file.
        output: PathBuf,
        /// Frequency in Hz.
        frequency: f64,
        /// Output sample rate in Hz.
        sample_rate: u32,
        /// Output integer PCM bit depth: 8, 16, 24, or 32.
        bit_depth: u16,
        /// Output length in milliseconds.
        length_ms: u32,
    },

    /// Generate a mono square-wave WAV file directly.
    SquareWave {
        /// Destination WAV file.
        output: PathBuf,
        /// Frequency in Hz.
        frequency: f64,
        /// Output sample rate in Hz.
        sample_rate: u32,
        /// Output integer PCM bit depth: 8, 16, 24, or 32.
        bit_depth: u16,
        /// Output length in milliseconds.
        length_ms: u32,
    },

    /// Generate a mono square-wave WAV file by additive synthesis.
    SquareWaveAdditive {
        /// Destination WAV file.
        output: PathBuf,
        /// Fundamental frequency in Hz.
        frequency: f64,
        /// Output sample rate in Hz.
        sample_rate: u32,
        /// Output integer PCM bit depth: 8, 16, 24, or 32.
        bit_depth: u16,
        /// Output length in milliseconds.
        length_ms: u32,
        /// Maximum odd harmonics to sum; frequencies at or above Nyquist are omitted.
        #[arg(default_value_t = 1_000)]
        harmonics: u32,
    },

    /// Generate a mono triangle-wave WAV file directly.
    TriangleWave {
        /// Destination WAV file.
        output: PathBuf,
        /// Frequency in Hz.
        frequency: f64,
        /// Output sample rate in Hz.
        sample_rate: u32,
        /// Output integer PCM bit depth: 8, 16, 24, or 32.
        bit_depth: u16,
        /// Output length in milliseconds.
        length_ms: u32,
    },

    /// Generate a mono triangle-wave WAV file by additive synthesis.
    TriangleWaveAdditive {
        /// Destination WAV file.
        output: PathBuf,
        /// Fundamental frequency in Hz.
        frequency: f64,
        /// Output sample rate in Hz.
        sample_rate: u32,
        /// Output integer PCM bit depth: 8, 16, 24, or 32.
        bit_depth: u16,
        /// Output length in milliseconds.
        length_ms: u32,
        /// Maximum odd harmonics to sum; frequencies at or above Nyquist are omitted.
        #[arg(default_value_t = 1_000)]
        harmonics: u32,
    },

    /// Generate a mono sawtooth-wave WAV file directly.
    SawtoothWave {
        /// Destination WAV file.
        output: PathBuf,
        /// Frequency in Hz.
        frequency: f64,
        /// Output sample rate in Hz.
        sample_rate: u32,
        /// Output integer PCM bit depth: 8, 16, 24, or 32.
        bit_depth: u16,
        /// Output length in milliseconds.
        length_ms: u32,
    },

    /// Generate a mono sawtooth-wave WAV file by additive synthesis.
    SawtoothWaveAdditive {
        /// Destination WAV file.
        output: PathBuf,
        /// Fundamental frequency in Hz.
        frequency: f64,
        /// Output sample rate in Hz.
        sample_rate: u32,
        /// Output integer PCM bit depth: 8, 16, 24, or 32.
        bit_depth: u16,
        /// Output length in milliseconds.
        length_ms: u32,
        /// Maximum harmonics to sum; frequencies at or above Nyquist are omitted.
        #[arg(default_value_t = 1_000)]
        harmonics: u32,
    },

    /// Generate a mono white-noise WAV file.
    WhiteNoise {
        /// Destination WAV file.
        output: PathBuf,
        /// Output sample rate in Hz.
        sample_rate: u32,
        /// Output integer PCM bit depth: 8, 16, 24, or 32.
        bit_depth: u16,
        /// Output length in milliseconds.
        length_ms: u32,
    },

    /// Generate a mono pink-noise WAV file.
    PinkNoise {
        /// Destination WAV file.
        output: PathBuf,
        /// Output sample rate in Hz.
        sample_rate: u32,
        /// Output integer PCM bit depth: 8, 16, 24, or 32.
        bit_depth: u16,
        /// Output length in milliseconds.
        length_ms: u32,
    },

    /// Load and display a WAV file's metadata.
    Metadata {
        /// WAV file to inspect.
        file: PathBuf,

        /// Load and scan samples to include peak and RMS levels.
        #[arg(short, long)]
        full: bool,
    },

    /// Normalize the peak level to 0 dBFS.
    Normalize {
        /// WAV file to process.
        input: PathBuf,

        /// Destination WAV file; defaults to overwriting the input file.
        output: Option<PathBuf>,
    },

    /// Normalize the peak to a specified level in dBFS.
    NormalizeDb {
        /// WAV file to process.
        input: PathBuf,

        /// Target peak level in dBFS. Positive values may cause clipping.
        #[arg(allow_hyphen_values = true)]
        target_db: f64,

        /// Destination WAV file; defaults to overwriting the input file.
        output: Option<PathBuf>,
    },

    /// Multiply every sample by a linear gain factor.
    Gain {
        /// WAV file to process.
        input: PathBuf,

        /// Linear gain factor. Magnitudes greater than 1 may cause clipping.
        #[arg(allow_hyphen_values = true)]
        gain: f64,

        /// Destination WAV file; defaults to overwriting the input file.
        output: Option<PathBuf>,
    },

    /// Apply an amplitude gain measured in decibels.
    GainDb {
        /// WAV file to process.
        input: PathBuf,

        /// Gain in dB. Positive values may cause clipping.
        #[arg(allow_hyphen_values = true)]
        gain_db: f64,

        /// Destination WAV file; defaults to overwriting the input file.
        output: Option<PathBuf>,
    },

    /// Apply a peaking EQ boost or cut around a center frequency.
    #[command(name = "eq-peak")]
    EqPeak {
        /// WAV file to process.
        input: PathBuf,

        /// Center frequency of the affected band, in Hz.
        frequency: f64,

        /// Gain at the center frequency, in dB.
        #[arg(allow_hyphen_values = true)]
        gain_db: f64,

        /// Width of the affected band, in octaves.
        bandwidth: f64,

        /// Destination WAV file; defaults to overwriting the input file.
        output: Option<PathBuf>,
    },

    /// Apply a low-shelf EQ boost or cut.
    #[command(name = "eq-lowshelf")]
    EqLowShelf {
        /// WAV file to process.
        input: PathBuf,

        /// Midpoint of the shelf transition, in Hz.
        frequency: f64,

        /// Gain below the shelf transition, in dB.
        #[arg(allow_hyphen_values = true)]
        gain_db: f64,

        /// Quality factor controlling the shape of the transition.
        q: f64,

        /// Destination WAV file; defaults to overwriting the input file.
        output: Option<PathBuf>,
    },

    /// Apply a high-shelf EQ boost or cut.
    #[command(name = "eq-highshelf")]
    EqHighShelf {
        /// WAV file to process.
        input: PathBuf,

        /// Midpoint of the shelf transition, in Hz.
        frequency: f64,

        /// Gain above the shelf transition, in dB.
        #[arg(allow_hyphen_values = true)]
        gain_db: f64,

        /// Quality factor controlling the shape of the transition.
        q: f64,

        /// Destination WAV file; defaults to overwriting the input file.
        output: Option<PathBuf>,
    },

    /// Apply an all-pass filter that changes phase while preserving magnitude.
    #[command(name = "eq-allpass")]
    EqAllPass {
        /// WAV file to process.
        input: PathBuf,

        /// Center frequency of the phase transition, in Hz.
        frequency: f64,

        /// Width of the phase transition, in octaves.
        bandwidth: f64,

        /// Destination WAV file; defaults to overwriting the input file.
        output: Option<PathBuf>,
    },

    /// Apply a constant-skirt-gain band-pass filter.
    #[command(name = "eq-bandpass")]
    EqBandPass {
        /// WAV file to process.
        input: PathBuf,

        /// Center frequency of the pass band, in Hz.
        frequency: f64,

        /// Width of the pass band, in octaves.
        bandwidth: f64,

        /// Destination WAV file; defaults to overwriting the input file.
        output: Option<PathBuf>,
    },

    /// Apply a 12 dB-per-octave low-pass filter.
    #[command(name = "eq-lowpass")]
    EqLowPass {
        /// WAV file to process.
        input: PathBuf,

        /// Cutoff frequency, in Hz.
        frequency: f64,

        /// Quality factor controlling resonance around the cutoff.
        q: f64,

        /// Destination WAV file; defaults to overwriting the input file.
        output: Option<PathBuf>,
    },

    /// Apply a 12 dB-per-octave high-pass filter.
    #[command(name = "eq-highpass")]
    EqHighPass {
        /// WAV file to process.
        input: PathBuf,

        /// Cutoff frequency, in Hz.
        frequency: f64,

        /// Quality factor controlling resonance around the cutoff.
        q: f64,

        /// Destination WAV file; defaults to overwriting the input file.
        output: Option<PathBuf>,
    },

    /// Reverse the audio while preserving the channel order in each frame.
    Reverse {
        /// WAV file to process.
        input: PathBuf,

        /// Destination WAV file; defaults to overwriting the input file.
        output: Option<PathBuf>,
    },

    /// Invert the polarity of every sample.
    FlipPhase {
        /// WAV file to process.
        input: PathBuf,

        /// Destination WAV file; defaults to overwriting the input file.
        output: Option<PathBuf>,
    },

    /// Swap the left and right channels of a stereo file.
    SwapChannels {
        /// WAV file to process.
        input: PathBuf,

        /// Destination WAV file; defaults to overwriting the input file.
        output: Option<PathBuf>,
    },

    /// Split a stereo WAV file into separate mono left and right files.
    SplitStereo {
        /// Stereo WAV file to split.
        input: PathBuf,

        /// Destination for the left channel; omit both outputs to derive them.
        #[arg(requires = "right_output")]
        left_output: Option<PathBuf>,

        /// Destination for the right channel; omit both outputs to derive them.
        #[arg(requires = "left_output")]
        right_output: Option<PathBuf>,
    },

    /// Split a stereo WAV file into separate mono mid and side files.
    SplitMidSide {
        /// Stereo WAV file to split.
        input: PathBuf,

        /// Destination for the mid signal; omit both outputs to derive them.
        #[arg(requires = "side_output")]
        mid_output: Option<PathBuf>,

        /// Destination for the side signal; omit both outputs to derive them.
        #[arg(requires = "mid_output")]
        side_output: Option<PathBuf>,
    },

    /// Split a WAV file at one or more millisecond positions.
    Split {
        /// WAV file to split.
        input: PathBuf,

        /// Increasing split positions in milliseconds from the start.
        #[arg(required = true, num_args = 1..)]
        split_times_ms: Vec<u64>,
    },

    /// Combine separate mono left and right WAV files into stereo.
    LeftRightToStereo {
        /// Mono left-channel input file.
        left_input: PathBuf,

        /// Mono right-channel input file.
        right_input: PathBuf,

        /// Destination stereo WAV file.
        output: PathBuf,
    },

    /// Decode separate mono mid and side WAV files into stereo.
    MidSideToStereo {
        /// Mono mid-signal input file.
        mid_input: PathBuf,

        /// Mono side-signal input file.
        side_input: PathBuf,

        /// Destination stereo WAV file.
        output: PathBuf,
    },

    /// Mix two or more WAV files starting at the first frame.
    Mix {
        /// Destination mixed WAV file.
        output: PathBuf,

        /// Input WAV files to mix.
        #[arg(required = true, num_args = 2..)]
        inputs: Vec<PathBuf>,
    },

    /// Join two or more compatible WAV files end-to-end.
    Join {
        /// Destination joined WAV file.
        output: PathBuf,

        /// Input WAV files to join in order.
        #[arg(required = true, num_args = 2..)]
        inputs: Vec<PathBuf>,
    },
}

/// Renders Markdown containing the long help for the CLI and every subcommand.
pub fn reference_markdown() -> String {
    let mut markdown = String::from(
        "<!-- This file is generated by `cargo run --example generate-cli-docs`. -->\n\
         <!-- Do not edit it manually; update the doc comments in `src/cli.rs`. -->\n\n\
         # Command-line reference\n\n\
         This reference is generated from the same Clap definitions used by the `wavlab` executable.\n",
    );

    render_command(&mut markdown, Cli::command(), "wavlab");
    markdown
}

fn render_command(markdown: &mut String, command: clap::Command, invocation: &str) {
    let subcommands = command
        .get_subcommands()
        .filter(|subcommand| subcommand.get_name() != "help" && !subcommand.is_hide_set())
        .cloned()
        .collect::<Vec<_>>();

    let mut help_command = command.bin_name(invocation);
    let mut help = Vec::new();
    help_command
        .write_long_help(&mut help)
        .expect("writing Clap help to memory cannot fail");
    let help = String::from_utf8(help).expect("Clap help is valid UTF-8");
    let help = help
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");

    write!(markdown, "\n## `{invocation}`\n\n```text\n{}\n```\n", help)
        .expect("writing Markdown to a string cannot fail");

    for subcommand in subcommands {
        let subcommand_invocation = format!("{invocation} {}", subcommand.get_name());
        render_command(markdown, subcommand, &subcommand_invocation);
    }
}

#[cfg(test)]
mod tests {
    use super::reference_markdown;

    #[test]
    fn checked_in_cli_reference_is_current() {
        assert_eq!(include_str!("../docs/cli.md"), reference_markdown());
    }
}
