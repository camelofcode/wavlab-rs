use super::{Eq, calculate_alpha_for_q, set_pass_denominator};
use crate::{WavError, WavFile};

impl Eq {
    /// Creates a low-pass filter.
    ///
    /// Frequencies below the center frequency pass through, while frequencies
    /// above it are attenuated at 12 dB per octave.
    ///
    /// # Arguments
    ///
    /// - `center_frequency`: Cutoff frequency, in hertz. It should be greater
    ///   than zero and below half the sample rate.
    /// - `sample_rate`: Number of samples per second, in hertz.
    /// - `q`: Quality factor controlling resonance around the cutoff. It must
    ///   be positive. Use approximately `0.7071` for a Butterworth response.
    pub fn low_pass(center_frequency: f64, sample_rate: u32, q: f64) -> Eq {
        Self::new(center_frequency, sample_rate, |coefficients| {
            calculate_alpha_for_q(coefficients, q);
            coefficients.b0 = (1.0 - coefficients.cos_omega) / 2.0;
            coefficients.b1 = 1.0 - coefficients.cos_omega;
            coefficients.b2 = (1.0 - coefficients.cos_omega) / 2.0;
            set_pass_denominator(coefficients);
        })
    }

    /// Updates this filter's low-pass coefficients without clearing its delay state.
    ///
    /// This is useful for continuously modulated filters: the previous input
    /// and output samples remain intact while the cutoff frequency moves. The
    /// arguments have the same requirements as [`Eq::low_pass`].
    pub fn set_low_pass(&mut self, center_frequency: f64, sample_rate: u32, q: f64) {
        let updated = Self::low_pass(center_frequency, sample_rate, q);
        self.b0a0 = updated.b0a0;
        self.b1a0 = updated.b1a0;
        self.b2a0 = updated.b2a0;
        self.a1a0 = updated.a1a0;
        self.a2a0 = updated.a2a0;
    }
}

impl WavFile {
    /// Applies a low-pass filter to the loaded samples in place.
    ///
    /// The file's sample rate is used to construct the filter. Each channel is
    /// processed by an independent filter so that interleaved channel samples
    /// do not share delay state. Samples are not clipped after processing.
    ///
    /// # Arguments
    ///
    /// - `center_frequency`: Cutoff frequency, in hertz.
    /// - `q`: Quality factor controlling resonance around the cutoff.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::SamplesNotLoaded`] if this file was opened with
    /// [`WavFile::open_metadata`].
    pub fn apply_low_pass_eq(&mut self, center_frequency: f64, q: f64) -> Result<(), WavError> {
        self.apply_eq(|sample_rate| Eq::low_pass(center_frequency, sample_rate, q))
    }
}

#[cfg(test)]
mod tests {
    use super::Eq;

    #[test]
    fn updating_coefficients_preserves_filter_state() {
        let mut filter = Eq::low_pass(1_000.0, 48_000, 1.0);
        let _ = filter.process_sample(0.75);
        let state = (filter.in1, filter.in2, filter.out1, filter.out2);

        filter.set_low_pass(2_000.0, 48_000, 1.0);

        assert_eq!((filter.in1, filter.in2, filter.out1, filter.out2), state);
    }
}
