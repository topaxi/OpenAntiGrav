//! The full-screen washes a craft's own state changes start: its destruction
//! and, for the local player, its reset.
//!
//! # Recovered (Pulse, PSP), 2026-09-30
//!
//! `ScreenFlash_Start` (`0x088f00c0`) has four callers in the craft's own
//! state machine:
//!
//! | Caller | When | Kind | Who |
//! | --- | --- | ---: | --- |
//! | `FUN_0883e064`, from `Ship_SetState` case 5 (`0x08844590`) | the explosion ends and the craft is out | 0 | every craft |
//! | `FUN_088407b0`, from `Ship_UpdateDestroyed` (`0x08847650`, `0x088478f4`) | 1.5 s later, as state 6 or 8 begins | 7, or 0 | kind 7 when `craft+0x368 == 0` (the local player), else 0 |
//! | `Ship_SetState` case 3 (`0x088441fc`) | a reset begins | 6 | the local player only |
//! | `Ship_UpdateRespawn` (`0x08847a2c`) | the respawn timer runs out | 6 | the local player only |
//!
//! Kind 0 is placed at the craft (`+0x794` node, `+0x30`); kinds 6 and 7 have
//! no falloff, so their position is unread. The 1.5 s is state 5's own timer,
//! `entity+0x874`, [`super::eliminator::DESTROYED_DWELL`].
//!
//! # What this port does with it
//!
//! [`oag_physics::CraftState`] carries the two edges (`Destroyed` to
//! `Eliminated` is state 4 to 5) and [`Race::respawn`] is the one place a craft
//! is put back, so the flash follows those rather than a state number this
//! engine does not have. **Chosen, not measured:**
//!
//! - the player's own state-5 wash is started like any craft's now that the
//!   destroy camera ([`super::destroy_camera`]) is the active camera by then;
//! - every [`Race::respawn`] of the player flashes kind 6, including the
//!   invented off-track rescue, because a teleport is what the player sees;
//! - the big explosion's flash is armed at the state 5 edge and fires
//!   [`BLOWUP_DELAY`] later whether or not the craft has been put back by
//!   then, at the position it was wrecked;
//! - the player's own two shakes at the explosions are armed as the original's
//!   `Camera_ArmShake` calls do: `(0.3, 0.4, mode 3)` on the state 5 edge and
//!   `(0.8, 0.6, mode 1)` with the big explosion, and a running shake is
//!   cancelled as state 4 begins. The first is left out where the race ends
//!   on that edge (everything but an Eliminator), since the finished race is
//!   not stepped and would hold it at its largest frame.
//!
//! The state 5 edge also throws `WO_SHIP_FXNODE_EXPLO` and
//! `WO_SHIP_DEATH_SPARKS` at each wreck node ([`super::wreck_fx`]).
//! `WO_SHIP_EXPLOSION`, [`BLOWUP_DELAY`] later, is thrown with the big flash,
//! at the live model's matrix ([`Race::throw_wreck_explosion`]).

use oag_core::math::{Mat4, Vec3};
use oag_gameplay::MAX_SHIPS;
use oag_physics::CraftState;
use oag_race::Mode;
use oag_render::camera::shake::Side;

use super::Race;

/// Seconds from the state 5 edge to the big explosion: `Ship_SetState` case
/// 5 writes `1.5` to the state timer and `Ship_UpdateDestroyed` counts it down.
pub(super) const BLOWUP_DELAY: f32 = super::eliminator::DESTROYED_DWELL;

/// `FUN_0883e064`'s `Camera_ArmShake(0.3, 0.4, camera, 3)` at state 5, read
/// 2026-10-01: (magnitude, duration in seconds, side).
const STATE_5_SHAKE: (f32, f32, Side) = (0.3, 0.4, Side::Ahead);

/// `FUN_088407b0`'s `Camera_ArmShake(0.8, 0.6, camera, 1)` at state 6.
const STATE_6_SHAKE: (f32, f32, Side) = (0.8, 0.6, Side::Elsewhere);

/// Per-craft edge state for [`Race::advance_craft_flashes`].
#[derive(Debug, Clone, Default)]
pub(super) struct CraftFlashes {
    previous: [CraftState; MAX_SHIPS],
    /// Seconds left before the big explosion, where the craft was wrecked and
    /// the model matrix it was wrecked with - the explosion is placed with it,
    /// and an Eliminator craft has been put back on the track by then.
    blowup: [Option<(f32, Vec3, Mat4)>; MAX_SHIPS],
}

impl Race {
    /// Starts the washes a craft's state edges call for, once a tick, after
    /// the destroyed-craft pass has settled every state.
    pub(super) fn advance_craft_flashes(&mut self) {
        let player = self.sim.world.primary_slot();
        let dt = self.sim.dt;
        for slot in 0..self.sim.world.ship_count as usize {
            let ship = &self.sim.world.ships[slot];
            let state = ship.physics.craft_state;
            let position = ship.physics.body.position;
            let was = std::mem::replace(&mut self.view.craft_flashes.previous[slot], state);
            let out_of_the_race_now =
                was == CraftState::Destroyed && state == CraftState::Eliminated;
            if slot == player && was != CraftState::Destroyed && state == CraftState::Destroyed {
                // `Ship_SetState` case 4, the local player's half: it zeroes
                // the camera's shake duration, cancelling a running one.
                self.view.shake.cancel();
            }
            if out_of_the_race_now {
                // Kind 0 falls off with distance from the *active* camera,
                // and by state 5 the original has put the player in its
                // destroy camera ([`super::destroy_camera`]); the flash reads
                // this port's own active camera in the same way.
                self.start_flash(oag_render::flash::BLAST, position);
                let model = super::drawable::model_matrix_of(&self.sim.world.ships[slot]);
                self.view.craft_flashes.blowup[slot] = Some((BLOWUP_DELAY, position, model));
                self.throw_wreck_fx(slot);
                // **Not where the race ends on this very tick**: a finished race
                // is not stepped, so the shake would be held for good at its
                // first and largest frame - a tilt of six degrees under a
                // four-degree field, a different scene under the results.
                // Chosen, not measured. The original goes on running.
                if slot == player && self.sim.world.mode() == Mode::Eliminator {
                    self.arm_player_blast_shake(STATE_5_SHAKE);
                }
            }
            if let Some((left, at, model)) = &mut self.view.craft_flashes.blowup[slot] {
                // The edge tick's own `dt` is not counted: the timer is armed
                // by the case-5 write, and the first subtraction is next frame's.
                if !out_of_the_race_now {
                    *left -= dt;
                }
                if *left <= 0.0 {
                    let (at, model) = (*at, *model);
                    self.view.craft_flashes.blowup[slot] = None;
                    let kind = if slot == player {
                        oag_render::flash::PLAYER_DESTROYED
                    } else {
                        oag_render::flash::BLAST
                    };
                    self.start_flash(kind, at);
                    self.throw_wreck_explosion(slot, model);
                    if slot == player {
                        self.hold_destroy_camera();
                        self.arm_player_blast_shake(STATE_6_SHAKE);
                    }
                }
            }
        }
    }

    /// `Camera_ArmShake(magnitude, duration, camera, mode)` as the craft's two
    /// explosions call it, for the local player only.
    fn arm_player_blast_shake(&mut self, (magnitude, duration, side): (f32, f32, Side)) {
        self.view
            .shake
            .arm_with(magnitude, duration, side, &mut self.view.shake_rng);
    }

    /// The local player was put back on the track: `Ship_SetState` case 3 and
    /// `Ship_UpdateRespawn` both start kind 6 for `craft+0x368 == 0` only.
    pub(super) fn flash_player_reset(&mut self, slot: usize) {
        if slot == self.sim.world.primary_slot() {
            self.start_flash(oag_render::flash::RESET, Vec3::ZERO);
        }
    }

    fn start_flash(&mut self, kind: oag_render::flash::Kind, at: Vec3) {
        if let Some(flash) = &mut self.view.screen_flash {
            flash.start(kind, at);
        }
    }
}
