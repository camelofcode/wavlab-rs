use crate::{
    constants::TAU,
    wav_file::{WavError, WavFile},
};

mod all_pass;
mod band_pass;
mod high_pass;
mod high_shelf;
mod low_pass;
mod low_shelf;
mod peak;

/// A stateful second-order IIR (biquad) equalization filter.
///
/// `Eq` implements the Robert Bristow-Johnson
/// [Audio EQ Cookbook](https://webaudio.github.io/Audio-EQ-Cookbook/audio-eq-cookbook.html)
/// formulas as a Direct Form I biquad. A filter stores the two preceding input
/// and output samples, so create a separate instance for every independently
/// processed audio channel.
///
/// Constructors expect finite arguments, a nonzero sample rate, a center
/// frequency between zero and the Nyquist frequency, and a positive Q or
/// bandwidth. They do not validate these conditions; invalid inputs can
/// produce non-finite coefficients and output samples.
///
/// # Example
///
/// ```
/// use wavlab::Eq;
///
/// let mut filter = Eq::low_pass(1_000.0, 48_000, 1.0);
/// let output_sample = filter.process_sample(0.5);
/// assert!(output_sample.is_finite());
/// ```
pub struct Eq {
    b0a0: f64,
    b1a0: f64,
    b2a0: f64,
    a1a0: f64,
    a2a0: f64,
    in1: f64,
    in2: f64,
    out1: f64,
    out2: f64,
}

struct EqCoefficients {
    omega: f64,
    sin_omega: f64,
    cos_omega: f64,
    alpha: f64,
    beta: f64,
    amplitude: f64,
    a0: f64,
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
}

impl Eq {
    /// Processes one sample and advances the filter's internal delay state.
    ///
    /// Samples must be supplied in chronological order. Do not use one `Eq`
    /// instance for interleaved channels, because samples from one channel
    /// would then affect the filter state of another.
    ///
    /// # Arguments
    ///
    /// - `input`: The next sample for the channel associated with this filter.
    ///
    /// The result is not clipped to the WAV sample range.
    pub fn process_sample(&mut self, input: f64) -> f64 {
        let yn: f64 = self.b0a0 * input + self.b1a0 * self.in1 + self.b2a0 * self.in2
            - self.a1a0 * self.out1
            - self.a2a0 * self.out2;

        self.in2 = self.in1;
        self.in1 = input;
        self.out2 = self.out1;
        self.out1 = yn;

        yn
    }

    fn new<FCoefficients>(
        center_frequency: f64,
        sample_rate: u32,
        calculate_coefficients: FCoefficients,
    ) -> Eq
    where
        FCoefficients: FnOnce(&mut EqCoefficients),
    {
        let omega: f64 = TAU * center_frequency / sample_rate as f64;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();

        let mut coefficients = EqCoefficients {
            omega,
            sin_omega,
            cos_omega,
            alpha: 0.0,
            beta: 0.0,
            amplitude: 0.0,
            a0: 0.0,
            a1: 0.0,
            a2: 0.0,
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
        };
        calculate_coefficients(&mut coefficients);

        Eq {
            b0a0: coefficients.b0 / coefficients.a0,
            b1a0: coefficients.b1 / coefficients.a0,
            b2a0: coefficients.b2 / coefficients.a0,
            a1a0: coefficients.a1 / coefficients.a0,
            a2a0: coefficients.a2 / coefficients.a0,
            in1: 0.0,
            in2: 0.0,
            out1: 0.0,
            out2: 0.0,
        }
    }
}

impl WavFile {
    fn apply_eq<CreateEq>(&mut self, create_eq: CreateEq) -> Result<(), WavError>
    where
        CreateEq: Fn(u32) -> Eq,
    {
        let number_of_channels = usize::from(self.number_of_channels());
        let sample_rate = self.sample_rate();
        let samples = self.samples_mut()?;
        let mut filters = (0..number_of_channels)
            .map(|_| create_eq(sample_rate))
            .collect::<Vec<_>>();

        for frame in samples.chunks_exact_mut(number_of_channels) {
            for (sample, filter) in frame.iter_mut().zip(&mut filters) {
                *sample = filter.process_sample(*sample);
            }
        }

        Ok(())
    }
}

fn calculate_alpha_for_q(coefficients: &mut EqCoefficients, q: f64) {
    coefficients.alpha = coefficients.sin_omega / (2.0 * q);
}

fn calculate_alpha_for_bandwidth(coefficients: &mut EqCoefficients, bandwidth: f64) {
    coefficients.alpha = coefficients.sin_omega
        * (2.0f64.ln() / 2.0 * bandwidth * coefficients.omega / coefficients.sin_omega).sinh();
}

fn calculate_beta(coefficients: &mut EqCoefficients, q: f64) {
    coefficients.beta = coefficients.amplitude.sqrt() / q;
}

fn calculate_amplitude(coefficients: &mut EqCoefficients, gain_decibels: f64) {
    coefficients.amplitude = 10.0f64.powf(gain_decibels / 40.0f64);
}

fn set_pass_denominator(coefficients: &mut EqCoefficients) {
    coefficients.a0 = 1.0 + coefficients.alpha;
    coefficients.a1 = -2.0 * coefficients.cos_omega;
    coefficients.a2 = 1.0 - coefficients.alpha;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::LN_2;

    const SAMPLE_RATE: u32 = 48_000;
    const CENTER_FREQUENCY: f64 = 6_000.0;
    const TOLERANCE: f64 = 1e-10;

    fn assert_approximately_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= TOLERANCE,
            "expected {expected}, got {actual}"
        );
    }

    fn magnitude_at(filter: &Eq, frequency: f64) -> f64 {
        let omega = TAU * frequency / SAMPLE_RATE as f64;
        let cos_omega = omega.cos();
        let sin_omega = omega.sin();
        let cos_two_omega = (2.0 * omega).cos();
        let sin_two_omega = (2.0 * omega).sin();

        let numerator_real = filter.b0a0 + filter.b1a0 * cos_omega + filter.b2a0 * cos_two_omega;
        let numerator_imaginary = -filter.b1a0 * sin_omega - filter.b2a0 * sin_two_omega;
        let denominator_real = 1.0 + filter.a1a0 * cos_omega + filter.a2a0 * cos_two_omega;
        let denominator_imaginary = -filter.a1a0 * sin_omega - filter.a2a0 * sin_two_omega;

        numerator_real.hypot(numerator_imaginary) / denominator_real.hypot(denominator_imaginary)
    }

    #[test]
    fn octave_bandwidth_uses_the_natural_logarithm_of_two() {
        let omega = TAU * CENTER_FREQUENCY / SAMPLE_RATE as f64;
        let mut coefficients = EqCoefficients {
            omega,
            sin_omega: omega.sin(),
            cos_omega: omega.cos(),
            alpha: 0.0,
            beta: 0.0,
            amplitude: 0.0,
            a0: 0.0,
            a1: 0.0,
            a2: 0.0,
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
        };

        calculate_alpha_for_bandwidth(&mut coefficients, 1.0);

        let expected = omega.sin() * (LN_2 / 2.0 * omega / omega.sin()).sinh();
        assert_approximately_eq(coefficients.alpha, expected);
    }

    #[test]
    fn peak_reaches_the_requested_gain_at_its_center_frequency() {
        let filter = Eq::peak(CENTER_FREQUENCY, SAMPLE_RATE, 1.0, 6.0);

        assert_approximately_eq(magnitude_at(&filter, 0.0), 1.0);
        assert_approximately_eq(
            magnitude_at(&filter, CENTER_FREQUENCY),
            10.0f64.powf(6.0 / 20.0),
        );
        assert_approximately_eq(magnitude_at(&filter, SAMPLE_RATE as f64 / 2.0), 1.0);
    }

    #[test]
    fn shelves_apply_gain_to_the_correct_end_of_the_spectrum() {
        let gain = 10.0f64.powf(6.0 / 20.0);
        let midpoint_gain = 10.0f64.powf(6.0 / 40.0);
        let low_shelf = Eq::low_shelf(CENTER_FREQUENCY, SAMPLE_RATE, 1.0, 6.0);
        let high_shelf = Eq::high_shelf(CENTER_FREQUENCY, SAMPLE_RATE, 1.0, 6.0);

        assert_approximately_eq(magnitude_at(&low_shelf, 0.0), gain);
        assert_approximately_eq(magnitude_at(&low_shelf, CENTER_FREQUENCY), midpoint_gain);
        assert_approximately_eq(magnitude_at(&low_shelf, SAMPLE_RATE as f64 / 2.0), 1.0);
        assert_approximately_eq(magnitude_at(&high_shelf, 0.0), 1.0);
        assert_approximately_eq(magnitude_at(&high_shelf, CENTER_FREQUENCY), midpoint_gain);
        assert_approximately_eq(magnitude_at(&high_shelf, SAMPLE_RATE as f64 / 2.0), gain);
    }

    #[test]
    fn low_and_high_pass_filters_reject_opposite_ends_of_the_spectrum() {
        let low_pass = Eq::low_pass(CENTER_FREQUENCY, SAMPLE_RATE, 1.0);
        let high_pass = Eq::high_pass(CENTER_FREQUENCY, SAMPLE_RATE, 1.0);

        assert_approximately_eq(magnitude_at(&low_pass, 0.0), 1.0);
        assert_approximately_eq(magnitude_at(&low_pass, SAMPLE_RATE as f64 / 2.0), 0.0);
        assert_approximately_eq(magnitude_at(&high_pass, 0.0), 0.0);
        assert_approximately_eq(magnitude_at(&high_pass, SAMPLE_RATE as f64 / 2.0), 1.0);
    }

    #[test]
    fn band_pass_rejects_both_ends_of_the_spectrum() {
        let filter = Eq::band_pass(CENTER_FREQUENCY, SAMPLE_RATE, 1.0);

        assert_approximately_eq(magnitude_at(&filter, 0.0), 0.0);
        assert!(magnitude_at(&filter, CENTER_FREQUENCY) > 0.0);
        assert_approximately_eq(magnitude_at(&filter, SAMPLE_RATE as f64 / 2.0), 0.0);
    }

    #[test]
    fn all_pass_has_unity_magnitude_across_the_spectrum() {
        let filter = Eq::all_pass(CENTER_FREQUENCY, SAMPLE_RATE, 1.0);

        for frequency in [0.0, 1_000.0, CENTER_FREQUENCY, 12_000.0, 24_000.0] {
            assert_approximately_eq(magnitude_at(&filter, frequency), 1.0);
        }
    }

    #[test]
    fn process_sample_advances_the_biquad_delay_state() {
        let mut filter = Eq::low_pass(CENTER_FREQUENCY, SAMPLE_RATE, 1.0);
        let [b0, b1, b2, a1, a2] = [
            filter.b0a0,
            filter.b1a0,
            filter.b2a0,
            filter.a1a0,
            filter.a2a0,
        ];

        let first = filter.process_sample(1.0);
        let second = filter.process_sample(0.0);
        let third = filter.process_sample(0.0);

        assert_approximately_eq(first, b0);
        assert_approximately_eq(second, b1 - a1 * first);
        assert_approximately_eq(third, b2 - a1 * second - a2 * first);
    }

    #[test]
    fn normalized_coefficients_are_finite_for_typical_inputs() {
        let filters = [
            Eq::peak(CENTER_FREQUENCY, SAMPLE_RATE, 1.0, 6.0),
            Eq::low_shelf(CENTER_FREQUENCY, SAMPLE_RATE, 1.0, 6.0),
            Eq::high_shelf(CENTER_FREQUENCY, SAMPLE_RATE, 1.0, 6.0),
            Eq::low_pass(CENTER_FREQUENCY, SAMPLE_RATE, 1.0),
            Eq::high_pass(CENTER_FREQUENCY, SAMPLE_RATE, 1.0),
            Eq::band_pass(CENTER_FREQUENCY, SAMPLE_RATE, 1.0),
            Eq::all_pass(CENTER_FREQUENCY, SAMPLE_RATE, 1.0),
        ];

        for filter in filters {
            for coefficient in [
                filter.b0a0,
                filter.b1a0,
                filter.b2a0,
                filter.a1a0,
                filter.a2a0,
            ] {
                assert!(coefficient.is_finite());
            }
        }
    }

    fn assert_wav_filter_matches_eq(
        mut expected_filter: Eq,
        apply_filter: impl FnOnce(&mut WavFile) -> Result<(), WavError>,
    ) {
        let input = [1.0, -0.5, 0.25, 0.0];
        let expected = input
            .iter()
            .map(|sample| expected_filter.process_sample(*sample))
            .collect::<Vec<_>>();
        let mut wav_file = WavFile::new(1, SAMPLE_RATE, 24).expect("valid PCM settings");
        wav_file
            .set_samples(input.to_vec())
            .expect("complete sample frames");

        apply_filter(&mut wav_file).expect("loaded samples");

        for (&actual, expected) in wav_file.samples().unwrap().iter().zip(expected) {
            assert_approximately_eq(actual, expected);
        }
    }

    #[test]
    fn wav_file_methods_construct_the_corresponding_filters() {
        assert_wav_filter_matches_eq(
            Eq::peak(CENTER_FREQUENCY, SAMPLE_RATE, 1.0, 6.0),
            |wav_file| wav_file.apply_peak_eq(CENTER_FREQUENCY, 1.0, 6.0),
        );
        assert_wav_filter_matches_eq(
            Eq::low_shelf(CENTER_FREQUENCY, SAMPLE_RATE, 1.0, 6.0),
            |wav_file| wav_file.apply_low_shelf_eq(CENTER_FREQUENCY, 1.0, 6.0),
        );
        assert_wav_filter_matches_eq(
            Eq::high_shelf(CENTER_FREQUENCY, SAMPLE_RATE, 1.0, 6.0),
            |wav_file| wav_file.apply_high_shelf_eq(CENTER_FREQUENCY, 1.0, 6.0),
        );
        assert_wav_filter_matches_eq(
            Eq::low_pass(CENTER_FREQUENCY, SAMPLE_RATE, 1.0),
            |wav_file| wav_file.apply_low_pass_eq(CENTER_FREQUENCY, 1.0),
        );
        assert_wav_filter_matches_eq(
            Eq::high_pass(CENTER_FREQUENCY, SAMPLE_RATE, 1.0),
            |wav_file| wav_file.apply_high_pass_eq(CENTER_FREQUENCY, 1.0),
        );
        assert_wav_filter_matches_eq(
            Eq::band_pass(CENTER_FREQUENCY, SAMPLE_RATE, 1.0),
            |wav_file| wav_file.apply_band_pass_eq(CENTER_FREQUENCY, 1.0),
        );
        assert_wav_filter_matches_eq(
            Eq::all_pass(CENTER_FREQUENCY, SAMPLE_RATE, 1.0),
            |wav_file| wav_file.apply_all_pass_eq(CENTER_FREQUENCY, 1.0),
        );
    }

    #[test]
    fn wav_file_uses_independent_filter_state_for_each_channel() {
        let input = [1.0, -0.25, 0.5, 0.0, 0.0, 0.75];
        let mut expected = input;
        let mut filters = [
            Eq::low_pass(CENTER_FREQUENCY, SAMPLE_RATE, 1.0),
            Eq::low_pass(CENTER_FREQUENCY, SAMPLE_RATE, 1.0),
        ];
        for (sample_index, sample) in expected.iter_mut().enumerate() {
            let channel = sample_index % filters.len();
            *sample = filters[channel].process_sample(*sample);
        }

        let mut wav_file = WavFile::new(2, SAMPLE_RATE, 24).expect("valid PCM settings");
        wav_file
            .set_samples(input.to_vec())
            .expect("complete sample frames");

        wav_file
            .apply_low_pass_eq(CENTER_FREQUENCY, 1.0)
            .expect("loaded samples");

        for (&actual, expected) in wav_file.samples().unwrap().iter().zip(expected) {
            assert_approximately_eq(actual, expected);
        }
    }
}
