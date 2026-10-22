use crate::{
    Delay, Lfo, LfoWaveform, WavFile,
    tools::conversions::{milliseconds_to_samples, samples_to_milliseconds},
};

use super::{EffectError, delay::validate_mix};

/// Maximum delay reached by a flanger at full depth, in milliseconds.
pub const MAX_FLANGER_DELAY_MILLISECONDS: f64 = 10.0;

/// Maximum magnitude accepted for flanger feedback.
pub const MAX_FLANGER_FEEDBACK: f64 = 0.95;

/// Minimum supported flanger LFO frequency, in Hz.
pub const MIN_FLANGER_FREQUENCY_HZ: f64 = 0.1;

/// Maximum supported flanger LFO frequency, in Hz.
pub const MAX_FLANGER_FREQUENCY_HZ: f64 = 10.0;

/// LFO and feedback settings for one channel of a flanger effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlangerChannelSettings {
    /// Normalized modulation depth from `0.0` to `1.0`.
    ///
    /// Full depth sweeps the delay from one sample to 10 milliseconds.
    pub depth: f64,
    /// Triangle-LFO frequency in Hz, from `0.1` to `10.0`.
    pub frequency_hz: f64,
    /// Signed feedback from `-0.95` to `0.95`.
    ///
    /// Positive values can produce a pronounced level increase when the delay
    /// approaches one sample. Negative feedback produces cancellation instead.
    pub feedback: f64,
}

impl FlangerChannelSettings {
    /// Creates flanger settings from depth, LFO frequency, and feedback.
    #[must_use]
    pub const fn new(depth: f64, frequency_hz: f64, feedback: f64) -> Self {
        Self {
            depth,
            frequency_hz,
            feedback,
        }
    }
}

impl WavFile {
    /// Applies flanging using the same settings independently on every channel.
    ///
    /// Mono input remains mono and stereo input remains stereo. The triangle
    /// LFO sweeps between a causal one-sample minimum and the maximum selected
    /// by `settings.depth`. `mix` is the wet proportion from `0.0` to `1.0`.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded or any setting is
    /// outside its documented range.
    pub fn flanger(
        &mut self,
        settings: FlangerChannelSettings,
        mix: f64,
    ) -> Result<(), EffectError> {
        let channels = vec![settings; usize::from(self.number_of_channels())];
        apply_flanger_channels(self, &channels, mix)
    }

    /// Applies stereo flanging with independent left and right settings.
    ///
    /// Mono input is promoted to stereo and supplied to both independently
    /// configured channels. Stereo input retains its original channel signals.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded or any setting is
    /// outside its documented range.
    pub fn flanger_stereo(
        &mut self,
        left: FlangerChannelSettings,
        right: FlangerChannelSettings,
        mix: f64,
    ) -> Result<(), EffectError> {
        apply_flanger_channels(self, &[left, right], mix)
    }
}

fn apply_flanger_channels(
    wav_file: &mut WavFile,
    channels: &[FlangerChannelSettings],
    mix: f64,
) -> Result<(), EffectError> {
    for settings in channels {
        validate_settings(*settings)?;
    }
    validate_mix(mix)?;

    let input_channels = usize::from(wav_file.number_of_channels());
    let output_channels = channels.len();
    let sample_rate = wav_file.sample_rate();
    let input = wav_file.samples()?.to_vec();
    let input_frames = input.len() / input_channels;
    let output_samples = input_frames
        .checked_mul(output_channels)
        .ok_or(EffectError::SizeOverflow("flanger output sample count"))?;
    let mut processors = channels
        .iter()
        .map(|settings| FlangerProcessor::new(*settings, sample_rate))
        .collect::<Result<Vec<_>, _>>()?;
    let mut output = Vec::with_capacity(output_samples);

    for frame_index in 0..input_frames {
        for (channel_index, processor) in processors.iter_mut().enumerate() {
            let input_channel = if input_channels == 1 {
                0
            } else {
                channel_index
            };
            let sample = input[frame_index * input_channels + input_channel];
            output.push(processor.next_sample(sample, mix));
        }
    }

    if input_channels == output_channels {
        wav_file.set_samples(output)?;
    } else {
        wav_file.set_channels_and_samples(
            u16::try_from(output_channels)
                .map_err(|_| EffectError::SizeOverflow("flanger output channel count"))?,
            output,
        )?;
    }
    Ok(())
}

struct FlangerProcessor {
    delay: Delay,
    lfo: Lfo,
    feedback: f64,
}

impl FlangerProcessor {
    fn new(settings: FlangerChannelSettings, sample_rate: u32) -> Result<Self, EffectError> {
        let minimum_delay = samples_to_milliseconds(1, sample_rate);
        let maximum_delay = (settings.depth * MAX_FLANGER_DELAY_MILLISECONDS).max(minimum_delay);
        let maximum_samples = milliseconds_to_samples(maximum_delay, sample_rate)?;

        Ok(Self {
            delay: Delay::new(maximum_delay, maximum_delay, sample_rate)?,
            lfo: Lfo::new(
                sample_rate,
                settings.frequency_hz,
                0.0,
                (maximum_samples - 1) as f64,
                LfoWaveform::Triangle,
            ),
            feedback: settings.feedback,
        })
    }

    fn next_sample(&mut self, sample: f64, mix: f64) -> f64 {
        let offset_samples = self.lfo.next_sample().round() as usize;
        self.delay
            .next_sample_with_offset_and_feedback(sample, offset_samples, self.feedback, mix)
    }
}

fn validate_settings(settings: FlangerChannelSettings) -> Result<(), EffectError> {
    if !settings.depth.is_finite() || !(0.0..=1.0).contains(&settings.depth) {
        return Err(EffectError::InvalidFlangerDepth(settings.depth));
    }
    if !settings.frequency_hz.is_finite()
        || !(MIN_FLANGER_FREQUENCY_HZ..=MAX_FLANGER_FREQUENCY_HZ).contains(&settings.frequency_hz)
    {
        return Err(EffectError::InvalidFlangerFrequency(settings.frequency_hz));
    }
    if !settings.feedback.is_finite()
        || !(-MAX_FLANGER_FEEDBACK..=MAX_FLANGER_FEEDBACK).contains(&settings.feedback)
    {
        return Err(EffectError::InvalidFlangerFeedback(settings.feedback));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{EffectError, FlangerChannelSettings, WavFile};

    fn impulse_file(channels: u16, frames: usize) -> WavFile {
        let mut wav_file = WavFile::new(channels, 1_000, 24).expect("valid PCM settings");
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
    fn zero_depth_uses_a_one_sample_delay() {
        let mut wav_file = impulse_file(1, 4);

        wav_file
            .flanger(FlangerChannelSettings::new(0.0, 1.0, 0.0), 1.0)
            .expect("valid flanger");

        assert_eq!(wav_file.samples().unwrap(), &[0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn full_depth_reaches_the_ten_millisecond_maximum() {
        let mut wav_file = impulse_file(1, 12);

        wav_file
            .flanger(FlangerChannelSettings::new(1.0, 0.1, 0.0), 1.0)
            .expect("valid flanger");

        assert_eq!(wav_file.samples().unwrap()[10], 1.0);
    }

    #[test]
    fn maximum_feedback_remains_decaying_at_the_minimum_delay() {
        let mut wav_file = impulse_file(1, 6);

        wav_file
            .flanger(FlangerChannelSettings::new(0.0, 1.0, 0.95), 1.0)
            .expect("valid flanger");

        let expected = [
            0.0,
            1.0,
            0.95,
            0.95_f64.powi(2),
            0.95_f64.powi(3),
            0.95_f64.powi(4),
        ];
        for (&actual, &expected) in wav_file.samples().unwrap().iter().zip(&expected) {
            assert!((actual - expected).abs() < 1e-12);
        }
    }

    #[test]
    fn common_settings_preserve_stereo_channels() {
        let mut wav_file = impulse_file(2, 12);

        wav_file
            .flanger(FlangerChannelSettings::new(1.0, 0.1, -0.5), 0.5)
            .expect("valid flanger");

        assert_eq!(wav_file.number_of_channels(), 2);
    }

    #[test]
    fn stereo_flanger_promotes_mono_and_uses_independent_settings() {
        let mut wav_file = impulse_file(1, 12);

        wav_file
            .flanger_stereo(
                FlangerChannelSettings::new(0.0, 1.0, 0.0),
                FlangerChannelSettings::new(1.0, 0.1, 0.0),
                1.0,
            )
            .expect("valid stereo flanger");

        assert_eq!(wav_file.number_of_channels(), 2);
        let samples = wav_file.samples().unwrap();
        assert_eq!(samples[2], 1.0);
        assert_eq!(samples[3], 0.0);
        assert_eq!(samples[20], 0.0);
        assert_eq!(samples[21], 1.0);
    }

    #[test]
    fn rejects_feedback_that_can_become_unstable() {
        let mut wav_file = impulse_file(1, 4);

        assert!(matches!(
            wav_file.flanger(FlangerChannelSettings::new(0.5, 1.0, 0.951), 0.5),
            Err(EffectError::InvalidFlangerFeedback(0.951))
        ));
    }
}
