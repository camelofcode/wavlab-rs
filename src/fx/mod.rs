//! Audio effects applied to loaded WAV samples.

mod analog_delay;
mod delay;
mod digital_delay;
mod error;
mod tape_delay;
mod tape_saturation;

pub use delay::DelayChannelSettings;
pub use error::EffectError;

/// Default low-pass cutoff used by analog and tape delays, in Hz.
pub const DEFAULT_DELAY_CUTOFF_HZ: f64 = 2_000.0;
