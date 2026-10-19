use std::f64::consts::PI;

/// A stateful unipolar sine low-frequency oscillator.
///
/// `Lfo` produces values in a configured range and is intended for modulating
/// effect parameters such as delay time, gain, or filter frequency. Call
/// [`Lfo::next_sample`] once per audio sample so that its frequency is measured
/// against the sample rate supplied to [`Lfo::new`].
///
/// The frequency is fixed when the oscillator is created.
///
/// # Example
///
/// ```
/// use wavlab::Lfo;
///
/// let mut lfo = Lfo::new(48_000, 0.5, 0.5, 5.0);
/// let modulation = lfo.next_sample();
/// assert_eq!(modulation, 0.5);
/// ```
#[derive(Clone, Debug)]
pub struct Lfo {
    phase: f64,
    radians_per_sample: f64,
    minimum: f64,
    range: f64,
}

impl Lfo {
    /// Creates an oscillator with the requested frequency and output range.
    ///
    /// `frequency_hz` is the frequency of the unipolar output and must be
    /// finite and non-negative. `minimum` and `maximum` are inclusive bounds;
    /// both and their difference must be finite, and `minimum` must not exceed
    /// `maximum`.
    ///
    /// # Panics
    ///
    /// Panics if `sample_rate` is zero, `frequency_hz` is negative or not
    /// finite, either range bound or their difference is not finite, or
    /// `minimum` exceeds `maximum`.
    pub fn new(sample_rate: u32, frequency_hz: f64, minimum: f64, maximum: f64) -> Self {
        assert!(sample_rate > 0, "LFO sample rate must be greater than zero");
        assert!(
            frequency_hz.is_finite() && frequency_hz >= 0.0,
            "LFO frequency must be finite and non-negative"
        );
        let range = maximum - minimum;
        assert!(
            minimum.is_finite() && maximum.is_finite() && minimum <= maximum && range.is_finite(),
            "LFO range must have finite bounds with minimum no greater than maximum"
        );

        Self {
            phase: 0.0,
            radians_per_sample: PI * frequency_hz / f64::from(sample_rate),
            minimum,
            range,
        }
    }

    /// Returns the current oscillator value and advances it by one sample.
    ///
    /// The internal waveform is the absolute value of a sine wave, scaled to
    /// the inclusive range supplied to [`Lfo::new`]. The first value is the
    /// configured minimum.
    #[must_use]
    pub fn next_sample(&mut self) -> f64 {
        let normalized = self.phase.sin().abs();
        self.phase = (self.phase + self.radians_per_sample).rem_euclid(PI);
        self.minimum + normalized * self.range
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::FRAC_1_SQRT_2;

    use super::Lfo;

    #[test]
    fn produces_one_unipolar_cycle_at_the_requested_frequency() {
        let mut lfo = Lfo::new(8, 1.0, 10.0, 20.0);
        let samples = (0..8).map(|_| lfo.next_sample()).collect::<Vec<_>>();

        let expected = [
            10.0,
            10.0 + 10.0 * (std::f64::consts::PI / 8.0).sin(),
            10.0 + 10.0 * FRAC_1_SQRT_2,
            10.0 + 10.0 * (3.0 * std::f64::consts::PI / 8.0).sin(),
            20.0,
            10.0 + 10.0 * (3.0 * std::f64::consts::PI / 8.0).sin(),
            10.0 + 10.0 * FRAC_1_SQRT_2,
            10.0 + 10.0 * (std::f64::consts::PI / 8.0).sin(),
        ];

        for (&actual, &expected) in samples.iter().zip(&expected) {
            assert!((actual - expected).abs() < 1e-12);
        }
        assert!((lfo.next_sample() - 10.0).abs() < 1e-12);
    }

    #[test]
    fn zero_frequency_stays_at_the_initial_value() {
        let mut lfo = Lfo::new(48_000, 0.0, -1.0, 1.0);

        assert!((0..16).all(|_| lfo.next_sample() == -1.0));
    }

    #[test]
    #[should_panic(expected = "LFO sample rate must be greater than zero")]
    fn rejects_a_zero_sample_rate() {
        let _ = Lfo::new(0, 1.0, 0.0, 1.0);
    }

    #[test]
    #[should_panic(expected = "LFO frequency must be finite and non-negative")]
    fn rejects_an_invalid_frequency() {
        let _ = Lfo::new(48_000, f64::NAN, 0.0, 1.0);
    }

    #[test]
    #[should_panic(
        expected = "LFO range must have finite bounds with minimum no greater than maximum"
    )]
    fn rejects_an_invalid_range() {
        let _ = Lfo::new(48_000, 1.0, 2.0, 1.0);
    }
}
