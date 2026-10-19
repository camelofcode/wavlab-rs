use std::f64::consts::FRAC_2_PI;

use crate::{constants::TAU, wav_file::WavFile};

use super::{generate_samples, validate_frequency, validate_harmonics};

impl WavFile {
    /// Replaces the samples with a directly generated triangle wave.
    ///
    /// The generated signal has a peak amplitude of `1.0` and is copied to
    /// every channel.
    ///
    /// # Errors
    ///
    /// Returns an error if `frequency` is not finite, positive, and below the
    /// Nyquist frequency, or if the sample count cannot be represented.
    pub fn generate_triangle_wave_pure(
        &mut self,
        length_milliseconds: u32,
        frequency: f64,
    ) -> Result<(), crate::WavError> {
        validate_frequency(self, frequency)?;
        let radians_per_frame = TAU * frequency / f64::from(self.sample_rate());
        generate_samples(self, length_milliseconds, |frame| {
            FRAC_2_PI * (frame as f64 * radians_per_frame).sin().asin()
        })
    }

    /// Replaces the samples with a triangle wave produced by additive synthesis.
    ///
    /// `harmonics` is the maximum number of odd harmonics to sum; components at
    /// or above the Nyquist frequency are omitted. The result is normalized to
    /// a peak amplitude of `1.0` and copied to every channel.
    ///
    /// # Errors
    ///
    /// Returns an error if `frequency` is invalid, `harmonics` is zero, the
    /// generated sample count cannot be represented, or normalization fails.
    pub fn generate_triangle_wave_additive(
        &mut self,
        length_milliseconds: u32,
        frequency: f64,
        harmonics: u32,
    ) -> Result<(), crate::WavError> {
        validate_frequency(self, frequency)?;
        validate_harmonics(harmonics)?;

        let radians_per_frame = TAU * frequency / f64::from(self.sample_rate());
        let nyquist_frequency = f64::from(self.sample_rate()) / 2.0;

        let mut coefficients = Vec::new();

        for index in 0..harmonics {
            let harmonic = 2.0 * f64::from(index) + 1.0;
            if frequency * harmonic >= nyquist_frequency {
                break;
            }

            let sign = if index.is_multiple_of(2) { 1.0 } else { -1.0 };
            let amplitude = sign / harmonic.powi(2);
            coefficients.push((harmonic, amplitude));
        }

        generate_samples(self, length_milliseconds, |frame| {
            let phase = frame as f64 * radians_per_frame;
            let mut sample = 0.0;

            for &(harmonic, amplitude) in &coefficients {
                sample += (phase * harmonic).sin() * amplitude;
            }

            sample
        })?;

        self.normalize()
    }
}
