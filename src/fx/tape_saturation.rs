use crate::wav_file::{WavError, WavFile};

impl WavFile {
    /// Applies sigmoid soft clipping that approximates tape saturation.
    ///
    /// `drive_db` controls the input gain before the saturation curve. `0.0`
    /// dB applies unity gain, positive values drive the curve harder, and
    /// negative values reduce the amount of saturation. Finite inputs and
    /// drive values produce output within `-1.0..=1.0`.
    ///
    /// The sigmoid is based on the [TAP Sigmoid Booster] algorithm.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::SamplesNotLoaded`] if this file was opened with
    /// [`WavFile::open_metadata`].
    ///
    /// [TAP Sigmoid Booster]: https://tomscii.sig7.se/tap-plugins/ladspa/sigmoid.html
    pub fn tape_saturation(&mut self, drive_db: f64) -> Result<(), WavError> {
        let input_gain = 10.0_f64.powf(drive_db / 20.0);

        for sample in self.samples_mut()? {
            *sample = saturate_sample(*sample, input_gain);
        }

        Ok(())
    }
}

/// Applies the saturation curve to one sample using a precomputed input gain.
///
/// The original logistic expression `2 / (1 + e^(-5x)) - 1` is equivalent to
/// `tanh(2.5x)`. The hyperbolic-tangent form avoids exponential overflow for
/// large negative inputs.
pub(super) fn saturate_sample(sample: f64, input_gain: f64) -> f64 {
    (2.5 * sample * input_gain).tanh()
}

#[cfg(test)]
mod tests {
    use super::WavFile;

    const EPSILON: f64 = 1e-12;

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
    fn uses_decibel_drive_as_pre_saturation_input_gain() {
        let input = [-1.0, -0.25, 0.0, 0.25, 1.0];
        let drive_db = 20.0 * 2.0_f64.log10();
        let input_gain = 2.0;
        let expected = input.map(|sample| {
            let pre_saturation = sample * input_gain;
            2.0 / (1.0 + (-5.0_f64 * pre_saturation).exp()) - 1.0
        });
        let mut wav_file = WavFile::new(1, 48_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(input.to_vec())
            .expect("complete sample frames");

        wav_file.tape_saturation(drive_db).expect("loaded samples");

        assert_samples_approximately_eq(wav_file.samples().unwrap(), &expected);
    }

    #[test]
    fn applies_the_curve_to_every_channel() {
        let mut wav_file = WavFile::new(2, 48_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![0.25, -0.25, 0.5, -0.5])
            .expect("complete sample frames");

        wav_file.tape_saturation(0.0).expect("loaded samples");

        let samples = wav_file.samples().unwrap();
        assert!(samples[0] > 0.25);
        assert!((samples[0] + samples[1]).abs() <= EPSILON);
        assert!((samples[2] + samples[3]).abs() <= EPSILON);
        assert!(samples.iter().all(|sample| (-1.0..=1.0).contains(sample)));
    }

    #[test]
    fn negative_drive_reduces_saturation_without_inverting_polarity() {
        let mut zero_db = WavFile::new(1, 48_000, 24).expect("valid PCM settings");
        zero_db
            .set_samples(vec![0.25])
            .expect("complete sample frames");
        let mut reduced = WavFile::new(1, 48_000, 24).expect("valid PCM settings");
        reduced
            .set_samples(vec![0.25])
            .expect("complete sample frames");

        zero_db.tape_saturation(0.0).expect("loaded samples");
        reduced.tape_saturation(-20.0).expect("loaded samples");

        let zero_db_sample = zero_db.samples().unwrap()[0];
        let reduced_sample = reduced.samples().unwrap()[0];
        assert!(reduced_sample > 0.0);
        assert!(reduced_sample < zero_db_sample);
    }
}
