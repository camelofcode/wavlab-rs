use crate::{constants::TAU, wav_file::WavFile};

use super::{GeneratorError, generate_samples, validate_frequency};

impl WavFile {
    /// Replaces the samples with a sine wave of the requested length and frequency.
    ///
    /// The generated signal has a peak amplitude of `1.0`. Multichannel files
    /// receive the same signal in every channel.
    ///
    /// # Errors
    ///
    /// Returns an error if `frequency` is not finite, positive, and below the
    /// Nyquist frequency, or if the sample count cannot be represented.
    pub fn generate_sine_wave(
        &mut self,
        length_milliseconds: u32,
        frequency: f64,
    ) -> Result<(), GeneratorError> {
        validate_frequency(self, frequency)?;

        let radians_per_frame = TAU * frequency / f64::from(self.sample_rate());

        generate_samples(self, length_milliseconds, |frame| {
            (frame as f64 * radians_per_frame).sin()
        })
    }
}
