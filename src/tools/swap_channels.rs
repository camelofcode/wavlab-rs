use crate::wav_file::{WavError, WavFile};

impl WavFile {
    /// Swaps the left and right samples in every loaded stereo frame.
    ///
    /// Mono files are left unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::SamplesNotLoaded`] if this is a stereo file opened
    /// with [`WavFile::open_metadata`].
    pub fn swap_channels(&mut self) -> Result<(), WavError> {
        if self.number_of_channels() == 1 {
            return Ok(());
        }

        let samples = self.samples_mut()?;
        for pair in samples.as_chunks_mut::<2>().0 {
            pair.swap(0, 1);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::WavFile;

    #[test]
    fn swaps_left_and_right_samples_in_every_stereo_frame() {
        let mut wav_file = WavFile::new(2, 48_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![0.1, 0.2, 0.3, 0.4])
            .expect("complete sample frames");

        wav_file.swap_channels().expect("loaded samples");

        assert_eq!(wav_file.samples().unwrap(), &[0.2, 0.1, 0.4, 0.3]);
    }

    #[test]
    fn leaves_mono_samples_unchanged() {
        let mut wav_file = WavFile::new(1, 48_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![0.1, 0.2])
            .expect("complete sample frames");

        wav_file.swap_channels().expect("mono is a no-op");

        assert_eq!(wav_file.samples().unwrap(), &[0.1, 0.2]);
    }
}
