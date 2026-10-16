use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use wavlab_rs::{WavError, WavFile};

#[derive(Debug, Parser)]
#[command(version, about = "Inspect and process PCM WAV files")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Load and display a WAV file's metadata.
    Metadata {
        /// WAV file to inspect.
        file: PathBuf,
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
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), WavError> {
    match cli.command {
        Command::Metadata { file } => {
            let wav_file = WavFile::open_metadata(file)?;
            println!("{wav_file}");
        }
        Command::Normalize { input, output } => {
            process_file(input, output, WavFile::normalize)?;
        }
        Command::NormalizeDb {
            input,
            target_db,
            output,
        } => {
            process_file(input, output, |wav_file| {
                wav_file.normalize_to_db(target_db)
            })?;
        }
        Command::Gain {
            input,
            gain,
            output,
        } => {
            process_file(input, output, |wav_file| wav_file.apply_gain(gain))?;
        }
        Command::GainDb {
            input,
            gain_db,
            output,
        } => {
            process_file(input, output, |wav_file| wav_file.apply_gain_db(gain_db))?;
        }
    }

    Ok(())
}

fn process_file(
    input: PathBuf,
    output: Option<PathBuf>,
    process: impl FnOnce(&mut WavFile) -> Result<(), WavError>,
) -> Result<(), WavError> {
    let output = output.unwrap_or_else(|| input.clone());
    let mut wav_file = WavFile::open(input)?;
    process(&mut wav_file)?;
    wav_file.save(output)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use clap::{CommandFactory, Parser};
    use wavlab_rs::WavFile;

    use super::{Cli, Command, run};

    static NEXT_TEST_FILE: AtomicU64 = AtomicU64::new(0);

    struct TestFiles(Vec<PathBuf>);

    impl TestFiles {
        fn new() -> Self {
            Self(Vec::new())
        }

        fn path(&mut self, name: &str) -> PathBuf {
            let id = NEXT_TEST_FILE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "wavlab-rs-cli-{}-{id}-{name}.wav",
                std::process::id()
            ));
            self.0.push(path.clone());
            path
        }
    }

    impl Drop for TestFiles {
        fn drop(&mut self) {
            for path in &self.0 {
                let _ = fs::remove_file(path);
            }
        }
    }

    fn save_mono_file(path: &Path, samples: Vec<f64>) {
        let mut wav_file = WavFile::new(1, 48_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(samples)
            .expect("complete sample frames");
        wav_file.save(path).expect("save test WAV file");
    }

    fn assert_samples_approximately_eq(actual: &[f64], expected: &[f64]) {
        const PCM_24_PROCESSING_TOLERANCE: f64 = 1e-6;

        assert_eq!(actual.len(), expected.len());
        for (&actual, &expected) in actual.iter().zip(expected) {
            assert!(
                (actual - expected).abs() <= PCM_24_PROCESSING_TOLERANCE,
                "expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_metadata_command() {
        let cli = Cli::try_parse_from(["wavlab-rs", "metadata", "example.wav"])
            .expect("valid metadata command");

        assert!(matches!(
            cli.command,
            Command::Metadata { file } if file == Path::new("example.wav")
        ));
    }

    #[test]
    fn parses_processing_commands() {
        let normalize = Cli::try_parse_from(["wavlab-rs", "normalize", "in.wav"])
            .expect("valid normalize command");
        assert!(matches!(
            normalize.command,
            Command::Normalize { input, output: None } if input == Path::new("in.wav")
        ));

        let normalize_db =
            Cli::try_parse_from(["wavlab-rs", "normalize-db", "in.wav", "-6.0", "out.wav"])
                .expect("valid normalize-db command");
        assert!(matches!(
            normalize_db.command,
            Command::NormalizeDb { input, target_db, output: Some(output) }
                if input == Path::new("in.wav")
                    && target_db == -6.0
                    && output == Path::new("out.wav")
        ));

        let gain = Cli::try_parse_from(["wavlab-rs", "gain", "in.wav", "2", "out.wav"])
            .expect("valid gain command");
        assert!(matches!(
            gain.command,
            Command::Gain { input, gain, output: Some(output) }
                if input == Path::new("in.wav")
                    && gain == 2.0
                    && output == Path::new("out.wav")
        ));

        let gain_db = Cli::try_parse_from(["wavlab-rs", "gain-db", "in.wav", "-3"])
            .expect("valid gain-db command");
        assert!(matches!(
            gain_db.command,
            Command::GainDb { input, gain_db, output: None }
                if input == Path::new("in.wav") && gain_db == -3.0
        ));
    }

    #[test]
    fn normalize_overwrites_input_when_output_is_omitted() {
        let mut files = TestFiles::new();
        let input = files.path("normalize-input");
        save_mono_file(&input, vec![0.25, -0.5]);

        run(Cli {
            command: Command::Normalize {
                input: input.clone(),
                output: None,
            },
        })
        .expect("normalize input file");

        let normalized = WavFile::open(input).expect("open normalized file");
        assert_samples_approximately_eq(normalized.samples().unwrap(), &[0.5, -1.0]);
    }

    #[test]
    fn processing_commands_write_to_the_requested_output() {
        let mut files = TestFiles::new();
        let input = files.path("processing-input");
        let normalize_db_output = files.path("normalize-db-output");
        let gain_output = files.path("gain-output");
        let gain_db_output = files.path("gain-db-output");
        save_mono_file(&input, vec![0.025, -0.05]);

        run(Cli {
            command: Command::NormalizeDb {
                input: input.clone(),
                target_db: -20.0,
                output: Some(normalize_db_output.clone()),
            },
        })
        .expect("normalize to dB output");
        run(Cli {
            command: Command::Gain {
                input: input.clone(),
                gain: 2.0,
                output: Some(gain_output.clone()),
            },
        })
        .expect("apply linear gain to output");
        run(Cli {
            command: Command::GainDb {
                input: input.clone(),
                gain_db: 20.0,
                output: Some(gain_db_output.clone()),
            },
        })
        .expect("apply dB gain to output");

        let normalized = WavFile::open(normalize_db_output).expect("open normalized output");
        assert_samples_approximately_eq(normalized.samples().unwrap(), &[0.05, -0.1]);

        let gained = WavFile::open(gain_output).expect("open linear gain output");
        assert_samples_approximately_eq(gained.samples().unwrap(), &[0.05, -0.1]);

        let gained_db = WavFile::open(gain_db_output).expect("open dB gain output");
        assert_samples_approximately_eq(gained_db.samples().unwrap(), &[0.25, -0.5]);

        let original = WavFile::open(input).expect("open original input");
        assert_samples_approximately_eq(original.samples().unwrap(), &[0.025, -0.05]);
    }
}
