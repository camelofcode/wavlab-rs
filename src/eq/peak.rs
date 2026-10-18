use super::{Eq, EqCoefficients, calculate_alpha_for_bandwidth, calculate_amplitude};
use crate::{WavError, WavFile};

impl Eq {
    /// Creates a peaking equalization filter.
    ///
    /// A positive gain boosts frequencies around the center frequency, while a
    /// negative gain cuts them. Frequencies far below and above the affected
    /// band retain unity gain.
    ///
    /// # Arguments
    ///
    /// - `center_frequency`: Center of the affected band, in hertz. It should
    ///   be greater than zero and below half the sample rate.
    /// - `sample_rate`: Number of samples per second, in hertz.
    /// - `bandwidth`: Width of the affected band, in octaves. It must be
    ///   positive.
    /// - `gain_decibels`: Gain at the center frequency, in decibels.
    pub fn peak(center_frequency: f64, sample_rate: u32, bandwidth: f64, gain_decibels: f64) -> Eq {
        Self::new(
            center_frequency,
            sample_rate,
            |coefficients: &mut EqCoefficients| {
                calculate_amplitude(coefficients, gain_decibels);
                calculate_alpha_for_bandwidth(coefficients, bandwidth);
                coefficients.b0 = 1.0 + coefficients.alpha * coefficients.amplitude;
                coefficients.b1 = -2.0 * coefficients.cos_omega;
                coefficients.b2 = 1.0 - coefficients.alpha * coefficients.amplitude;
                coefficients.a0 = 1.0 + coefficients.alpha / coefficients.amplitude;
                coefficients.a1 = -2.0 * coefficients.cos_omega;
                coefficients.a2 = 1.0 - coefficients.alpha / coefficients.amplitude;
            },
        )
    }
}

impl WavFile {
    /// Applies a peaking equalization filter to the loaded samples in place.
    ///
    /// The file's sample rate is used to construct the filter. Each channel is
    /// processed by an independent filter so that interleaved channel samples
    /// do not share delay state. Samples are not clipped after processing.
    ///
    /// # Arguments
    ///
    /// - `center_frequency`: Center of the affected band, in hertz.
    /// - `bandwidth`: Width of the affected band, in octaves.
    /// - `gain_decibels`: Gain at the center frequency, in decibels.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::SamplesNotLoaded`] if this file was opened with
    /// [`WavFile::open_metadata`].
    pub fn apply_peak_eq(
        &mut self,
        center_frequency: f64,
        bandwidth: f64,
        gain_decibels: f64,
    ) -> Result<(), WavError> {
        self.apply_eq(|sample_rate| {
            Eq::peak(center_frequency, sample_rate, bandwidth, gain_decibels)
        })
    }
}
