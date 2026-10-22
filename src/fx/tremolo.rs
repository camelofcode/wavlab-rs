use crate::{Lfo, LfoWaveform, WavFile};

use super::EffectError;

/// LFO settings for a tremolo effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TremoloSettings {
    /// Modulation depth from `0.0` (unchanged) to `1.0` (down to silence).
    pub depth: f64,
    /// LFO frequency in Hz.
    pub frequency_hz: f64,
    /// LFO waveform used to modulate the gain.
    pub waveform: LfoWaveform,
}

impl TremoloSettings {
    /// Creates tremolo settings from depth, frequency, and waveform.
    #[must_use]
    pub const fn new(depth: f64, frequency_hz: f64, waveform: LfoWaveform) -> Self {
        Self {
            depth,
            frequency_hz,
            waveform,
        }
    }
}

impl WavFile {
    /// Applies LFO-controlled amplitude modulation to every channel.
    ///
    /// The same gain is applied to every sample in a frame, preserving the
    /// stereo image. At full depth the gain ranges from silence to unity; at
    /// zero depth the samples are unchanged.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, depth is outside
    /// `0.0..=1.0`, or frequency is negative or non-finite.
    pub fn tremolo(&mut self, settings: TremoloSettings) -> Result<(), EffectError> {
        validate_settings(settings)?;

        let channels = usize::from(self.number_of_channels());
        let mut lfo = Lfo::new(
            self.sample_rate(),
            settings.frequency_hz,
            1.0 - settings.depth,
            1.0,
            settings.waveform,
        );

        for frame in self.samples_mut()?.chunks_exact_mut(channels) {
            let gain = lfo.next_sample();
            for sample in frame {
                *sample *= gain;
            }
        }
        Ok(())
    }
}

fn validate_settings(settings: TremoloSettings) -> Result<(), EffectError> {
    if !settings.depth.is_finite() || !(0.0..=1.0).contains(&settings.depth) {
        return Err(EffectError::InvalidTremoloDepth(settings.depth));
    }
    if !settings.frequency_hz.is_finite() || settings.frequency_hz < 0.0 {
        return Err(EffectError::InvalidLfoFrequency(settings.frequency_hz));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{EffectError, LfoWaveform, TremoloSettings, WavFile};

    fn constant_file(channels: u16, frames: usize) -> WavFile {
        let mut wav_file = WavFile::new(channels, 4, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![1.0; frames * usize::from(channels)])
            .expect("complete sample frames");
        wav_file
    }

    #[test]
    fn full_depth_reaches_silence_and_unity_gain() {
        let mut wav_file = constant_file(1, 4);

        wav_file
            .tremolo(TremoloSettings::new(1.0, 1.0, LfoWaveform::Triangle))
            .expect("valid tremolo");

        assert_eq!(wav_file.samples().unwrap(), &[0.0, 0.5, 1.0, 0.5]);
    }

    #[test]
    fn zero_depth_preserves_every_channel() {
        let mut wav_file = constant_file(2, 4);
        let original = wav_file.samples().unwrap().to_vec();

        wav_file
            .tremolo(TremoloSettings::new(0.0, 1.0, LfoWaveform::Square))
            .expect("valid tremolo");

        assert_eq!(wav_file.samples().unwrap(), original);
    }

    #[test]
    fn selected_waveform_controls_the_gain_shape() {
        let mut wav_file = constant_file(1, 4);

        wav_file
            .tremolo(TremoloSettings::new(1.0, 1.0, LfoWaveform::Square))
            .expect("valid tremolo");

        assert_eq!(wav_file.samples().unwrap(), &[0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn rejects_invalid_settings_without_changing_samples() {
        let mut wav_file = constant_file(1, 4);
        let original = wav_file.samples().unwrap().to_vec();

        assert!(matches!(
            wav_file.tremolo(TremoloSettings::new(1.1, 1.0, LfoWaveform::Sine)),
            Err(EffectError::InvalidTremoloDepth(1.1))
        ));
        assert_eq!(wav_file.samples().unwrap(), original);
    }
}
