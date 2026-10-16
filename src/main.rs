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
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use clap::{CommandFactory, Parser};

    use super::{Cli, Command};

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
}
