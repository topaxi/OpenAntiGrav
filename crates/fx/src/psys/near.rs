//! The near-camera law of HD's, Omega's and 2048's billboard builder: by its depth in the view a
//! particle is dropped, faded or shrunk.
//!
//! - nearer than [`NearLaw::cull_depth`]: no quad;
//! - nearer than [`NearLaw::fade_depth`]: alpha times
//!   `(depth - cull_depth) / (fade_depth - cull_depth)`;
//! - at any depth: the half size is at most `depth * cap`, the emitter's own
//!   [`EmitterSpec::near_cap`], or for a sprite template (which has no cap field)
//!   [`NearLaw::template_cap`].
//!
//! Billboards only (render modes 1 to 4), the heat-haze quads included: the streak classes have
//! their own depth rule and read no cap. Read on HD's SPU job, Omega's CPU twin and 2048's two
//! quad builders (`docs/ghidra/functions/ps3-hdfury-eu/particle-render.md`). Which titles run it
//! is `oag_title::Effects::near_law`; an effect without it ([`Effect::near`] `None`) draws as
//! authored.

use oag_core::math::Vec3;

use super::guard::GuardBand;
use super::{Effect, EmitterSpec, Particle};

/// The two depths of the law, in world units along the view direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearLaw {
    /// Nearer than this a billboard draws nothing.
    pub cull_depth: f32,
    /// Nearer than this its alpha is faded.
    pub fade_depth: f32,
    /// The cap of a sprite template's quad, `None` for uncapped.
    pub template_cap: Option<f32>,
}

/// The eye a frame's quads are built for: what the law reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearEye {
    /// The camera's position.
    pub eye: Vec3,
    /// The unit direction it looks along.
    pub forward: Vec3,
    /// How far between two ticks the frame is drawn: `psys.wesl` draws a particle at
    /// `position + shift * (draw_alpha - 1)`, and its depth is taken there.
    pub draw_alpha: f32,
}

/// What the view of a frame knows that can drop or reshape a particle's quad. The default is
/// neither.
#[derive(Debug, Clone, Copy, Default)]
pub struct Cull<'a> {
    /// The camera, for the Pulse PSP strips that are built screen-parallel
    /// ([`super::streak`]).
    pub guard: Option<&'a GuardBand>,
    /// The eye the near laws read, for the effects that carry one ([`Effect::near`],
    /// [`super::guard`]).
    pub near: Option<NearEye>,
}

impl Cull<'_> {
    /// No cull of either kind.
    pub const NONE: Self = Self {
        guard: None,
        near: None,
    };
}

impl NearEye {
    pub(super) fn depth(&self, particle: &Particle) -> f32 {
        let drawn = particle.position + particle.shift() * (self.draw_alpha - 1.0);
        (drawn - self.eye).dot(self.forward)
    }
}

impl Effect {
    /// Runs this effect's billboards under `law`.
    pub fn with_near_law(&mut self, law: NearLaw) {
        self.near = Some(law);
    }
}

/// A billboard's half size and alpha under the law, or `None` when it draws no quad. Both come
/// back unchanged when the effect carries no law or the build knows no eye.
pub(super) fn apply(
    effect: &Effect,
    spec: &EmitterSpec,
    particle: &Particle,
    eye: Option<NearEye>,
    half: f32,
    alpha: f32,
) -> Option<(f32, f32)> {
    let (Some(law), Some(eye)) = (effect.near, eye) else {
        return Some((half, alpha));
    };
    let depth = eye.depth(particle);
    if depth < law.cull_depth {
        return None;
    }
    let alpha = if depth < law.fade_depth {
        alpha * (depth - law.cull_depth) / (law.fade_depth - law.cull_depth)
    } else {
        alpha
    };
    let cap = if spec.template {
        law.template_cap
    } else {
        spec.near_cap
    };
    let half = cap.map_or(half, |cap| half.min(depth * cap));
    Some((half, alpha))
}

#[cfg(test)]
mod tests;
