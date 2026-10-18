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
