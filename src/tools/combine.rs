use crate::constants::MINUS_THREE_DB;
use crate::{ProcessingError, WavFile};

impl WavFile {
    /// Combines separate mono left and right signals into a stereo file.
    ///
    /// The inputs must have the same sample rate. If their lengths differ,
    /// the shorter input is padded with silence. The output uses the greater
    /// of the two PCM bit depths. Other metadata is not copied.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessingError::ExpectedMono`] if either input is not mono,
    /// [`ProcessingError::SampleRateMismatch`] if their sample rates differ, or
    /// [`ProcessingError::Wav`] if either input was opened with
    /// [`WavFile::open_metadata`].
    pub fn combine_left_right(left: &WavFile, right: &WavFile) -> Result<WavFile, ProcessingError> {
        combine_mono_pair(left, right, |left, right| [left, right])
    }

    /// Decodes separate mono mid and side signals into a stereo file.
    ///
    /// This exactly inverts [`WavFile::split_mid_side`]: left is
    /// `(mid + side) / (2 * -3 dB)` and right is
    /// `(mid - side) / (2 * -3 dB)`. If the input lengths differ, the shorter
    /// input is padded with silence. The output uses the greater PCM bit depth.
    /// Other metadata is not copied.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessingError::ExpectedMono`] if either input is not mono,
    /// [`ProcessingError::SampleRateMismatch`] if their sample rates differ, or
    /// [`ProcessingError::Wav`] if either input was opened with
    /// [`WavFile::open_metadata`].
    pub fn combine_mid_side(mid: &WavFile, side: &WavFile) -> Result<WavFile, ProcessingError> {
        let decode_scale = 0.5 / MINUS_THREE_DB;
        combine_mono_pair(mid, side, |mid, side| {
            [(mid + side) * decode_scale, (mid - side) * decode_scale]
        })
    }

    /// Mixes two or more loaded files starting at the first frame.
    ///
    /// All inputs must have the same sample rate. If any input is stereo, mono
    /// inputs are sent equally to both output channels. Shorter inputs are
    /// padded with silence. Samples are averaged across every input and the
    /// completed mix is normalized to 0 dBFS. The output uses the greatest
    /// input PCM bit depth. Other metadata is not copied.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessingError::TooFewInputs`] for fewer than two files,
    /// [`ProcessingError::SampleRateMismatch`] if their sample rates differ, or
    /// [`ProcessingError::Wav`] if any input was opened with
    /// [`WavFile::open_metadata`].
    pub fn mix(inputs: &[WavFile]) -> Result<WavFile, ProcessingError> {
        validate_minimum_inputs(inputs)?;
        let sample_rate = validate_sample_rates(inputs)?;
        let input_samples = loaded_samples(inputs)?;
        let output_channels = inputs
            .iter()
            .map(WavFile::number_of_channels)
            .max()
            .expect("minimum input count was validated");
        let bits_per_sample = maximum_bit_depth(inputs);
        let frame_count = inputs
            .iter()
            .zip(&input_samples)
            .map(|(input, samples)| samples.len() / usize::from(input.number_of_channels()))
            .max()
            .expect("minimum input count was validated");
        let output_sample_count = frame_count
            .checked_mul(usize::from(output_channels))
            .ok_or(ProcessingError::SizeOverflow("mixed sample data"))?;
        let mut output_samples = Vec::with_capacity(output_sample_count);
        let input_count = inputs.len() as f64;

        for frame in 0..frame_count {
            if output_channels == 1 {
                let sum: f64 = input_samples
                    .iter()
                    .map(|samples| samples.get(frame).copied().unwrap_or(0.0))
                    .sum();
                output_samples.push(sum / input_count);
                continue;
            }

            let mut left_sum = 0.0;
            let mut right_sum = 0.0;
            for (input, samples) in inputs.iter().zip(&input_samples) {
                if input.number_of_channels() == 1 {
                    let sample = samples.get(frame).copied().unwrap_or(0.0);
                    left_sum += sample;
                    right_sum += sample;
                } else {
                    left_sum += samples.get(frame * 2).copied().unwrap_or(0.0);
                    right_sum += samples.get(frame * 2 + 1).copied().unwrap_or(0.0);
                }
            }
            output_samples.push(left_sum / input_count);
            output_samples.push(right_sum / input_count);
        }

        let mut output = WavFile::new(output_channels, sample_rate, bits_per_sample)?;
        output.set_samples(output_samples)?;
        output.normalize()?;
        Ok(output)
    }

    /// Joins two or more loaded files end-to-end.
    ///
    /// All inputs must have the same sample rate and channel count. Their
    /// samples are concatenated in input order. The output uses the greatest
    /// input PCM bit depth. Other metadata is not copied.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessingError::TooFewInputs`] for fewer than two files,
    /// [`ProcessingError::SampleRateMismatch`] or [`ProcessingError::ChannelCountMismatch`]
    /// for incompatible formats, or [`ProcessingError::Wav`] if any
    /// input was opened with [`WavFile::open_metadata`].
    pub fn join(inputs: &[WavFile]) -> Result<WavFile, ProcessingError> {
        validate_minimum_inputs(inputs)?;
        let sample_rate = validate_sample_rates(inputs)?;
        let number_of_channels = validate_channel_counts(inputs)?;
        let input_samples = loaded_samples(inputs)?;
        let total_samples = input_samples.iter().try_fold(0_usize, |total, samples| {
            total
                .checked_add(samples.len())
                .ok_or(ProcessingError::SizeOverflow("joined sample data"))
        })?;
        let mut joined_samples = Vec::with_capacity(total_samples);
        for samples in input_samples {
            joined_samples.extend_from_slice(samples);
        }

        let mut output = WavFile::new(number_of_channels, sample_rate, maximum_bit_depth(inputs))?;
        output.set_samples(joined_samples)?;
        Ok(output)
    }
}

fn combine_mono_pair(
    first: &WavFile,
    second: &WavFile,
    combine: impl Fn(f64, f64) -> [f64; 2],
) -> Result<WavFile, ProcessingError> {
    for input in [first, second] {
        if input.number_of_channels() != 1 {
            return Err(ProcessingError::ExpectedMono(input.number_of_channels()));
        }
    }

    if first.sample_rate() != second.sample_rate() {
        return Err(ProcessingError::SampleRateMismatch {
            expected: first.sample_rate(),
            actual: second.sample_rate(),
        });
    }

    let first_samples = first.samples()?;
    let second_samples = second.samples()?;
    let frame_count = first_samples.len().max(second_samples.len());
    let sample_count = frame_count
        .checked_mul(2)
        .ok_or(ProcessingError::SizeOverflow("combined sample data"))?;
    let mut stereo_samples = Vec::with_capacity(sample_count);

    for frame in 0..frame_count {
        stereo_samples.extend(combine(
            first_samples.get(frame).copied().unwrap_or(0.0),
            second_samples.get(frame).copied().unwrap_or(0.0),
        ));
    }

    let mut output = WavFile::new(
        2,
        first.sample_rate(),
        first.bits_per_sample().max(second.bits_per_sample()),
    )?;
    output.set_samples(stereo_samples)?;
    Ok(output)
}

fn validate_minimum_inputs(inputs: &[WavFile]) -> Result<(), ProcessingError> {
    const MINIMUM_INPUTS: usize = 2;
    if inputs.len() < MINIMUM_INPUTS {
        return Err(ProcessingError::TooFewInputs {
            minimum: MINIMUM_INPUTS,
            actual: inputs.len(),
        });
    }
    Ok(())
}

fn validate_sample_rates(inputs: &[WavFile]) -> Result<u32, ProcessingError> {
    let expected = inputs[0].sample_rate();
    for input in &inputs[1..] {
        if input.sample_rate() != expected {
            return Err(ProcessingError::SampleRateMismatch {
                expected,
                actual: input.sample_rate(),
            });
        }
    }
    Ok(expected)
}

fn validate_channel_counts(inputs: &[WavFile]) -> Result<u16, ProcessingError> {
    let expected = inputs[0].number_of_channels();
    for input in &inputs[1..] {
        if input.number_of_channels() != expected {
            return Err(ProcessingError::ChannelCountMismatch {
                expected,
                actual: input.number_of_channels(),
            });
        }
    }
    Ok(expected)
}

fn maximum_bit_depth(inputs: &[WavFile]) -> u16 {
    inputs
        .iter()
        .map(WavFile::bits_per_sample)
        .max()
        .expect("minimum input count was validated")
}

fn loaded_samples(inputs: &[WavFile]) -> Result<Vec<&[f64]>, ProcessingError> {
    inputs
        .iter()
        .map(|input| input.samples().map_err(ProcessingError::from))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::{ProcessingError, WavFile};

    const EPSILON: f64 = 1e-12;

    fn file(channels: u16, sample_rate: u32, bits: u16, samples: Vec<f64>) -> WavFile {
        let mut file = WavFile::new(channels, sample_rate, bits).expect("valid PCM settings");
        file.set_samples(samples).expect("complete sample frames");
        file
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
    fn combines_left_and_right_with_silence_padding() {
        let left = file(1, 48_000, 16, vec![0.1, 0.2, 0.3]);
        let right = file(1, 48_000, 24, vec![-0.1, -0.2]);

        let stereo = WavFile::combine_left_right(&left, &right).expect("compatible mono files");

        assert_eq!(stereo.number_of_channels(), 2);
        assert_eq!(stereo.sample_rate(), 48_000);
        assert_eq!(stereo.bits_per_sample(), 24);
        assert_eq!(stereo.samples().unwrap(), &[0.1, -0.1, 0.2, -0.2, 0.3, 0.0]);
    }

    #[test]
    fn left_right_combination_rejects_incompatible_inputs() {
        let mono = file(1, 48_000, 24, vec![0.1]);
        let stereo = file(2, 48_000, 24, vec![0.1, 0.2]);
        let other_rate = file(1, 44_100, 24, vec![0.1]);

        assert!(matches!(
            WavFile::combine_left_right(&stereo, &mono),
            Err(ProcessingError::ExpectedMono(2))
        ));
        assert!(matches!(
            WavFile::combine_left_right(&mono, &other_rate),
            Err(ProcessingError::SampleRateMismatch { .. })
        ));
    }

    #[test]
    fn mid_side_combination_round_trips_a_stereo_file() {
        let stereo = file(2, 48_000, 24, vec![0.1, -0.2, 0.3, 0.4]);
        let (mid, side) = stereo.split_mid_side().expect("stereo file");

        let decoded = WavFile::combine_mid_side(&mid, &side).expect("compatible mono files");

        assert_samples_approximately_eq(decoded.samples().unwrap(), stereo.samples().unwrap());
    }

    #[test]
    fn mixes_mono_files_with_silence_padding_and_normalization() {
        let first = file(1, 48_000, 16, vec![0.5, 0.25]);
        let second = file(1, 48_000, 24, vec![0.5]);

        let mixed = WavFile::mix(&[first, second]).expect("compatible files");

        assert_eq!(mixed.number_of_channels(), 1);
        assert_eq!(mixed.bits_per_sample(), 24);
        assert_eq!(mixed.samples().unwrap(), &[1.0, 0.25]);
    }

    #[test]
    fn mixing_promotes_mono_input_to_stereo() {
        let mono = file(1, 48_000, 16, vec![0.5]);
        let stereo = file(2, 48_000, 32, vec![0.5, -0.5]);

        let mixed = WavFile::mix(&[mono, stereo]).expect("compatible files");

        assert_eq!(mixed.number_of_channels(), 2);
        assert_eq!(mixed.bits_per_sample(), 32);
        assert_eq!(mixed.samples().unwrap(), &[1.0, 0.0]);
    }

    #[test]
    fn mixing_rejects_too_few_or_mismatched_inputs() {
        let first = file(1, 48_000, 24, vec![0.1]);
        let second = file(1, 44_100, 24, vec![0.1]);

        assert!(matches!(
            WavFile::mix(&[]),
            Err(ProcessingError::TooFewInputs {
                minimum: 2,
                actual: 0
            })
        ));
        assert!(matches!(
            WavFile::mix(&[first, second]),
            Err(ProcessingError::SampleRateMismatch { .. })
        ));
    }

    #[test]
    fn joins_files_end_to_end() {
        let first = file(2, 48_000, 16, vec![0.1, -0.1, 0.2, -0.2]);
        let second = file(2, 48_000, 24, vec![0.3, -0.3]);

        let joined = WavFile::join(&[first, second]).expect("compatible files");

        assert_eq!(joined.number_of_channels(), 2);
        assert_eq!(joined.sample_rate(), 48_000);
        assert_eq!(joined.bits_per_sample(), 24);
        assert_eq!(
            joined.samples().unwrap(),
            &[0.1, -0.1, 0.2, -0.2, 0.3, -0.3]
        );
    }

    #[test]
    fn joining_rejects_incompatible_inputs() {
        let mono = file(1, 48_000, 24, vec![0.1]);
        let stereo = file(2, 48_000, 24, vec![0.1, 0.2]);
        let other_rate = file(1, 44_100, 24, vec![0.1]);

        assert!(matches!(
            WavFile::join(&[]),
            Err(ProcessingError::TooFewInputs { .. })
        ));
        assert!(matches!(
            WavFile::join(&[mono, stereo]),
            Err(ProcessingError::ChannelCountMismatch { .. })
        ));

        let first = file(1, 48_000, 24, vec![0.1]);
        assert!(matches!(
            WavFile::join(&[first, other_rate]),
            Err(ProcessingError::SampleRateMismatch { .. })
        ));
    }
}
