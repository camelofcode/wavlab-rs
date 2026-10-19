use std::fmt::{self, Display, Formatter};

/// An error encountered while configuring a [`Delay`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayError {
    /// The sample rate must be greater than zero.
    InvalidSampleRate(u32),
    /// The requested delay resolves to fewer than one sample.
    DelayTooShort {
        /// The requested delay in milliseconds.
        milliseconds: u32,
        /// The sample rate used for the conversion.
        sample_rate: u32,
    },
    /// The requested delay exceeds the configured maximum.
    DelayExceedsMaximum {
        /// The requested delay in milliseconds.
        milliseconds: u32,
        /// The maximum delay in milliseconds.
        maximum_milliseconds: u32,
    },
    /// The delay buffer size cannot be represented on this platform.
    SizeOverflow,
}

impl Display for DelayError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSampleRate(sample_rate) => write!(
                formatter,
                "invalid delay sample rate {sample_rate}; it must be greater than zero"
            ),
            Self::DelayTooShort {
                milliseconds,
                sample_rate,
            } => write!(
                formatter,
                "delay of {milliseconds} ms is shorter than one sample at {sample_rate} Hz"
            ),
            Self::DelayExceedsMaximum {
                milliseconds,
                maximum_milliseconds,
            } => write!(
                formatter,
                "delay of {milliseconds} ms exceeds the configured maximum of {maximum_milliseconds} ms"
            ),
            Self::SizeOverflow => {
                formatter.write_str("delay buffer is too large for this platform")
            }
        }
    }
}

impl std::error::Error for DelayError {}

/// A single-channel delay line with feedback and wet/dry mixing.
///
/// Create one `Delay` per independently processed audio channel. Each call to
/// a `next_sample` method reads the delayed value, writes the current input,
/// and advances the internal ring buffer by one sample.
///
/// # Example
///
/// ```
/// use wavlab::Delay;
///
/// let mut delay = Delay::new(1_000, 250, 48_000)?;
/// let output = delay.next_sample(1.0, 0.4, 0.5);
/// # Ok::<(), wavlab::DelayError>(())
/// ```
#[derive(Clone, Debug)]
pub struct Delay {
    sample_rate: u32,
    maximum_milliseconds: u32,
    delay_samples: usize,
    write_position: usize,
    buffer: Vec<f64>,
}

impl Delay {
    /// Creates a delay line and fills its buffer with silence.
    ///
    /// `maximum_milliseconds` determines the allocated buffer size, while
    /// `initial_milliseconds` sets the initial distance between the write and
    /// read positions. Both delays must resolve to at least one sample, and the
    /// initial delay must not exceed the maximum.
    ///
    /// # Errors
    ///
    /// Returns an error for a zero sample rate, a delay shorter than one
    /// sample, an initial delay greater than the maximum, or a buffer size that
    /// cannot be represented on this platform.
    pub fn new(
        maximum_milliseconds: u32,
        initial_milliseconds: u32,
        sample_rate: u32,
    ) -> Result<Self, DelayError> {
        if sample_rate == 0 {
            return Err(DelayError::InvalidSampleRate(sample_rate));
        }
        if initial_milliseconds > maximum_milliseconds {
            return Err(DelayError::DelayExceedsMaximum {
                milliseconds: initial_milliseconds,
                maximum_milliseconds,
            });
        }

        let maximum_samples = milliseconds_to_samples(maximum_milliseconds, sample_rate)?;
        let delay_samples = milliseconds_to_samples(initial_milliseconds, sample_rate)?;

        Ok(Self {
            sample_rate,
            maximum_milliseconds,
            delay_samples,
            write_position: 0,
            buffer: vec![0.0; maximum_samples],
        })
    }

    /// Changes the delay time without clearing the buffered samples.
    ///
    /// # Errors
    ///
    /// Returns an error if `milliseconds` exceeds the configured maximum,
    /// resolves to fewer than one sample, or cannot be represented on this
    /// platform. The existing delay time is retained when an error occurs.
    pub fn set_delay_time(&mut self, milliseconds: u32) -> Result<(), DelayError> {
        if milliseconds > self.maximum_milliseconds {
            return Err(DelayError::DelayExceedsMaximum {
                milliseconds,
                maximum_milliseconds: self.maximum_milliseconds,
            });
        }

        self.delay_samples = milliseconds_to_samples(milliseconds, self.sample_rate)?;
        Ok(())
    }

    /// Processes one sample with feedback and wet/dry mixing.
    ///
    /// `feedback` scales the delayed sample before it is added to the input and
    /// written back into the delay line. Values with a magnitude of `1.0` or
    /// greater may create a non-decaying or growing signal. `mix` is the wet
    /// proportion: `0.0` returns only the input and `1.0` returns only the
    /// delayed sample. Values outside `0.0..=1.0` are permitted and extrapolate
    /// the mix.
    #[must_use]
    pub fn next_sample(&mut self, sample: f64, feedback: f64, mix: f64) -> f64 {
        self.next_sample_with(sample, feedback, mix, |feedback_sample| feedback_sample)
    }

    /// Processes one sample with feedback processing and wet/dry mixing.
    ///
    /// `feedback_processor` receives the scaled delayed sample before it is
    /// added to the current input. `mix` is the wet proportion: `0.0` returns
    /// only the input and `1.0` returns only the delayed sample. Values outside
    /// `0.0..=1.0` are permitted and extrapolate the mix.
    #[must_use]
    pub fn next_sample_with(
        &mut self,
        sample: f64,
        feedback: f64,
        mix: f64,
        feedback_processor: impl FnOnce(f64) -> f64,
    ) -> f64 {
        let output = self.delayed_sample(0);
        self.buffer[self.write_position] = sample + feedback_processor(feedback * output);
        self.advance();
        mix_samples(sample, output, mix)
    }

    /// Processes one sample using a temporary offset from the delay time.
    ///
    /// The non-negative `offset_samples` moves the read position forward around
    /// the ring buffer, shortening the effective delay. This is suitable for an
    /// integer-sample modulation offset produced by an LFO. `mix` is the wet
    /// proportion, as in [`Delay::next_sample_with`]. No feedback is applied.
    #[must_use]
    pub fn next_sample_with_offset(&mut self, sample: f64, offset_samples: usize, mix: f64) -> f64 {
        let output = self.delayed_sample(offset_samples);
        self.buffer[self.write_position] = sample;
        self.advance();
        mix_samples(sample, output, mix)
    }

    fn delayed_sample(&self, offset_samples: usize) -> f64 {
        let read_position =
            (self.write_position + self.buffer.len() - self.delay_samples) % self.buffer.len();
        let offset = offset_samples % self.buffer.len();
        self.buffer[(read_position + offset) % self.buffer.len()]
    }

    fn advance(&mut self) {
        self.write_position = (self.write_position + 1) % self.buffer.len();
    }
}

fn milliseconds_to_samples(milliseconds: u32, sample_rate: u32) -> Result<usize, DelayError> {
    let samples = u64::from(milliseconds) * u64::from(sample_rate) / 1_000;
    if samples == 0 {
        return Err(DelayError::DelayTooShort {
            milliseconds,
            sample_rate,
        });
    }
    usize::try_from(samples).map_err(|_| DelayError::SizeOverflow)
}

fn mix_samples(input: f64, delayed: f64, mix: f64) -> f64 {
    delayed * mix + input * (1.0 - mix)
}

#[cfg(test)]
mod tests {
    use super::{Delay, DelayError};

    #[test]
    fn delays_an_impulse_by_the_requested_time() {
        let mut delay = Delay::new(10, 3, 1_000).unwrap();
        let input = [1.0, 0.0, 0.0, 0.0, 0.0];
        let output = input.map(|sample| delay.next_sample(sample, 0.0, 1.0));

        assert_eq!(output, [0.0, 0.0, 0.0, 1.0, 0.0]);
    }

    #[test]
    fn feeds_the_delayed_signal_back_into_the_buffer() {
        let mut delay = Delay::new(10, 2, 1_000).unwrap();
        let input = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let output = input.map(|sample| delay.next_sample(sample, 0.5, 1.0));

        assert_eq!(output, [0.0, 0.0, 1.0, 0.0, 0.5, 0.0, 0.25]);
    }

    #[test]
    fn basic_processing_applies_wet_dry_mix() {
        let mut delay = Delay::new(10, 2, 1_000).unwrap();

        assert_eq!(delay.next_sample(1.0, 0.0, 0.25), 0.75);
        assert_eq!(delay.next_sample(0.0, 0.0, 0.25), 0.0);
        assert_eq!(delay.next_sample(0.0, 0.0, 0.25), 0.25);
    }

    #[test]
    fn applies_feedback_processing_and_wet_dry_mix() {
        let mut delay = Delay::new(10, 2, 1_000).unwrap();

        assert_eq!(delay.next_sample_with(1.0, 0.5, 0.25, |value| -value), 0.75);
        assert_eq!(delay.next_sample_with(0.0, 0.5, 0.25, |value| -value), 0.0);
        assert_eq!(delay.next_sample_with(0.0, 0.5, 0.25, |value| -value), 0.25);
        assert_eq!(delay.next_sample_with(0.0, 0.5, 0.25, |value| -value), 0.0);
        assert_eq!(
            delay.next_sample_with(0.0, 0.5, 0.25, |value| -value),
            -0.125
        );
    }

    #[test]
    fn an_offset_shortens_the_effective_delay() {
        let mut delay = Delay::new(10, 4, 1_000).unwrap();
        let input = [1.0, 0.0, 0.0];
        let output = input.map(|sample| delay.next_sample_with_offset(sample, 2, 1.0));

        assert_eq!(output, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn changes_delay_time_without_clearing_the_buffer() {
        let mut delay = Delay::new(10, 4, 1_000).unwrap();
        assert_eq!(delay.next_sample(1.0, 0.0, 1.0), 0.0);
        delay.set_delay_time(2).unwrap();
        assert_eq!(delay.next_sample(0.0, 0.0, 1.0), 0.0);
        assert_eq!(delay.next_sample(0.0, 0.0, 1.0), 1.0);
    }

    #[test]
    fn rejects_invalid_configuration_without_changing_the_delay() {
        assert_eq!(
            Delay::new(10, 2, 0).unwrap_err(),
            DelayError::InvalidSampleRate(0)
        );
        assert!(matches!(
            Delay::new(10, 11, 1_000),
            Err(DelayError::DelayExceedsMaximum { .. })
        ));
        assert!(matches!(
            Delay::new(0, 0, 1_000),
            Err(DelayError::DelayTooShort { .. })
        ));

        let mut delay = Delay::new(10, 2, 1_000).unwrap();
        assert!(delay.set_delay_time(11).is_err());
        assert_eq!(delay.next_sample(1.0, 0.0, 1.0), 0.0);
        assert_eq!(delay.next_sample(0.0, 0.0, 1.0), 0.0);
        assert_eq!(delay.next_sample(0.0, 0.0, 1.0), 1.0);
    }
}
