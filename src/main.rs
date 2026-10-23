use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use wavlab::cli::{
    Cli, Command, LfoEffectArgs, ModulatedFilterArgs, MonoChorusArgs, MonoDelayArgs,
    MonoFlangerArgs, MonoPhaserArgs, MonoVibratoArgs, StereoChorusArgs, StereoDelayArgs,
    StereoFlangerArgs, StereoPhaserArgs, StereoVibratoArgs,
};
use wavlab::{
    AutoPanSettings, ChorusChannelSettings, DelayChannelSettings, EffectError,
    FlangerChannelSettings, GeneratorError, ModulatedFilterSettings, PhaserChannelSettings,
    TremoloSettings, VibratoChannelSettings, WavError, WavFile,
};

type CliResult<T> = Result<T, Box<dyn std::error::Error>>;

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> CliResult<()> {
    match cli.command {
        Command::SineWave {
            output,
            frequency,
            sample_rate,
            bit_depth,
            length_ms,
        } => generate_file(output, sample_rate, bit_depth, |wav_file| {
            wav_file.generate_sine_wave(length_ms, frequency)
        })?,
        Command::SquareWave {
            output,
            frequency,
            sample_rate,
            bit_depth,
            length_ms,
        } => generate_file(output, sample_rate, bit_depth, |wav_file| {
            wav_file.generate_square_wave_pure(length_ms, frequency)
        })?,
        Command::SquareWaveAdditive {
            output,
            frequency,
            sample_rate,
            bit_depth,
            length_ms,
            harmonics,
        } => generate_file(output, sample_rate, bit_depth, |wav_file| {
            wav_file.generate_square_wave_additive(length_ms, frequency, harmonics)
        })?,
        Command::TriangleWave {
            output,
            frequency,
            sample_rate,
            bit_depth,
            length_ms,
        } => generate_file(output, sample_rate, bit_depth, |wav_file| {
            wav_file.generate_triangle_wave_pure(length_ms, frequency)
        })?,
        Command::TriangleWaveAdditive {
            output,
            frequency,
            sample_rate,
            bit_depth,
            length_ms,
            harmonics,
        } => generate_file(output, sample_rate, bit_depth, |wav_file| {
            wav_file.generate_triangle_wave_additive(length_ms, frequency, harmonics)
        })?,
        Command::SawtoothWave {
            output,
            frequency,
            sample_rate,
            bit_depth,
            length_ms,
        } => generate_file(output, sample_rate, bit_depth, |wav_file| {
            wav_file.generate_sawtooth_wave_pure(length_ms, frequency)
        })?,
        Command::SawtoothWaveAdditive {
            output,
            frequency,
            sample_rate,
            bit_depth,
            length_ms,
            harmonics,
        } => generate_file(output, sample_rate, bit_depth, |wav_file| {
            wav_file.generate_sawtooth_wave_additive(length_ms, frequency, harmonics)
        })?,
        Command::WhiteNoise {
            output,
            sample_rate,
            bit_depth,
            length_ms,
        } => generate_file(output, sample_rate, bit_depth, |wav_file| {
            wav_file.generate_white_noise(length_ms)
        })?,
        Command::PinkNoise {
            output,
            sample_rate,
            bit_depth,
            length_ms,
        } => generate_file(output, sample_rate, bit_depth, |wav_file| {
            wav_file.generate_pink_noise(length_ms)
        })?,
        Command::Metadata { file, full } => {
            let wav_file = if full {
                WavFile::open(file)?
            } else {
                WavFile::open_metadata(file)?
            };
            println!("{}", metadata_report(&wav_file, full)?);
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
        Command::Saturate {
            input,
            drive_db,
            output,
        } => {
            process_file(input, output, |wav_file| wav_file.tape_saturation(drive_db))?;
        }
        Command::DigitalDelayMono(delay) => {
            let MonoDelayArgs {
                input,
                delay_ms,
                feedback,
                mix,
                output,
                tail_ms,
            } = delay;
            process_effect_file(input, output, |wav_file| {
                wav_file.digital_delay(DelayChannelSettings::new(delay_ms, feedback), mix, tail_ms)
            })?;
        }
        Command::DigitalDelayStereo(delay) => {
            let StereoDelayArgs {
                input,
                left_delay_ms,
                left_feedback,
                right_delay_ms,
                right_feedback,
                mix,
                output,
                tail_ms,
            } = delay;
            process_effect_file(input, output, |wav_file| {
                wav_file.digital_delay_stereo(
                    DelayChannelSettings::new(left_delay_ms, left_feedback),
                    DelayChannelSettings::new(right_delay_ms, right_feedback),
                    mix,
                    tail_ms,
                )
            })?;
        }
        Command::AnalogDelayMono { delay, cutoff_hz } => {
            let MonoDelayArgs {
                input,
                delay_ms,
                feedback,
                mix,
                output,
                tail_ms,
            } = delay;
            process_effect_file(input, output, |wav_file| {
                wav_file.analog_delay_with_cutoff(
                    DelayChannelSettings::new(delay_ms, feedback),
                    mix,
                    cutoff_hz,
                    tail_ms,
                )
            })?;
        }
        Command::AnalogDelayStereo { delay, cutoff_hz } => {
            let StereoDelayArgs {
                input,
                left_delay_ms,
                left_feedback,
                right_delay_ms,
                right_feedback,
                mix,
                output,
                tail_ms,
            } = delay;
            process_effect_file(input, output, |wav_file| {
                wav_file.analog_delay_stereo_with_cutoff(
                    DelayChannelSettings::new(left_delay_ms, left_feedback),
                    DelayChannelSettings::new(right_delay_ms, right_feedback),
                    mix,
                    cutoff_hz,
                    tail_ms,
                )
            })?;
        }
        Command::TapeDelayMono {
            delay,
            drive_db,
            cutoff_hz,
        } => {
            let MonoDelayArgs {
                input,
                delay_ms,
                feedback,
                mix,
                output,
                tail_ms,
            } = delay;
            process_effect_file(input, output, |wav_file| {
                wav_file.tape_delay_with_cutoff(
                    DelayChannelSettings::new(delay_ms, feedback),
                    mix,
                    cutoff_hz,
                    drive_db,
                    tail_ms,
                )
            })?;
        }
        Command::TapeDelayStereo {
            delay,
            drive_db,
            cutoff_hz,
        } => {
            let StereoDelayArgs {
                input,
                left_delay_ms,
                left_feedback,
                right_delay_ms,
                right_feedback,
                mix,
                output,
                tail_ms,
            } = delay;
            process_effect_file(input, output, |wav_file| {
                wav_file.tape_delay_stereo_with_cutoff(
                    DelayChannelSettings::new(left_delay_ms, left_feedback),
                    DelayChannelSettings::new(right_delay_ms, right_feedback),
                    mix,
                    cutoff_hz,
                    drive_db,
                    tail_ms,
                )
            })?;
        }
        Command::ChorusMono(chorus) => {
            let MonoChorusArgs {
                input,
                depth,
                frequency_hz,
                mix,
                output,
            } = chorus;
            process_effect_file(input, output, |wav_file| {
                wav_file.chorus(ChorusChannelSettings::new(depth, frequency_hz), mix)
            })?;
        }
        Command::ChorusStereo(chorus) => {
            let StereoChorusArgs {
                input,
                left_depth,
                left_frequency_hz,
                right_depth,
                right_frequency_hz,
                mix,
                output,
            } = chorus;
            process_effect_file(input, output, |wav_file| {
                wav_file.chorus_stereo(
                    ChorusChannelSettings::new(left_depth, left_frequency_hz),
                    ChorusChannelSettings::new(right_depth, right_frequency_hz),
                    mix,
                )
            })?;
        }
        Command::VibratoMono(vibrato) => {
            let MonoVibratoArgs {
                input,
                depth,
                frequency_hz,
                output,
            } = vibrato;
            process_effect_file(input, output, |wav_file| {
                wav_file.vibrato(VibratoChannelSettings::new(depth, frequency_hz))
            })?;
        }
        Command::VibratoStereo(vibrato) => {
            let StereoVibratoArgs {
                input,
                left_depth,
                left_frequency_hz,
                right_depth,
                right_frequency_hz,
                output,
            } = vibrato;
            process_effect_file(input, output, |wav_file| {
                wav_file.vibrato_stereo(
                    VibratoChannelSettings::new(left_depth, left_frequency_hz),
                    VibratoChannelSettings::new(right_depth, right_frequency_hz),
                )
            })?;
        }
        Command::FlangerMono(flanger) => {
            let MonoFlangerArgs {
                input,
                depth,
                frequency_hz,
                feedback,
                mix,
                output,
            } = flanger;
            process_effect_file(input, output, |wav_file| {
                wav_file.flanger(
                    FlangerChannelSettings::new(depth, frequency_hz, feedback),
                    mix,
                )
            })?;
        }
        Command::FlangerStereo(flanger) => {
            let StereoFlangerArgs {
                input,
                left_depth,
                left_frequency_hz,
                left_feedback,
                right_depth,
                right_frequency_hz,
                right_feedback,
                mix,
                output,
            } = flanger;
            process_effect_file(input, output, |wav_file| {
                wav_file.flanger_stereo(
                    FlangerChannelSettings::new(left_depth, left_frequency_hz, left_feedback),
                    FlangerChannelSettings::new(right_depth, right_frequency_hz, right_feedback),
                    mix,
                )
            })?;
        }
        Command::Tremolo(tremolo) => {
            let LfoEffectArgs {
                input,
                depth,
                frequency_hz,
                waveform,
                output,
            } = tremolo;
            process_effect_file(input, output, |wav_file| {
                wav_file.tremolo(TremoloSettings::new(depth, frequency_hz, waveform.into()))
            })?;
        }
        Command::Autopan(autopan) => {
            let LfoEffectArgs {
                input,
                depth,
                frequency_hz,
                waveform,
                output,
            } = autopan;
            process_effect_file(input, output, |wav_file| {
                wav_file.autopan(AutoPanSettings::new(depth, frequency_hz, waveform.into()))
            })?;
        }
        Command::PhaserMono(phaser) => {
            let MonoPhaserArgs {
                input,
                depth,
                frequency_hz,
                center_frequency_hz,
                waveform,
                mix,
                output,
                stages,
            } = phaser;
            process_effect_file(input, output, |wav_file| {
                wav_file.phaser(
                    PhaserChannelSettings::new(
                        depth,
                        frequency_hz,
                        center_frequency_hz,
                        waveform.into(),
                    ),
                    stages,
                    mix,
                )
            })?;
        }
        Command::PhaserStereo(phaser) => {
            let StereoPhaserArgs {
                input,
                left_depth,
                left_frequency_hz,
                left_center_frequency_hz,
                left_waveform,
                right_depth,
                right_frequency_hz,
                right_center_frequency_hz,
                right_waveform,
                mix,
                output,
                stages,
            } = phaser;
            process_effect_file(input, output, |wav_file| {
                wav_file.phaser_stereo(
                    PhaserChannelSettings::new(
                        left_depth,
                        left_frequency_hz,
                        left_center_frequency_hz,
                        left_waveform.into(),
                    ),
                    PhaserChannelSettings::new(
                        right_depth,
                        right_frequency_hz,
                        right_center_frequency_hz,
                        right_waveform.into(),
                    ),
                    stages,
                    mix,
                )
            })?;
        }
        Command::ModulatedFilter(filter) => {
            let ModulatedFilterArgs {
                input,
                minimum_frequency_hz,
                maximum_frequency_hz,
                frequency_hz,
                filter_type,
                waveform,
                q,
                output,
            } = filter;
            process_effect_file(input, output, |wav_file| {
                wav_file.modulated_filter(ModulatedFilterSettings::new(
                    minimum_frequency_hz,
                    maximum_frequency_hz,
                    frequency_hz,
                    filter_type.into(),
                    waveform.into(),
                    q,
                ))
            })?;
        }
        Command::Reverb {
            input,
            room_size,
            decay_seconds,
            mix,
            output,
            tail_ms,
        } => {
            process_effect_file(input, output, |wav_file| {
                wav_file.reverb(room_size, decay_seconds, mix, tail_ms)
            })?;
        }
        Command::EqPeak {
            input,
            frequency,
            gain_db,
            bandwidth,
            output,
        } => {
            process_file(input, output, |wav_file| {
                wav_file.apply_peak_eq(frequency, bandwidth, gain_db)
            })?;
        }
        Command::EqLowShelf {
            input,
            frequency,
            gain_db,
            q,
            output,
        } => {
            process_file(input, output, |wav_file| {
                wav_file.apply_low_shelf_eq(frequency, q, gain_db)
            })?;
        }
        Command::EqHighShelf {
            input,
            frequency,
            gain_db,
            q,
            output,
        } => {
            process_file(input, output, |wav_file| {
                wav_file.apply_high_shelf_eq(frequency, q, gain_db)
            })?;
        }
        Command::EqAllPass {
            input,
            frequency,
            bandwidth,
            output,
        } => {
            process_file(input, output, |wav_file| {
                wav_file.apply_all_pass_eq(frequency, bandwidth)
            })?;
        }
        Command::EqBandPass {
            input,
            frequency,
            bandwidth,
            output,
        } => {
            process_file(input, output, |wav_file| {
                wav_file.apply_band_pass_eq(frequency, bandwidth)
            })?;
        }
        Command::EqLowPass {
            input,
            frequency,
            q,
            output,
        } => {
            process_file(input, output, |wav_file| {
                wav_file.apply_low_pass_eq(frequency, q)
            })?;
        }
        Command::EqHighPass {
            input,
            frequency,
            q,
            output,
        } => {
            process_file(input, output, |wav_file| {
                wav_file.apply_high_pass_eq(frequency, q)
            })?;
        }
        Command::Reverse { input, output } => {
            process_file(input, output, WavFile::reverse)?;
        }
        Command::FlipPhase { input, output } => {
            process_file(input, output, WavFile::flip_phase)?;
        }
        Command::SwapChannels { input, output } => {
            process_file(input, output, WavFile::swap_channels)?;
        }
        Command::SplitStereo {
            input,
            left_output,
            right_output,
        } => {
            let (left_output, right_output) = match (left_output, right_output) {
                (Some(left), Some(right)) => (left, right),
                (None, None) => default_split_stereo_outputs(&input),
                _ => unreachable!("clap requires both split-stereo output paths or neither"),
            };
            let stereo = WavFile::open(input)?;
            let (left, right) = stereo.split_stereo()?;
            left.save(left_output)?;
            right.save(right_output)?;
        }
        Command::SplitMidSide {
            input,
            mid_output,
            side_output,
        } => {
            let (mid_output, side_output) = match (mid_output, side_output) {
                (Some(mid), Some(side)) => (mid, side),
                (None, None) => default_split_mid_side_outputs(&input),
                _ => unreachable!("clap requires both split-mid-side output paths or neither"),
            };
            let stereo = WavFile::open(input)?;
            let (mid, side) = stereo.split_mid_side()?;
            mid.save(mid_output)?;
            side.save(side_output)?;
        }
        Command::Split {
            input,
            split_times_ms,
        } => {
            let wav_file = WavFile::open(&input)?;
            let outputs = wav_file.split_at_times(&split_times_ms)?;

            for (index, output) in outputs.into_iter().enumerate() {
                output.save(path_with_stem_suffix(&input, &format!("_{}", index + 1)))?;
            }
        }
        Command::LeftRightToStereo {
            left_input,
            right_input,
            output,
        } => {
            let left = WavFile::open(left_input)?;
            let right = WavFile::open(right_input)?;
            WavFile::combine_left_right(&left, &right)?.save(output)?;
        }
        Command::MidSideToStereo {
            mid_input,
            side_input,
            output,
        } => {
            let mid = WavFile::open(mid_input)?;
            let side = WavFile::open(side_input)?;
            WavFile::combine_mid_side(&mid, &side)?.save(output)?;
        }
        Command::Mix { output, inputs } => {
            WavFile::mix(&open_files(inputs)?)?.save(output)?;
        }
        Command::Join { output, inputs } => {
            WavFile::join(&open_files(inputs)?)?.save(output)?;
        }
    }

    Ok(())
}

fn generate_file(
    output: PathBuf,
    sample_rate: u32,
    bit_depth: u16,
    generate: impl FnOnce(&mut WavFile) -> Result<(), GeneratorError>,
) -> Result<(), GeneratorError> {
    let mut wav_file = WavFile::new(1, sample_rate, bit_depth)?;
    generate(&mut wav_file)?;
    wav_file.save(output)?;
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

fn process_effect_file(
    input: PathBuf,
    output: Option<PathBuf>,
    process: impl FnOnce(&mut WavFile) -> Result<(), EffectError>,
) -> CliResult<()> {
    let output = output.unwrap_or_else(|| input.clone());
    let mut wav_file = WavFile::open(input)?;
    process(&mut wav_file)?;
    wav_file.save(output)?;
    Ok(())
}

fn open_files(paths: Vec<PathBuf>) -> Result<Vec<WavFile>, WavError> {
    paths.into_iter().map(WavFile::open).collect()
}

fn metadata_report(wav_file: &WavFile, full: bool) -> Result<String, WavError> {
    use std::fmt::Write as _;

    let mut report = wav_file.to_string();
    if full {
        let peak_db = wav_file.peak_db()?;
        let rms_db = wav_file.rms_db()?;
        write!(
            report,
            "\n\nLevels\n  Peak: {peak_db:.2} dBFS\n  RMS: {rms_db:.2} dBFS"
        )
        .expect("writing to a string cannot fail");
    }
    Ok(report)
}

fn default_split_stereo_outputs(input: &Path) -> (PathBuf, PathBuf) {
    (
        path_with_stem_suffix(input, "_l"),
        path_with_stem_suffix(input, "_r"),
    )
}

fn default_split_mid_side_outputs(input: &Path) -> (PathBuf, PathBuf) {
    (
        path_with_stem_suffix(input, "_mid"),
        path_with_stem_suffix(input, "_side"),
    )
}

fn path_with_stem_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut file_name = path.file_stem().unwrap_or_default().to_os_string();
    file_name.push(suffix);

    if let Some(extension) = path.extension() {
        file_name.push(".");
        file_name.push(extension);
    }

    path.with_file_name(file_name)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use clap::{CommandFactory, Parser};
    use wavlab::cli::{LfoWaveformArg, ModulatedFilterTypeArg};
    use wavlab::{
        AutoPanSettings, ChorusChannelSettings, DelayChannelSettings, EffectError,
        FlangerChannelSettings, LfoWaveform, ModulatedFilterSettings, ModulatedFilterType,
        PhaserChannelSettings, TremoloSettings, VibratoChannelSettings, WavError, WavFile,
        constants::MINUS_THREE_DB,
    };

    use super::{
        Cli, Command, LfoEffectArgs, ModulatedFilterArgs, MonoChorusArgs, MonoDelayArgs,
        MonoFlangerArgs, MonoPhaserArgs, MonoVibratoArgs, StereoChorusArgs, StereoDelayArgs,
        StereoFlangerArgs, StereoPhaserArgs, StereoVibratoArgs, run,
    };

    static NEXT_TEST_FILE: AtomicU64 = AtomicU64::new(0);

    struct TestFiles(Vec<PathBuf>);

    impl TestFiles {
        fn new() -> Self {
            Self(Vec::new())
        }

        fn path(&mut self, name: &str) -> PathBuf {
            let id = NEXT_TEST_FILE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("wavlab-cli-{}-{id}-{name}.wav", std::process::id()));
            self.track(path)
        }

        fn track(&mut self, path: PathBuf) -> PathBuf {
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

    fn save_file(path: &Path, number_of_channels: u16, samples: Vec<f64>) {
        let mut wav_file =
            WavFile::new(number_of_channels, 48_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(samples)
            .expect("complete sample frames");
        wav_file.save(path).expect("save test WAV file");
    }

    fn save_mono_file(path: &Path, samples: Vec<f64>) {
        save_file(path, 1, samples);
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

    fn assert_processing_command_matches(
        input: &Path,
        output: &Path,
        command: Command,
        process: impl FnOnce(&mut WavFile) -> Result<(), WavError>,
    ) {
        let mut expected = WavFile::open(input).expect("open expected input");
        process(&mut expected).expect("process expected samples");

        run(Cli { command }).expect("run processing command");

        let actual = WavFile::open(output).expect("open command output");
        assert_samples_approximately_eq(actual.samples().unwrap(), expected.samples().unwrap());
    }

    fn assert_effect_command_matches(
        input: &Path,
        output: &Path,
        command: Command,
        process: impl FnOnce(&mut WavFile) -> Result<(), EffectError>,
    ) {
        let mut expected = WavFile::open(input).expect("open expected input");
        process(&mut expected).expect("process expected samples");

        run(Cli { command }).expect("run effect command");

        let actual = WavFile::open(output).expect("open command output");
        assert_eq!(actual.number_of_channels(), expected.number_of_channels());
        assert_samples_approximately_eq(actual.samples().unwrap(), expected.samples().unwrap());
    }

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_metadata_command() {
        let cli = Cli::try_parse_from(["wavlab", "metadata", "example.wav"])
            .expect("valid metadata command");

        assert!(matches!(
            cli.command,
            Command::Metadata { file, full: false } if file == Path::new("example.wav")
        ));

        let full = Cli::try_parse_from(["wavlab", "metadata", "--full", "example.wav"])
            .expect("valid full metadata command");
        assert!(matches!(
            full.command,
            Command::Metadata { file, full: true } if file == Path::new("example.wav")
        ));

        let short = Cli::try_parse_from(["wavlab", "metadata", "-f", "example.wav"])
            .expect("valid short full metadata command");
        assert!(matches!(
            short.command,
            Command::Metadata { file, full: true } if file == Path::new("example.wav")
        ));
    }

    #[test]
    fn full_metadata_report_includes_peak_and_rms_levels() {
        let mut wav_file = WavFile::new(1, 48_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![0.5, -0.5, 0.0, 0.0])
            .expect("complete sample frames");

        let full = super::metadata_report(&wav_file, true).expect("loaded samples");
        let summary = super::metadata_report(&wav_file, false).expect("metadata report");

        assert!(full.contains("Samples: 4 loaded"));
        assert!(full.contains("Peak: -6.02 dBFS"));
        assert!(full.contains("RMS: -9.03 dBFS"));
        assert!(!summary.contains("Levels"));
    }

    #[test]
    fn parses_eq_commands() {
        let peak = Cli::try_parse_from([
            "wavlab", "eq-peak", "in.wav", "1000", "-6", "1.5", "out.wav",
        ])
        .expect("valid peak EQ command");
        assert!(matches!(
            peak.command,
            Command::EqPeak {
                input,
                frequency,
                gain_db,
                bandwidth,
                output: Some(output),
            } if input == Path::new("in.wav")
                && frequency == 1_000.0
                && gain_db == -6.0
                && bandwidth == 1.5
                && output == Path::new("out.wav")
        ));

        let low_shelf =
            Cli::try_parse_from(["wavlab", "eq-lowshelf", "in.wav", "200", "4.5", "0.7"])
                .expect("valid low-shelf EQ command");
        assert!(matches!(
            low_shelf.command,
            Command::EqLowShelf {
                input,
                frequency,
                gain_db,
                q,
                output: None,
            } if input == Path::new("in.wav")
                && frequency == 200.0
                && gain_db == 4.5
                && q == 0.7
        ));

        let high_shelf = Cli::try_parse_from([
            "wavlab",
            "eq-highshelf",
            "in.wav",
            "8000",
            "-3",
            "1",
            "out.wav",
        ])
        .expect("valid high-shelf EQ command");
        assert!(matches!(
            high_shelf.command,
            Command::EqHighShelf {
                input,
                frequency,
                gain_db,
                q,
                output: Some(output),
            } if input == Path::new("in.wav")
                && frequency == 8_000.0
                && gain_db == -3.0
                && q == 1.0
                && output == Path::new("out.wav")
        ));

        let all_pass = Cli::try_parse_from(["wavlab", "eq-allpass", "in.wav", "1000", "1"])
            .expect("valid all-pass EQ command");
        assert!(matches!(
            all_pass.command,
            Command::EqAllPass {
                input,
                frequency,
                bandwidth,
                output: None,
            } if input == Path::new("in.wav")
                && frequency == 1_000.0
                && bandwidth == 1.0
        ));

        let band_pass =
            Cli::try_parse_from(["wavlab", "eq-bandpass", "in.wav", "1000", "1", "out.wav"])
                .expect("valid band-pass EQ command");
        assert!(matches!(
            band_pass.command,
            Command::EqBandPass {
                input,
                frequency,
                bandwidth,
                output: Some(output),
            } if input == Path::new("in.wav")
                && frequency == 1_000.0
                && bandwidth == 1.0
                && output == Path::new("out.wav")
        ));

        let low_pass = Cli::try_parse_from(["wavlab", "eq-lowpass", "in.wav", "500", "0.75"])
            .expect("valid low-pass EQ command");
        assert!(matches!(
            low_pass.command,
            Command::EqLowPass {
                input,
                frequency,
                q,
                output: None,
            } if input == Path::new("in.wav")
                && frequency == 500.0
                && q == 0.75
        ));

        let high_pass =
            Cli::try_parse_from(["wavlab", "eq-highpass", "in.wav", "80", "0.75", "out.wav"])
                .expect("valid high-pass EQ command");
        assert!(matches!(
            high_pass.command,
            Command::EqHighPass {
                input,
                frequency,
                q,
                output: Some(output),
            } if input == Path::new("in.wav")
                && frequency == 80.0
                && q == 0.75
                && output == Path::new("out.wav")
        ));
    }

    #[test]
    fn parses_processing_commands() {
        let normalize = Cli::try_parse_from(["wavlab", "normalize", "in.wav"])
            .expect("valid normalize command");
        assert!(matches!(
            normalize.command,
            Command::Normalize { input, output: None } if input == Path::new("in.wav")
        ));

        let normalize_db =
            Cli::try_parse_from(["wavlab", "normalize-db", "in.wav", "-6.0", "out.wav"])
                .expect("valid normalize-db command");
        assert!(matches!(
            normalize_db.command,
            Command::NormalizeDb { input, target_db, output: Some(output) }
                if input == Path::new("in.wav")
                    && target_db == -6.0
                    && output == Path::new("out.wav")
        ));

        let gain = Cli::try_parse_from(["wavlab", "gain", "in.wav", "2", "out.wav"])
            .expect("valid gain command");
        assert!(matches!(
            gain.command,
            Command::Gain { input, gain, output: Some(output) }
                if input == Path::new("in.wav")
                    && gain == 2.0
                    && output == Path::new("out.wav")
        ));

        let gain_db = Cli::try_parse_from(["wavlab", "gain-db", "in.wav", "-3"])
            .expect("valid gain-db command");
        assert!(matches!(
            gain_db.command,
            Command::GainDb { input, gain_db, output: None }
                if input == Path::new("in.wav") && gain_db == -3.0
        ));

        let saturate = Cli::try_parse_from(["wavlab", "saturate", "in.wav", "-0.25", "out.wav"])
            .expect("valid saturate command");
        assert!(matches!(
            saturate.command,
            Command::Saturate { input, drive_db, output: Some(output) }
                if input == Path::new("in.wav")
                    && drive_db == -0.25
                    && output == Path::new("out.wav")
        ));

        let reverse = Cli::try_parse_from(["wavlab", "reverse", "in.wav", "out.wav"])
            .expect("valid reverse command");
        assert!(matches!(
            reverse.command,
            Command::Reverse { input, output: Some(output) }
                if input == Path::new("in.wav") && output == Path::new("out.wav")
        ));

        let flip_phase = Cli::try_parse_from(["wavlab", "flip-phase", "in.wav"])
            .expect("valid flip-phase command");
        assert!(matches!(
            flip_phase.command,
            Command::FlipPhase { input, output: None } if input == Path::new("in.wav")
        ));

        let swap_channels = Cli::try_parse_from(["wavlab", "swap-channels", "in.wav", "out.wav"])
            .expect("valid swap-channels command");
        assert!(matches!(
            swap_channels.command,
            Command::SwapChannels { input, output: Some(output) }
                if input == Path::new("in.wav") && output == Path::new("out.wav")
        ));

        let split_stereo =
            Cli::try_parse_from(["wavlab", "split-stereo", "in.wav", "left.wav", "right.wav"])
                .expect("valid split-stereo command");
        assert!(matches!(
            split_stereo.command,
            Command::SplitStereo {
                input,
                left_output: Some(left_output),
                right_output: Some(right_output),
            }
                if input == Path::new("in.wav")
                    && left_output == Path::new("left.wav")
                    && right_output == Path::new("right.wav")
        ));

        let split_stereo_with_defaults = Cli::try_parse_from(["wavlab", "split-stereo", "in.wav"])
            .expect("valid split-stereo command using default outputs");
        assert!(matches!(
            split_stereo_with_defaults.command,
            Command::SplitStereo {
                input,
                left_output: None,
                right_output: None,
            } if input == Path::new("in.wav")
        ));

        assert!(
            Cli::try_parse_from(["wavlab", "split-stereo", "in.wav", "left.wav"]).is_err(),
            "split-stereo should require both output paths or neither"
        );

        let split_mid_side =
            Cli::try_parse_from(["wavlab", "split-mid-side", "in.wav", "mid.wav", "side.wav"])
                .expect("valid split-mid-side command");
        assert!(matches!(
            split_mid_side.command,
            Command::SplitMidSide {
                input,
                mid_output: Some(mid_output),
                side_output: Some(side_output),
            }
                if input == Path::new("in.wav")
                    && mid_output == Path::new("mid.wav")
                    && side_output == Path::new("side.wav")
        ));

        let split_mid_side_with_defaults =
            Cli::try_parse_from(["wavlab", "split-mid-side", "in.wav"])
                .expect("valid split-mid-side command using default outputs");
        assert!(matches!(
            split_mid_side_with_defaults.command,
            Command::SplitMidSide {
                input,
                mid_output: None,
                side_output: None,
            } if input == Path::new("in.wav")
        ));

        assert!(
            Cli::try_parse_from(["wavlab", "split-mid-side", "in.wav", "mid.wav"]).is_err(),
            "split-mid-side should require both output paths or neither"
        );

        let split = Cli::try_parse_from(["wavlab", "split", "in.wav", "100", "250"])
            .expect("valid split command");
        assert!(matches!(
            split.command,
            Command::Split { input, split_times_ms }
                if input == Path::new("in.wav") && split_times_ms == [100, 250]
        ));

        assert!(
            Cli::try_parse_from(["wavlab", "split", "in.wav"]).is_err(),
            "split should require at least one time"
        );

        let left_right = Cli::try_parse_from([
            "wavlab",
            "left-right-to-stereo",
            "left.wav",
            "right.wav",
            "out.wav",
        ])
        .expect("valid left-right-to-stereo command");
        assert!(matches!(
            left_right.command,
            Command::LeftRightToStereo { left_input, right_input, output }
                if left_input == Path::new("left.wav")
                    && right_input == Path::new("right.wav")
                    && output == Path::new("out.wav")
        ));

        let mid_side = Cli::try_parse_from([
            "wavlab",
            "mid-side-to-stereo",
            "mid.wav",
            "side.wav",
            "out.wav",
        ])
        .expect("valid mid-side-to-stereo command");
        assert!(matches!(
            mid_side.command,
            Command::MidSideToStereo { mid_input, side_input, output }
                if mid_input == Path::new("mid.wav")
                    && side_input == Path::new("side.wav")
                    && output == Path::new("out.wav")
        ));

        let mix = Cli::try_parse_from(["wavlab", "mix", "out.wav", "first.wav", "second.wav"])
            .expect("valid mix command");
        assert!(matches!(
            mix.command,
            Command::Mix { output, inputs }
                if output == Path::new("out.wav")
                    && inputs == [PathBuf::from("first.wav"), PathBuf::from("second.wav")]
        ));

        let join = Cli::try_parse_from(["wavlab", "join", "out.wav", "first.wav", "second.wav"])
            .expect("valid join command");
        assert!(matches!(
            join.command,
            Command::Join { output, inputs }
                if output == Path::new("out.wav")
                    && inputs == [PathBuf::from("first.wav"), PathBuf::from("second.wav")]
        ));

        assert!(
            Cli::try_parse_from(["wavlab", "mix", "out.wav", "only.wav"]).is_err(),
            "mix should require at least two inputs"
        );
        assert!(
            Cli::try_parse_from(["wavlab", "join", "out.wav", "only.wav"]).is_err(),
            "join should require at least two inputs"
        );
    }

    #[test]
    fn parses_delay_commands() {
        let digital_mono = Cli::try_parse_from([
            "wavlab",
            "digital-delay-mono",
            "in.wav",
            "250.5",
            "0.4",
            "0.5",
            "out.wav",
            "--tail-ms",
            "1000",
        ])
        .expect("valid mono digital delay");
        assert!(matches!(
            digital_mono.command,
            Command::DigitalDelayMono(MonoDelayArgs {
                input,
                delay_ms: 250.5,
                feedback: 0.4,
                mix: 0.5,
                output: Some(output),
                tail_ms: 1000,
            }) if input == Path::new("in.wav") && output == Path::new("out.wav")
        ));

        let digital_stereo = Cli::try_parse_from([
            "wavlab",
            "digital-delay-stereo",
            "in.wav",
            "200",
            "0.3",
            "350",
            "0.4",
            "0.5",
        ])
        .expect("valid stereo digital delay");
        assert!(matches!(
            digital_stereo.command,
            Command::DigitalDelayStereo(StereoDelayArgs {
                input,
                left_delay_ms: 200.0,
                left_feedback: 0.3,
                right_delay_ms: 350.0,
                right_feedback: 0.4,
                mix: 0.5,
                output: None,
                tail_ms: 0,
            }) if input == Path::new("in.wav")
        ));

        let analog_mono = Cli::try_parse_from([
            "wavlab",
            "analog-delay-mono",
            "in.wav",
            "250",
            "0.4",
            "0.5",
            "--cutoff-hz",
            "3000",
        ])
        .expect("valid mono analog delay");
        assert!(matches!(
            analog_mono.command,
            Command::AnalogDelayMono {
                delay: MonoDelayArgs {
                    delay_ms: 250.0,
                    ..
                },
                cutoff_hz: 3_000.0,
            }
        ));

        let analog_stereo = Cli::try_parse_from([
            "wavlab",
            "analog-delay-stereo",
            "in.wav",
            "200",
            "0.3",
            "350",
            "0.4",
            "0.5",
        ])
        .expect("valid stereo analog delay");
        assert!(matches!(
            analog_stereo.command,
            Command::AnalogDelayStereo {
                cutoff_hz: 2_000.0,
                ..
            }
        ));

        let tape_mono = Cli::try_parse_from([
            "wavlab",
            "tape-delay-mono",
            "in.wav",
            "250",
            "0.4",
            "0.5",
            "--drive-db",
            "6",
        ])
        .expect("valid mono tape delay");
        assert!(matches!(
            tape_mono.command,
            Command::TapeDelayMono {
                drive_db: 6.0,
                cutoff_hz: 2_000.0,
                ..
            }
        ));

        let tape_stereo = Cli::try_parse_from([
            "wavlab",
            "tape-delay-stereo",
            "in.wav",
            "200",
            "0.3",
            "350",
            "0.4",
            "0.5",
            "--drive-db",
            "-3",
            "--cutoff-hz",
            "1500",
        ])
        .expect("valid stereo tape delay");
        assert!(matches!(
            tape_stereo.command,
            Command::TapeDelayStereo {
                drive_db: -3.0,
                cutoff_hz: 1_500.0,
                ..
            }
        ));
    }

    #[test]
    fn parses_chorus_commands() {
        let mono = Cli::try_parse_from([
            "wavlab",
            "chorus-mono",
            "in.wav",
            "0.75",
            "1.5",
            "0.5",
            "out.wav",
        ])
        .expect("valid mono chorus");
        assert!(matches!(
            mono.command,
            Command::ChorusMono(MonoChorusArgs {
                input,
                depth: 0.75,
                frequency_hz: 1.5,
                mix: 0.5,
                output: Some(output),
            }) if input == Path::new("in.wav") && output == Path::new("out.wav")
        ));

        let stereo = Cli::try_parse_from([
            "wavlab",
            "chorus-stereo",
            "in.wav",
            "0.5",
            "0.8",
            "1",
            "1.2",
            "0.4",
        ])
        .expect("valid stereo chorus");
        assert!(matches!(
            stereo.command,
            Command::ChorusStereo(StereoChorusArgs {
                input,
                left_depth: 0.5,
                left_frequency_hz: 0.8,
                right_depth: 1.0,
                right_frequency_hz: 1.2,
                mix: 0.4,
                output: None,
            }) if input == Path::new("in.wav")
        ));
    }

    #[test]
    fn parses_vibrato_commands() {
        let mono =
            Cli::try_parse_from(["wavlab", "vibrato-mono", "in.wav", "0.75", "5", "out.wav"])
                .expect("valid mono vibrato");
        assert!(matches!(
            mono.command,
            Command::VibratoMono(MonoVibratoArgs {
                input,
                depth: 0.75,
                frequency_hz: 5.0,
                output: Some(output),
            }) if input == Path::new("in.wav") && output == Path::new("out.wav")
        ));

        let stereo =
            Cli::try_parse_from(["wavlab", "vibrato-stereo", "in.wav", "0.5", "4", "1", "6"])
                .expect("valid stereo vibrato");
        assert!(matches!(
            stereo.command,
            Command::VibratoStereo(StereoVibratoArgs {
                input,
                left_depth: 0.5,
                left_frequency_hz: 4.0,
                right_depth: 1.0,
                right_frequency_hz: 6.0,
                output: None,
            }) if input == Path::new("in.wav")
        ));
    }

    #[test]
    fn parses_flanger_commands() {
        let mono = Cli::try_parse_from([
            "wavlab",
            "flanger-mono",
            "in.wav",
            "0.75",
            "0.5",
            "-0.7",
            "0.5",
            "out.wav",
        ])
        .expect("valid mono flanger");
        assert!(matches!(
            mono.command,
            Command::FlangerMono(MonoFlangerArgs {
                input,
                depth: 0.75,
                frequency_hz: 0.5,
                feedback: -0.7,
                mix: 0.5,
                output: Some(output),
            }) if input == Path::new("in.wav") && output == Path::new("out.wav")
        ));

        let stereo = Cli::try_parse_from([
            "wavlab",
            "flanger-stereo",
            "in.wav",
            "0.5",
            "0.4",
            "-0.6",
            "1",
            "0.7",
            "0.5",
            "0.5",
        ])
        .expect("valid stereo flanger");
        assert!(matches!(
            stereo.command,
            Command::FlangerStereo(StereoFlangerArgs {
                input,
                left_depth: 0.5,
                left_frequency_hz: 0.4,
                left_feedback: -0.6,
                right_depth: 1.0,
                right_frequency_hz: 0.7,
                right_feedback: 0.5,
                mix: 0.5,
                output: None,
            }) if input == Path::new("in.wav")
        ));
    }

    #[test]
    fn parses_tremolo_and_autopan_commands() {
        let tremolo = Cli::try_parse_from([
            "wavlab", "tremolo", "in.wav", "0.75", "5", "triangle", "out.wav",
        ])
        .expect("valid tremolo");
        assert!(matches!(
            tremolo.command,
            Command::Tremolo(LfoEffectArgs {
                input,
                depth: 0.75,
                frequency_hz: 5.0,
                waveform: LfoWaveformArg::Triangle,
                output: Some(output),
            }) if input == Path::new("in.wav") && output == Path::new("out.wav")
        ));

        let autopan = Cli::try_parse_from(["wavlab", "autopan", "in.wav", "1", "0.5", "square"])
            .expect("valid autopan");
        assert!(matches!(
            autopan.command,
            Command::Autopan(LfoEffectArgs {
                input,
                depth: 1.0,
                frequency_hz: 0.5,
                waveform: LfoWaveformArg::Square,
                output: None,
            }) if input == Path::new("in.wav")
        ));

        assert!(Cli::try_parse_from(["wavlab", "tremolo", "in.wav", "1", "5", "saw"]).is_err());
    }

    #[test]
    fn parses_phaser_commands() {
        let mono = Cli::try_parse_from([
            "wavlab",
            "phaser-mono",
            "in.wav",
            "0.75",
            "0.5",
            "1000",
            "triangle",
            "0.5",
            "out.wav",
            "--stages",
            "4",
        ])
        .expect("valid mono phaser");
        assert!(matches!(
            mono.command,
            Command::PhaserMono(MonoPhaserArgs {
                input,
                depth: 0.75,
                frequency_hz: 0.5,
                center_frequency_hz: 1_000.0,
                waveform: LfoWaveformArg::Triangle,
                mix: 0.5,
                output: Some(output),
                stages: 4,
            }) if input == Path::new("in.wav") && output == Path::new("out.wav")
        ));

        let stereo = Cli::try_parse_from([
            "wavlab",
            "phaser-stereo",
            "in.wav",
            "0.5",
            "0.4",
            "800",
            "sine",
            "1",
            "0.7",
            "1200",
            "square",
            "0.5",
        ])
        .expect("valid stereo phaser");
        assert!(matches!(
            stereo.command,
            Command::PhaserStereo(StereoPhaserArgs {
                input,
                left_depth: 0.5,
                left_frequency_hz: 0.4,
                left_center_frequency_hz: 800.0,
                left_waveform: LfoWaveformArg::Sine,
                right_depth: 1.0,
                right_frequency_hz: 0.7,
                right_center_frequency_hz: 1_200.0,
                right_waveform: LfoWaveformArg::Square,
                mix: 0.5,
                output: None,
                stages: 3,
            }) if input == Path::new("in.wav")
        ));
    }

    #[test]
    fn parses_modulated_filter_command() {
        let filter = Cli::try_parse_from([
            "wavlab",
            "modulated-filter",
            "in.wav",
            "200",
            "2000",
            "1.5",
            "low-pass",
            "triangle",
            "0.8",
            "out.wav",
        ])
        .expect("valid modulated filter");

        assert!(matches!(
            filter.command,
            Command::ModulatedFilter(ModulatedFilterArgs {
                input,
                minimum_frequency_hz: 200.0,
                maximum_frequency_hz: 2_000.0,
                frequency_hz: 1.5,
                filter_type: ModulatedFilterTypeArg::LowPass,
                waveform: LfoWaveformArg::Triangle,
                q: 0.8,
                output: Some(output),
            }) if input == Path::new("in.wav") && output == Path::new("out.wav")
        ));
    }

    #[test]
    fn parses_reverb_command() {
        let reverb = Cli::try_parse_from([
            "wavlab",
            "reverb",
            "in.wav",
            "0.6",
            "2.5",
            "0.35",
            "out.wav",
            "--tail-ms",
            "3000",
        ])
        .expect("valid reverb command");

        assert!(matches!(
            reverb.command,
            Command::Reverb {
                input,
                room_size: 0.6,
                decay_seconds: 2.5,
                mix: 0.35,
                output: Some(output),
                tail_ms: 3000,
            } if input == Path::new("in.wav") && output == Path::new("out.wav")
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
        let saturation_output = files.path("saturation-output");
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
        assert_processing_command_matches(
            &input,
            &saturation_output,
            Command::Saturate {
                input: input.clone(),
                drive_db: 6.0,
                output: Some(saturation_output.clone()),
            },
            |wav_file| wav_file.tape_saturation(6.0),
        );

        let normalized = WavFile::open(normalize_db_output).expect("open normalized output");
        assert_samples_approximately_eq(normalized.samples().unwrap(), &[0.05, -0.1]);

        let gained = WavFile::open(gain_output).expect("open linear gain output");
        assert_samples_approximately_eq(gained.samples().unwrap(), &[0.05, -0.1]);

        let gained_db = WavFile::open(gain_db_output).expect("open dB gain output");
        assert_samples_approximately_eq(gained_db.samples().unwrap(), &[0.25, -0.5]);

        let original = WavFile::open(input).expect("open original input");
        assert_samples_approximately_eq(original.samples().unwrap(), &[0.025, -0.05]);
    }

    #[test]
    fn delay_commands_write_the_expected_output() {
        let mut files = TestFiles::new();
        let input = files.path("delay-input");
        save_mono_file(&input, vec![0.5, 0.0, 0.0]);

        let digital_mono_output = files.path("digital-delay-mono-output");
        assert_effect_command_matches(
            &input,
            &digital_mono_output,
            Command::DigitalDelayMono(MonoDelayArgs {
                input: input.clone(),
                delay_ms: 1.0,
                feedback: 0.5,
                mix: 0.5,
                output: Some(digital_mono_output.clone()),
                tail_ms: 2,
            }),
            |wav_file| wav_file.digital_delay(DelayChannelSettings::new(1.0, 0.5), 0.5, 2),
        );

        let digital_stereo_output = files.path("digital-delay-stereo-output");
        assert_effect_command_matches(
            &input,
            &digital_stereo_output,
            Command::DigitalDelayStereo(StereoDelayArgs {
                input: input.clone(),
                left_delay_ms: 1.0,
                left_feedback: 0.5,
                right_delay_ms: 2.0,
                right_feedback: 0.25,
                mix: 0.5,
                output: Some(digital_stereo_output.clone()),
                tail_ms: 2,
            }),
            |wav_file| {
                wav_file.digital_delay_stereo(
                    DelayChannelSettings::new(1.0, 0.5),
                    DelayChannelSettings::new(2.0, 0.25),
                    0.5,
                    2,
                )
            },
        );

        let analog_mono_output = files.path("analog-delay-mono-output");
        assert_effect_command_matches(
            &input,
            &analog_mono_output,
            Command::AnalogDelayMono {
                delay: MonoDelayArgs {
                    input: input.clone(),
                    delay_ms: 1.0,
                    feedback: 0.5,
                    mix: 0.5,
                    output: Some(analog_mono_output.clone()),
                    tail_ms: 2,
                },
                cutoff_hz: 1_500.0,
            },
            |wav_file| {
                wav_file.analog_delay_with_cutoff(
                    DelayChannelSettings::new(1.0, 0.5),
                    0.5,
                    1_500.0,
                    2,
                )
            },
        );

        let analog_stereo_output = files.path("analog-delay-stereo-output");
        assert_effect_command_matches(
            &input,
            &analog_stereo_output,
            Command::AnalogDelayStereo {
                delay: StereoDelayArgs {
                    input: input.clone(),
                    left_delay_ms: 1.0,
                    left_feedback: 0.5,
                    right_delay_ms: 2.0,
                    right_feedback: 0.25,
                    mix: 0.5,
                    output: Some(analog_stereo_output.clone()),
                    tail_ms: 2,
                },
                cutoff_hz: 1_500.0,
            },
            |wav_file| {
                wav_file.analog_delay_stereo_with_cutoff(
                    DelayChannelSettings::new(1.0, 0.5),
                    DelayChannelSettings::new(2.0, 0.25),
                    0.5,
                    1_500.0,
                    2,
                )
            },
        );

        let tape_mono_output = files.path("tape-delay-mono-output");
        assert_effect_command_matches(
            &input,
            &tape_mono_output,
            Command::TapeDelayMono {
                delay: MonoDelayArgs {
                    input: input.clone(),
                    delay_ms: 1.0,
                    feedback: 0.5,
                    mix: 0.5,
                    output: Some(tape_mono_output.clone()),
                    tail_ms: 2,
                },
                drive_db: 3.0,
                cutoff_hz: 1_500.0,
            },
            |wav_file| {
                wav_file.tape_delay_with_cutoff(
                    DelayChannelSettings::new(1.0, 0.5),
                    0.5,
                    1_500.0,
                    3.0,
                    2,
                )
            },
        );

        let tape_stereo_output = files.path("tape-delay-stereo-output");
        assert_effect_command_matches(
            &input,
            &tape_stereo_output,
            Command::TapeDelayStereo {
                delay: StereoDelayArgs {
                    input: input.clone(),
                    left_delay_ms: 1.0,
                    left_feedback: 0.5,
                    right_delay_ms: 2.0,
                    right_feedback: 0.25,
                    mix: 0.5,
                    output: Some(tape_stereo_output.clone()),
                    tail_ms: 2,
                },
                drive_db: 3.0,
                cutoff_hz: 1_500.0,
            },
            |wav_file| {
                wav_file.tape_delay_stereo_with_cutoff(
                    DelayChannelSettings::new(1.0, 0.5),
                    DelayChannelSettings::new(2.0, 0.25),
                    0.5,
                    1_500.0,
                    3.0,
                    2,
                )
            },
        );
    }

    #[test]
    fn chorus_commands_write_the_expected_output() {
        let mut files = TestFiles::new();
        let input = files.path("chorus-input");
        save_mono_file(&input, vec![0.5; 1_500]);

        let mono_output = files.path("chorus-mono-output");
        assert_effect_command_matches(
            &input,
            &mono_output,
            Command::ChorusMono(MonoChorusArgs {
                input: input.clone(),
                depth: 0.75,
                frequency_hz: 1.5,
                mix: 0.5,
                output: Some(mono_output.clone()),
            }),
            |wav_file| wav_file.chorus(ChorusChannelSettings::new(0.75, 1.5), 0.5),
        );

        let stereo_output = files.path("chorus-stereo-output");
        assert_effect_command_matches(
            &input,
            &stereo_output,
            Command::ChorusStereo(StereoChorusArgs {
                input: input.clone(),
                left_depth: 0.5,
                left_frequency_hz: 0.8,
                right_depth: 1.0,
                right_frequency_hz: 1.2,
                mix: 0.4,
                output: Some(stereo_output.clone()),
            }),
            |wav_file| {
                wav_file.chorus_stereo(
                    ChorusChannelSettings::new(0.5, 0.8),
                    ChorusChannelSettings::new(1.0, 1.2),
                    0.4,
                )
            },
        );
    }

    #[test]
    fn vibrato_commands_write_the_expected_output() {
        let mut files = TestFiles::new();
        let input = files.path("vibrato-input");
        save_mono_file(&input, vec![0.5; 1_500]);

        let mono_output = files.path("vibrato-mono-output");
        assert_effect_command_matches(
            &input,
            &mono_output,
            Command::VibratoMono(MonoVibratoArgs {
                input: input.clone(),
                depth: 0.75,
                frequency_hz: 5.0,
                output: Some(mono_output.clone()),
            }),
            |wav_file| wav_file.vibrato(VibratoChannelSettings::new(0.75, 5.0)),
        );

        let stereo_output = files.path("vibrato-stereo-output");
        assert_effect_command_matches(
            &input,
            &stereo_output,
            Command::VibratoStereo(StereoVibratoArgs {
                input: input.clone(),
                left_depth: 0.5,
                left_frequency_hz: 4.0,
                right_depth: 1.0,
                right_frequency_hz: 6.0,
                output: Some(stereo_output.clone()),
            }),
            |wav_file| {
                wav_file.vibrato_stereo(
                    VibratoChannelSettings::new(0.5, 4.0),
                    VibratoChannelSettings::new(1.0, 6.0),
                )
            },
        );
    }

    #[test]
    fn flanger_commands_write_the_expected_output() {
        let mut files = TestFiles::new();
        let input = files.path("flanger-input");
        save_mono_file(&input, vec![0.5; 1_500]);

        let mono_output = files.path("flanger-mono-output");
        assert_effect_command_matches(
            &input,
            &mono_output,
            Command::FlangerMono(MonoFlangerArgs {
                input: input.clone(),
                depth: 0.75,
                frequency_hz: 0.5,
                feedback: -0.7,
                mix: 0.5,
                output: Some(mono_output.clone()),
            }),
            |wav_file| wav_file.flanger(FlangerChannelSettings::new(0.75, 0.5, -0.7), 0.5),
        );

        let stereo_output = files.path("flanger-stereo-output");
        assert_effect_command_matches(
            &input,
            &stereo_output,
            Command::FlangerStereo(StereoFlangerArgs {
                input: input.clone(),
                left_depth: 0.5,
                left_frequency_hz: 0.4,
                left_feedback: -0.6,
                right_depth: 1.0,
                right_frequency_hz: 0.7,
                right_feedback: 0.5,
                mix: 0.5,
                output: Some(stereo_output.clone()),
            }),
            |wav_file| {
                wav_file.flanger_stereo(
                    FlangerChannelSettings::new(0.5, 0.4, -0.6),
                    FlangerChannelSettings::new(1.0, 0.7, 0.5),
                    0.5,
                )
            },
        );
    }

    #[test]
    fn tremolo_and_autopan_commands_write_the_expected_output() {
        let mut files = TestFiles::new();
        let input = files.path("lfo-effects-input");
        save_mono_file(&input, vec![0.5; 1_500]);

        let tremolo_output = files.path("tremolo-output");
        assert_effect_command_matches(
            &input,
            &tremolo_output,
            Command::Tremolo(LfoEffectArgs {
                input: input.clone(),
                depth: 0.75,
                frequency_hz: 5.0,
                waveform: LfoWaveformArg::Triangle,
                output: Some(tremolo_output.clone()),
            }),
            |wav_file| wav_file.tremolo(TremoloSettings::new(0.75, 5.0, LfoWaveform::Triangle)),
        );

        let autopan_output = files.path("autopan-output");
        assert_effect_command_matches(
            &input,
            &autopan_output,
            Command::Autopan(LfoEffectArgs {
                input: input.clone(),
                depth: 1.0,
                frequency_hz: 0.5,
                waveform: LfoWaveformArg::Square,
                output: Some(autopan_output.clone()),
            }),
            |wav_file| wav_file.autopan(AutoPanSettings::new(1.0, 0.5, LfoWaveform::Square)),
        );
    }

    #[test]
    fn phaser_commands_write_the_expected_output() {
        let mut files = TestFiles::new();
        let input = files.path("phaser-input");
        save_mono_file(&input, vec![0.5; 1_500]);

        let mono_output = files.path("phaser-mono-output");
        assert_effect_command_matches(
            &input,
            &mono_output,
            Command::PhaserMono(MonoPhaserArgs {
                input: input.clone(),
                depth: 0.75,
                frequency_hz: 0.5,
                center_frequency_hz: 1_000.0,
                waveform: LfoWaveformArg::Triangle,
                mix: 0.5,
                output: Some(mono_output.clone()),
                stages: 4,
            }),
            |wav_file| {
                wav_file.phaser(
                    PhaserChannelSettings::new(0.75, 0.5, 1_000.0, LfoWaveform::Triangle),
                    4,
                    0.5,
                )
            },
        );

        let stereo_output = files.path("phaser-stereo-output");
        assert_effect_command_matches(
            &input,
            &stereo_output,
            Command::PhaserStereo(StereoPhaserArgs {
                input: input.clone(),
                left_depth: 0.5,
                left_frequency_hz: 0.4,
                left_center_frequency_hz: 800.0,
                left_waveform: LfoWaveformArg::Sine,
                right_depth: 1.0,
                right_frequency_hz: 0.7,
                right_center_frequency_hz: 1_200.0,
                right_waveform: LfoWaveformArg::Square,
                mix: 0.5,
                output: Some(stereo_output.clone()),
                stages: 3,
            }),
            |wav_file| {
                wav_file.phaser_stereo(
                    PhaserChannelSettings::new(0.5, 0.4, 800.0, LfoWaveform::Sine),
                    PhaserChannelSettings::new(1.0, 0.7, 1_200.0, LfoWaveform::Square),
                    3,
                    0.5,
                )
            },
        );
    }

    #[test]
    fn modulated_filter_command_writes_the_expected_output() {
        let mut files = TestFiles::new();
        let input = files.path("modulated-filter-input");
        let output = files.path("modulated-filter-output");
        save_mono_file(&input, vec![0.5; 1_500]);

        assert_effect_command_matches(
            &input,
            &output,
            Command::ModulatedFilter(ModulatedFilterArgs {
                input: input.clone(),
                minimum_frequency_hz: 200.0,
                maximum_frequency_hz: 2_000.0,
                frequency_hz: 1.5,
                filter_type: ModulatedFilterTypeArg::HighPass,
                waveform: LfoWaveformArg::Sine,
                q: 0.8,
                output: Some(output.clone()),
            }),
            |wav_file| {
                wav_file.modulated_filter(ModulatedFilterSettings::new(
                    200.0,
                    2_000.0,
                    1.5,
                    ModulatedFilterType::HighPass,
                    LfoWaveform::Sine,
                    0.8,
                ))
            },
        );
    }

    #[test]
    fn reverb_command_writes_the_expected_output() {
        let mut files = TestFiles::new();
        let input = files.path("reverb-input");
        let output = files.path("reverb-output");
        save_mono_file(&input, vec![0.5, 0.0, 0.0]);

        assert_effect_command_matches(
            &input,
            &output,
            Command::Reverb {
                input: input.clone(),
                room_size: 0.5,
                decay_seconds: 1.0,
                mix: 0.4,
                output: Some(output.clone()),
                tail_ms: 100,
            },
            |wav_file| wav_file.reverb(0.5, 1.0, 0.4, 100),
        );
    }

    #[test]
    fn eq_commands_write_the_expected_output() {
        let mut files = TestFiles::new();
        let input = files.path("eq-input");
        save_file(&input, 2, vec![0.1, -0.05, 0.025, 0.0, -0.075, 0.05]);

        let peak_output = files.path("eq-peak-output");
        assert_processing_command_matches(
            &input,
            &peak_output,
            Command::EqPeak {
                input: input.clone(),
                frequency: 1_000.0,
                gain_db: 6.0,
                bandwidth: 1.0,
                output: Some(peak_output.clone()),
            },
            |wav_file| wav_file.apply_peak_eq(1_000.0, 1.0, 6.0),
        );

        let low_shelf_output = files.path("eq-lowshelf-output");
        assert_processing_command_matches(
            &input,
            &low_shelf_output,
            Command::EqLowShelf {
                input: input.clone(),
                frequency: 200.0,
                gain_db: -3.0,
                q: 1.0,
                output: Some(low_shelf_output.clone()),
            },
            |wav_file| wav_file.apply_low_shelf_eq(200.0, 1.0, -3.0),
        );

        let high_shelf_output = files.path("eq-highshelf-output");
        assert_processing_command_matches(
            &input,
            &high_shelf_output,
            Command::EqHighShelf {
                input: input.clone(),
                frequency: 8_000.0,
                gain_db: 3.0,
                q: 1.0,
                output: Some(high_shelf_output.clone()),
            },
            |wav_file| wav_file.apply_high_shelf_eq(8_000.0, 1.0, 3.0),
        );

        let all_pass_output = files.path("eq-allpass-output");
        assert_processing_command_matches(
            &input,
            &all_pass_output,
            Command::EqAllPass {
                input: input.clone(),
                frequency: 1_000.0,
                bandwidth: 1.0,
                output: Some(all_pass_output.clone()),
            },
            |wav_file| wav_file.apply_all_pass_eq(1_000.0, 1.0),
        );

        let band_pass_output = files.path("eq-bandpass-output");
        assert_processing_command_matches(
            &input,
            &band_pass_output,
            Command::EqBandPass {
                input: input.clone(),
                frequency: 1_000.0,
                bandwidth: 1.0,
                output: Some(band_pass_output.clone()),
            },
            |wav_file| wav_file.apply_band_pass_eq(1_000.0, 1.0),
        );

        let low_pass_output = files.path("eq-lowpass-output");
        assert_processing_command_matches(
            &input,
            &low_pass_output,
            Command::EqLowPass {
                input: input.clone(),
                frequency: 1_000.0,
                q: 0.75,
                output: Some(low_pass_output.clone()),
            },
            |wav_file| wav_file.apply_low_pass_eq(1_000.0, 0.75),
        );

        let high_pass_output = files.path("eq-highpass-output");
        assert_processing_command_matches(
            &input,
            &high_pass_output,
            Command::EqHighPass {
                input: input.clone(),
                frequency: 100.0,
                q: 0.75,
                output: Some(high_pass_output.clone()),
            },
            |wav_file| wav_file.apply_high_pass_eq(100.0, 0.75),
        );

        let original = WavFile::open(input).expect("open original EQ input");
        assert_samples_approximately_eq(
            original.samples().unwrap(),
            &[0.1, -0.05, 0.025, 0.0, -0.075, 0.05],
        );
    }

    #[test]
    fn frame_processing_commands_write_to_the_requested_output() {
        let mut files = TestFiles::new();
        let input = files.path("frame-processing-input");
        let reverse_output = files.path("reverse-output");
        let flip_phase_output = files.path("flip-phase-output");
        let swap_channels_output = files.path("swap-channels-output");
        save_file(&input, 2, vec![0.1, 0.2, 0.3, 0.4]);

        run(Cli {
            command: Command::Reverse {
                input: input.clone(),
                output: Some(reverse_output.clone()),
            },
        })
        .expect("reverse output");
        run(Cli {
            command: Command::FlipPhase {
                input: input.clone(),
                output: Some(flip_phase_output.clone()),
            },
        })
        .expect("flip phase output");
        run(Cli {
            command: Command::SwapChannels {
                input: input.clone(),
                output: Some(swap_channels_output.clone()),
            },
        })
        .expect("swap channels output");

        let reversed = WavFile::open(reverse_output).expect("open reversed output");
        assert_samples_approximately_eq(reversed.samples().unwrap(), &[0.3, 0.4, 0.1, 0.2]);

        let flipped = WavFile::open(flip_phase_output).expect("open flipped output");
        assert_samples_approximately_eq(flipped.samples().unwrap(), &[-0.1, -0.2, -0.3, -0.4]);

        let swapped = WavFile::open(swap_channels_output).expect("open swapped output");
        assert_samples_approximately_eq(swapped.samples().unwrap(), &[0.2, 0.1, 0.4, 0.3]);

        let original = WavFile::open(input).expect("open original input");
        assert_samples_approximately_eq(original.samples().unwrap(), &[0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn split_stereo_command_writes_two_mono_files() {
        let mut files = TestFiles::new();
        let input = files.path("split-stereo-input");
        let left_output = files.path("split-stereo-left");
        let right_output = files.path("split-stereo-right");
        save_file(&input, 2, vec![0.1, -0.2, 0.3, -0.4]);

        run(Cli {
            command: Command::SplitStereo {
                input,
                left_output: Some(left_output.clone()),
                right_output: Some(right_output.clone()),
            },
        })
        .expect("split stereo file");

        let left = WavFile::open(left_output).expect("open left output");
        let right = WavFile::open(right_output).expect("open right output");
        assert_eq!(left.number_of_channels(), 1);
        assert_eq!(right.number_of_channels(), 1);
        assert_samples_approximately_eq(left.samples().unwrap(), &[0.1, 0.3]);
        assert_samples_approximately_eq(right.samples().unwrap(), &[-0.2, -0.4]);
    }

    #[test]
    fn split_stereo_command_derives_output_paths_when_omitted() {
        let mut files = TestFiles::new();
        let input = files.path("split-stereo-default-input");
        let (left_output, right_output) = super::default_split_stereo_outputs(&input);
        files.track(left_output.clone());
        files.track(right_output.clone());
        save_file(&input, 2, vec![0.1, -0.2, 0.3, -0.4]);

        run(Cli {
            command: Command::SplitStereo {
                input,
                left_output: None,
                right_output: None,
            },
        })
        .expect("split stereo file using default outputs");

        let left = WavFile::open(left_output).expect("open default left output");
        let right = WavFile::open(right_output).expect("open default right output");
        assert_samples_approximately_eq(left.samples().unwrap(), &[0.1, 0.3]);
        assert_samples_approximately_eq(right.samples().unwrap(), &[-0.2, -0.4]);
    }

    #[test]
    fn default_split_stereo_outputs_preserve_directory_and_extension() {
        let (left, right) =
            super::default_split_stereo_outputs(Path::new("audio/example.original.wav"));

        assert_eq!(left, Path::new("audio/example.original_l.wav"));
        assert_eq!(right, Path::new("audio/example.original_r.wav"));
    }

    #[test]
    fn split_mid_side_command_derives_and_writes_output_files() {
        let mut files = TestFiles::new();
        let input = files.path("split-mid-side-input");
        let (mid_output, side_output) = super::default_split_mid_side_outputs(&input);
        files.track(mid_output.clone());
        files.track(side_output.clone());
        save_file(&input, 2, vec![0.5, 0.5, 0.5, -0.5]);

        run(Cli {
            command: Command::SplitMidSide {
                input,
                mid_output: None,
                side_output: None,
            },
        })
        .expect("split stereo file into mid and side");

        let mid = WavFile::open(mid_output).expect("open default mid output");
        let side = WavFile::open(side_output).expect("open default side output");
        assert_eq!(mid.number_of_channels(), 1);
        assert_eq!(side.number_of_channels(), 1);
        assert_samples_approximately_eq(mid.samples().unwrap(), &[MINUS_THREE_DB, 0.0]);
        assert_samples_approximately_eq(side.samples().unwrap(), &[0.0, MINUS_THREE_DB]);
    }

    #[test]
    fn split_command_writes_numbered_output_files() {
        let mut files = TestFiles::new();
        let input = files.path("split-at-times-input");
        let first_output = files.track(super::path_with_stem_suffix(&input, "_1"));
        let second_output = files.track(super::path_with_stem_suffix(&input, "_2"));
        let third_output = files.track(super::path_with_stem_suffix(&input, "_3"));
        let mut wav_file = WavFile::new(1, 1_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5])
            .expect("complete sample frames");
        wav_file.save(&input).expect("save test WAV file");

        run(Cli {
            command: Command::Split {
                input,
                split_times_ms: vec![2, 5],
            },
        })
        .expect("split file at times");

        let first = WavFile::open(first_output).expect("open first split output");
        let second = WavFile::open(second_output).expect("open second split output");
        let third = WavFile::open(third_output).expect("open third split output");
        assert_samples_approximately_eq(first.samples().unwrap(), &[0.0, 0.1]);
        assert_samples_approximately_eq(second.samples().unwrap(), &[0.2, 0.3, 0.4]);
        assert_samples_approximately_eq(third.samples().unwrap(), &[0.5]);
    }

    #[test]
    fn default_mid_side_outputs_preserve_directory_and_extension() {
        let (mid, side) =
            super::default_split_mid_side_outputs(Path::new("audio/example.original.wav"));

        assert_eq!(mid, Path::new("audio/example.original_mid.wav"));
        assert_eq!(side, Path::new("audio/example.original_side.wav"));
    }

    #[test]
    fn channel_combination_commands_write_stereo_files() {
        let mut files = TestFiles::new();
        let left_input = files.path("combine-left-input");
        let right_input = files.path("combine-right-input");
        let left_right_output = files.path("combine-left-right-output");
        let mid_input = files.path("combine-mid-input");
        let side_input = files.path("combine-side-input");
        let mid_side_output = files.path("combine-mid-side-output");
        save_mono_file(&left_input, vec![0.1, 0.2]);
        save_mono_file(&right_input, vec![-0.1]);
        save_mono_file(&mid_input, vec![0.5]);
        save_mono_file(&side_input, vec![0.25]);

        run(Cli {
            command: Command::LeftRightToStereo {
                left_input,
                right_input,
                output: left_right_output.clone(),
            },
        })
        .expect("combine left and right inputs");
        run(Cli {
            command: Command::MidSideToStereo {
                mid_input,
                side_input,
                output: mid_side_output.clone(),
            },
        })
        .expect("combine mid and side inputs");

        let left_right = WavFile::open(left_right_output).expect("open left/right output");
        assert_eq!(left_right.number_of_channels(), 2);
        assert_samples_approximately_eq(left_right.samples().unwrap(), &[0.1, -0.1, 0.2, 0.0]);

        let mid_side = WavFile::open(mid_side_output).expect("open mid/side output");
        let decode_scale = 0.5 / MINUS_THREE_DB;
        assert_eq!(mid_side.number_of_channels(), 2);
        assert_samples_approximately_eq(
            mid_side.samples().unwrap(),
            &[0.75 * decode_scale, 0.25 * decode_scale],
        );
    }

    #[test]
    fn mix_and_join_commands_write_combined_files() {
        let mut files = TestFiles::new();
        let first_input = files.path("multi-combine-first-input");
        let second_input = files.path("multi-combine-second-input");
        let mix_output = files.path("mix-output");
        let join_output = files.path("join-output");
        save_mono_file(&first_input, vec![0.5, 0.25]);
        save_mono_file(&second_input, vec![0.5]);

        run(Cli {
            command: Command::Mix {
                output: mix_output.clone(),
                inputs: vec![first_input.clone(), second_input.clone()],
            },
        })
        .expect("mix input files");
        run(Cli {
            command: Command::Join {
                output: join_output.clone(),
                inputs: vec![first_input, second_input],
            },
        })
        .expect("join input files");

        let mixed = WavFile::open(mix_output).expect("open mixed output");
        assert_samples_approximately_eq(mixed.samples().unwrap(), &[1.0, 0.25]);

        let joined = WavFile::open(join_output).expect("open joined output");
        assert_samples_approximately_eq(joined.samples().unwrap(), &[0.5, 0.25, 0.5]);
    }

    #[test]
    fn generator_commands_write_mono_files_in_the_requested_format() {
        let mut files = TestFiles::new();
        let mut commands = Vec::new();

        macro_rules! add_tonal_command {
            ($name:literal, $variant:ident $(, $harmonics:expr)?) => {{
                let output = files.path($name);
                commands.push((
                    output.clone(),
                    Command::$variant {
                        output,
                        frequency: 1_000.0,
                        sample_rate: 8_000,
                        bit_depth: 16,
                        length_ms: 1,
                        $(harmonics: $harmonics,)?
                    },
                ));
            }};
        }

        add_tonal_command!("sine", SineWave);
        add_tonal_command!("square", SquareWave);
        add_tonal_command!("square-additive", SquareWaveAdditive, 2);
        add_tonal_command!("triangle", TriangleWave);
        add_tonal_command!("triangle-additive", TriangleWaveAdditive, 2);
        add_tonal_command!("sawtooth", SawtoothWave);
        add_tonal_command!("sawtooth-additive", SawtoothWaveAdditive, 2);

        let white_output = files.path("white-noise");
        commands.push((
            white_output.clone(),
            Command::WhiteNoise {
                output: white_output,
                sample_rate: 8_000,
                bit_depth: 16,
                length_ms: 1,
            },
        ));
        let pink_output = files.path("pink-noise");
        commands.push((
            pink_output.clone(),
            Command::PinkNoise {
                output: pink_output,
                sample_rate: 8_000,
                bit_depth: 16,
                length_ms: 1,
            },
        ));

        for (output, command) in commands {
            run(Cli { command }).expect("run generator command");
            let generated = WavFile::open(output).expect("open generated output");
            assert_eq!(generated.number_of_channels(), 1);
            assert_eq!(generated.sample_rate(), 8_000);
            assert_eq!(generated.bits_per_sample(), 16);
            assert_eq!(generated.number_of_samples(), 8);
        }
    }
}
