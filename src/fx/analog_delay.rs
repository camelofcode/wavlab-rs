use std::f64::consts::FRAC_1_SQRT_2;

use crate::{Eq, WavFile};

use super::{
    DEFAULT_DELAY_CUTOFF_HZ, DelayChannelSettings, EffectError,
    delay::{apply_delay, apply_stereo_delay, validate_cutoff},
};

impl WavFile {
    /// Applies a delay with a low-pass filter in each channel's feedback path.
    ///
    /// This uses [`DEFAULT_DELAY_CUTOFF_HZ`] as the low-pass cutoff. Use
    /// [`WavFile::analog_delay_with_cutoff`] to select another frequency.
    /// `feedback` must be between `-1.0` and `1.0`, `mix` must be between `0.0`
    /// and `1.0`, and `tail_milliseconds` controls the appended feedback tail.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, a parameter is
    /// invalid, the delay is shorter than one sample, or the output is too
    /// large for the platform.
    pub fn analog_delay(
        &mut self,
        settings: DelayChannelSettings,
        mix: f64,
        tail_milliseconds: u32,
    ) -> Result<(), EffectError> {
        self.analog_delay_with_cutoff(settings, mix, DEFAULT_DELAY_CUTOFF_HZ, tail_milliseconds)
    }

    /// Applies an analog-style delay with the requested feedback low-pass cutoff.
    ///
    /// A separate Butterworth low-pass filter is maintained for every channel.
    /// Filtering only affects samples returned through the feedback path, so
    /// the first repeat is clean and subsequent repeats become progressively
    /// darker.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, a parameter is
    /// invalid, the cutoff is not between zero and Nyquist, the delay is
    /// shorter than one sample, or the output is too large for the platform.
    pub fn analog_delay_with_cutoff(
        &mut self,
        settings: DelayChannelSettings,
        mix: f64,
        cutoff_hz: f64,
        tail_milliseconds: u32,
    ) -> Result<(), EffectError> {
        let sample_rate = self.sample_rate();
        validate_cutoff(cutoff_hz, sample_rate)?;
        let mut filters = (0..usize::from(self.number_of_channels()))
            .map(|_| Eq::low_pass(cutoff_hz, sample_rate, FRAC_1_SQRT_2))
            .collect::<Vec<_>>();

        apply_delay(
            self,
            settings,
            mix,
            tail_milliseconds,
            |channel, feedback_sample| filters[channel].process_sample(feedback_sample),
        )
    }

    /// Applies an analog-style stereo delay with independent channel settings.
    ///
    /// This uses [`DEFAULT_DELAY_CUTOFF_HZ`] for both feedback filters. Mono
    /// input is promoted to stereo and supplied to both delay lines. Use
    /// [`WavFile::analog_delay_stereo_with_cutoff`] to select another cutoff.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, a parameter is
    /// invalid, either delay is shorter than one sample, or the output is too
    /// large for the platform.
    pub fn analog_delay_stereo(
        &mut self,
        left: DelayChannelSettings,
        right: DelayChannelSettings,
        mix: f64,
        tail_milliseconds: u32,
    ) -> Result<(), EffectError> {
        self.analog_delay_stereo_with_cutoff(
            left,
            right,
            mix,
            DEFAULT_DELAY_CUTOFF_HZ,
            tail_milliseconds,
        )
    }

    /// Applies an analog-style stereo delay with an explicit feedback cutoff.
    ///
    /// Left and right channels have independent delay and Butterworth low-pass
    /// state. Mono input is promoted to stereo and supplied to both channels.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, a parameter is
    /// invalid, the cutoff is not between zero and Nyquist, either delay is
    /// shorter than one sample, or the output is too large for the platform.
    pub fn analog_delay_stereo_with_cutoff(
        &mut self,
        left: DelayChannelSettings,
        right: DelayChannelSettings,
        mix: f64,
        cutoff_hz: f64,
        tail_milliseconds: u32,
    ) -> Result<(), EffectError> {
        let sample_rate = self.sample_rate();
        validate_cutoff(cutoff_hz, sample_rate)?;
        let mut filters = (0..2)
            .map(|_| Eq::low_pass(cutoff_hz, sample_rate, FRAC_1_SQRT_2))
            .collect::<Vec<_>>();

        apply_stereo_delay(
            self,
            left,
            right,
            mix,
            tail_milliseconds,
            |channel, feedback_sample| filters[channel].process_sample(feedback_sample),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{DelayChannelSettings, EffectError, WavFile, fx::DEFAULT_DELAY_CUTOFF_HZ};

    const EPSILON: f64 = 1e-12;

    fn impulse_file(sample_rate: u32, samples: usize) -> WavFile {
        let mut wav_file = WavFile::new(1, sample_rate, 24).expect("valid PCM settings");
        let mut input = vec![0.0; samples];
        input[0] = 1.0;
        wav_file.set_samples(input).expect("complete sample frames");
        wav_file
    }

    #[test]
    fn low_pass_filter_colours_feedback_after_the_first_repeat() {
        let mut digital = impulse_file(1_000, 3);
        let mut analog = impulse_file(1_000, 3);

        digital
            .digital_delay(DelayChannelSettings::new(2, 0.5), 1.0, 4)
            .expect("valid digital delay");
        analog
            .analog_delay_with_cutoff(DelayChannelSettings::new(2, 0.5), 1.0, 200.0, 4)
            .expect("valid analog delay");

        assert_eq!(digital.samples().unwrap()[2], analog.samples().unwrap()[2]);
        assert!((digital.samples().unwrap()[4] - 0.5).abs() <= EPSILON);
        assert!((analog.samples().unwrap()[4] - 0.5).abs() > EPSILON);
    }

    #[test]
    fn default_method_uses_the_documented_cutoff() {
        let mut default = impulse_file(8_000, 10);
        let mut explicit = impulse_file(8_000, 10);

        default
            .analog_delay(DelayChannelSettings::new(1, 0.5), 1.0, 2)
            .expect("valid default delay");
        explicit
            .analog_delay_with_cutoff(
                DelayChannelSettings::new(1, 0.5),
                1.0,
                DEFAULT_DELAY_CUTOFF_HZ,
                2,
            )
            .expect("valid explicit delay");

        assert_eq!(default.samples().unwrap(), explicit.samples().unwrap());
    }

    #[test]
    fn rejects_a_cutoff_at_or_above_nyquist() {
        let mut wav_file = impulse_file(1_000, 2);

        assert!(matches!(
            wav_file.analog_delay_with_cutoff(DelayChannelSettings::new(1, 0.5), 1.0, 500.0, 0,),
            Err(EffectError::InvalidCutoffFrequency { .. })
        ));
        assert_eq!(wav_file.samples().unwrap(), &[1.0, 0.0]);
    }

    #[test]
    fn stereo_method_promotes_mono_input() {
        let mut wav_file = impulse_file(8_000, 10);

        wav_file
            .analog_delay_stereo(
                DelayChannelSettings::new(1, 0.5),
                DelayChannelSettings::new(2, 0.25),
                1.0,
                2,
            )
            .expect("valid stereo delay");

        assert_eq!(wav_file.number_of_channels(), 2);
        assert_eq!(wav_file.samples().unwrap().len(), 52);
        assert_eq!(wav_file.samples().unwrap()[16], 1.0);
        assert_eq!(wav_file.samples().unwrap()[33], 1.0);
    }
}
