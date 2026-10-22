use crate::{Eq, Lfo, LfoWaveform, WavFile};

use super::{EffectError, delay::validate_mix};

/// Default number of second-order all-pass stages in a phaser.
///
/// Three biquads form a sixth-order all-pass cascade.
pub const DEFAULT_PHASER_STAGES: usize = 3;

/// Maximum number of second-order all-pass stages accepted by the phaser.
pub const MAX_PHASER_STAGES: usize = 16;

/// Frequency swing at full phaser depth, in octaves either side of center.
pub const MAX_PHASER_SWEEP_OCTAVES: f64 = 2.0;

const PHASER_STAGE_BANDWIDTH_OCTAVES: f64 = 1.0;
const NYQUIST_MARGIN: f64 = 0.999_999;

/// LFO and center-frequency settings for one channel of a phaser.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhaserChannelSettings {
    /// Normalized sweep depth from `0.0` to `1.0`.
    ///
    /// Full depth sweeps by two octaves below and above the center frequency,
    /// constrained to valid frequencies below Nyquist.
    pub depth: f64,
    /// LFO frequency in Hz.
    pub frequency_hz: f64,
    /// Geometric center of the frequency sweep, in Hz.
    pub center_frequency_hz: f64,
    /// LFO waveform used to sweep the all-pass stages.
    pub waveform: LfoWaveform,
}

impl PhaserChannelSettings {
    /// Creates phaser settings from depth, LFO frequency, center, and waveform.
    #[must_use]
    pub const fn new(
        depth: f64,
        frequency_hz: f64,
        center_frequency_hz: f64,
        waveform: LfoWaveform,
    ) -> Self {
        Self {
            depth,
            frequency_hz,
            center_frequency_hz,
            waveform,
        }
    }
}

impl WavFile {
    /// Applies a phaser using the same settings independently on every channel.
    ///
    /// Mono input remains mono and stereo input remains stereo. Each channel
    /// has its own filter and LFO state. `stages` is the number of second-order
    /// all-pass filters connected in series, and `mix` is the wet proportion.
    /// A half-wet mix produces the characteristic moving cancellation notches.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, any setting is
    /// invalid, or `stages` is outside `1..=MAX_PHASER_STAGES`.
    pub fn phaser(
        &mut self,
        settings: PhaserChannelSettings,
        stages: usize,
        mix: f64,
    ) -> Result<(), EffectError> {
        let channels = vec![settings; usize::from(self.number_of_channels())];
        apply_phaser_channels(self, &channels, stages, mix)
    }

    /// Applies a stereo phaser with independent left and right LFO settings.
    ///
    /// Mono input is promoted to stereo and supplied to both independently
    /// configured channels. Stereo input retains its original channel signals.
    /// The all-pass stage count and wet/dry mix are common to both channels.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, any setting is
    /// invalid, or `stages` is outside `1..=MAX_PHASER_STAGES`.
    pub fn phaser_stereo(
        &mut self,
        left: PhaserChannelSettings,
        right: PhaserChannelSettings,
        stages: usize,
        mix: f64,
    ) -> Result<(), EffectError> {
        apply_phaser_channels(self, &[left, right], stages, mix)
    }
}

fn apply_phaser_channels(
    wav_file: &mut WavFile,
    channels: &[PhaserChannelSettings],
    stages: usize,
    mix: f64,
) -> Result<(), EffectError> {
    for settings in channels {
        validate_settings(*settings, wav_file.sample_rate())?;
    }
    if !(1..=MAX_PHASER_STAGES).contains(&stages) {
        return Err(EffectError::InvalidPhaserStages(stages));
    }
    validate_mix(mix)?;

    let input_channels = usize::from(wav_file.number_of_channels());
    let output_channels = channels.len();
    let sample_rate = wav_file.sample_rate();
    let input = wav_file.samples()?.to_vec();
    let input_frames = input.len() / input_channels;
    let output_samples = input_frames
        .checked_mul(output_channels)
        .ok_or(EffectError::SizeOverflow("phaser output sample count"))?;
    let mut processors = channels
        .iter()
        .map(|settings| PhaserProcessor::new(*settings, sample_rate, stages))
        .collect::<Vec<_>>();
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
                .map_err(|_| EffectError::SizeOverflow("phaser output channel count"))?,
            output,
        )?;
    }
    Ok(())
}

struct PhaserProcessor {
    filters: Vec<Eq>,
    lfo: Lfo,
    settings: PhaserChannelSettings,
    sample_rate: u32,
}

impl PhaserProcessor {
    fn new(settings: PhaserChannelSettings, sample_rate: u32, stages: usize) -> Self {
        Self {
            filters: (0..stages)
                .map(|_| {
                    Eq::all_pass(
                        settings.center_frequency_hz,
                        sample_rate,
                        PHASER_STAGE_BANDWIDTH_OCTAVES,
                    )
                })
                .collect(),
            lfo: Lfo::new(
                sample_rate,
                settings.frequency_hz,
                -1.0,
                1.0,
                settings.waveform,
            ),
            settings,
            sample_rate,
        }
    }

    fn next_sample(&mut self, dry: f64, mix: f64) -> f64 {
        let lfo_sample = self.lfo.next_sample();
        let sweep_octaves = lfo_sample * self.settings.depth * MAX_PHASER_SWEEP_OCTAVES;
        let nyquist = f64::from(self.sample_rate) / 2.0;
        let center_frequency = (self.settings.center_frequency_hz * 2.0f64.powf(sweep_octaves))
            .min(nyquist * NYQUIST_MARGIN);

        let mut wet = dry;
        for filter in &mut self.filters {
            filter.set_all_pass(
                center_frequency,
                self.sample_rate,
                PHASER_STAGE_BANDWIDTH_OCTAVES,
            );
            wet = filter.process_sample(wet);
        }

        dry * (1.0 - mix) + wet * mix
    }
}

fn validate_settings(settings: PhaserChannelSettings, sample_rate: u32) -> Result<(), EffectError> {
    if !settings.depth.is_finite() || !(0.0..=1.0).contains(&settings.depth) {
        return Err(EffectError::InvalidPhaserDepth(settings.depth));
    }
    if !settings.frequency_hz.is_finite() || settings.frequency_hz < 0.0 {
        return Err(EffectError::InvalidLfoFrequency(settings.frequency_hz));
    }
    let nyquist = f64::from(sample_rate) / 2.0;
    if !settings.center_frequency_hz.is_finite()
        || settings.center_frequency_hz <= 0.0
        || settings.center_frequency_hz >= nyquist
    {
        return Err(EffectError::InvalidPhaserCenterFrequency {
            center_frequency_hz: settings.center_frequency_hz,
            sample_rate,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{EffectError, LfoWaveform, PhaserChannelSettings, WavFile};

    fn impulse_file(channels: u16, frames: usize) -> WavFile {
        let mut wav_file = WavFile::new(channels, 8_000, 24).expect("valid PCM settings");
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
    fn dry_mix_preserves_mono_and_stereo_samples() {
        for channels in [1, 2] {
            let mut wav_file = impulse_file(channels, 16);
            let original = wav_file.samples().unwrap().to_vec();

            wav_file
                .phaser(
                    PhaserChannelSettings::new(1.0, 1.0, 1_000.0, LfoWaveform::Sine),
                    3,
                    0.0,
                )
                .expect("valid phaser");

            assert_eq!(wav_file.number_of_channels(), channels);
            assert_eq!(wav_file.samples().unwrap(), original);
        }
    }

    #[test]
    fn wet_phaser_changes_an_impulse_and_remains_finite() {
        let mut wav_file = impulse_file(1, 64);

        wav_file
            .phaser(
                PhaserChannelSettings::new(1.0, 2.0, 1_000.0, LfoWaveform::Triangle),
                3,
                0.5,
            )
            .expect("valid phaser");

        let samples = wav_file.samples().unwrap();
        assert_ne!(samples[0], 1.0);
        assert!(samples.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn stereo_phaser_promotes_mono_and_uses_independent_settings() {
        let mut wav_file = impulse_file(1, 32);

        wav_file
            .phaser_stereo(
                PhaserChannelSettings::new(0.0, 0.5, 500.0, LfoWaveform::Sine),
                PhaserChannelSettings::new(0.0, 0.5, 2_000.0, LfoWaveform::Sine),
                3,
                0.5,
            )
            .expect("valid stereo phaser");

        assert_eq!(wav_file.number_of_channels(), 2);
        assert_ne!(
            wav_file.samples().unwrap()[0],
            wav_file.samples().unwrap()[1]
        );
    }

    #[test]
    fn rejects_invalid_settings_without_changing_samples() {
        let mut wav_file = impulse_file(1, 8);
        let original = wav_file.samples().unwrap().to_vec();

        assert!(matches!(
            wav_file.phaser(
                PhaserChannelSettings::new(1.1, 1.0, 1_000.0, LfoWaveform::Sine),
                3,
                0.5,
            ),
            Err(EffectError::InvalidPhaserDepth(1.1))
        ));
        assert!(matches!(
            wav_file.phaser(
                PhaserChannelSettings::new(1.0, 1.0, 1_000.0, LfoWaveform::Sine),
                0,
                0.5,
            ),
            Err(EffectError::InvalidPhaserStages(0))
        ));
        assert_eq!(wav_file.samples().unwrap(), original);
    }
}
