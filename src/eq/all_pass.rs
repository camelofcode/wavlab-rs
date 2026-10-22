use super::{Eq, calculate_alpha_for_bandwidth, set_pass_denominator};
use crate::{WavError, WavFile};

impl Eq {
    /// Creates an all-pass filter.
    ///
    /// The filter preserves the magnitude of every frequency while changing
    /// phase around the center frequency.
    ///
    /// # Arguments
    ///
    /// - `center_frequency`: Center of the phase transition, in hertz. It
    ///   should be greater than zero and below half the sample rate.
    /// - `sample_rate`: Number of samples per second, in hertz.
    /// - `bandwidth`: Width of the phase transition, in octaves. It must be
    ///   positive.
    pub fn all_pass(center_frequency: f64, sample_rate: u32, bandwidth: f64) -> Eq {
        Self::new(center_frequency, sample_rate, |coefficients| {
            calculate_alpha_for_bandwidth(coefficients, bandwidth);
            coefficients.b0 = 1.0 - coefficients.alpha;
            coefficients.b1 = -2.0 * coefficients.cos_omega;
            coefficients.b2 = 1.0 + coefficients.alpha;
            set_pass_denominator(coefficients);
        })
    }

    /// Updates this filter's all-pass coefficients without clearing its delay state.
    ///
    /// This is useful for continuously modulated filters: the previous input
    /// and output samples remain intact while the phase-transition frequency
    /// moves. The arguments have the same requirements as [`Eq::all_pass`].
    pub fn set_all_pass(&mut self, center_frequency: f64, sample_rate: u32, bandwidth: f64) {
        let updated = Self::all_pass(center_frequency, sample_rate, bandwidth);
        self.b0a0 = updated.b0a0;
        self.b1a0 = updated.b1a0;
        self.b2a0 = updated.b2a0;
        self.a1a0 = updated.a1a0;
        self.a2a0 = updated.a2a0;
    }
}

impl WavFile {
    /// Applies an all-pass filter to the loaded samples in place.
    ///
    /// The file's sample rate is used to construct the filter. Each channel is
    /// processed by an independent filter so that interleaved channel samples
    /// do not share delay state. Samples are not clipped after processing.
    ///
    /// # Arguments
    ///
    /// - `center_frequency`: Center of the phase transition, in hertz.
    /// - `bandwidth`: Width of the phase transition, in octaves.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::SamplesNotLoaded`] if this file was opened with
    /// [`WavFile::open_metadata`].
    pub fn apply_all_pass_eq(
        &mut self,
        center_frequency: f64,
        bandwidth: f64,
    ) -> Result<(), WavError> {
        self.apply_eq(|sample_rate| Eq::all_pass(center_frequency, sample_rate, bandwidth))
    }
}

#[cfg(test)]
mod tests {
    use super::Eq;

    #[test]
    fn updating_coefficients_preserves_filter_state() {
        let mut filter = Eq::all_pass(1_000.0, 48_000, 1.0);
        let _ = filter.process_sample(0.75);
        let state = (filter.in1, filter.in2, filter.out1, filter.out2);

        filter.set_all_pass(2_000.0, 48_000, 1.0);

        assert_eq!((filter.in1, filter.in2, filter.out1, filter.out2), state);
    }
}
