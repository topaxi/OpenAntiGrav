//! The turned, stretched quad a class 3 particle is drawn as.
//!
//! Two laws share one type because they share one picture - a camera-facing
//! quad of half-height `size`, half-width `aspect * size`, turned by a roll -
//! and differ in where each number comes from and in how the quad is built.
//! Both are read off Pulse's PSP executable alone, so every other source
//! calls [`Effect::without_pulse_psp_draw`](super::Effect::without_pulse_psp_draw),
//! which takes the rotation back.
//!
//! **A sprite template** (`FUN_088f58a4`'s particles) is sampled by
//! `ParticleSystem_UpdateParticleFields` (`0x088f7e64`) and drawn by
//! `ParticleSystem_DrawParticle` into `ParticleSystem_DrawRotatedSprite`:
//!
//! - **stretch** `v` off the record's `+0xf0` block at the particle's age:
//!   `v > 0` is aspect `1 + v`; `v <= 0` grows the size by `1 - v` and sets
//!   aspect `1 / (1 - v)`.
//! - **roll**: under flag `0x20` the channel's value is the angle that tick;
//!   else it is a rate and the angle accumulates, turning the other way for a
//!   particle the `0x08` coin flipped. Flag `0x10` starts at `U(0, 2 pi)`.
//! - the quad is a true rectangle: `aspect * size` along the turned x axis.
//!
//! **An emitter's own particle** is integrated by
//! `ParticleSystem_UpdateParticles` (`0x088f635c`) and drawn, all of an
//! instance's at once, by `ParticleSystem_DrawRolledQuads` (`0x089178c0`),
//! a different routine with a different law:
//!
//! - **aspect** is the emitter record's constant `+0x4c8`
//!   ([`pob::Emitter::aspect`]); nothing stretches it over the life.
//! - **roll** is always a rate. `ParticleSystem_InitParticle` starts it at
//!   `U(-pi, pi)` under flag `0x4`, else `0`, and stores the `+0x698`
//!   channel's value in the particle's rate word - drawn **once**, at spawn,
//!   for a random channel - with a keyframed channel leaving it zero. Flag
//!   `0x8` flips a coin per particle. Per tick the angle then gains
//!   `rate * dt` (constant and random channels, negated on a flipped
//!   particle) or the keyframed channel's own value at the age the tick
//!   *began* with, **added** on a flipped particle and **subtracted** on an
//!   unflipped one. There is no absolute mode.
//! - the quad is the turned *unit* square with the aspect applied to the
//!   screen's x afterwards: corners `(+-w, +-h)` times the rotated
//!   `(+-1, +-1)`, so its edges are `(w cos, -h sin)` and `(w sin, h cos)`.
//!   A parallelogram once the aspect is not `1` and the roll is not a
//!   quarter turn, and the template's rectangle otherwise. The corpus authors
//!   a non-unit aspect on two emitters only, both the Shuriken's, both
//!   without a roll.
//!
//! The roll's sense - which way a positive angle turns the quad - is the
//! template's, `(cos, -sin)` for the turned x axis, read in both routines'
//! instructions.

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_vex::pob::{self, Channel, ChannelMode, flags};

use super::{Effect, Particle, channel_sample, quad};
use crate::mesh::GpuVertex;

/// A template's flag `0x10`: start at a random roll.
const RANDOM_START: u32 = 0x10;
/// A template's flag `0x08`: a coin picks which way the roll turns.
const RANDOM_SENSE: u32 = 0x08;
/// A template's flag `0x20`: the roll channel is the angle itself, not a rate.
const ABSOLUTE: u32 = 0x20;

/// Where a class 3 quad's aspect and roll come from.
#[derive(Debug, Clone, PartialEq)]
enum Law {
    /// A sprite template: the `+0xf0` stretch channel.
    Template { stretch: Channel },
    /// An emitter's particle: the constant `+0x4c8`.
    Emitter { aspect: f32 },
}

/// A class 3 particle's rotating, stretched quad.
#[derive(Debug, Clone, PartialEq)]
pub struct Rotation {
    law: Law,
    roll: Channel,
    flags: u32,
}

impl Rotation {
    /// The rotation `record` authors, if it is a template with a stretch
    /// block.
    pub(super) fn of_template(record: &pob::Emitter) -> Option<Self> {
        Some(Self {
            law: Law::Template {
                stretch: record.stretch.clone()?,
            },
            roll: record.rotation_speed.clone(),
            flags: record.flags,
        })
    }

    /// The rotation an emitter's own particles take: class 3 only, which is
    /// the one draw class the batched routine rotates.
    pub(super) fn of_emitter(record: &pob::Emitter) -> Option<Self> {
        (record.draw_class() == Some(3)).then(|| Self {
            law: Law::Emitter {
                aspect: if record.aspect.is_finite() {
                    record.aspect
                } else {
                    1.0
                },
            },
            roll: record.rotation_speed.clone(),
            flags: record.flags,
        })
    }

    #[cfg(test)]
    pub(super) fn emitter(aspect: f32, roll: Channel, flags: u32) -> Self {
        Self {
            law: Law::Emitter { aspect },
            roll,
            flags,
        }
    }

    fn is_template(&self) -> bool {
        matches!(self.law, Law::Template { .. })
    }

    /// Sets a new particle's roll, its turning sense (the factor `1` or `-1`
    /// on the roll channel's value) and the once-drawn sample of a random
    /// roll channel, drawing from `rng` only for what the flags and the
    /// channel ask for a draw.
    pub(super) fn start(&self, particle: &mut Particle, rng: &mut Rng) {
        let (random, sense) = if self.is_template() {
            (RANDOM_START, RANDOM_SENSE)
        } else {
            (flags::RANDOM_ROLL, flags::RANDOM_ROTATION_SIGN)
        };
        let roll = if self.flags & random == 0 {
            0.0
        } else if self.is_template() {
            rng.next_f32() * std::f32::consts::TAU
        } else {
            (rng.next_f32() * 2.0 - 1.0) * std::f32::consts::PI
        };
        let flipped = self.flags & sense != 0 && rng.below(2) == 1;
        // A keyframed emitter channel is subtracted on the particle the coin
        // left alone and added on the one it flipped: the other way round
        // from every other channel.
        let keyed_emitter = !self.is_template() && self.roll.mode == ChannelMode::Keyframed;
        particle.roll = roll;
        particle.turn = if flipped != keyed_emitter { -1.0 } else { 1.0 };
        if !self.is_template() && self.roll.mode == ChannelMode::Random {
            particle.spin_sample = rng.next_f32();
        }
    }

    /// `particle`'s roll after one update, whose age went from `before` to
    /// `after` (normalised).
    pub(super) fn advance(
        &self,
        particle: &Particle,
        before: f32,
        after: f32,
        dt_ticks: f32,
    ) -> f32 {
        if self.is_template() {
            return self.roll_at(particle, after, dt_ticks);
        }
        let v = channel_sample(&self.roll, before, particle.spin_sample);
        particle.roll + particle.turn * v * dt_ticks
    }

    /// A template's roll after one update at normalised `age`.
    pub(super) fn roll_at(&self, particle: &Particle, age: f32, dt_ticks: f32) -> f32 {
        let v = channel_sample(&self.roll, age, particle.size_sample);
        if self.flags & ABSOLUTE != 0 {
            v * dt_ticks
        } else {
            particle.roll + particle.turn * v * dt_ticks
        }
    }

    /// The sprite's six vertices: `half` is the size channel's value before
    /// a template's stretch takes its share.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn quad(
        &self,
        particle: &Particle,
        age: f32,
        half: f32,
        right: Vec3,
        up: Vec3,
        rgb: [f32; 3],
        alpha: f32,
    ) -> [GpuVertex; 6] {
        let (sin, cos) = particle.roll.sin_cos();
        let (across, down) = match &self.law {
            Law::Template { stretch } => {
                let v = channel_sample(stretch, age, particle.size_sample);
                let (half, aspect) = if v > 0.0 {
                    (half, 1.0 + v)
                } else {
                    (half * (1.0 - v), 1.0 / (1.0 - v))
                };
                (
                    (right * cos - up * sin) * (half * aspect),
                    (right * sin + up * cos) * half,
                )
            }
            Law::Emitter { aspect } => {
                let width = half * aspect;
                (
                    right * (width * cos) - up * (half * sin),
                    right * (width * sin) + up * (half * cos),
                )
            }
        };
        quad(particle.position, across, down, 0.5, rgb, alpha)
    }
}

impl Effect {
    /// Takes the rotation back from every emitter and template, with
    /// everything else [`Self::without_pulse_psp_draw`] takes back.
    pub(super) fn mute_rotation(&mut self) {
        for spec in &mut self.emitters {
            spec.rotation = None;
        }
    }
}

#[cfg(test)]
mod tests;
