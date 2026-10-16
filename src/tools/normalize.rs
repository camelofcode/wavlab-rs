use crate::wav_file::{WavError, WavFile};

impl WavFile {
    /// Scales the loaded samples so the largest absolute value is `1.0`
    /// (0 dBFS).
    ///
    /// Empty files and silent files are left unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::SamplesNotLoaded`] if this file was opened with
    /// [`WavFile::open_metadata`].
    pub fn normalize(&mut self) -> Result<(), WavError> {
        let samples = self.samples_mut()?;
        let peak = samples
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0_f64, f64::max);

        if peak == 0.0 {
            return Ok(());
        }

        let gain = peak.recip();
        for sample in samples {
            *sample *= gain;
        }

        Ok(())
    }

    /// Scales the loaded samples to the requested peak level in dBFS.
    ///
    /// For example, `-6.0` produces a peak approximately 6 dB below full
    /// scale, while `0.0` is equivalent to [`WavFile::normalize`]. For
    /// non-silent audio, a positive target produces samples above full scale;
    /// those samples are retained in memory but will be clipped when the file
    /// is saved as PCM. Empty files and silent files are left unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::SamplesNotLoaded`] if this file was opened with
    /// [`WavFile::open_metadata`].
    pub fn normalize_to_db(&mut self, target_db: f64) -> Result<(), WavError> {
        self.normalize()?;
        self.apply_gain_db(target_db)
    }
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
    fn normalizes_to_the_largest_absolute_sample() {
        let mut wav_file = WavFile::new(1, 48_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![0.25, -0.5, 0.125])
            .expect("complete sample frames");

        wav_file.normalize().expect("loaded samples");

        assert_eq!(wav_file.samples().unwrap(), &[0.5, -1.0, 0.25]);
    }

    #[test]
    fn leaves_silence_unchanged() {
        let mut wav_file = WavFile::new(2, 48_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![0.0, 0.0])
            .expect("complete sample frames");

        wav_file.normalize().expect("loaded samples");

        assert_eq!(wav_file.samples().unwrap(), &[0.0, 0.0]);
    }

    #[test]
    fn normalizes_to_a_peak_decibel_level() {
        let mut wav_file = WavFile::new(1, 48_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![0.25, -0.5, 0.125])
            .expect("complete sample frames");

        wav_file.normalize_to_db(-20.0).expect("loaded samples");

        assert_samples_approximately_eq(wav_file.samples().unwrap(), &[0.05, -0.1, 0.025]);
    }

    #[test]
    fn positive_target_db_permits_samples_above_full_scale() {
        let mut wav_file = WavFile::new(1, 48_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![0.25, -0.5])
            .expect("complete sample frames");
        let six_db = 20.0 * 2.0_f64.log10();

        wav_file.normalize_to_db(six_db).expect("loaded samples");

        assert_samples_approximately_eq(wav_file.samples().unwrap(), &[1.0, -2.0]);
    }
}
