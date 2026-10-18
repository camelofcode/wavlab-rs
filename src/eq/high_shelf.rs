use super::{Eq, calculate_amplitude, calculate_beta};
use crate::{WavError, WavFile};

impl Eq {
    /// Creates a high-shelf equalization filter.
    ///
    /// The filter has unity gain below the transition and applies the requested
    /// gain above it. A positive gain boosts high frequencies; a negative gain
    /// cuts them.
    ///
    /// # Arguments
    ///
    /// - `center_frequency`: Midpoint of the shelf transition, in hertz. It
    ///   should be greater than zero and below half the sample rate.
    /// - `sample_rate`: Number of samples per second, in hertz.
    /// - `q`: Quality factor controlling the shape of the transition. It must
    ///   be positive; higher values produce a more resonant transition.
    /// - `gain_decibels`: Gain above the shelf transition, in decibels.
    pub fn high_shelf(center_frequency: f64, sample_rate: u32, q: f64, gain_decibels: f64) -> Eq {
        Self::new(center_frequency, sample_rate, |coefficients| {
            calculate_amplitude(coefficients, gain_decibels);
            calculate_beta(coefficients, q);

            let amplitude = coefficients.amplitude;
            let cos_omega = coefficients.cos_omega;
            let beta_sin_omega = coefficients.beta * coefficients.sin_omega;

            coefficients.b0 =
                amplitude * ((amplitude + 1.0) + (amplitude - 1.0) * cos_omega + beta_sin_omega);
            coefficients.b1 =
                -2.0 * amplitude * ((amplitude - 1.0) + (amplitude + 1.0) * cos_omega);
            coefficients.b2 =
                amplitude * ((amplitude + 1.0) + (amplitude - 1.0) * cos_omega - beta_sin_omega);
            coefficients.a0 = (amplitude + 1.0) - (amplitude - 1.0) * cos_omega + beta_sin_omega;
            coefficients.a1 = 2.0 * ((amplitude - 1.0) - (amplitude + 1.0) * cos_omega);
            coefficients.a2 = (amplitude + 1.0) - (amplitude - 1.0) * cos_omega - beta_sin_omega;
        })
    }
}

impl WavFile {
    /// Applies a high-shelf equalization filter to the loaded samples in place.
    ///
    /// The file's sample rate is used to construct the filter. Each channel is
    /// processed by an independent filter so that interleaved channel samples
    /// do not share delay state. Samples are not clipped after processing.
    ///
    /// # Arguments
    ///
    /// - `center_frequency`: Midpoint of the shelf transition, in hertz.
    /// - `q`: Quality factor controlling the shape of the transition.
    /// - `gain_decibels`: Gain above the shelf transition, in decibels.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::SamplesNotLoaded`] if this file was opened with
    /// [`WavFile::open_metadata`].
    pub fn apply_high_shelf_eq(
        &mut self,
        center_frequency: f64,
        q: f64,
        gain_decibels: f64,
    ) -> Result<(), WavError> {
        self.apply_eq(|sample_rate| Eq::high_shelf(center_frequency, sample_rate, q, gain_decibels))
    }
}
