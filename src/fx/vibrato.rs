use crate::{ChorusChannelSettings, WavFile};

use super::EffectError;

/// LFO settings for one channel of a vibrato effect.
///
/// Vibrato uses the same 14 millisecond center delay and normalized depth as
/// chorus: `0.0` applies no modulation and `1.0` moves the delay by plus or
/// minus 10 milliseconds.
pub type VibratoChannelSettings = ChorusChannelSettings;

impl WavFile {
    /// Applies vibrato using the same settings independently on every channel.
    ///
    /// This is chorus with a fully wet mix, so only the pitch-modulated delay
    /// signal is returned. Mono input remains mono and stereo input remains
    /// stereo.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, a parameter is
    /// invalid, or one of the delay endpoints is shorter than one sample.
    pub fn vibrato(&mut self, settings: VibratoChannelSettings) -> Result<(), EffectError> {
        self.chorus(settings, 1.0)
    }

    /// Applies stereo vibrato with independent left and right settings.
    ///
    /// Mono input is promoted to stereo and supplied to both independently
    /// configured channels. Stereo input retains its original channel signals.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, a parameter is
    /// invalid, or one of the delay endpoints is shorter than one sample.
    pub fn vibrato_stereo(
        &mut self,
        left: VibratoChannelSettings,
        right: VibratoChannelSettings,
    ) -> Result<(), EffectError> {
        self.chorus_stereo(left, right, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::{ChorusChannelSettings, VibratoChannelSettings, WavFile};

    fn test_file(channels: u16) -> WavFile {
        let mut wav_file = WavFile::new(channels, 1_000, 24).expect("valid PCM settings");
        let samples = if channels == 1 {
            vec![1.0; 30]
        } else {
            [1.0, -1.0].repeat(30)
        };
        wav_file
            .set_samples(samples)
            .expect("complete sample frames");
        wav_file
    }

    #[test]
    fn vibrato_is_fully_wet_chorus() {
        let settings = VibratoChannelSettings::new(0.75, 2.0);
        let mut vibrato = test_file(2);
        let mut chorus = test_file(2);

        vibrato.vibrato(settings).expect("valid vibrato");
        chorus.chorus(settings, 1.0).expect("valid chorus");

        assert_eq!(vibrato.samples().unwrap(), chorus.samples().unwrap());
    }

    #[test]
    fn stereo_vibrato_is_fully_wet_stereo_chorus() {
        let left = ChorusChannelSettings::new(0.5, 0.8);
        let right = ChorusChannelSettings::new(1.0, 1.2);
        let mut vibrato = test_file(1);
        let mut chorus = test_file(1);

        vibrato
            .vibrato_stereo(left, right)
            .expect("valid stereo vibrato");
        chorus
            .chorus_stereo(left, right, 1.0)
            .expect("valid stereo chorus");

        assert_eq!(vibrato.number_of_channels(), 2);
        assert_eq!(vibrato.samples().unwrap(), chorus.samples().unwrap());
    }
}
