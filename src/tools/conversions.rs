use super::DelayError;

pub(crate) fn milliseconds_to_samples(
    milliseconds: f64,
    sample_rate: u32,
) -> Result<usize, DelayError> {
    if !milliseconds.is_finite() || milliseconds <= 0.0 {
        return Err(DelayError::InvalidDelayTime(milliseconds));
    }

    let samples = (milliseconds * f64::from(sample_rate) / 1_000.0).round();
    if samples < 1.0 {
        return Err(DelayError::DelayTooShort {
            milliseconds,
            sample_rate,
        });
    }
    if samples > usize::MAX as f64 {
        return Err(DelayError::SizeOverflow);
    }
    Ok(samples as usize)
}

pub(crate) fn samples_to_milliseconds(samples: usize, sample_rate: u32) -> f64 {
    samples as f64 * 1_000.0 / f64::from(sample_rate)
}

#[cfg(test)]
mod tests {
    use super::{milliseconds_to_samples, samples_to_milliseconds};

    #[test]
    fn converts_fractional_milliseconds_to_the_nearest_sample() {
        assert_eq!(milliseconds_to_samples(2.5, 1_000).unwrap(), 3);
        assert_eq!(milliseconds_to_samples(1.0, 48_000).unwrap(), 48);
    }

    #[test]
    fn converts_samples_to_fractional_milliseconds() {
        assert_eq!(samples_to_milliseconds(1, 48_000), 1_000.0 / 48_000.0);
    }
}
