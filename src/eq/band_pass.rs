use super::{Eq, calculate_alpha_for_bandwidth, set_pass_denominator};
use crate::{WavError, WavFile};

impl Eq {
    /// Creates a constant-skirt-gain band-pass filter.
    ///
    /// Frequencies around the center frequency pass through, while frequencies
    /// above and below the band are attenuated. The peak gain depends on the
    /// selected bandwidth, matching the original C++ implementation.
    ///
    /// # Arguments
    ///
    /// - `center_frequency`: Center of the pass band, in hertz. It should be
    ///   greater than zero and below half the sample rate.
    /// - `sample_rate`: Number of samples per second, in hertz.
    /// - `bandwidth`: Width of the pass band, in octaves. It must be positive.
    pub fn band_pass(center_frequency: f64, sample_rate: u32, bandwidth: f64) -> Eq {
        Self::new(center_frequency, sample_rate, |coefficients| {
            calculate_alpha_for_bandwidth(coefficients, bandwidth);
            coefficients.b0 = coefficients.sin_omega / 2.0;
            coefficients.b1 = 0.0;
            coefficients.b2 = -coefficients.sin_omega / 2.0;
            set_pass_denominator(coefficients);
        })
    }
}

impl WavFile {
    /// Applies a band-pass filter to the loaded samples in place.
    ///
    /// The file's sample rate is used to construct the filter. Each channel is
    /// processed by an independent filter so that interleaved channel samples
    /// do not share delay state. Samples are not clipped after processing.
    ///
    /// # Arguments
    ///
    /// - `center_frequency`: Center of the pass band, in hertz.
    /// - `bandwidth`: Width of the pass band, in octaves.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::SamplesNotLoaded`] if this file was opened with
    /// [`WavFile::open_metadata`].
    pub fn apply_band_pass_eq(
        &mut self,
        center_frequency: f64,
        bandwidth: f64,
    ) -> Result<(), WavError> {
        self.apply_eq(|sample_rate| Eq::band_pass(center_frequency, sample_rate, bandwidth))
    }
}
