//! Two small samplers every emitter step shares. Split out of `psys.rs`, which is baselined.

use oag_core::Rng;

use super::{Channel, ChannelMode};

/// A channel's value at normalized age `age`, with `sample` standing in for
/// the uniform draw a [`ChannelMode::Random`] channel makes once at spawn.
pub(super) fn channel_sample(channel: &Channel, age: f32, sample: f32) -> f32 {
    match channel.mode {
        // `Psys_RandFloatRange(lo, hi)` - the keyframes are authored on
        // some of these channels and the interpreter never reads them.
        ChannelMode::Random => channel.lo + (channel.hi - channel.lo) * sample,
        ChannelMode::Constant => channel.hi,
        ChannelMode::Keyframed | ChannelMode::Unknown(_) => channel.scaled_at(age),
    }
}

/// `Psys_RandIntRange(min, max)`, inclusive.
pub(super) fn random_range(rng: &mut Rng, range: (u32, u32)) -> f32 {
    let (min, max) = range;
    if max <= min {
        return min as f32;
    }
    (min + rng.below(max - min + 1)) as f32
}
