use crate::{Delay, WavFile};

use super::EffectError;

/// Delay time and feedback for one channel of a stereo delay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayChannelSettings {
    /// Delay time in milliseconds; fractional values are supported.
    pub delay_milliseconds: f64,
    /// Feedback from `-1.0` to `1.0`.
    pub feedback: f64,
}

impl DelayChannelSettings {
    /// Creates channel settings from a delay time and feedback amount.
    #[must_use]
    pub const fn new(delay_milliseconds: f64, feedback: f64) -> Self {
        Self {
            delay_milliseconds,
            feedback,
        }
    }
}

pub(super) fn apply_delay<ProcessFeedback>(
    wav_file: &mut WavFile,
    settings: DelayChannelSettings,
    mix: f64,
    tail_milliseconds: u32,
    process_feedback: ProcessFeedback,
) -> Result<(), EffectError>
where
    ProcessFeedback: FnMut(usize, f64) -> f64,
{
    let channels = vec![settings; usize::from(wav_file.number_of_channels())];
    apply_delay_channels(
        wav_file,
        &channels,
        mix,
        tail_milliseconds,
        process_feedback,
    )
}

pub(super) fn apply_stereo_delay<ProcessFeedback>(
    wav_file: &mut WavFile,
    left: DelayChannelSettings,
    right: DelayChannelSettings,
    mix: f64,
    tail_milliseconds: u32,
    process_feedback: ProcessFeedback,
) -> Result<(), EffectError>
where
    ProcessFeedback: FnMut(usize, f64) -> f64,
{
    let channels = [left, right];
    apply_delay_channels(
        wav_file,
        &channels,
        mix,
        tail_milliseconds,
        process_feedback,
    )
}

fn apply_delay_channels<ProcessFeedback>(
    wav_file: &mut WavFile,
    channels: &[DelayChannelSettings],
    mix: f64,
    tail_milliseconds: u32,
    mut process_feedback: ProcessFeedback,
) -> Result<(), EffectError>
where
    ProcessFeedback: FnMut(usize, f64) -> f64,
{
    for channel in channels {
        validate_feedback(channel.feedback)?;
    }
    validate_mix(mix)?;

    let input_channels = usize::from(wav_file.number_of_channels());
    let output_channels = channels.len();
    let sample_rate = wav_file.sample_rate();
    let input = wav_file.samples()?.to_vec();
    let input_frames = input.len() / input_channels;
    let tail_frames = milliseconds_to_frames(tail_milliseconds, sample_rate)?;
    let output_frames = input_frames
        .checked_add(tail_frames)
        .ok_or(EffectError::SizeOverflow("delay output frame count"))?;
    let output_samples = output_frames
        .checked_mul(output_channels)
        .ok_or(EffectError::SizeOverflow("delay output sample count"))?;
    let mut output = Vec::with_capacity(output_samples);
    let mut delays = channels
        .iter()
        .map(|channel| {
            Delay::new(
                channel.delay_milliseconds,
                channel.delay_milliseconds,
                sample_rate,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    for frame_index in 0..output_frames {
        for (channel_index, (delay, channel)) in delays.iter_mut().zip(channels).enumerate() {
            let input_sample = if frame_index < input_frames {
                let input_channel = if input_channels == 1 {
                    0
                } else {
                    channel_index
                };
                input[frame_index * input_channels + input_channel]
            } else {
                0.0
            };
            output.push(delay.next_sample_with(
                input_sample,
                channel.feedback,
                mix,
                |feedback_sample| process_feedback(channel_index, feedback_sample),
            ));
        }
    }

    if input_channels == output_channels {
        wav_file.set_samples(output)?;
    } else {
        wav_file.set_channels_and_samples(
            u16::try_from(output_channels)
                .map_err(|_| EffectError::SizeOverflow("delay output channel count"))?,
            output,
        )?;
    }
    Ok(())
}

pub(super) fn validate_cutoff(cutoff_hz: f64, sample_rate: u32) -> Result<(), EffectError> {
    let nyquist_hz = f64::from(sample_rate) / 2.0;
    if !cutoff_hz.is_finite() || cutoff_hz <= 0.0 || cutoff_hz >= nyquist_hz {
        return Err(EffectError::InvalidCutoffFrequency {
            cutoff_hz,
            sample_rate,
        });
    }
    Ok(())
}

pub(super) fn validate_drive(drive_db: f64) -> Result<(), EffectError> {
    if !drive_db.is_finite() {
        return Err(EffectError::InvalidDrive(drive_db));
    }
    Ok(())
}

fn validate_feedback(feedback: f64) -> Result<(), EffectError> {
    if !feedback.is_finite() || !(-1.0..=1.0).contains(&feedback) {
        return Err(EffectError::InvalidFeedback(feedback));
    }
    Ok(())
}

pub(super) fn validate_mix(mix: f64) -> Result<(), EffectError> {
    if !mix.is_finite() || !(0.0..=1.0).contains(&mix) {
        return Err(EffectError::InvalidMix(mix));
    }
    Ok(())
}

pub(super) fn milliseconds_to_frames(
    milliseconds: u32,
    sample_rate: u32,
) -> Result<usize, EffectError> {
    let frames = u64::from(milliseconds) * u64::from(sample_rate) / 1_000;
    usize::try_from(frames).map_err(|_| EffectError::SizeOverflow("delay tail"))
}
