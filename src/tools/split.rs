use crate::constants::MINUS_THREE_DB;
use crate::wav_file::{WavError, WavFile};

impl WavFile {
    /// Splits the loaded stereo samples into separate mono files.
    ///
    /// The returned left and right files have the same sample rate and PCM bit
    /// depth as the source. Other metadata is not copied.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::ExpectedStereo`] if this is not a stereo file, or
    /// [`WavError::SamplesNotLoaded`] if it was opened with
    /// [`WavFile::open_metadata`].
    pub fn split_stereo(&self) -> Result<(WavFile, WavFile), WavError> {
        if self.number_of_channels() != 2 {
            return Err(WavError::ExpectedStereo(self.number_of_channels()));
        }

        let samples = self.samples()?;
        let frame_count = samples.len() / 2;
        let mut left_samples = Vec::with_capacity(frame_count);
        let mut right_samples = Vec::with_capacity(frame_count);

        let (frames, remainder) = samples.as_chunks::<2>();
        debug_assert!(remainder.is_empty());

        for &[left, right] in frames {
            left_samples.push(left);
            right_samples.push(right);
        }

        let left = file_with_samples_like(self, 1, left_samples)?;
        let right = file_with_samples_like(self, 1, right_samples)?;

        Ok((left, right))
    }

    /// Splits the loaded stereo samples into separate mono mid and side files.
    ///
    /// Each mid sample is `(left + right) * -3 dB`, and each side sample is
    /// `(left - right) * -3 dB`. The returned files have the same sample rate
    /// and PCM bit depth as the source. Other metadata is not copied. The
    /// results are not clamped, so correlated full-scale input may produce
    /// samples that will be clipped when saved as PCM.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::ExpectedStereo`] if this is not a stereo file, or
    /// [`WavError::SamplesNotLoaded`] if it was opened with
    /// [`WavFile::open_metadata`].
    pub fn split_mid_side(&self) -> Result<(WavFile, WavFile), WavError> {
        if self.number_of_channels() != 2 {
            return Err(WavError::ExpectedStereo(self.number_of_channels()));
        }

        let samples = self.samples()?;
        let frame_count = samples.len() / 2;
        let mut mid_samples = Vec::with_capacity(frame_count);
        let mut side_samples = Vec::with_capacity(frame_count);
        let (frames, remainder) = samples.as_chunks::<2>();
        debug_assert!(remainder.is_empty());

        for &[left, right] in frames {
            mid_samples.push((left + right) * MINUS_THREE_DB);
            side_samples.push((left - right) * MINUS_THREE_DB);
        }

        let mid = file_with_samples_like(self, 1, mid_samples)?;
        let side = file_with_samples_like(self, 1, side_samples)?;

        Ok((mid, side))
    }

    /// Splits the loaded audio at the requested millisecond positions.
    ///
    /// Split times are measured from the beginning of the file and converted
    /// to sample-frame boundaries by rounding down. They must resolve to
    /// distinct, increasing positions after the first frame and before the end
    /// of the file. An empty slice returns one file containing all samples.
    ///
    /// Each returned file has the same channel count, sample rate, and PCM bit
    /// depth as the source. Other metadata is not copied.
    ///
    /// # Errors
    ///
    /// Returns [`WavError::InvalidSplitTime`] if any time does not identify a
    /// valid next boundary, or [`WavError::SamplesNotLoaded`] if this file was
    /// opened with [`WavFile::open_metadata`].
    pub fn split_at_times(&self, split_times_ms: &[u64]) -> Result<Vec<WavFile>, WavError> {
        let samples = self.samples()?;
        let number_of_channels = usize::from(self.number_of_channels());
        let frame_count = samples.len() / number_of_channels;
        let mut previous_frame = 0_u128;
        let mut split_sample_positions = Vec::with_capacity(split_times_ms.len());

        for &milliseconds in split_times_ms {
            let frame = u128::from(milliseconds) * u128::from(self.sample_rate()) / 1_000;

            if frame <= previous_frame || frame >= frame_count as u128 {
                return Err(WavError::InvalidSplitTime(milliseconds));
            }

            previous_frame = frame;
            split_sample_positions.push(frame as usize * number_of_channels);
        }

        let mut outputs = Vec::with_capacity(split_sample_positions.len() + 1);
        let mut start = 0;

        for end in split_sample_positions
            .into_iter()
            .chain(std::iter::once(samples.len()))
        {
            outputs.push(file_with_samples_like(
                self,
                self.number_of_channels(),
                samples[start..end].to_vec(),
            )?);
            start = end;
        }

        Ok(outputs)
    }
}

fn file_with_samples_like(
    source: &WavFile,
    number_of_channels: u16,
    samples: Vec<f64>,
) -> Result<WavFile, WavError> {
    let mut output = WavFile::new(
        number_of_channels,
        source.sample_rate(),
        source.bits_per_sample(),
    )?;
    output.set_samples(samples)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use crate::constants::MINUS_THREE_DB;
    use crate::{WavError, WavFile};

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
    fn splits_interleaved_stereo_samples_into_mono_files() {
        let mut stereo = WavFile::new(2, 96_000, 32).expect("valid PCM settings");
        stereo
            .set_samples(vec![0.1, -0.2, 0.3, -0.4, 0.5, -0.6])
            .expect("complete sample frames");

        let (left, right) = stereo.split_stereo().expect("stereo file");

        assert_eq!(left.number_of_channels(), 1);
        assert_eq!(right.number_of_channels(), 1);
        assert_eq!(left.sample_rate(), 96_000);
        assert_eq!(right.sample_rate(), 96_000);
        assert_eq!(left.bits_per_sample(), 32);
        assert_eq!(right.bits_per_sample(), 32);
        assert_eq!(left.samples().unwrap(), &[0.1, 0.3, 0.5]);
        assert_eq!(right.samples().unwrap(), &[-0.2, -0.4, -0.6]);
    }

    #[test]
    fn splits_an_empty_stereo_file() {
        let stereo = WavFile::new(2, 48_000, 24).expect("valid PCM settings");

        let (left, right) = stereo.split_stereo().expect("stereo file");

        assert!(left.samples().unwrap().is_empty());
        assert!(right.samples().unwrap().is_empty());
    }

    #[test]
    fn rejects_a_mono_file() {
        let mono = WavFile::new(1, 48_000, 24).expect("valid PCM settings");

        let error = match mono.split_stereo() {
            Ok(_) => panic!("mono file was accepted as stereo"),
            Err(error) => error,
        };

        assert!(matches!(error, WavError::ExpectedStereo(1)));
    }

    #[test]
    fn splits_stereo_samples_into_mid_and_side_files() {
        let mut stereo = WavFile::new(2, 96_000, 32).expect("valid PCM settings");
        stereo
            .set_samples(vec![0.5, 0.5, 0.5, -0.5, -0.25, 0.5])
            .expect("complete sample frames");

        let (mid, side) = stereo.split_mid_side().expect("stereo file");

        assert_eq!(mid.number_of_channels(), 1);
        assert_eq!(side.number_of_channels(), 1);
        assert_eq!(mid.sample_rate(), 96_000);
        assert_eq!(side.bits_per_sample(), 32);
        assert_samples_approximately_eq(
            mid.samples().unwrap(),
            &[MINUS_THREE_DB, 0.0, 0.25 * MINUS_THREE_DB],
        );
        assert_samples_approximately_eq(
            side.samples().unwrap(),
            &[0.0, MINUS_THREE_DB, -0.75 * MINUS_THREE_DB],
        );
    }

    #[test]
    fn mid_side_split_rejects_a_mono_file() {
        let mono = WavFile::new(1, 48_000, 24).expect("valid PCM settings");

        assert!(matches!(
            mono.split_mid_side(),
            Err(WavError::ExpectedStereo(1))
        ));
    }

    #[test]
    fn splits_mono_audio_at_multiple_times() {
        let mut input = WavFile::new(1, 1_000, 16).expect("valid PCM settings");
        input
            .set_samples(vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5])
            .expect("complete sample frames");

        let outputs = input.split_at_times(&[2, 5]).expect("valid split times");

        assert_eq!(outputs.len(), 3);
        assert_eq!(outputs[0].samples().unwrap(), &[0.0, 0.1]);
        assert_eq!(outputs[1].samples().unwrap(), &[0.2, 0.3, 0.4]);
        assert_eq!(outputs[2].samples().unwrap(), &[0.5]);
        assert!(
            outputs
                .iter()
                .all(|output| output.number_of_channels() == 1)
        );
        assert!(outputs.iter().all(|output| output.sample_rate() == 1_000));
        assert!(outputs.iter().all(|output| output.bits_per_sample() == 16));
    }

    #[test]
    fn splits_stereo_audio_only_at_frame_boundaries() {
        let mut input = WavFile::new(2, 1_000, 24).expect("valid PCM settings");
        input
            .set_samples(vec![0.1, -0.1, 0.2, -0.2, 0.3, -0.3])
            .expect("complete sample frames");

        let outputs = input.split_at_times(&[2]).expect("valid split time");

        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].samples().unwrap(), &[0.1, -0.1, 0.2, -0.2]);
        assert_eq!(outputs[1].samples().unwrap(), &[0.3, -0.3]);
        assert!(
            outputs
                .iter()
                .all(|output| output.number_of_channels() == 2)
        );
    }

    #[test]
    fn split_at_no_times_returns_one_complete_file() {
        let mut input = WavFile::new(1, 1_000, 24).expect("valid PCM settings");
        input
            .set_samples(vec![0.1, 0.2])
            .expect("complete sample frames");

        let outputs = input.split_at_times(&[]).expect("no split times");

        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].samples().unwrap(), &[0.1, 0.2]);
    }

    #[test]
    fn rejects_split_times_that_do_not_create_nonempty_segments() {
        let mut input = WavFile::new(1, 1_000, 24).expect("valid PCM settings");
        input
            .set_samples(vec![0.1, 0.2, 0.3, 0.4])
            .expect("complete sample frames");

        assert!(matches!(
            input.split_at_times(&[0]),
            Err(WavError::InvalidSplitTime(0))
        ));
        assert!(matches!(
            input.split_at_times(&[3, 2]),
            Err(WavError::InvalidSplitTime(2))
        ));
        assert!(matches!(
            input.split_at_times(&[4]),
            Err(WavError::InvalidSplitTime(4))
        ));
    }
}
