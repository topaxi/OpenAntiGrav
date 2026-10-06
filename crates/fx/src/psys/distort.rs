//! The geometry of blend class 8, [`Blend::Distort`]: the quads the heat-haze
//! program draws into the offset target.
//!
//! The program is `psys_normal_heathaze` of Wipeout: Omega Collection's
//! executable, read in `docs/ghidra/functions/ps4-omega-eu/heat-haze.md`. What
//! this module owes it:
//!
//! - the vertex colour is `(kColourScale, kColourScale, kColourScale, alpha)`,
//!   where `kColourScale` is the emitter's own authored float
//!   ([`EmitterSpec::distort_strength`]) and the **palette colour is not used
//!   at all**;
//! - the fragment's displacement is the sprite's `rg - 0.5`, so an emitter
//!   whose sprite is not on the sheet draws **nothing**: a procedural stand-in
//!   would invent a displacement the asset does not author.
//!
//! The shader (`distort.wesl`) holds the rest: the `2 / |w|^0.75` fade and the
//! product. The depth gate (`scene depth >= particle w`) is the hardware depth
//! test of [`super::pipeline::Pipeline::encode_distort`].

use oag_core::math::Vec3;
use oag_mesh::mesh::GpuVertex;

use super::{Blend, Effect, Render, Stage, System, billboard, channel_sample};

impl System {
    /// This frame's [`Blend::Distort`] quads, appended to `out`.
    pub fn extend_distort_vertices(
        &self,
        out: &mut Vec<GpuVertex>,
        effect: &Effect,
        right: Vec3,
        up: Vec3,
    ) {
        for particle in &self.particles {
            if !particle.alive() {
                continue;
            }
            let spec = &effect.emitters[usize::from(particle.spec)];
            if spec.blend != Blend::Distort
                || spec.sheet_rect.is_none()
                || !matches!(spec.render, Render::Billboard)
            {
                continue;
            }
            let age = 1.0 - (particle.life / particle.max_life).clamp(0.0, 1.0);
            let half = channel_sample(&spec.size, age, particle.size_sample) * particle.scale;
            let alpha =
                channel_sample(&spec.alpha, age, particle.alpha_sample) / spec.colour_divisor;
            let k = spec.distort_strength;
            out.extend_from_slice(&billboard(
                spec,
                particle,
                age,
                half,
                right,
                up,
                [k, k, k],
                alpha,
            ));
        }
    }
}

impl Stage {
    /// Every playing instance's [`Blend::Distort`] quads, appended to `out`.
    pub fn extend_distort_vertices(&self, out: &mut Vec<GpuVertex>, right: Vec3, up: Vec3) {
        for instance in &self.instances {
            let Some(effect) = instance.effect.as_deref() else {
                continue;
            };
            instance
                .system
                .extend_distort_vertices(out, effect, right, up);
        }
    }
}

#[cfg(test)]
mod tests;
