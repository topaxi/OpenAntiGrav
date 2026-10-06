//! An emitter's clock and its burst-level spawn laws: the playback rate, the
//! lifetime co-factor, the evenly stepped ring and the sub-frame spread, plus the
//! instance frame a local-space emitter's particles ride. Its own file because
//! `psys.rs` is baselined.
//!
//! All of it is read off Pulse's PSP `BOOT.BIN` (`particle-system.md`, "The
//! emitter's clock and the burst laws", 2026-10-04):
//!
//! - **The playback rate** (`res+0x4cc`). `ParticleSystem_DeriveScaledParams`
//!   (`0x088f4910`) stores `res+0x4cc * instance+0x3c` at `+0x70` (the
//!   co-factor is `1.0`, `FUN_088f57d4`). `ParticleSystem_Update` (`0x088f5b9c`)
//!   multiplies the frame's tick count by it into `+0xc` and `DAT_08b6207c`, and
//!   **every** consumer reads those: the duration countdown (`+0x138 -= +0xc`), the
//!   emission countdown (`ParticleSystem_UpdateEmission`, `+0x4 -= +0xc`), the drag
//!   exponent, the integration (`position += velocity * +0xc`), the particle's
//!   age and life, the roll and the atlas frame. So an emitter of rate `4` runs
//!   four of its own ticks per frame.
//! - **Selector 5 is the lifetime co-factor.** `ParticleSystem_Update` stores
//!   selector `5` into instance `+0x54`, and `ParticleSystem_InitParticle`
//!   (`0x088f6e6c`) sets a particle's life to `RandSpread(+0x5c, +0x60) * +0x54 / 60`
//!   seconds. Two sites, one field: only particles born after the store live
//!   longer.
//! - **Flag `0x200000` steps a ring evenly.** `ParticleSystem_EmitRing`
//!   (`0x088fc634`) draws one `phi0 = U(0, 2 pi)` per burst and gives particle `i`
//!   (from `0`) the angle `phi0 + (i + 1) * 2 pi / count`, `count` being the
//!   whole burst `ParticleSystem_UpdateEmission` hands it in one call.
//! - **Flag `0x20` spreads a burst over the frame's motion.**
//!   `ParticleSystem_CacheModeFlags` (`0x088f4dcc`) copies it to `DAT_08b620a0`;
//!   `ParticleSystem_Update` stores `+0xc0 = previous - current` translation
//!   (`vsub.q C320, C300(+0xd0), C310(+0x120)` at `0x088f60c0`), and every emit
//!   function adds `+0xc0 * i / count` to particle `i`'s position: the first at
//!   the emitter, the rest pulled back along the path it just covered.
//! - **Flag `0x2` keeps a particle in the emitter's frame.** `InitParticle` skips
//!   the instance matrix under it, and `ParticleSystem_DrawEmitterPool`
//!   (`0x08918bf8`) draws the pool through `view * instance+0xf0` under it, so the
//!   particles ride whatever moves that matrix. The matrix moves when the owner
//!   spawned the node with its matrix **by pointer** (`Psys_Spawn_q`'s `param_5 & 1`,
//!   `FUN_08916200`), which the node's per-frame update (`FUN_08915cdc`) copies into
//!   `+0xf0` through `FUN_088f4498`. Whether a given owner does that is a property of
//!   its call site, so a caller opts in with [`System::set_rides_frame`]; only the
//!   Repulser's blast is read to do so.

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_pob::{self as pob, Channel};

use super::sample::channel_sample;
use super::{Playing, Stage, System};

/// The emitter's clock and its burst-level spawn laws - see [this module](self).
#[derive(Debug, Clone, PartialEq)]
pub struct Playback {
    /// `res+0x4cc`: emitter ticks per frame tick. `1.0` where the file authors a
    /// value that is not positive and finite.
    pub rate: f32,
    /// The selector-5 attribute record, over the emitter's run: a newborn's lifetime co-factor.
    pub lifetime_animation: Option<Channel>,
    /// Flag `0x200000`: a ring's angles step evenly from a random start.
    pub even_ring: bool,
    /// The ring's arc: `2 pi`, or `pi` for shape 8's half ring
    /// ([`super::spawn::arc_of`]). The even step is `arc / count` from a start drawn
    /// `U(0, arc)`.
    pub arc: f32,
    /// Flag `0x20`: a burst is spread back along the emitter's motion this frame.
    pub subframe_spread: bool,
}

impl Playback {
    /// What `record` authors.
    #[must_use]
    pub fn of(record: &pob::Emitter) -> Self {
        let rate = record.playback_rate;
        Self {
            rate: if rate.is_finite() && rate > 0.0 {
                rate
            } else {
                1.0
            },
            lifetime_animation: record
                .attribute_animations
                .iter()
                .find(|animation| animation.selector == 5)
                .map(|animation| animation.channel.clone()),
            even_ring: record.flags & pob::flags::UNIFORM_SPHERE != 0,
            arc: super::spawn::arc_of(record.shape),
            subframe_spread: record.flags & pob::flags::SUBFRAME_SPREAD != 0,
        }
    }
}

impl Playback {
    /// What a burst draws once: the even ring's start angle (`ParticleSystem_EmitRing`
    /// draws it before its loop), and the frame's pull-back, zero without the flag.
    /// The start comes back with the arc it steps over.
    pub(super) fn burst_draws(&self, pull_back: Vec3, rng: &mut Rng) -> (Option<(f32, f32)>, Vec3) {
        let start = self
            .even_ring
            .then(|| (rng.next_f32() * self.arc, self.arc));
        let pull_back = if self.subframe_spread {
            pull_back
        } else {
            Vec3::ZERO
        };
        (start, pull_back)
    }
}

impl Default for Playback {
    /// Rate `1`, no attribute record, no burst flag: what every emitter played as
    /// before these were read.
    fn default() -> Self {
        Self {
            rate: 1.0,
            lifetime_animation: None,
            even_ring: false,
            arc: std::f32::consts::TAU,
            subframe_spread: false,
        }
    }
}

/// What one particle of a burst inherits from the burst as a whole.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Burst {
    /// The extent multiplier: the emission-scale channel times the selector-2 record.
    pub extent: f32,
    /// The root's frame scale, see [`System::set_frame_scale`].
    pub frame_scale: f32,
    /// The lifetime co-factor, the selector-5 record (`1.0` without one).
    pub lifetime: f32,
    /// The ring angle under [`Playback::even_ring`], else `None` (drawn per particle).
    pub phi: Option<f32>,
    /// Added to the particle's spawn position: the sub-frame pull-back.
    pub shift: Vec3,
}

impl Burst {
    /// Particle `index` of `count`: its even ring angle off `start` (the start angle and
    /// the arc it steps over), and its share
    /// of `pull_back` (the previous translation minus the current one).
    pub(super) fn particle(
        self,
        (index, count): (usize, usize),
        start: Option<(f32, f32)>,
        pull_back: Vec3,
    ) -> Self {
        Self {
            phi: start.map(|(start, arc)| start + arc / count.max(1) as f32 * (index + 1) as f32),
            shift: pull_back * (index as f32 / count.max(1) as f32),
            ..self
        }
    }
}

/// An instance frame: the anchor and the frame's world `X` and `Y`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Frame {
    pub anchor: Vec3,
    pub across: Vec3,
    pub up: Vec3,
}

impl Frame {
    /// `v`, written in this frame's axes, in world axes.
    fn to_world(self, v: Vec3) -> Vec3 {
        self.across * v.x + self.up * v.y + self.across.cross(self.up) * v.z
    }

    /// `v`, in world axes, written in this frame's axes.
    fn to_local(self, v: Vec3) -> Vec3 {
        Vec3::new(
            v.dot(self.across),
            v.dot(self.up),
            v.dot(self.across.cross(self.up)),
        )
    }
}

impl super::EmitterSpec {
    /// The burst this emitter fires with `ticks_left` of its run to go: the extent
    /// multiplier (the emission-scale channel times the selector-2 record) and the
    /// lifetime co-factor (selector 5), both at the emitter's normalised age.
    pub(super) fn burst(&self, ticks_left: f32, frame_scale: f32) -> Burst {
        let run = self.run_ticks();
        let age = if run.is_finite() && run > 0.0 {
            (1.0 - ticks_left / run).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let animated = |channel: &Option<Channel>| {
            channel
                .as_ref()
                .map_or(1.0, |channel| channel_sample(channel, age, 0.5))
        };
        Burst {
            extent: channel_sample(&self.emission_scale, age, 0.5)
                * animated(&self.extent_animation),
            frame_scale,
            lifetime: animated(&self.playback.lifetime_animation),
            phi: None,
            shift: Vec3::ZERO,
        }
    }
}

impl System {
    /// Whether this instance's local-space emitters (flag `0x2`) ride its frame - see
    /// [this module](self). Off unless a caller turns it on; turning it on starts from
    /// the frame the instance holds now.
    pub fn set_rides_frame(&mut self, rides: bool) {
        self.ridden = rides.then(|| self.frame());
    }

    fn frame(&self) -> Frame {
        Frame {
            anchor: self.anchor,
            across: self.across,
            up: self.up,
        }
    }

    /// Carries every live particle of a local-space root emitter from the frame it
    /// was last carried to into the instance's current one, as drawing it through
    /// the live matrix does. Velocity turns with the frame; drag and gravity act on
    /// it in world axes, which equals the frame's own for the only emitter riding
    /// today (`WO_REPULSER_BLAST`: isotropic drag, no gravity flag).
    pub(super) fn ride(&mut self, effect: &super::Effect) {
        let Some(from) = self.ridden else {
            return;
        };
        let to = self.frame();
        self.ridden = Some(to);
        if from == to {
            return;
        }
        let roots = effect.roots();
        for particle in &mut self.particles {
            let spec = usize::from(particle.spec);
            if !particle.alive() || !effect.emitters[spec].world_space || !roots.contains(&spec) {
                continue;
            }
            let carry = |p: Vec3| to.anchor + to.to_world(from.to_local(p - from.anchor));
            particle.position = carry(particle.position);
            particle.origin = carry(particle.origin);
            particle.velocity = to.to_world(from.to_local(particle.velocity));
        }
    }
}

impl Stage {
    /// Sets whether an instance's local-space particles ride its frame - see
    /// [`System::set_rides_frame`]. A no-op on a stale handle.
    pub fn set_rides_frame(&mut self, playing: Playing, rides: bool) {
        if let Some(instance) = self.get_mut(playing) {
            instance.system.set_rides_frame(rides);
        }
    }

    /// The `+Y` [`Self::orient`] last gave an attached instance, or world up
    /// when none did. `None` on a stale handle.
    #[must_use]
    pub fn up_of(&self, playing: Playing) -> Option<Vec3> {
        self.instances
            .get(usize::from(playing.index))
            .filter(|i| i.generation == playing.generation && i.attached)
            .map(|i| i.up)
    }
}

#[cfg(test)]
mod tests;
