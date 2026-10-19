use crate::WavFile;

use super::{
    DelayChannelSettings, EffectError,
    delay::{apply_delay, apply_stereo_delay},
};

impl WavFile {
    /// Applies a clean delay independently to every channel.
    ///
    /// `feedback` is the delayed signal returned to the delay line and must be
    /// between `-1.0` and `1.0`. `mix` is the wet proportion from `0.0` (dry)
    /// to `1.0` (wet). `tail_milliseconds` appends silence to the input before
    /// processing so delayed repeats can continue beyond the original file.
    /// The feedback path does not colour or otherwise process the repeats.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, a parameter is
    /// invalid, the delay is shorter than one sample, or the output is too
    /// large for the platform.
    pub fn digital_delay(
        &mut self,
        settings: DelayChannelSettings,
        mix: f64,
        tail_milliseconds: u32,
    ) -> Result<(), EffectError> {
        apply_delay(
            self,
            settings,
            mix,
            tail_milliseconds,
            |_, feedback_sample| feedback_sample,
        )
    }

    /// Applies a clean stereo delay with independent channel settings.
    ///
    /// Mono input is promoted to stereo and supplied to both delay lines.
    /// Stereo input retains its original left and right signals. The feedback
    /// path does not colour or otherwise process the repeats.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, a parameter is
    /// invalid, either delay is shorter than one sample, or the output is too
    /// large for the platform.
    pub fn digital_delay_stereo(
        &mut self,
        left: DelayChannelSettings,
        right: DelayChannelSettings,
        mix: f64,
        tail_milliseconds: u32,
    ) -> Result<(), EffectError> {
        apply_stereo_delay(
            self,
            left,
            right,
            mix,
            tail_milliseconds,
            |_, feedback_sample| feedback_sample,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{DelayChannelSettings, DelayError, EffectError, WavFile};

    #[test]
    fn produces_clean_feedback_repeats_and_an_explicit_tail() {
        let mut wav_file = WavFile::new(1, 1_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![1.0, 0.0, 0.0])
            .expect("complete sample frames");

        wav_file
            .digital_delay(DelayChannelSettings::new(2.0, 0.5), 1.0, 4)
            .expect("valid delay");

        assert_eq!(
            wav_file.samples().unwrap(),
            &[0.0, 0.0, 1.0, 0.0, 0.5, 0.0, 0.25]
        );
    }

    #[test]
    fn keeps_stereo_feedback_state_independent() {
        let mut wav_file = WavFile::new(2, 1_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![1.0, 0.0, 0.0, 1.0])
            .expect("complete sample frames");

        wav_file
            .digital_delay(DelayChannelSettings::new(1.0, 0.0), 1.0, 1)
            .expect("valid delay");

        assert_eq!(wav_file.samples().unwrap(), &[0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn stereo_delay_promotes_mono_and_uses_independent_settings() {
        let mut wav_file = WavFile::new(1, 1_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![1.0, 0.0])
            .expect("complete sample frames");

        wav_file
            .digital_delay_stereo(
                DelayChannelSettings::new(1.0, 0.0),
                DelayChannelSettings::new(2.0, 0.0),
                1.0,
                2,
            )
            .expect("valid stereo delay");

        assert_eq!(wav_file.number_of_channels(), 2);
        assert_eq!(
            wav_file.samples().unwrap(),
            &[0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        );
    }

    #[test]
    fn rejects_invalid_parameters_without_changing_samples() {
        let mut wav_file = WavFile::new(1, 1_000, 24).expect("valid PCM settings");
        wav_file
            .set_samples(vec![1.0, 0.0])
            .expect("complete sample frames");

        assert!(matches!(
            wav_file.digital_delay(DelayChannelSettings::new(2.0, 0.5), 1.1, 0),
            Err(EffectError::InvalidMix(1.1))
        ));
        assert!(matches!(
            wav_file.digital_delay(DelayChannelSettings::new(0.0, 0.5), 1.0, 0),
            Err(EffectError::Delay(DelayError::InvalidDelayTime(0.0)))
        ));
        assert_eq!(wav_file.samples().unwrap(), &[1.0, 0.0]);
    }
}
