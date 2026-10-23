use crate::{Eq, Lfo, LfoWaveform, WavFile};

use super::EffectError;

/// The response used by a [`ModulatedFilterSettings`] sweep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModulatedFilterType {
    /// Pass frequencies below the moving cutoff.
    LowPass,
    /// Pass frequencies above the moving cutoff.
    HighPass,
}

/// Cutoff and LFO settings for a modulated low-pass or high-pass filter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModulatedFilterSettings {
    /// Lowest cutoff reached by the sweep, in Hz.
    pub minimum_frequency_hz: f64,
    /// Highest cutoff reached by the sweep, in Hz.
    pub maximum_frequency_hz: f64,
    /// LFO frequency in Hz.
    pub frequency_hz: f64,
    /// Whether the moving filter is low-pass or high-pass.
    pub filter_type: ModulatedFilterType,
    /// LFO waveform used to move the cutoff.
    pub waveform: LfoWaveform,
    /// Quality factor controlling resonance around the cutoff.
    ///
    /// Use approximately `0.7071` for a Butterworth response. Higher values
    /// produce a more resonant peak around the moving cutoff.
    pub q: f64,
}

impl ModulatedFilterSettings {
    /// Creates settings for a cutoff sweep between two frequencies.
    #[must_use]
    pub const fn new(
        minimum_frequency_hz: f64,
        maximum_frequency_hz: f64,
        frequency_hz: f64,
        filter_type: ModulatedFilterType,
        waveform: LfoWaveform,
        q: f64,
    ) -> Self {
        Self {
            minimum_frequency_hz,
            maximum_frequency_hz,
            frequency_hz,
            filter_type,
            waveform,
            q,
        }
    }
}

impl WavFile {
    /// Applies an LFO-modulated low-pass or high-pass filter in place.
    ///
    /// The cutoff moves linearly between `minimum_frequency_hz` and
    /// `maximum_frequency_hz`. Each channel is processed by an independent
    /// filter and LFO so interleaved channels do not share filter state. The
    /// selected waveform begins at the minimum frequency.
    ///
    /// # Errors
    ///
    /// Returns an [`EffectError`] if samples are not loaded, the LFO frequency
    /// is negative or non-finite, the cutoff range is invalid for the file's
    /// sample rate, or Q is not finite and positive.
    pub fn modulated_filter(
        &mut self,
        settings: ModulatedFilterSettings,
    ) -> Result<(), EffectError> {
        validate_settings(settings, self.sample_rate())?;

        let number_of_channels = usize::from(self.number_of_channels());
        let sample_rate = self.sample_rate();
        let samples = self.samples_mut()?;
        let mut processors = (0..number_of_channels)
            .map(|_| ModulatedFilterProcessor::new(settings, sample_rate))
            .collect::<Vec<_>>();

        for frame in samples.chunks_exact_mut(number_of_channels) {
            for (sample, processor) in frame.iter_mut().zip(&mut processors) {
                *sample = processor.next_sample(*sample);
            }
        }

        Ok(())
    }
}

struct ModulatedFilterProcessor {
    filter: Eq,
    filter_type: ModulatedFilterType,
    lfo: Lfo,
    q: f64,
    sample_rate: u32,
}

impl ModulatedFilterProcessor {
    fn new(settings: ModulatedFilterSettings, sample_rate: u32) -> Self {
        let filter = match settings.filter_type {
            ModulatedFilterType::LowPass => {
                Eq::low_pass(settings.minimum_frequency_hz, sample_rate, settings.q)
            }
            ModulatedFilterType::HighPass => {
                Eq::high_pass(settings.minimum_frequency_hz, sample_rate, settings.q)
            }
        };

        Self {
            filter,
            filter_type: settings.filter_type,
            lfo: Lfo::new(
                sample_rate,
                settings.frequency_hz,
                settings.minimum_frequency_hz,
                settings.maximum_frequency_hz,
                settings.waveform,
            ),
            q: settings.q,
            sample_rate,
        }
    }

    fn next_sample(&mut self, sample: f64) -> f64 {
        let cutoff_frequency = self.lfo.next_sample();
        match self.filter_type {
            ModulatedFilterType::LowPass => {
                self.filter
                    .set_low_pass(cutoff_frequency, self.sample_rate, self.q);
            }
            ModulatedFilterType::HighPass => {
                self.filter
                    .set_high_pass(cutoff_frequency, self.sample_rate, self.q);
            }
        }
        self.filter.process_sample(sample)
    }
}

fn validate_settings(
    settings: ModulatedFilterSettings,
    sample_rate: u32,
) -> Result<(), EffectError> {
    if !settings.frequency_hz.is_finite() || settings.frequency_hz < 0.0 {
        return Err(EffectError::InvalidLfoFrequency(settings.frequency_hz));
    }

    let nyquist = f64::from(sample_rate) / 2.0;
    if !settings.minimum_frequency_hz.is_finite()
        || !settings.maximum_frequency_hz.is_finite()
        || settings.minimum_frequency_hz <= 0.0
        || settings.minimum_frequency_hz > settings.maximum_frequency_hz
        || settings.maximum_frequency_hz >= nyquist
    {
        return Err(EffectError::InvalidModulatedFilterFrequencyRange {
            minimum_frequency_hz: settings.minimum_frequency_hz,
            maximum_frequency_hz: settings.maximum_frequency_hz,
            sample_rate,
        });
    }

    if !settings.q.is_finite() || settings.q <= 0.0 {
        return Err(EffectError::InvalidModulatedFilterQ(settings.q));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        EffectError, Eq, Lfo, LfoWaveform, ModulatedFilterSettings, ModulatedFilterType, WavFile,
    };

    const SAMPLE_RATE: u32 = 8_000;

    fn test_file(channels: u16, samples: Vec<f64>) -> WavFile {
        let mut wav_file = WavFile::new(channels, SAMPLE_RATE, 24).expect("valid PCM settings");
        wav_file
            .set_samples(samples)
            .expect("complete sample frames");
        wav_file
    }

    fn expected_samples(
        input: &[f64],
        filter_type: ModulatedFilterType,
        waveform: LfoWaveform,
    ) -> Vec<f64> {
        let mut filter = match filter_type {
            ModulatedFilterType::LowPass => Eq::low_pass(500.0, SAMPLE_RATE, 1.0),
            ModulatedFilterType::HighPass => Eq::high_pass(500.0, SAMPLE_RATE, 1.0),
        };
        let mut lfo = Lfo::new(SAMPLE_RATE, 1_000.0, 500.0, 1_500.0, waveform);

        input
            .iter()
            .map(|sample| {
                let cutoff = lfo.next_sample();
                match filter_type {
                    ModulatedFilterType::LowPass => filter.set_low_pass(cutoff, SAMPLE_RATE, 1.0),
                    ModulatedFilterType::HighPass => filter.set_high_pass(cutoff, SAMPLE_RATE, 1.0),
                }
                filter.process_sample(*sample)
            })
            .collect()
    }

    #[test]
    fn low_pass_cutoff_follows_the_selected_lfo() {
        let input = [1.0, -0.5, 0.25, 0.0, 0.0, 0.0, 0.0, 0.0];
        let expected =
            expected_samples(&input, ModulatedFilterType::LowPass, LfoWaveform::Triangle);
        let mut wav_file = test_file(1, input.to_vec());

        wav_file
            .modulated_filter(ModulatedFilterSettings::new(
                500.0,
                1_500.0,
                1_000.0,
                ModulatedFilterType::LowPass,
                LfoWaveform::Triangle,
                1.0,
            ))
            .expect("valid modulated filter");

        assert_eq!(wav_file.samples().unwrap(), expected);
    }

    #[test]
    fn high_pass_cutoff_follows_the_selected_lfo() {
        let input = [1.0, -0.5, 0.25, 0.0, 0.0, 0.0, 0.0, 0.0];
        let expected = expected_samples(&input, ModulatedFilterType::HighPass, LfoWaveform::Square);
        let mut wav_file = test_file(1, input.to_vec());

        wav_file
            .modulated_filter(ModulatedFilterSettings::new(
                500.0,
                1_500.0,
                1_000.0,
                ModulatedFilterType::HighPass,
                LfoWaveform::Square,
                1.0,
            ))
            .expect("valid modulated filter");

        assert_eq!(wav_file.samples().unwrap(), expected);
    }

    #[test]
    fn channels_have_independent_filter_state() {
        let mono_input = vec![1.0, 0.0, 0.0, 0.0];
        let settings = ModulatedFilterSettings::new(
            500.0,
            1_500.0,
            1_000.0,
            ModulatedFilterType::LowPass,
            LfoWaveform::Sine,
            1.0,
        );
        let mut mono = test_file(1, mono_input.clone());
        mono.modulated_filter(settings).unwrap();

        let stereo_input = mono_input
            .iter()
            .flat_map(|sample| [*sample, *sample])
            .collect();
        let mut stereo = test_file(2, stereo_input);
        stereo.modulated_filter(settings).unwrap();

        let mono_output = mono.samples().unwrap();
        for (frame_index, expected) in mono_output.iter().enumerate() {
            let sample_index = frame_index * 2;
            assert_eq!(
                &stereo.samples().unwrap()[sample_index..sample_index + 2],
                [*expected, *expected]
            );
        }
    }

    #[test]
    fn rejects_invalid_settings_without_changing_samples() {
        let mut wav_file = test_file(1, vec![1.0, 0.0]);
        let original = wav_file.samples().unwrap().to_vec();

        let invalid_range = ModulatedFilterSettings::new(
            1_500.0,
            500.0,
            1.0,
            ModulatedFilterType::LowPass,
            LfoWaveform::Sine,
            1.0,
        );
        assert!(matches!(
            wav_file.modulated_filter(invalid_range),
            Err(EffectError::InvalidModulatedFilterFrequencyRange { .. })
        ));

        let invalid_q = ModulatedFilterSettings::new(
            500.0,
            1_500.0,
            1.0,
            ModulatedFilterType::HighPass,
            LfoWaveform::Sine,
            0.0,
        );
        assert!(matches!(
            wav_file.modulated_filter(invalid_q),
            Err(EffectError::InvalidModulatedFilterQ(0.0))
        ));
        assert_eq!(wav_file.samples().unwrap(), original);
    }
}
