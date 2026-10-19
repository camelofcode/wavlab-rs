use crate::wav_file::WavFile;

use super::{GeneratorError, generate_samples};

impl WavFile {
    /// Replaces the samples with uniformly distributed white noise.
    ///
    /// Each frame receives a random value in `-1.0..=1.0`; that value is copied
    /// to every channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the generated sample count cannot be represented.
    pub fn generate_white_noise(&mut self, length_milliseconds: u32) -> Result<(), GeneratorError> {
        generate_samples(self, length_milliseconds, |_| {
            rand::random_range(-1.0..=1.0)
        })
    }

    /// Replaces the samples with filtered and normalized pink noise.
    ///
    /// The filter is Paul Kellet's economy pink-noise approximation. One value
    /// is generated per frame and copied to every channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the generated sample count cannot be represented or
    /// the samples cannot be normalized.
    pub fn generate_pink_noise(&mut self, length_milliseconds: u32) -> Result<(), GeneratorError> {
        let mut buffers = [0.0; 7];

        generate_samples(self, length_milliseconds, |_| {
            let white = rand::random_range(-1.0..=1.0);
            buffers[0] = 0.998_860 * buffers[0] + 0.055_517_9 * white;
            buffers[1] = 0.993_320 * buffers[1] + 0.075_075_9 * white;
            buffers[2] = 0.969_000 * buffers[2] + 0.153_852_0 * white;
            buffers[3] = 0.866_500 * buffers[3] + 0.310_485_6 * white;
            buffers[4] = 0.550_000 * buffers[4] + 0.532_952_2 * white;
            buffers[5] = -0.761_60 * buffers[5] - 0.016_898_0 * white;

            let sample = 0.129 * (buffers.iter().sum::<f64>() + 0.5362 * white);
            buffers[6] = 0.115_926 * white;
            sample
        })?;
        self.normalize()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::WavFile;

    #[test]
    fn white_noise_has_the_requested_stereo_duration_and_matching_channels() {
        let mut wav_file = WavFile::new(2, 8_000, 16).unwrap();
        wav_file.generate_white_noise(250).unwrap();

        assert_eq!(wav_file.number_of_samples(), 4_000);
        assert!(
            wav_file
                .samples()
                .unwrap()
                .as_chunks::<2>()
                .0
                .iter()
                .all(|frame| frame[0] == frame[1])
        );
    }
}
