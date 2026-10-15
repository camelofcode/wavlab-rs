const U8_SILENCE: u8 = 128;
const U8_NEGATIVE_SCALE: f64 = U8_SILENCE as f64;
const U8_POSITIVE_SCALE: f64 = (u8::MAX - U8_SILENCE) as f64;

const I16_NEGATIVE_SCALE: f64 = -(i16::MIN as f64);
const I16_POSITIVE_SCALE: f64 = i16::MAX as f64;

const I24_MIN: i32 = -8_388_608;
const I24_MAX: i32 = 8_388_607;
const I24_NEGATIVE_SCALE: f64 = -(I24_MIN as f64);
const I24_POSITIVE_SCALE: f64 = I24_MAX as f64;

const I32_NEGATIVE_SCALE: f64 = -(i32::MIN as f64);
const I32_POSITIVE_SCALE: f64 = i32::MAX as f64;

pub(super) fn pcm_u8_to_f64(sample: u8) -> f64 {
    let centered = i16::from(sample) - i16::from(U8_SILENCE);

    if centered > 0 {
        f64::from(centered) / U8_POSITIVE_SCALE
    } else {
        f64::from(centered) / U8_NEGATIVE_SCALE
    }
}

pub(super) fn f64_to_pcm_u8(sample: f64) -> u8 {
    let sample = clamp_normalized(sample);
    let scaled = if sample >= 0.0 {
        sample * U8_POSITIVE_SCALE
    } else {
        sample * U8_NEGATIVE_SCALE
    };

    (scaled.round() as i16 + i16::from(U8_SILENCE)) as u8
}

pub(super) fn pcm_i16_to_f64(sample: i16) -> f64 {
    signed_pcm_to_f64(i64::from(sample), I16_NEGATIVE_SCALE, I16_POSITIVE_SCALE)
}

pub(super) fn f64_to_pcm_i16(sample: f64) -> i16 {
    f64_to_signed_pcm(sample, I16_NEGATIVE_SCALE, I16_POSITIVE_SCALE) as i16
}

pub(super) fn pcm_i24_to_f64(sample: i32) -> f64 {
    debug_assert!((I24_MIN..=I24_MAX).contains(&sample));
    signed_pcm_to_f64(i64::from(sample), I24_NEGATIVE_SCALE, I24_POSITIVE_SCALE)
}

pub(super) fn f64_to_pcm_i24(sample: f64) -> i32 {
    f64_to_signed_pcm(sample, I24_NEGATIVE_SCALE, I24_POSITIVE_SCALE) as i32
}

pub(super) fn pcm_i32_to_f64(sample: i32) -> f64 {
    signed_pcm_to_f64(i64::from(sample), I32_NEGATIVE_SCALE, I32_POSITIVE_SCALE)
}

pub(super) fn f64_to_pcm_i32(sample: f64) -> i32 {
    f64_to_signed_pcm(sample, I32_NEGATIVE_SCALE, I32_POSITIVE_SCALE) as i32
}

fn signed_pcm_to_f64(sample: i64, negative_scale: f64, positive_scale: f64) -> f64 {
    let scale = if sample > 0 {
        positive_scale
    } else {
        negative_scale
    };

    sample as f64 / scale
}

fn f64_to_signed_pcm(sample: f64, negative_scale: f64, positive_scale: f64) -> i64 {
    let sample = clamp_normalized(sample);
    let scale = if sample >= 0.0 {
        positive_scale
    } else {
        negative_scale
    };

    (sample * scale).round() as i64
}

fn clamp_normalized(sample: f64) -> f64 {
    if sample.is_nan() {
        0.0
    } else {
        sample.clamp(-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        I24_MAX, I24_MIN, U8_SILENCE, f64_to_pcm_i16, f64_to_pcm_i24, f64_to_pcm_i32,
        f64_to_pcm_u8, pcm_i16_to_f64, pcm_i24_to_f64, pcm_i32_to_f64, pcm_u8_to_f64,
    };

    #[test]
    fn converts_unsigned_8_bit_boundaries_and_silence() {
        assert_eq!(pcm_u8_to_f64(u8::MIN), -1.0);
        assert_eq!(pcm_u8_to_f64(U8_SILENCE), 0.0);
        assert_eq!(pcm_u8_to_f64(u8::MAX), 1.0);

        assert_eq!(f64_to_pcm_u8(-1.0), u8::MIN);
        assert_eq!(f64_to_pcm_u8(0.0), U8_SILENCE);
        assert_eq!(f64_to_pcm_u8(1.0), u8::MAX);
    }

    #[test]
    fn converts_signed_pcm_boundaries() {
        assert_eq!(pcm_i16_to_f64(i16::MIN), -1.0);
        assert_eq!(pcm_i16_to_f64(0), 0.0);
        assert_eq!(pcm_i16_to_f64(i16::MAX), 1.0);

        assert_eq!(pcm_i24_to_f64(I24_MIN), -1.0);
        assert_eq!(pcm_i24_to_f64(0), 0.0);
        assert_eq!(pcm_i24_to_f64(I24_MAX), 1.0);

        assert_eq!(pcm_i32_to_f64(i32::MIN), -1.0);
        assert_eq!(pcm_i32_to_f64(0), 0.0);
        assert_eq!(pcm_i32_to_f64(i32::MAX), 1.0);
    }

    #[test]
    fn round_trips_representative_integer_samples() {
        for sample in [
            u8::MIN,
            1,
            U8_SILENCE - 1,
            U8_SILENCE,
            U8_SILENCE + 1,
            u8::MAX - 1,
            u8::MAX,
        ] {
            assert_eq!(f64_to_pcm_u8(pcm_u8_to_f64(sample)), sample);
        }

        for sample in [i16::MIN, i16::MIN + 1, -1, 0, 1, i16::MAX - 1, i16::MAX] {
            assert_eq!(f64_to_pcm_i16(pcm_i16_to_f64(sample)), sample);
        }

        for sample in [I24_MIN, I24_MIN + 1, -1, 0, 1, I24_MAX - 1, I24_MAX] {
            assert_eq!(f64_to_pcm_i24(pcm_i24_to_f64(sample)), sample);
        }

        for sample in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
            assert_eq!(f64_to_pcm_i32(pcm_i32_to_f64(sample)), sample);
        }
    }

    #[test]
    fn clamps_out_of_range_floating_point_samples() {
        assert_eq!(f64_to_pcm_u8(-2.0), u8::MIN);
        assert_eq!(f64_to_pcm_u8(2.0), u8::MAX);
        assert_eq!(f64_to_pcm_i16(-2.0), i16::MIN);
        assert_eq!(f64_to_pcm_i16(2.0), i16::MAX);
        assert_eq!(f64_to_pcm_i24(-2.0), I24_MIN);
        assert_eq!(f64_to_pcm_i24(2.0), I24_MAX);
        assert_eq!(f64_to_pcm_i32(-2.0), i32::MIN);
        assert_eq!(f64_to_pcm_i32(2.0), i32::MAX);
    }

    #[test]
    fn converts_nan_to_silence() {
        assert_eq!(f64_to_pcm_u8(f64::NAN), U8_SILENCE);
        assert_eq!(f64_to_pcm_i16(f64::NAN), 0);
        assert_eq!(f64_to_pcm_i24(f64::NAN), 0);
        assert_eq!(f64_to_pcm_i32(f64::NAN), 0);
    }
}
