use crate::wav_file::{WavError, WavFile};

impl WavFile {
    /// Returns the peak level of the loaded samples in dBFS.
    ///
    /// The peak is the greatest absolute sample value across every channel.
    /// Silence and empty files return negative infinity.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::SamplesNotLoaded`] if this file was opened with
    /// [`WavFile::open_metadata`].
    pub fn peak_db(&self) -> Result<f64, WavError> {
        let peak = self
            .samples()?
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0_f64, f64::max);

        Ok(amplitude_to_db(peak))
    }

    /// Returns the RMS level of the loaded samples in dBFS.
    ///
    /// The root mean square is calculated across every interleaved sample in
    /// every channel. Silence and empty files return negative infinity.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::SamplesNotLoaded`] if this file was opened with
    /// [`WavFile::open_metadata`].
    pub fn rms_db(&self) -> Result<f64, WavError> {
        let samples = self.samples()?;
        if samples.is_empty() {
            return Ok(f64::NEG_INFINITY);
        }

        let mean_square =
            samples.iter().map(|sample| sample * sample).sum::<f64>() / samples.len() as f64;
        Ok(amplitude_to_db(mean_square.sqrt()))
    }
}

fn amplitude_to_db(amplitude: f64) -> f64 {
    if amplitude == 0.0 {
        f64::NEG_INFINITY
    } else {
        20.0 * amplitude.log10()
    }
}

#[cfg(test)]
mod tests {
    use crate::WavFile;

    const EPSILON: f64 = 1e-12;

    fn file_with(samples: Vec<f64>) -> WavFile {
        let mut file = WavFile::new(1, 48_000, 24).expect("valid PCM settings");
        file.set_samples(samples).expect("complete sample frames");
        file
    }

    #[test]
    fn measures_peak_from_the_largest_absolute_sample() {
        let file = file_with(vec![0.25, -0.5, 0.125]);

        let peak_db = file.peak_db().expect("loaded samples");

        assert!((peak_db - 20.0 * 0.5_f64.log10()).abs() <= EPSILON);
    }

    #[test]
    fn measures_rms_across_all_samples() {
        let file = file_with(vec![0.5, -0.5, 0.0, 0.0]);

        let rms_db = file.rms_db().expect("loaded samples");

        assert!((rms_db - 20.0 * 0.125_f64.sqrt().log10()).abs() <= EPSILON);
    }

    #[test]
    fn silence_has_negative_infinite_peak_and_rms_levels() {
        let silence = file_with(vec![0.0, 0.0]);
        let empty = file_with(Vec::new());

        assert_eq!(silence.peak_db().unwrap(), f64::NEG_INFINITY);
        assert_eq!(silence.rms_db().unwrap(), f64::NEG_INFINITY);
        assert_eq!(empty.peak_db().unwrap(), f64::NEG_INFINITY);
        assert_eq!(empty.rms_db().unwrap(), f64::NEG_INFINITY);
    }
}
