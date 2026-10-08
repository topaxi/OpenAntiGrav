//! Wipeout HD's Missile detonation: `MissileManager`'s pool of sixteen
//! explosion objects (vtable `0x00864b38`), each one `HD_missile_explosion.vex`
//! played on its own keys.
//!
//! # What is read (`weapons.md`, 2026-10-07, `hd-weapon-fx`)
//!
//! - **Who enters the pool.** Only a missile that reaches a craft: the
//!   craft-hit branch of `0x001423a8` takes an entry, a wall never does.
//!   Confidence 70.
//! - **The place.** The struck craft's own position (`m5`: the hit position
//!   the entry stored was the rival's).
//! - **The clock.** One second, the age `p` linear over it (confidence 80),
//!   the entry freed once `p` reaches 1. The model's own keys (`sphere` and
//!   `bloom` scaled 1 to 18 over 60 frames) run on that age, and so do the
//!   `UV_offset` and `Shockwave_scalar` the three materials read.
//! - **The pool.** Sixteen entries, taken while the count is below sixteen.
//!
//! # What is chosen, not measured
//!
//! - **Orientation.** `Start` copies the rows of the matrix its caller passes;
//!   which matrix `0x001423a8` passes was not resolved, so the struck craft's
//!   own orientation stands in. Only the rays and the rings are not
//!   rotationally symmetric.
//! - **The viewport gate.** The original takes an entry per viewport the
//!   point is visible in; this draws for the one camera.
//! - **Not played.** The sixteen randomly rotated entries `Start` builds at
//!   `+0x190` (confidence 45, read by nothing in the class).
//!
//! The point light `(1 - p)^2` is `crate::weapon_light`'s.

use super::*;
use oag_mesh::mesh_render::SpuLight;

/// The original's pool size: `pool_take` enters an object while `count < 16`.
pub(crate) const SLOTS: usize = 16;

/// How long an entry lives, seconds: the age field runs 0 to 1 at 1 per second.
pub(super) const LIFETIME_SECONDS: f32 = 1.0;

/// One live explosion. **View state**, on the same terms as every blast here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MissileBlast {
    pub(super) position: Vec3,
    /// The place's frame; see the module doc (chosen).
    pub(super) orientation: Quat,
    pub(super) age: f32,
}

impl Race {
    /// Starts an explosion at the struck craft, dropped when the pool is full
    /// (the original's `count < 16` test).
    pub(crate) fn spawn_hd_missile_blast(&mut self, position: Vec3, orientation: Quat) {
        if !self.view.hd_missile_blast {
            return;
        }
        if let Some(slot) = self.view.hd_missile_blasts.iter().position(Option::is_none) {
            log::debug!("HD missile blast starts at {position:?}");
            self.view.hd_missile_blasts[slot] = Some(MissileBlast {
                position,
                orientation,
                age: 0.0,
            });
        }
    }

    /// Ages every live explosion and frees the ones whose age reached a second.
    pub(crate) fn advance_hd_missile_blasts(&mut self, dt: f32) {
        for slot in &mut self.view.hd_missile_blasts {
            let Some(blast) = slot else { continue };
            blast.age += dt;
            if blast.age >= LIFETIME_SECONDS {
                *slot = None;
            }
        }
    }

    /// Each live explosion's point light, in slot order; see
    /// [`crate::weapon_light`].
    pub(crate) fn hd_missile_blast_lights(&self) -> impl Iterator<Item = SpuLight> + '_ {
        self.view
            .hd_missile_blasts
            .iter()
            .flatten()
            .filter_map(|blast| crate::weapon_light::missile_record(blast.position, blast.age))
    }

    /// Each live explosion's placement matrix and age, in slot order.
    #[must_use]
    pub(crate) fn hd_missile_blast_draws(&self) -> [Option<(Mat4, f32)>; SLOTS] {
        self.view.hd_missile_blasts.map(|slot| {
            slot.map(|blast| {
                (
                    Mat4::from_rotation_translation(blast.orientation, blast.position),
                    blast.age,
                )
            })
        })
    }
}

#[cfg(test)]
mod tests;
