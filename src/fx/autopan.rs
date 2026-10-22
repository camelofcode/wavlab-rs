use crate::{Lfo, LfoWaveform, WavFile};

use super::EffectError;

/// LFO settings for an automatic panning effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoPanSettings {
    /// Panning depth from `0.0` (unchanged) to `1.0` (fully left to right).
    pub depth: f64,
    /// LFO frequency in Hz.
    pub frequency_hz: f64,
    /// LFO waveform used to control the panning position.
    pub waveform: LfoWaveform,
}

impl AutoPanSettings {
    /// Creates automatic-panning settings from depth, frequency, and waveform.
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
    /// Applies LFO-controlled panning and always produces stereo output.
    ///
    /// Mono input is supplied to both output channels. Stereo input retains
    /// its left and right signals while their gains move in opposition. At
    /// zero depth stereo input is unchanged; at full depth the LFO sweeps from
    /// fully left to fully right.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, depth is outside
    /// `0.0..=1.0`, frequency is negative or non-finite, or the stereo output
    /// size cannot be represented.
    pub fn autopan(&mut self, settings: AutoPanSettings) -> Result<(), EffectError> {
        validate_settings(settings)?;

        let input_channels = usize::from(self.number_of_channels());
        let input = self.samples()?.to_vec();
        let frames = input.len() / input_channels;
        let output_samples = frames
            .checked_mul(2)
            .ok_or(EffectError::SizeOverflow("autopan output sample count"))?;
        let mut output = Vec::with_capacity(output_samples);
        let mut lfo = Lfo::new(
            self.sample_rate(),
            settings.frequency_hz,
            0.0,
            1.0,
            settings.waveform,
        );

        for frame_index in 0..frames {
            let position = lfo.next_sample();
            let left_gain = 1.0 - settings.depth * position;
            let right_gain = 1.0 - settings.depth * (1.0 - position);
            let left = input[frame_index * input_channels];
            let right = if input_channels == 1 {
                left
            } else {
                input[frame_index * input_channels + 1]
            };
            output.push(left * left_gain);
            output.push(right * right_gain);
        }

        if input_channels == 2 {
            self.set_samples(output)?;
        } else {
            self.set_channels_and_samples(2, output)?;
        }
        Ok(())
    }
}

fn validate_settings(settings: AutoPanSettings) -> Result<(), EffectError> {
    if !settings.depth.is_finite() || !(0.0..=1.0).contains(&settings.depth) {
        return Err(EffectError::InvalidAutoPanDepth(settings.depth));
    }
    if !settings.frequency_hz.is_finite() || settings.frequency_hz < 0.0 {
        return Err(EffectError::InvalidLfoFrequency(settings.frequency_hz));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{AutoPanSettings, EffectError, LfoWaveform, WavFile};

    fn file(channels: u16, samples: Vec<f64>) -> WavFile {
        let mut wav_file = WavFile::new(channels, 4, 24).expect("valid PCM settings");
        wav_file
            .set_samples(samples)
            .expect("complete sample frames");
        wav_file
    }

    #[test]
    fn full_depth_pans_mono_fully_left_and_right() {
        let mut wav_file = file(1, vec![1.0; 4]);

        wav_file
            .autopan(AutoPanSettings::new(1.0, 1.0, LfoWaveform::Triangle))
            .expect("valid autopan");

        assert_eq!(wav_file.number_of_channels(), 2);
        assert_eq!(
            wav_file.samples().unwrap(),
            &[1.0, 0.0, 0.5, 0.5, 0.0, 1.0, 0.5, 0.5]
        );
    }

    #[test]
    fn zero_depth_preserves_stereo_input() {
        let mut wav_file = file(2, vec![1.0, -0.5, 0.25, -0.125]);
        let original = wav_file.samples().unwrap().to_vec();

        wav_file
            .autopan(AutoPanSettings::new(0.0, 1.0, LfoWaveform::Sine))
            .expect("valid autopan");

        assert_eq!(wav_file.samples().unwrap(), original);
    }

    #[test]
    fn selected_waveform_controls_the_pan_shape() {
        let mut wav_file = file(1, vec![1.0; 4]);

        wav_file
            .autopan(AutoPanSettings::new(1.0, 1.0, LfoWaveform::Square))
            .expect("valid autopan");

        assert_eq!(
            wav_file.samples().unwrap(),
            &[1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0]
        );
    }

    #[test]
    fn rejects_invalid_settings_without_changing_samples() {
        let mut wav_file = file(1, vec![1.0; 4]);
        let original = wav_file.samples().unwrap().to_vec();

        assert!(matches!(
            wav_file.autopan(AutoPanSettings::new(-0.1, 1.0, LfoWaveform::Sine)),
            Err(EffectError::InvalidAutoPanDepth(-0.1))
        ));
        assert_eq!(wav_file.number_of_channels(), 1);
        assert_eq!(wav_file.samples().unwrap(), original);
    }
}
