use std::f64::consts::FRAC_1_SQRT_2;

use crate::{Eq, WavFile};

use super::{
    DEFAULT_DELAY_CUTOFF_HZ, DelayChannelSettings, EffectError,
    delay::{apply_delay, apply_stereo_delay, validate_cutoff, validate_drive},
    tape_saturation::saturate_sample,
};

impl WavFile {
    /// Applies a delay with saturation and low-pass filtering in the feedback path.
    ///
    /// This uses [`DEFAULT_DELAY_CUTOFF_HZ`] as the low-pass cutoff. `drive_db`
    /// is the input gain applied before feedback saturation: positive values
    /// create stronger saturation and negative values reduce it. Use
    /// [`WavFile::tape_delay_with_cutoff`] to select another cutoff frequency.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, a parameter is
    /// invalid, the delay is shorter than one sample, or the output is too
    /// large for the platform.
    pub fn tape_delay(
        &mut self,
        settings: DelayChannelSettings,
        mix: f64,
        drive_db: f64,
        tail_milliseconds: u32,
    ) -> Result<(), EffectError> {
        self.tape_delay_with_cutoff(
            settings,
            mix,
            DEFAULT_DELAY_CUTOFF_HZ,
            drive_db,
            tail_milliseconds,
        )
    }

    /// Applies a tape-style delay with explicit cutoff and saturation drive.
    ///
    /// Each channel has independent saturation, Butterworth low-pass, and
    /// delay state. Saturation is applied before low-pass filtering in the
    /// feedback path. The first repeat is unaffected; processing accumulates
    /// on subsequent repeats.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, a parameter is
    /// invalid, the cutoff is not between zero and Nyquist, the delay is
    /// shorter than one sample, or the output is too large for the platform.
    pub fn tape_delay_with_cutoff(
        &mut self,
        settings: DelayChannelSettings,
        mix: f64,
        cutoff_hz: f64,
        drive_db: f64,
        tail_milliseconds: u32,
    ) -> Result<(), EffectError> {
        let sample_rate = self.sample_rate();
        validate_cutoff(cutoff_hz, sample_rate)?;
        validate_drive(drive_db)?;
        let input_gain = 10.0_f64.powf(drive_db / 20.0);
        let mut filters = (0..usize::from(self.number_of_channels()))
            .map(|_| Eq::low_pass(cutoff_hz, sample_rate, FRAC_1_SQRT_2))
            .collect::<Vec<_>>();

        apply_delay(
            self,
            settings,
            mix,
            tail_milliseconds,
            |channel, feedback_sample| {
                let saturated = saturate_sample(feedback_sample, input_gain);
                filters[channel].process_sample(saturated)
            },
        )
    }

    /// Applies a tape-style stereo delay with independent channel settings.
    ///
    /// This uses [`DEFAULT_DELAY_CUTOFF_HZ`] for both feedback filters and the
    /// same saturation drive for both channels. Mono input is promoted to
    /// stereo and supplied to both delay lines. Use
    /// [`WavFile::tape_delay_stereo_with_cutoff`] to select another cutoff.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, a parameter is
    /// invalid, either delay is shorter than one sample, or the output is too
    /// large for the platform.
    pub fn tape_delay_stereo(
        &mut self,
        left: DelayChannelSettings,
        right: DelayChannelSettings,
        mix: f64,
        drive_db: f64,
        tail_milliseconds: u32,
    ) -> Result<(), EffectError> {
        self.tape_delay_stereo_with_cutoff(
            left,
            right,
            mix,
            DEFAULT_DELAY_CUTOFF_HZ,
            drive_db,
            tail_milliseconds,
        )
    }

    /// Applies a tape-style stereo delay with explicit cutoff and drive.
    ///
    /// Each channel has independent delay, saturation, and Butterworth
    /// low-pass state. Mono input is promoted to stereo and supplied to both
    /// channels.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, a parameter is
    /// invalid, the cutoff is not between zero and Nyquist, either delay is
    /// shorter than one sample, or the output is too large for the platform.
    pub fn tape_delay_stereo_with_cutoff(
        &mut self,
        left: DelayChannelSettings,
        right: DelayChannelSettings,
        mix: f64,
        cutoff_hz: f64,
        drive_db: f64,
        tail_milliseconds: u32,
    ) -> Result<(), EffectError> {
        let sample_rate = self.sample_rate();
        validate_cutoff(cutoff_hz, sample_rate)?;
        validate_drive(drive_db)?;
        let input_gain = 10.0_f64.powf(drive_db / 20.0);
        let mut filters = (0..2)
            .map(|_| Eq::low_pass(cutoff_hz, sample_rate, FRAC_1_SQRT_2))
            .collect::<Vec<_>>();

        apply_stereo_delay(
            self,
            left,
            right,
            mix,
            tail_milliseconds,
            |channel, feedback_sample| {
                let saturated = saturate_sample(feedback_sample, input_gain);
                filters[channel].process_sample(saturated)
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{DelayChannelSettings, EffectError, WavFile, fx::DEFAULT_DELAY_CUTOFF_HZ};

    fn mono_impulse_file() -> WavFile {
        let mut wav_file = WavFile::new(1, 8_000, 24).expect("valid PCM settings");
        let mut samples = vec![0.0; 10];
        samples[0] = 1.0;
        wav_file
            .set_samples(samples)
            .expect("complete sample frames");
        wav_file
    }

    #[test]
    fn saturation_changes_feedback_after_the_first_repeat() {
        let mut analog = mono_impulse_file();
        let mut tape = mono_impulse_file();

        analog
            .analog_delay_with_cutoff(DelayChannelSettings::new(1, 0.5), 1.0, 2_000.0, 2)
            .expect("valid analog delay");
        tape.tape_delay_with_cutoff(DelayChannelSettings::new(1, 0.5), 1.0, 2_000.0, 0.0, 2)
            .expect("valid tape delay");

        assert_eq!(analog.samples().unwrap()[8], tape.samples().unwrap()[8]);
        assert_ne!(analog.samples().unwrap()[16], tape.samples().unwrap()[16]);
    }

    #[test]
    fn positive_drive_increases_feedback_saturation() {
        let mut zero_db = mono_impulse_file();
        let mut driven = mono_impulse_file();

        zero_db
            .tape_delay_with_cutoff(DelayChannelSettings::new(1, 0.5), 1.0, 2_000.0, 0.0, 2)
            .expect("valid tape delay");
        driven
            .tape_delay_with_cutoff(DelayChannelSettings::new(1, 0.5), 1.0, 2_000.0, 12.0, 2)
            .expect("valid tape delay");

        assert!(driven.samples().unwrap()[16] > zero_db.samples().unwrap()[16]);
    }

    #[test]
    fn default_method_uses_the_documented_cutoff() {
        let mut default = mono_impulse_file();
        let mut explicit = mono_impulse_file();

        default
            .tape_delay(DelayChannelSettings::new(1, 0.5), 1.0, 0.0, 2)
            .expect("valid default delay");
        explicit
            .tape_delay_with_cutoff(
                DelayChannelSettings::new(1, 0.5),
                1.0,
                DEFAULT_DELAY_CUTOFF_HZ,
                0.0,
                2,
            )
            .expect("valid explicit delay");

        assert_eq!(default.samples().unwrap(), explicit.samples().unwrap());
    }

    #[test]
    fn rejects_non_finite_drive() {
        let mut wav_file = mono_impulse_file();

        assert!(matches!(
            wav_file.tape_delay(
                DelayChannelSettings::new(1, 0.5),
                1.0,
                f64::NAN,
                0,
            ),
            Err(EffectError::InvalidDrive(drive)) if drive.is_nan()
        ));
    }

    #[test]
    fn stereo_method_promotes_mono_input() {
        let mut wav_file = mono_impulse_file();

        wav_file
            .tape_delay_stereo(
                DelayChannelSettings::new(1, 0.5),
                DelayChannelSettings::new(2, 0.25),
                1.0,
                6.0,
                2,
            )
            .expect("valid stereo delay");

        assert_eq!(wav_file.number_of_channels(), 2);
        assert_eq!(wav_file.samples().unwrap().len(), 52);
        assert_eq!(wav_file.samples().unwrap()[16], 1.0);
        assert_eq!(wav_file.samples().unwrap()[33], 1.0);
    }
}
