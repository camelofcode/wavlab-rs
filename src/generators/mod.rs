mod error;
mod noise;
mod sawtooth;
mod sine;
mod square;
mod triangle;

pub use error::GeneratorError;

use crate::WavFile;

fn validate_frequency(wav_file: &WavFile, frequency: f64) -> Result<(), GeneratorError> {
    let nyquist_frequency = f64::from(wav_file.sample_rate()) / 2.0;
    if frequency.is_finite() && frequency > 0.0 && frequency < nyquist_frequency {
        Ok(())
    } else {
        Err(GeneratorError::InvalidFrequency(frequency))
    }
}

fn validate_harmonics(harmonics: u32) -> Result<(), GeneratorError> {
    if harmonics > 0 {
        Ok(())
    } else {
        Err(GeneratorError::InvalidHarmonicCount(harmonics))
    }
}

fn generate_samples(
    wav_file: &mut WavFile,
    length_milliseconds: u32,
    mut generate_frame: impl FnMut(usize) -> f64,
) -> Result<(), GeneratorError> {
    let frame_count = u64::from(wav_file.sample_rate()) * u64::from(length_milliseconds) / 1_000;
    let frame_count = usize::try_from(frame_count)
        .map_err(|_| GeneratorError::SizeOverflow("generated frame count"))?;
    let channels = usize::from(wav_file.number_of_channels());
    let sample_count = frame_count
        .checked_mul(channels)
        .ok_or(GeneratorError::SizeOverflow("generated sample count"))?;
    let mut samples = Vec::with_capacity(sample_count);

    for frame in 0..frame_count {
        samples.extend(std::iter::repeat_n(generate_frame(frame), channels));
    }

    wav_file.set_samples(samples)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{GeneratorError, WavFile};

    fn mono_test_file() -> WavFile {
        WavFile::new(1, 8, 16).unwrap()
    }

    fn assert_samples_close(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len());
        for (&actual, &expected) in actual.iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 1e-12,
                "expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn direct_generators_produce_one_cycle_at_the_requested_frequency() {
        let mut wav_file = mono_test_file();

        wav_file.generate_square_wave_pure(1_000, 1.0).unwrap();
        assert_eq!(
            wav_file.samples().unwrap(),
            &[1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0]
        );

        wav_file.generate_triangle_wave_pure(1_000, 1.0).unwrap();
        assert_samples_close(
            wav_file.samples().unwrap(),
            &[0.0, 0.5, 1.0, 0.5, 0.0, -0.5, -1.0, -0.5],
        );

        wav_file.generate_sawtooth_wave_pure(1_000, 1.0).unwrap();
        assert_eq!(
            wav_file.samples().unwrap(),
            &[0.0, 0.25, 0.5, 0.75, -1.0, -0.75, -0.5, -0.25]
        );
    }

    #[test]
    fn additive_generators_reject_zero_harmonics() {
        let mut wav_file = mono_test_file();

        assert!(matches!(
            wav_file.generate_square_wave_additive(1_000, 1.0, 0),
            Err(GeneratorError::InvalidHarmonicCount(0))
        ));
        assert!(matches!(
            wav_file.generate_triangle_wave_additive(1_000, 1.0, 0),
            Err(GeneratorError::InvalidHarmonicCount(0))
        ));
        assert!(matches!(
            wav_file.generate_sawtooth_wave_additive(1_000, 1.0, 0),
            Err(GeneratorError::InvalidHarmonicCount(0))
        ));
    }

    #[test]
    fn tonal_generators_reject_invalid_frequencies() {
        let mut wav_file = mono_test_file();

        assert!(matches!(
            wav_file.generate_sine_wave(1_000, 0.0),
            Err(GeneratorError::InvalidFrequency(0.0))
        ));
        assert!(matches!(
            wav_file.generate_sine_wave(1_000, f64::NAN),
            Err(GeneratorError::InvalidFrequency(value)) if value.is_nan()
        ));
        assert!(matches!(
            wav_file.generate_sine_wave(1_000, 4.0),
            Err(GeneratorError::InvalidFrequency(4.0))
        ));
    }
}
