use crate::{Delay, WavFile};

use super::{
    EffectError,
    delay::{milliseconds_to_frames, validate_mix},
};

// This is the commonly used concrete timing set for Schroeder's design. The
// 1962 paper specifies four mutually incommensurate comb delays spread over
// roughly 30-45 ms, followed by all-pass delays of about 5 and 1.7 ms.
const COMB_DELAYS_MILLISECONDS: [f64; 4] = [29.7, 37.1, 41.1, 43.7];
const ALL_PASS_DELAYS_MILLISECONDS: [f64; 2] = [5.0, 1.7];
const ALL_PASS_GAIN: f64 = 0.7;
const COMB_COUNT: f64 = COMB_DELAYS_MILLISECONDS.len() as f64;

impl WavFile {
    /// Applies a Schroeder algorithmic reverb independently to each channel.
    ///
    /// This implements the design from Manfred R. Schroeder's 1962 paper
    /// ["Natural Sounding Artificial Reverberation"]: four feedback comb
    /// filters in parallel followed by two all-pass filters in series.
    ///
    /// `room_size` ranges from `0.0` to `1.0`. It scales the conventional
    /// delay set from half size through its standard values at `0.5` to one and
    /// a half times those values. `decay_seconds` is the RT60: the time for the
    /// comb-filter tail to decay by 60 dB. `mix` is the wet proportion from
    /// `0.0` (dry) to `1.0` (wet), and
    /// `tail_milliseconds` appends silence so the decay can continue beyond
    /// the input file.
    ///
    /// The C++ predecessor was recognizably based on this design, but its two
    /// final stages were feedback comb filters rather than all-pass filters.
    /// This implementation follows the published design instead.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, `room_size` or
    /// `mix` is outside `0.0..=1.0`, `decay_seconds` is not positive, or the
    /// required output or delay storage is too large for the platform.
    ///
    /// ["Natural Sounding Artificial Reverberation"]: https://aes.org/publications/elibrary/elibrary-page/?id=849
    pub fn reverb(
        &mut self,
        room_size: f64,
        decay_seconds: f64,
        mix: f64,
        tail_milliseconds: u32,
    ) -> Result<(), EffectError> {
        validate_reverb_parameters(room_size, decay_seconds, mix)?;

        let channel_count = usize::from(self.number_of_channels());
        let sample_rate = self.sample_rate();
        let input = self.samples()?.to_vec();
        let input_frames = input.len() / channel_count;
        let tail_frames = milliseconds_to_frames(tail_milliseconds, sample_rate)?;
        let output_frames = input_frames
            .checked_add(tail_frames)
            .ok_or(EffectError::SizeOverflow("reverb output frame count"))?;
        let output_samples = output_frames
            .checked_mul(channel_count)
            .ok_or(EffectError::SizeOverflow("reverb output sample count"))?;
        let mut output = Vec::new();
        output
            .try_reserve_exact(output_samples)
            .map_err(|_| EffectError::SizeOverflow("reverb output sample storage"))?;

        let room_scale = 0.5 + room_size;
        let mut channels = Vec::with_capacity(channel_count);
        for _ in 0..channel_count {
            channels.push(ReverbChannel::new(sample_rate, room_scale, decay_seconds)?);
        }

        for frame_index in 0..output_frames {
            for (channel_index, channel) in channels.iter_mut().enumerate() {
                let input_sample = if frame_index < input_frames {
                    input[frame_index * channel_count + channel_index]
                } else {
                    0.0
                };
                let wet_sample = channel.next_sample(input_sample);
                output.push(input_sample * (1.0 - mix) + wet_sample * mix);
            }
        }

        self.set_samples(output)?;
        Ok(())
    }
}

fn validate_reverb_parameters(
    room_size: f64,
    decay_seconds: f64,
    mix: f64,
) -> Result<(), EffectError> {
    if !room_size.is_finite() || !(0.0..=1.0).contains(&room_size) {
        return Err(EffectError::InvalidRoomSize(room_size));
    }
    if !decay_seconds.is_finite() || decay_seconds <= 0.0 {
        return Err(EffectError::InvalidDecayTime(decay_seconds));
    }
    validate_mix(mix)
}

struct ReverbChannel {
    combs: Vec<ReverbComb>,
    all_passes: Vec<SchroederAllPass>,
}

impl ReverbChannel {
    fn new(sample_rate: u32, room_scale: f64, decay_seconds: f64) -> Result<Self, EffectError> {
        let mut combs = Vec::with_capacity(COMB_DELAYS_MILLISECONDS.len());
        for base_delay in COMB_DELAYS_MILLISECONDS {
            let delay_milliseconds = base_delay * room_scale;
            // 0.001 is -60 dB in amplitude, so each comb reaches the
            // requested RT60 independently of its delay length.
            let feedback = 0.001_f64.powf(delay_milliseconds / (decay_seconds * 1_000.0));
            combs.push(ReverbComb::new(delay_milliseconds, sample_rate, feedback)?);
        }

        let mut all_passes = Vec::with_capacity(ALL_PASS_DELAYS_MILLISECONDS.len());
        for base_delay in ALL_PASS_DELAYS_MILLISECONDS {
            let delay_milliseconds = base_delay * room_scale;
            all_passes.push(SchroederAllPass::new(
                delay_milliseconds,
                sample_rate,
                ALL_PASS_GAIN,
            )?);
        }

        Ok(Self { combs, all_passes })
    }

    fn next_sample(&mut self, input: f64) -> f64 {
        let mut output = 0.0;
        for comb in &mut self.combs {
            output += comb.next_sample(input);
        }
        output /= COMB_COUNT;

        for all_pass in &mut self.all_passes {
            output = all_pass.next_sample(output);
        }
        output
    }
}

struct ReverbComb {
    delay: Delay,
    feedback: f64,
}

impl ReverbComb {
    fn new(delay_milliseconds: f64, sample_rate: u32, feedback: f64) -> Result<Self, EffectError> {
        Ok(Self {
            delay: Delay::new(delay_milliseconds, delay_milliseconds, sample_rate)?,
            feedback,
        })
    }

    fn next_sample(&mut self, input: f64) -> f64 {
        self.delay.next_sample(input, self.feedback, 1.0)
    }
}

struct SchroederAllPass {
    delay: Delay,
    gain: f64,
}

impl SchroederAllPass {
    fn new(delay_milliseconds: f64, sample_rate: u32, gain: f64) -> Result<Self, EffectError> {
        Ok(Self {
            delay: Delay::new(delay_milliseconds, delay_milliseconds, sample_rate)?,
            gain,
        })
    }

    fn next_sample(&mut self, input: f64) -> f64 {
        let delayed = self.delay.next_sample(input, self.gain, 1.0);
        // Delay has already stored `input + gain * delayed`. Combining that
        // state with the feed-forward term produces Schroeder's all-pass form.
        delayed * (1.0 - self.gain * self.gain) - self.gain * input
    }
}

#[cfg(test)]
mod tests {
    use super::{EffectError, ReverbChannel, ReverbComb, SchroederAllPass};
    use crate::WavFile;

    const EPSILON: f64 = 1e-12;

    fn impulse_file(channels: u16, sample_rate: u32, frames: usize) -> WavFile {
        let mut wav_file = WavFile::new(channels, sample_rate, 24).expect("valid PCM settings");
        let mut samples = vec![0.0; frames * usize::from(channels)];
        for sample in samples.iter_mut().take(usize::from(channels)) {
            *sample = 1.0;
        }
        wav_file
            .set_samples(samples)
            .expect("complete sample frames");
        wav_file
    }

    #[test]
    fn feedback_comb_repeats_at_its_delay_and_decay() {
        let mut comb = ReverbComb::new(2.0, 1_000, 0.5).expect("small delay buffer");
        let output = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0].map(|sample| comb.next_sample(sample));

        assert_eq!(output, [0.0, 0.0, 1.0, 0.0, 0.5, 0.0, 0.25]);
    }

    #[test]
    fn all_pass_has_the_expected_impulse_response() {
        let mut all_pass = SchroederAllPass::new(2.0, 1_000, 0.5).expect("small delay buffer");
        let output = [1.0, 0.0, 0.0, 0.0, 0.0].map(|sample| all_pass.next_sample(sample));

        assert_eq!(output, [-0.5, 0.0, 0.75, 0.0, 0.375]);
    }

    #[test]
    fn conventional_room_size_uses_the_reference_delays_and_rt60_gain() {
        let channel = ReverbChannel::new(1_000, 1.0, 1.0).expect("valid reverb channel");

        let expected_feedback = 0.001_f64.powf(0.0297);

        assert!((channel.combs[0].feedback - expected_feedback).abs() <= EPSILON);
    }

    #[test]
    fn appends_the_requested_tail_and_preserves_channels() {
        let mut wav_file = impulse_file(2, 1_000, 2);

        wav_file.reverb(0.5, 1.0, 0.5, 100).expect("valid reverb");

        assert_eq!(wav_file.number_of_channels(), 2);
        assert_eq!(wav_file.samples().unwrap().len(), 204);
        for [left, right] in wav_file.samples().unwrap().as_chunks::<2>().0 {
            assert!((left - right).abs() <= EPSILON);
        }
    }

    #[test]
    fn zero_mix_returns_the_input_followed_by_silence() {
        let mut wav_file = impulse_file(1, 1_000, 2);

        wav_file.reverb(0.5, 1.0, 0.0, 2).expect("valid dry reverb");

        assert_eq!(wav_file.samples().unwrap(), &[1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn rejects_invalid_parameters_without_changing_samples() {
        let mut wav_file = impulse_file(1, 48_000, 2);

        assert!(matches!(
            wav_file.reverb(1.1, 1.0, 0.5, 0),
            Err(EffectError::InvalidRoomSize(1.1))
        ));
        assert!(matches!(
            wav_file.reverb(0.5, 0.0, 0.5, 0),
            Err(EffectError::InvalidDecayTime(0.0))
        ));
        assert!(matches!(
            wav_file.reverb(0.5, 1.0, -0.1, 0),
            Err(EffectError::InvalidMix(-0.1))
        ));
        assert_eq!(wav_file.samples().unwrap(), &[1.0, 0.0]);
    }
}
