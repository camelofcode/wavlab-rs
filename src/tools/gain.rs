use crate::wav_file::{WavError, WavFile};

impl WavFile {
    /// Multiplies every loaded sample by a linear gain factor.
    ///
    /// A gain of `1.0` leaves the samples unchanged, `0.0` produces silence,
    /// and a negative gain also reverses their polarity. Samples are not
    /// clamped after applying the gain. A gain whose magnitude is greater than
    /// `1.0` can therefore move samples outside `-1.0..=1.0`; those samples
    /// will be clipped when the file is saved as PCM.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::SamplesNotLoaded`] if this file was opened with
    /// [`WavFile::open_metadata`].
    pub fn apply_gain(&mut self, gain: f64) -> Result<(), WavError> {
        let samples = self.samples_mut()?;
        for sample in samples {
            *sample *= gain;
        }

        Ok(())
    }

    /// Applies an amplitude gain measured in decibels.
    ///
    /// The decibel value is converted to a linear amplitude multiplier using
    /// `10^(gain_db / 20)`. `0.0` dB leaves the samples unchanged, negative
    /// values attenuate them, and positive values amplify them. Positive gain
    /// may produce values outside `-1.0..=1.0`, which will be clipped when the
    /// file is saved as PCM.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::SamplesNotLoaded`] if this file was opened with
    /// [`WavFile::open_metadata`].
    pub fn apply_gain_db(&mut self, gain_db: f64) -> Result<(), WavError> {
        self.apply_gain(10.0_f64.powf(gain_db / 20.0))
    }
}

#[cfg(test)]
mod tests {
    use super::WavFile;

    const EPSILON: f64 = 1e-12;

    fn mono_file_with(samples: Vec<f64>) -> WavFile {
        let mut wav_file = WavFile::new(1, 48_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(samples)
            .expect("complete sample frames");
        wav_file
    }

    fn assert_samples_approximately_eq(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len());
        for (&actual, &expected) in actual.iter().zip(expected) {
            assert!(
                (actual - expected).abs() <= EPSILON,
                "expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn applies_linear_gain_to_every_sample() {
        let mut wav_file = mono_file_with(vec![0.25, -0.5, 0.0]);

        wav_file.apply_gain(2.0).expect("loaded samples");

        assert_eq!(wav_file.samples().unwrap(), &[0.5, -1.0, 0.0]);
    }

    #[test]
    fn permits_gain_that_will_clip_when_saved() {
        let mut wav_file = mono_file_with(vec![0.75, -0.75]);

        wav_file.apply_gain(2.0).expect("loaded samples");

        assert_eq!(wav_file.samples().unwrap(), &[1.5, -1.5]);
    }

    #[test]
    fn negative_linear_gain_reverses_polarity() {
        let mut wav_file = mono_file_with(vec![0.25, -0.5]);

        wav_file.apply_gain(-2.0).expect("loaded samples");

        assert_eq!(wav_file.samples().unwrap(), &[-0.5, 1.0]);
    }

    #[test]
    fn applies_decibel_gain_as_an_amplitude_ratio() {
        let mut wav_file = mono_file_with(vec![0.25, -0.5]);
        let six_db = 20.0 * 2.0_f64.log10();

        wav_file.apply_gain_db(six_db).expect("loaded samples");

        assert_samples_approximately_eq(wav_file.samples().unwrap(), &[0.5, -1.0]);
    }

    #[test]
    fn negative_decibel_gain_attenuates_samples() {
        let mut wav_file = mono_file_with(vec![0.5, -1.0]);

        wav_file.apply_gain_db(-20.0).expect("loaded samples");

        assert_samples_approximately_eq(wav_file.samples().unwrap(), &[0.05, -0.1]);
    }
}
