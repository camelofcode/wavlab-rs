use crate::{Delay, Lfo, LfoWaveform, WavFile, tools::conversions::milliseconds_to_samples};

use super::{EffectError, delay::validate_mix};

/// Fixed center delay used by the chorus, in milliseconds.
pub const CHORUS_CENTER_DELAY_MILLISECONDS: f64 = 14.0;

/// Delay modulation at the maximum chorus depth, in milliseconds either side
/// of the center delay.
pub const MAX_CHORUS_DEPTH_MILLISECONDS: f64 = 10.0;

/// LFO settings for one channel of a chorus effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChorusChannelSettings {
    /// Normalized modulation depth from `0.0` (no modulation) to `1.0`
    /// (plus or minus 10 milliseconds around the center delay).
    pub depth: f64,
    /// LFO frequency in Hz.
    pub frequency_hz: f64,
}

impl ChorusChannelSettings {
    /// Creates chorus settings from a normalized depth and LFO frequency.
    #[must_use]
    pub const fn new(depth: f64, frequency_hz: f64) -> Self {
        Self {
            depth,
            frequency_hz,
        }
    }
}

impl WavFile {
    /// Applies chorus using the same settings independently on every channel.
    ///
    /// Mono input remains mono and stereo input remains stereo. The delay is
    /// centered at 14 milliseconds. `settings.depth` scales its modulation
    /// from no movement to plus or minus 10 milliseconds, and `mix` is the wet
    /// proportion from `0.0` (dry) to `1.0` (wet).
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, a parameter is
    /// invalid, or one of the delay endpoints is shorter than one sample.
    pub fn chorus(&mut self, settings: ChorusChannelSettings, mix: f64) -> Result<(), EffectError> {
        let channels = vec![settings; usize::from(self.number_of_channels())];
        apply_chorus_channels(self, &channels, mix)
    }

    /// Applies stereo chorus with independent left and right channel settings.
    ///
    /// Mono input is promoted to stereo and supplied to both chorus channels.
    /// Stereo input retains its original left and right signals. Each channel
    /// owns independent delay and LFO state.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, a parameter is
    /// invalid, or one of the delay endpoints is shorter than one sample.
    pub fn chorus_stereo(
        &mut self,
        left: ChorusChannelSettings,
        right: ChorusChannelSettings,
        mix: f64,
    ) -> Result<(), EffectError> {
        apply_chorus_channels(self, &[left, right], mix)
    }
}

fn apply_chorus_channels(
    wav_file: &mut WavFile,
    channels: &[ChorusChannelSettings],
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
        .ok_or(EffectError::SizeOverflow("chorus output sample count"))?;

    let mut processors = channels
        .iter()
        .map(|settings| ChorusProcessor::new(*settings, sample_rate))
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
                .map_err(|_| EffectError::SizeOverflow("chorus output channel count"))?,
            output,
        )?;
    }
    Ok(())
}

struct ChorusProcessor {
    delay: Delay,
    lfo: Lfo,
}

impl ChorusProcessor {
    fn new(settings: ChorusChannelSettings, sample_rate: u32) -> Result<Self, EffectError> {
        let depth_milliseconds = settings.depth * MAX_CHORUS_DEPTH_MILLISECONDS;
        let minimum_delay = CHORUS_CENTER_DELAY_MILLISECONDS - depth_milliseconds;
        let maximum_delay = CHORUS_CENTER_DELAY_MILLISECONDS + depth_milliseconds;
        let minimum_samples = milliseconds_to_samples(minimum_delay, sample_rate)?;
        let maximum_samples = milliseconds_to_samples(maximum_delay, sample_rate)?;
        let maximum_offset = maximum_samples - minimum_samples;

        Ok(Self {
            delay: Delay::new(maximum_delay, maximum_delay, sample_rate)?,
            lfo: Lfo::new(
                sample_rate,
                settings.frequency_hz,
                0.0,
                maximum_offset as f64,
                LfoWaveform::Sine,
            ),
        })
    }

    fn next_sample(&mut self, sample: f64, mix: f64) -> f64 {
        let offset_samples = self.lfo.next_sample().round() as usize;
        self.delay
            .next_sample_with_offset(sample, offset_samples, mix)
    }
}

fn validate_settings(settings: ChorusChannelSettings) -> Result<(), EffectError> {
    if !settings.depth.is_finite() || !(0.0..=1.0).contains(&settings.depth) {
        return Err(EffectError::InvalidChorusDepth(settings.depth));
    }
    if !settings.frequency_hz.is_finite() || settings.frequency_hz < 0.0 {
        return Err(EffectError::InvalidLfoFrequency(settings.frequency_hz));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{ChorusChannelSettings, EffectError, WavFile};

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
    fn zero_depth_uses_the_fourteen_millisecond_center_delay() {
        let mut wav_file = impulse_file(1, 20);

        wav_file
            .chorus(ChorusChannelSettings::new(0.0, 2.0), 1.0)
            .expect("valid chorus");

        let samples = wav_file.samples().unwrap();
        assert!(samples[..14].iter().all(|sample| *sample == 0.0));
        assert_eq!(samples[14], 1.0);
    }

    #[test]
    fn full_depth_reaches_four_and_twenty_four_millisecond_delays() {
        let mut minimum = impulse_file(1, 26);
        let mut maximum = impulse_file(1, 26);

        minimum
            .chorus(ChorusChannelSettings::new(1.0, 125.0), 1.0)
            .expect("valid modulated chorus");
        maximum
            .chorus(ChorusChannelSettings::new(1.0, 0.0), 1.0)
            .expect("valid static chorus");

        assert_eq!(minimum.samples().unwrap()[4], 1.0);
        assert_eq!(maximum.samples().unwrap()[24], 1.0);
    }

    #[test]
    fn common_settings_preserve_stereo_channels() {
        let mut wav_file = impulse_file(2, 20);

        wav_file
            .chorus(ChorusChannelSettings::new(0.0, 1.0), 1.0)
            .expect("valid chorus");

        assert_eq!(wav_file.number_of_channels(), 2);
        assert_eq!(&wav_file.samples().unwrap()[28..30], &[1.0, 1.0]);
    }

    #[test]
    fn stereo_chorus_promotes_mono_and_uses_independent_settings() {
        let mut wav_file = impulse_file(1, 26);

        wav_file
            .chorus_stereo(
                ChorusChannelSettings::new(0.0, 0.0),
                ChorusChannelSettings::new(1.0, 0.0),
                1.0,
            )
            .expect("valid stereo chorus");

        assert_eq!(wav_file.number_of_channels(), 2);
        let samples = wav_file.samples().unwrap();
        assert_eq!(samples[14 * 2], 1.0);
        assert_eq!(samples[14 * 2 + 1], 0.0);
        assert_eq!(samples[24 * 2], 0.0);
        assert_eq!(samples[24 * 2 + 1], 1.0);
    }

    #[test]
    fn rejects_invalid_settings_without_changing_samples() {
        let mut wav_file = impulse_file(1, 20);
        let original = wav_file.samples().unwrap().to_vec();

        assert!(matches!(
            wav_file.chorus(ChorusChannelSettings::new(1.1, 1.0), 0.5),
            Err(EffectError::InvalidChorusDepth(1.1))
        ));
        assert!(matches!(
            wav_file.chorus(ChorusChannelSettings::new(0.5, f64::NAN), 0.5),
            Err(EffectError::InvalidLfoFrequency(value)) if value.is_nan()
        ));
        assert_eq!(wav_file.samples().unwrap(), original);
    }
}
