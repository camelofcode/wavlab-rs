use crate::{constants::TAU, wav_file::WavFile};

use super::{GeneratorError, generate_samples, validate_frequency, validate_harmonics};

impl WavFile {
    /// Replaces the samples with a directly generated rising sawtooth wave.
    ///
    /// The generated signal ranges from `-1.0` to `1.0` and is copied to every
    /// channel.
    ///
    /// # Errors
    ///
    /// Returns an error if `frequency` is not finite, positive, and below the
    /// Nyquist frequency, or if the sample count cannot be represented.
    pub fn generate_sawtooth_wave_pure(
        &mut self,
        length_milliseconds: u32,
        frequency: f64,
    ) -> Result<(), GeneratorError> {
        validate_frequency(self, frequency)?;

        let cycles_per_frame = frequency / f64::from(self.sample_rate());

        generate_samples(self, length_milliseconds, |frame| {
            let cycles = frame as f64 * cycles_per_frame;
            2.0 * (cycles - (cycles + 0.5).floor())
        })
    }

    /// Replaces the samples with a sawtooth wave produced by additive synthesis.
    ///
    /// `harmonics` is the maximum number of consecutive harmonics to sum;
    /// components at or above the Nyquist frequency are omitted. The result is
    /// normalized to a peak amplitude of `1.0` and copied to every channel.
    ///
    /// # Errors
    ///
    /// Returns an error if `frequency` is invalid, `harmonics` is zero, the
    /// generated sample count cannot be represented, or normalization fails.
    pub fn generate_sawtooth_wave_additive(
        &mut self,
        length_milliseconds: u32,
        frequency: f64,
        harmonics: u32,
    ) -> Result<(), GeneratorError> {
        validate_frequency(self, frequency)?;

        validate_harmonics(harmonics)?;

        let radians_per_frame = TAU * frequency / f64::from(self.sample_rate());
        let nyquist_frequency = f64::from(self.sample_rate()) / 2.0;
        let mut coefficients = Vec::new();

        for harmonic in 1..=harmonics {
            let harmonic = f64::from(harmonic);
            if frequency * harmonic >= nyquist_frequency {
                break;
            }

            coefficients.push((harmonic, harmonic.recip()));
        }

        generate_samples(self, length_milliseconds, |frame| {
            let phase = frame as f64 * radians_per_frame;
            let mut sample = 0.0;

            for &(harmonic, amplitude) in &coefficients {
                sample += (phase * harmonic).sin() * amplitude;
            }

            sample
        })?;

        self.normalize()?;
        Ok(())
    }
}
