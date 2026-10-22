//! Audio effects applied to loaded WAV samples.

mod analog_delay;
mod autopan;
mod chorus;
mod delay;
mod digital_delay;
mod error;
mod flanger;
mod phaser;
mod reverb;
mod tape_delay;
mod tape_saturation;
mod tremolo;
mod vibrato;

pub use autopan::AutoPanSettings;
pub use chorus::{
    CHORUS_CENTER_DELAY_MILLISECONDS, ChorusChannelSettings, MAX_CHORUS_DEPTH_MILLISECONDS,
};
pub use delay::DelayChannelSettings;
pub use error::EffectError;
pub use flanger::{
    FlangerChannelSettings, MAX_FLANGER_DELAY_MILLISECONDS, MAX_FLANGER_FEEDBACK,
    MAX_FLANGER_FREQUENCY_HZ, MIN_FLANGER_FREQUENCY_HZ,
};
pub use phaser::{
    DEFAULT_PHASER_STAGES, MAX_PHASER_STAGES, MAX_PHASER_SWEEP_OCTAVES, PhaserChannelSettings,
};
pub use tremolo::TremoloSettings;
pub use vibrato::VibratoChannelSettings;

/// Default low-pass cutoff used by analog and tape delays, in Hz.
pub const DEFAULT_DELAY_CUTOFF_HZ: f64 = 2_000.0;
