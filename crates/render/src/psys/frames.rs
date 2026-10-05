//! The sprite-atlas frame advancing over a particle's life.
//!
//! Read off Pulse's PSP `BOOT.BIN` on 2026-09-24 (`particle-system.md`, "The
//! atlas frame advances"):
//!
//! - `ParticleSystem_CacheModeFlags` (`0x088f4dcc`) turns the advance on for
//!   an emitter when the frame-rate channel's `hi` (`+0x788`, the `+0x778`
//!   block's `+0x10`) is above zero **and** the grid is not a single frame.
//! - `ParticleSystem_UpdateParticles` (`0x088f635c`) keeps a float frame per
//!   particle (`+0x98`), seeded with the spawn frame. Under resource flag
//!   `0x40` it is `age * (frames - 0.02)`; otherwise it grows by the
//!   frame-rate channel's value times the tick's `dt` in ticks. Once it
//!   reaches `frames - 0.01` that is subtracted once, and the drawn frame is
//!   its floor.
//! - The channel's value is the load-time-baked keyframe track
//!   (`FUN_088f9024`, which merges the alpha, size, roll and frame-rate
//!   channels onto one timeline). A random channel bakes to zero, and nothing
//!   at spawn samples it; the baker gives up on every channel of an emitter
//!   when any keyframed one has a period, leaving the rates zero.
//!
//! **Pulse on the PSP only** - see [`super::Effect::without_pulse_psp_draw`].

use oag_pob::{self as pob, Channel, ChannelMode};

use super::Particle;

/// How a particle's atlas frame moves after spawn.
#[derive(Debug, Clone, PartialEq)]
pub enum FrameAdvance {
    /// It keeps its spawn frame.
    Still,
    /// Resource flag `0x40`: spread over the particle's life.
    OverLife,
    /// Frames per tick, off the `+0x778` channel.
    Rate(Channel),
}

/// Resource flag `0x40`: walk the atlas once over the particle's life.
const OVER_LIFE: u32 = 0x40;

impl FrameAdvance {
    /// What `record`, whose grid holds `frames` frames, authors.
    #[must_use]
    pub fn of(record: &pob::Emitter, frames: u16) -> Self {
        if record.frame_rate.hi <= 0.0 || frames == 1 {
            return Self::Still;
        }
        if record.flags & OVER_LIFE != 0 {
            return Self::OverLife;
        }
        let looping =
            |channel: &Channel| channel.mode == ChannelMode::Keyframed && channel.period > 0.0;
        let channels = [
            &record.alpha,
            &record.size,
            &record.rotation_speed,
            &record.frame_rate,
        ];
        if channels.into_iter().any(looping) {
            return Self::Still;
        }
        Self::Rate(record.frame_rate.clone())
    }

    /// Advances `particle`'s frame by one step of `dt_ticks`, from normalized
    /// age `before` to `after`, in a grid of `frames`.
    pub(super) fn step(
        &self,
        particle: &mut Particle,
        before: f32,
        after: f32,
        dt_ticks: f32,
        frames: u16,
    ) {
        let top = f32::from(frames) - 0.01;
        let mut at = match self {
            Self::Still => return,
            Self::OverLife => after * (top - 0.01),
            Self::Rate(channel) => particle.frame_at + rate(channel, before) * dt_ticks,
        };
        if at >= top {
            at -= top;
        }
        particle.frame_at = at;
        particle.frame = at.max(0.0).floor() as u16;
    }
}

/// The baked channel's value at `age`: the keyframed track, the constant, or
/// zero for a random one.
fn rate(channel: &Channel, age: f32) -> f32 {
    match channel.mode {
        ChannelMode::Keyframed => channel.scaled_at(age),
        ChannelMode::Constant => channel.hi,
        ChannelMode::Random | ChannelMode::Unknown(_) => 0.0,
    }
}
