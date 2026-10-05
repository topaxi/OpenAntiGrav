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
//! The sprite is the template record's own, at `+0x890`
//! (`FUN_08928b10` binds it for a template particle): on the collision sparks it is
//! the pool the parent shares, on the explosion's `Glow` a 32x32 radial glow
//! where its parent `SHIP_DEBRIS` is a 128x64 debris atlas. Severity multiplies a template's size as it does any
//! particle's - the live capture's `shazam` reads `3.9 * 2.4 = 9.36`.
//!
//! **A looping size channel is unrolled.** A record's channel authors a
//! `period` in ticks (the `glow`'s is ten, over a forty-tick life): the live
//! particle's size peaked twice in sixteen frames, `v` at `fract(age / 10)`.
//! [`unroll`] repeats the keys over the normalised life, which is the same
//! curve without giving the pool a second notion of age.

use oag_core::math::Vec3;
use oag_vex::pob::{self, Channel, ChannelMode};

use super::roll::Rotation;
use super::{ColourScale, Effect, EmitterSpec, Particle, System};

impl Effect {
    /// Appends a one-shot spec for every template on a root emitter, and
    /// starts each with the effect.
    pub(super) fn add_templates(
        &mut self,
        system: &pob::ParticleSystem,
        data: &[u8],
        records: &[pob::Emitter],
        scale: ColourScale,
        external: &mut dyn FnMut(&str) -> Option<super::Sprite>,
    ) {
        let parents: Vec<usize> = self.roots.clone();
        for parent in parents {
            for template in &records[parent].initial_particles {
                if self.emitters.len() >= super::MAX_EMITTER_STATES {
                    return;
                }
                // The template's own sprite, not its parent's: none is a stand-in.
                let Some(sprite) =
                    super::Sprite::from_template(system, data, template).or_else(|| {
                        let path = system.texture_path(data, template)?;
                        external(path)
                    })
                else {
                    continue;
                };
                let Ok(mut spec) = EmitterSpec::from_record(template, scale, Some(sprite)) else {
                    continue;
                };
                let life = spec.lifetime_ticks.0.max(1.0);
                spec.template = true;
                spec.streak = spec.streak.for_template();
                spec.rotation = Rotation::of_template(template);
                spec.size = unroll(&spec.size, life);
                spec.alpha = unroll(&spec.alpha, life);
                spec.atlas = super::Atlas::SINGLE;
                // `ParticleSystem_Update` hands the template list its own instance's
                // rate-scaled `+0xc`: a template ages at its parent's rate, not its own.
                spec.playback.rate = self.emitters[parent].playback.rate;
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

impl System {
    /// Frees every template particle, at once - `ParticleSystem_StopAndClear`'s
    /// `FUN_088f48c0`, which empties the instance's template list.
    pub(super) fn clear_templates(&mut self, effect: &Effect) {
        for particle in &mut self.particles {
            if particle.alive() && effect.emitters[usize::from(particle.spec)].template {
                *particle = Particle::DEAD;
            }
        }
    }
}

/// A template follows its instance: `ParticleSystem_UpdateParticleFields`
/// copies the owning instance's node position (`instance + 0x120`) into the
/// particle every update, for as long as the handle resolves. So an attached
/// flare's `glow` rides the rocket, where an emitter's own particles stay
/// where they were born.
pub(super) fn ride(spec: &EmitterSpec, particle: &mut Particle, anchor: Vec3) {
    if spec.template {
        particle.position = anchor;
    }
}

/// `channel`, repeated over a particle life of `life_ticks`, when it authors a
/// period; `channel` unchanged otherwise.
///
/// Equal-time keys survive when their values differ: that pair is a jump,
/// which [`Channel::value_at`] plays as one (the left value up to the time,
/// the right one after it).
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
            // Two keys at one time are how the format authors a jump - the
            // welder's `GLOW` flashes four times a cycle that way - so only a
            // key repeating both time and value is redundant. Dropping every
            // equal-time key erased the jumps, and the flashes with them.
            match keys.last() {
                Some(&(last, _)) if at < last - 1.0e-6 => {}
                Some(&(last, lv)) if (at - last).abs() <= 1.0e-6 && lv == v => {}
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
