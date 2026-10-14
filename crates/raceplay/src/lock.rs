//! The lock-on as the simulation keeps it: a per-slot quantity advanced from
//! world state alone, with no camera and no reticle.
//!
//! The Missile and the LeachBeam fire at the craft their holder has *locked*,
//! and a lock is not a property of geometry alone - it is a reticle held on a
//! target for a time (`docs/ghidra/functions/psp-pulse-usa/lock-sight.md`). The
//! law is [`oag_race::sight`], which is written over a screen position. Until
//! 2026-10-14 the primary's reticle ran that law on the chase camera's
//! projection and the shot read the reticle back, so the simulation read the
//! view: a replay was exact only as far as the reticle it started from, and a
//! server with no camera could not agree with its client about a lock.
//!
//! Now each person-flown slot owns a [`Sight`](oag_race::sight::Sight) in
//! [`RaceSim`], advanced here by the same law off a projection through a
//! **fixed virtual camera** computed from the slot's craft alone, and hashed
//! while its slot holds a lockable weapon. The reticle the player sees runs the
//! law again on the camera they are looking through, to put the brackets on the
//! craft, and is told the lock by [`Sight::follow`](oag_race::sight::Sight::follow)
//! instead of deciding it.
//!
//! # What is the original's and what is ours
//!
//! The law - the `0.8` s hold and the reticle's arrival for the Missile, the
//! extent's `0.34` s closing for the Pulse LeachBeam, the on-screen test, the
//! `250` unit range - is `HudSight_Update` and `FUN_0881e8c8`, unchanged. What
//! it is *fed* is **chosen, not measured**: the original projects through the
//! one camera it has; this projects through the team's far chase rig at rest
//! (`oag_render::camera::chase::anchor` and `look_at_point`: the settled eye,
//! with no spring lag, no impact shake and no boost kick), at the authored
//! field of view widened by the same speed term, in the title's own HUD grid.
//! It differs from what the player sees in four ways: the eye's spring lag, the
//! player's field-of-view setting, a cockpit or close view (the rig is the far
//! one whichever is on screen), and the aspect fit for a window narrower than
//! the authored shape. All four move where the craft sits on screen by a few
//! pixels, which is what the hold absorbs; none of them can be read by a
//! server. Every slot flown by a person uses the primary's team rig, because
//! only that team's `handlingstats.xml` is loaded.

use super::*;
use crate::weapons::SIGHT_FAR;
use oag_core::math::Vec4;

impl Race {
    /// Advances every person-flown slot's lock one tick.
    ///
    /// Runs at the end of the tick, after the weapon fire and the camera, as the
    /// reticle always has, so a shot reads the lock the previous tick left. In
    /// [`Race::tick_sim_only`] too: it is the simulation's.
    ///
    /// A slot holding nothing that locks is returned to a cleared lock, so a
    /// lock never outlives its weapon. Opponents have no lock (their shot takes
    /// the window's candidate at once - see [`Race::fire_missile`]).
    pub(super) fn advance_locks(&mut self) {
        let dt = self.sim.dt;
        for slot in 0..self.sim.world.ship_count as usize {
            if !self.sim.world.person_flown(slot) {
                continue;
            }
            let Some(held) = self.sight_held(slot) else {
                self.sim.locks[slot] = self.sim.locks[slot].cleared();
                continue;
            };
            let projected = self
                .sight_target(slot)
                .and_then(|target| self.lock_projection(slot, target as usize));
            let lock = &mut self.sim.locks[slot];
            lock.set_held(held);
            lock.update(dt, projected);
        }
    }

    /// Where `target` is on screen for `slot`'s virtual camera, or `None` behind
    /// it, out of its range or off the grid. See the module documentation.
    fn lock_projection(&self, slot: usize, target: usize) -> Option<sight::Projected> {
        let body = &self.sim.world.ships[slot].physics.body;
        let rig = &self.sim.lock_rig;
        let craft = Target {
            position: body.position,
            forward: body.forward(),
            up: body.up(),
        };
        let eye = oag_render::camera::chase::anchor(craft, rig);
        let look = oag_render::camera::chase::look_at_point(craft, rig);
        let view = oag_core::math::camera::look_at(eye, look, craft.up);
        let screen = self.sim.locks[slot].screen();
        let widened = (rig.fov + SPEED_FOV_GAIN_DEG * craft.forward.dot(body.linear_velocity))
            .clamp(FOV_GUARD_DEG.0, FOV_GUARD_DEG.1);
        let projection = lock_perspective(widened.to_radians(), screen[0] / screen[1]);
        let world = self.sim.world.ships[target].physics.body.position;
        sight::project(view, projection * view, world, screen)
    }

    /// Whether `slot` holds a lock this tick that its shot may take a target
    /// from, for a slot that has one at all. `None` for an opponent, whose shot
    /// needs no lock.
    pub(super) fn lock_taken(&self, slot: usize) -> Option<bool> {
        self.sim
            .world
            .person_flown(slot)
            .then(|| self.sim.locks[slot].locked())
    }

    /// `slot`'s lock, for a test or a tool that wants the simulation's reticle
    /// rather than the picture's.
    #[must_use]
    pub fn lock_of(&self, slot: usize) -> &sight::Sight {
        &self.sim.locks[slot]
    }
}

/// A right-handed perspective projection built without a platform `tan`.
///
/// `glam`'s own calls `f32::tan`, which the host's libm answers to its own last
/// bit, and this one feeds a hashed lock. Depth is the usual `0..1` range with
/// the near plane at 1 and the far at [`SIGHT_FAR`]; the reticle reads only
/// `x`, `y` and `w`.
fn lock_perspective(fov: f32, aspect: f32) -> Mat4 {
    let (sin, cos) = oag_core::math::sin_cos(fov * 0.5);
    let f = cos / sin;
    let (near, far) = (1.0, SIGHT_FAR);
    Mat4::from_cols(
        Vec4::new(f / aspect, 0.0, 0.0, 0.0),
        Vec4::new(0.0, f, 0.0, 0.0),
        Vec4::new(0.0, 0.0, far / (near - far), -1.0),
        Vec4::new(0.0, 0.0, near * far / (near - far), 0.0),
    )
}
