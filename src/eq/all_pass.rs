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
