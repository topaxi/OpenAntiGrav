//! The sprite templates an emitter starts with, played as one-shot emitters.
//!
//! `oag_vex::pob::Emitter::initial_particles` reads what
//! `FUN_088f58a4` (the emitter instance's initialiser) makes a particle from
//! the moment an instance exists: a white `shazam` flash on the collision
//! sparks, a `glow` on nearly every effect, the Quake's 100-unit `shazzam`,
//! the Plasma's `PLASMA_GLOW`. The record is an emitter reduced to its
//! particle: a size channel, an alpha channel, a colour table walked over the
//! particle's life, a lifetime, a blend and a draw class. So it is translated
//! by the same [`EmitterSpec::from_record`] every emitter is, into a spec that
//! emits once, at the first update, with no speed - and started with the root
//! that owns it.
//!
//! **A template on an emitter that is itself a child is not played**: it would
//! have to start with that child's own instances, and no consumer here carries
//! that. [`Effect::skipped_templates`] counts them.
//!
//! **Pulse on the PSP only**, the layout being read off that executable alone:
//! every other source calls [`Effect::without_pulse_psp_draw`], which mutes the
//! templates it added.
//!
//! The sprite is the parent emitter's own: both records point at the pool the
//! emitters do. Severity multiplies a template's size as it does any
//! particle's - the live capture's `shazam` reads `3.9 * 2.4 = 9.36`.
//!
//! **A looping size channel is unrolled.** A record's channel authors a
//! `period` in ticks (the `glow`'s is ten, over a forty-tick life): the live
//! particle's size peaked twice in sixteen frames, `v` at `fract(age / 10)`.
//! [`unroll`] repeats the keys over the normalised life, which is the same
//! curve without giving the pool a second notion of age.

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_vex::pob::{self, Channel, ChannelMode};

use super::{ColourScale, Effect, EmitterSpec, Particle, channel_sample, quad};
use crate::mesh::GpuVertex;

/// `ParticleSystem_InitParticleFields`' flag `0x10`: start at a random roll.
const RANDOM_START: u32 = 0x10;
/// Flag `0x08`: a coin at spawn picks which way the roll turns.
const RANDOM_SENSE: u32 = 0x08;
/// Flag `0x20`: the roll channel is the angle itself, not a rate.
const ABSOLUTE: u32 = 0x20;

/// What a template's draw class 3 adds to a plain billboard: the quad is
/// `ParticleSystem_DrawRotatedSprite`'s, `aspect * size` wide and `size` tall,
/// turned by the particle's roll. Both come off channels the parse had
/// left unread (`+0xf0` and `+0x390`), and the law is
/// `FUN_088f7e64`'s, the per-tick field update:
///
/// - **stretch** `v` at the particle's age: `v > 0` is aspect `1 + v`; `v <= 0`
///   grows the size by `1 - v` and sets aspect `1 / (1 - v)`.
/// - **roll**: under `0x20` the channel's value is the angle that tick; else it
///   is a rate and the angle accumulates, turning the other way for a particle
///   the `0x08` coin flipped.
///
/// Measured on a struck craft: `shazam` aspect `1.5` and roll `0`, `glow`
/// aspect `1.7` and roll `2 pi` falling by `~0.7` per tick - exactly the
/// `0.5`/`0.7` stretch constants and the glow's keyed `0..2 pi` roll channel.
#[derive(Debug, Clone, PartialEq)]
pub struct Rotation {
    stretch: Channel,
    roll: Channel,
    flags: u32,
}

impl Rotation {
    /// The rotation `record` authors, if it is a template with a stretch
    /// block.
    pub(super) fn of(record: &pob::Emitter) -> Option<Self> {
        Some(Self {
            stretch: record.stretch.clone()?,
            roll: record.rotation_speed.clone(),
            flags: record.flags,
        })
    }

    /// A new particle's roll and turning sense, drawing from `rng` only for
    /// the flags that ask for a draw.
    pub(super) fn start(&self, rng: &mut Rng) -> (f32, f32) {
        let roll = if self.flags & RANDOM_START != 0 {
            rng.next_f32() * std::f32::consts::TAU
        } else {
            0.0
        };
        let turn = if self.flags & RANDOM_SENSE != 0 && rng.below(2) == 1 {
            -1.0
        } else {
            1.0
        };
        (roll, turn)
    }

    /// `particle`'s roll after one update at normalised `age`.
    pub(super) fn roll_at(&self, particle: &Particle, age: f32, dt_ticks: f32) -> f32 {
        let v = channel_sample(&self.roll, age, particle.size_sample);
        if self.flags & ABSOLUTE != 0 {
            v * dt_ticks
        } else {
            particle.roll + particle.turn * v * dt_ticks
        }
    }

    /// The sprite's six vertices: `half` is the size channel's value before
    /// the stretch takes its share.
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
        let v = channel_sample(&self.stretch, age, particle.size_sample);
        let (half, aspect) = if v > 0.0 {
            (half, 1.0 + v)
        } else {
            (half * (1.0 - v), 1.0 / (1.0 - v))
        };
        let (sin, cos) = particle.roll.sin_cos();
        let across = (right * cos - up * sin) * (half * aspect);
        let down = (right * sin + up * cos) * half;
        quad(particle.position, across, down, 0.5, rgb, alpha)
    }
}

impl Effect {
    /// Appends a one-shot spec for every template on a root emitter, and
    /// starts each with the effect.
    pub(super) fn add_templates(&mut self, records: &[pob::Emitter], scale: ColourScale) {
        let parents: Vec<usize> = self.roots.clone();
        for parent in parents {
            for template in &records[parent].initial_particles {
                if self.emitters.len() >= super::MAX_EMITTER_STATES {
                    return;
                }
                let sprite = self.emitters[parent].sprite.clone();
                let Ok(mut spec) = EmitterSpec::from_record(template, scale, sprite) else {
                    continue;
                };
                let life = spec.lifetime_ticks.0.max(1.0);
                spec.template = true;
                spec.rotation = Rotation::of(template);
                spec.size = unroll(&spec.size, life);
                spec.alpha = unroll(&spec.alpha, life);
                spec.atlas = super::Atlas::SINGLE;
                self.roots.push(self.emitters.len());
                self.emitters.push(spec);
            }
        }
        self.skipped_templates = records
            .iter()
            .enumerate()
            .filter(|(index, _)| !self.roots.contains(index))
            .map(|(_, record)| record.initial_particles.len())
            .sum();
    }

    /// Templates on child emitters, which are not played - see
    /// [this module](self).
    #[must_use]
    pub fn skipped_templates(&self) -> usize {
        self.skipped_templates
    }

    /// Mutes the templates, with everything else
    /// [`Self::without_pulse_psp_draw`] takes back.
    pub(super) fn mute_templates(&mut self) {
        for spec in self.emitters.iter_mut().filter(|spec| spec.template) {
            spec.per_emission = (0, 0);
        }
    }
}

/// `channel`, repeated over a particle life of `life_ticks`, when it authors a
/// period; `channel` unchanged otherwise.
pub(super) fn unroll(channel: &Channel, life_ticks: f32) -> Channel {
    let period = channel.period;
    if period <= 0.0 || channel.mode != ChannelMode::Keyframed || channel.keys.len() < 2 {
        return channel.clone();
    }
    let cycles = life_ticks / period;
    let mut keys: Vec<(f32, f32)> = Vec::new();
    let mut cycle = 0.0_f32;
    'cycles: while cycle < cycles {
        for &(t, v) in &channel.keys {
            let at = (cycle + t) / cycles;
            if at >= 1.0 {
                let end = (cycles - cycle) / 1.0;
                keys.push((1.0, channel.value_at(end.min(1.0))));
                break 'cycles;
            }
            match keys.last() {
                Some(&(last, _)) if at <= last + 1.0e-6 => {}
                _ => keys.push((at, v)),
            }
        }
        cycle += 1.0;
    }
    if keys.last().is_none_or(|&(t, _)| t < 1.0) {
        let last = keys.last().map_or(0.0, |&(_, v)| v);
        keys.push((1.0, last));
    }
    Channel {
        period: 0.0,
        keys,
        ..channel.clone()
    }
}

#[cfg(test)]
mod tests;
