use crate::wav_file::{WavError, WavFile};

impl WavFile {
    /// Reverses the order of the loaded sample frames.
    ///
    /// For stereo files, the left and right samples within each interleaved
    /// frame retain their channel positions.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::SamplesNotLoaded`] if this file was opened with
    /// [`WavFile::open_metadata`].
    pub fn reverse(&mut self) -> Result<(), WavError> {
        let number_of_channels = usize::from(self.number_of_channels());
        let samples = self.samples_mut()?;
        samples.reverse();
        for frame in samples.chunks_exact_mut(number_of_channels) {
            frame.reverse();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::WavFile;

    #[test]
    fn reverses_mono_samples() {
        let mut wav_file = WavFile::new(1, 48_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![0.25, -0.5, 0.75])
            .expect("complete sample frames");

        wav_file.reverse().expect("loaded samples");

        assert_eq!(wav_file.samples().unwrap(), &[0.75, -0.5, 0.25]);
    }

    #[test]
    fn reverses_stereo_frames_without_swapping_channels() {
        let mut wav_file = WavFile::new(2, 48_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6])
            .expect("complete sample frames");

        wav_file.reverse().expect("loaded samples");

        assert_eq!(wav_file.samples().unwrap(), &[0.5, 0.6, 0.3, 0.4, 0.1, 0.2]);
    }
}
