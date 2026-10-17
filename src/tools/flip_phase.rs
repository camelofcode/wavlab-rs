use crate::wav_file::{WavError, WavFile};

impl WavFile {
    /// Inverts the polarity of every loaded sample.
    ///
    /// Each sample is multiplied by `-1.0`. This changes the waveform's
    /// polarity without changing its level or duration.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::SamplesNotLoaded`] if this file was opened with
    /// [`WavFile::open_metadata`].
    pub fn flip_phase(&mut self) -> Result<(), WavError> {
        let samples = self.samples_mut()?;
        for sample in samples {
            *sample = -*sample;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::WavFile;

    #[test]
    fn flips_the_polarity_of_every_sample() {
        let mut wav_file = WavFile::new(1, 48_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![0.25, -0.5, 0.0])
            .expect("complete sample frames");

        wav_file.flip_phase().expect("loaded samples");

        assert_eq!(wav_file.samples().unwrap(), &[-0.25, 0.5, -0.0]);
    }
}
