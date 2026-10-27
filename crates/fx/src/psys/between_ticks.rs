//! Particles drawn between two simulation ticks, on the GPU.
//!
//! A pool advances once a tick. Each particle keeps where the last advance
//! found it ([`Particle::before`]), and every vertex of its quad carries the
//! tick's move, `position - before`, in two attribute slots a particle has no
//! other use for (`lightmap_texcoord` and `sun_mask`, see [`set_shift`]). The
//! vertex shader draws at `position + shift * (draw_alpha - 1)`, the draw
//! alpha in the uniform block's unused model matrix, so the CPU writes the
//! same number of vertices a frame as before and moves nothing per frame. At
//! `draw_alpha == 1` (captures, a paused race) the term is `shift * 0.0` and
//! every vertex lands where it did before.
//!
//! - **A particle born this tick** has no previous position: its spawn sets
//!   `before` to where it appears, so it draws at its spawn point at every
//!   alpha. Spawns stay discrete, as in the originals.
//! - **A particle that died this tick** is already out of the pool and is not
//!   drawn between the ticks either: a discrete end, like its birth.
//! - **A pool that did not advance this tick** (the flyby, the cosmetic ticks
//!   after the finish, a pool nobody stepped) draws with no shift:
//!   [`System::hold_still`] runs at the start of every tick path.
//! - **A move longer than [`SNAP_DISTANCE`]** in one tick draws where it lands,
//!   the backstop `oag_raceplay::between_ticks` uses for poses.
//! - **A weather field** ([`super::field`]) holds field coordinates and is moved
//!   into the world with the drawn camera, so [`System::in_world`] gives it no
//!   shift; its particles stand still in the world between ticks as they did.
//!
//! Size, colour, sprite frame and roll still step per tick; only the position
//! is blended.

use oag_core::math::Vec3;
use oag_mesh::mesh::GpuVertex;

use super::{Particle, Stage, System};

/// The furthest a particle may move in one tick and still be swept, in world
/// units: `oag_raceplay::between_ticks::SNAP_DISTANCE`'s value, for the same
/// reason. Chosen, not measured.
pub const SNAP_DISTANCE: f32 = 64.0;

impl Particle {
    /// How far this particle moved in the last tick its pool advanced, or zero
    /// across a snap.
    pub(super) fn shift(&self) -> Vec3 {
        let shift = self.position - self.before;
        if shift.is_finite() && shift.length_squared() <= SNAP_DISTANCE * SNAP_DISTANCE {
            shift
        } else {
            Vec3::ZERO
        }
    }
}

/// Writes a particle vertex's tick move into the slots `psys.wesl` reads it
/// from: `lightmap_texcoord` (x, y) and `sun_mask` (z). The particle pipeline
/// binds neither for its own meaning: particles carry no lightmap and are not
/// masked from the sun.
pub(super) fn set_shift(vertex: &mut GpuVertex, shift: Vec3) {
    vertex.lightmap_texcoord = [shift.x, shift.y];
    vertex.sun_mask = shift.z;
}

/// The move [`set_shift`] wrote, for tests.
#[cfg(test)]
pub(super) fn shift_of(vertex: &GpuVertex) -> Vec3 {
    Vec3::new(
        vertex.lightmap_texcoord[0],
        vertex.lightmap_texcoord[1],
        vertex.sun_mask,
    )
}

/// Where `vertex` draws at `draw_alpha`, as `psys.wesl` places it.
#[cfg(test)]
pub(super) fn drawn_at(vertex: &GpuVertex, draw_alpha: f32) -> Vec3 {
    Vec3::from(vertex.position) + shift_of(vertex) * (draw_alpha - 1.0)
}

impl System {
    /// Forgets the last tick's motion: every particle draws where it is at
    /// every alpha until the pool advances again.
    ///
    /// [`System::advance`] runs it first, so `before` is the position the tick
    /// started from; the race runs it on every pool at the start of every tick
    /// path, so a pool that tick path does not advance has no stale move.
    ///
    /// Live particles only: a spawn sets its own `before`.
    pub fn hold_still(&mut self) {
        for particle in self.particles.iter_mut().filter(|p| p.alive()) {
            particle.before = particle.position;
        }
    }
}

impl Stage {
    /// [`System::hold_still`] on every instance that holds anything; an idle
    /// one has no live particle to hold.
    pub fn hold_still(&mut self) {
        for instance in self.instances.iter_mut().filter(|i| i.busy()) {
            instance.system.hold_still();
        }
    }
}

#[cfg(test)]
mod tests;
