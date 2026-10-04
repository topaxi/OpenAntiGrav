//! Two small samplers every emitter step shares. Split out of `psys.rs`, which is baselined.

use oag_core::Rng;

use oag_vex::pob;

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

/// An emitter record's particle lifetime, centre and spread in ticks.
///
/// Under [`pob::flags::IMMORTAL`] (`0x800`) `ParticleSystem_InitParticle`
/// (`0x088f6e6c`) stores `FLT_MAX` instead of the authored value, so the
/// particle never expires and its normalized age stays at `0`. Read live on
/// Outpost 7's `WO_SNOW` (flags `0x5001805`, authored `5` ticks): its 64
/// flakes carry `FLT_MAX` at particle `+0x88`/`+0x8c` and the pool holds all
/// 64 for the whole race. See `docs/ghidra/functions/psp-pulse-usa/weather.md`.
pub(super) fn lifetime_ticks(record: &pob::Emitter) -> (f32, f32) {
    if record.flags & pob::flags::IMMORTAL != 0 {
        return (f32::MAX, 0.0);
    }
    (
        record.lifetime_ticks.0 as f32,
        record.lifetime_ticks.1 as f32,
    )
}
