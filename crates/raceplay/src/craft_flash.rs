//! The full-screen washes a craft's own state changes start: its destruction
//! and, for the local player, its reset.
//!
//! # Recovered (Pulse, PSP)
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
//! `Eliminated` is state 4 to 5) and [`crate::Race::respawn`] is the one place a craft
//! is put back, so the flash follows those rather than a state number this
//! engine does not have. **Chosen, not measured:**
//!
//! - the player's own state-5 wash is started like any craft's now that the
//!   destroy camera ([`super::destroy_camera`]) is the active camera by then;
//! - every [`crate::Race::respawn`] of the player flashes kind 6, including the
//!   invented off-track rescue, because a teleport is what the player sees;
//! - the big explosion's flash is armed at the state 5 edge and fires
//!   [`BLOWUP_DELAY`] later whether or not the craft has been put back by
//!   then, at the position it was wrecked;
//! - the player's own two shakes at the explosions are armed as the original's
//!   `Camera_ArmShake` calls do: `(0.3, 0.4, mode 3)` on the state 5 edge and
//!   `(0.8, 0.6, mode 1)` with the big explosion, and a running shake is
//!   cancelled as state 4 begins. Both play out under the results of a race
//!   the wreck ended, because a finished race steps its cosmetics
//!   ([`crate::Race::tick_cosmetics`]).
//!
//! The state 5 edge also throws `WO_SHIP_FXNODE_EXPLO` and
//! `WO_SHIP_DEATH_SPARKS` at each wreck node ([`super::wreck_fx`]).
//! `WO_SHIP_EXPLOSION`, [`BLOWUP_DELAY`] later, is thrown with the big flash,
//! at the live model's matrix ([`crate::RaceView::throw_wreck_explosion`]).

#[cfg(feature = "view")]
use crate::RaceView;
#[cfg(feature = "view")]
use crate::SimRace;
use oag_core::math::{Mat4, Vec3};
use oag_gameplay::MAX_SHIPS;
use oag_physics::CraftState;
use oag_render::camera::shake::Side;

/// Seconds from the state 5 edge to the big explosion: `Ship_SetState` case
/// 5 writes `1.5` to the state timer and `Ship_UpdateDestroyed` counts it down.
pub(super) const BLOWUP_DELAY: f32 = super::eliminator::DESTROYED_DWELL;

/// `FUN_0883e064`'s `Camera_ArmShake(0.3, 0.4, camera, 3)` at state 5:
/// (magnitude, duration in seconds, side).
const STATE_5_SHAKE: (f32, f32, Side) = (0.3, 0.4, Side::Ahead);

/// `FUN_088407b0`'s `Camera_ArmShake(0.8, 0.6, camera, 1)` at state 6.
const STATE_6_SHAKE: (f32, f32, Side) = (0.8, 0.6, Side::Elsewhere);

/// Per-craft edge state for [`crate::RaceView::advance_craft_flashes`].
#[derive(Debug, Clone, Default)]
pub(super) struct CraftFlashes {
    previous: [CraftState; MAX_SHIPS],
    /// Seconds left before the big explosion, where the craft was wrecked and
    /// the model matrix it was wrecked with - the explosion is placed with it,
    /// and an Eliminator craft has been put back on the track by then.
    blowup: [Option<(f32, Vec3, Mat4)>; MAX_SHIPS],
    /// Whether each craft's wreck has been squashed by the big blast and is no longer drawn
    /// (`DestroyedCraft::wreck_hidden_at_blast`). View state beside the blast's own timer, so a
    /// world that stands still under the results (a Zone wreck) hides it on the same tick.
    wreck_gone: [bool; MAX_SHIPS],
}

#[cfg(feature = "view")]
impl RaceView {
    /// Starts the washes a craft's state edges call for, once a tick, after
    /// the destroyed-craft pass has settled every state.
    pub(super) fn advance_craft_flashes(&mut self, core: &SimRace) {
        self.advance_zone_end(core);
        let player = core.player_slot();
        let dt = core.sim.dt;
        for slot in 0..core.sim.world.ship_count as usize {
            let ship = &core.sim.world.ships[slot];
            let state = ship.physics.craft_state;
            let position = self.drawn_position(core, slot);
            let was = std::mem::replace(&mut self.craft_flashes.previous[slot], state);
            if state != CraftState::Eliminated {
                self.craft_flashes.wreck_gone[slot] = false;
            }
            // A Zone craft throws no blast, plume, node sparks, wash or shake: its end is the
            // shell and the debris (`zone-destroy.md`, row Z3).
            if core.sim.zone_end().is_some() {
                continue;
            }
            let out_of_the_race_now =
                was == CraftState::Destroyed && state == CraftState::Eliminated;
            let blast_at = core.sim.destroyed.blast_at;
            if was != CraftState::Destroyed && state == CraftState::Destroyed {
                if let Some(fx) = self.seat_contact_mut(core, slot) {
                    // `Ship_SetState` case 4, the local player's half: it zeroes
                    // the camera's shake duration, cancelling a running one.
                    fx.shake.cancel();
                }
                if blast_at == oag_title::destroyed::BlastAt::Explosion {
                    // `Ship_PlayDestroyExplosion`, on the entry to state 4.
                    let model = self.drawn_model(core, slot);
                    self.throw_wreck_explosion(core, slot, model);
                }
            }
            if out_of_the_race_now {
                self.throw_gaseous_flash(core, position);
                // Kind 0 falls off with distance from the *active* camera,
                // and by state 5 the original has put the player in its
                // destroy camera ([`super::destroy_camera`]); the flash reads
                // this port's own active camera in the same way.
                self.start_flash(oag_fx::flash::BLAST, position);
                let model = self.drawn_model(core, slot);
                if blast_at == oag_title::destroyed::BlastAt::DwellEnd {
                    self.craft_flashes.blowup[slot] = Some((BLOWUP_DELAY, position, model));
                }
                self.throw_wreck_fx(core, slot);
                self.throw_death_plume(core, slot);
                // Every mode, the edge that ends a Single Race or a Zone run
                // included: the finished race steps its cosmetics
                // ([`Race::tick_cosmetics`]), so the shake decays under the
                // results as the original's does.
                self.arm_player_blast_shake(core, slot, STATE_5_SHAKE);
            }
            if let Some((left, at, model)) = &mut self.craft_flashes.blowup[slot] {
                // The edge tick's own `dt` is not counted: the timer is armed
                // by the case-5 write, and the first subtraction is next frame's.
                if !out_of_the_race_now {
                    *left -= dt;
                }
                if *left <= 0.0 {
                    let (at, model) = (*at, *model);
                    self.craft_flashes.blowup[slot] = None;
                    let kind = if slot == player {
                        oag_fx::flash::PLAYER_DESTROYED
                    } else {
                        oag_fx::flash::BLAST
                    };
                    self.start_flash(kind, at);
                    self.throw_wreck_explosion(core, slot, model);
                    self.throw_gaseous_flash(core, at);
                    self.craft_flashes.wreck_gone[slot] = core.sim.destroyed.wreck_hidden_at_blast;
                    if slot == player {
                        self.hold_destroy_camera();
                    }
                    self.arm_player_blast_shake(core, slot, STATE_6_SHAKE);
                }
            }
        }
    }

    /// `Camera_ArmShake(magnitude, duration, camera, mode)` as the craft's two
    /// explosions call it, for a person's own craft: the player's and every other local seat's.
    fn arm_player_blast_shake(
        &mut self,
        core: &SimRace,
        slot: usize,
        (magnitude, duration, side): (f32, f32, Side),
    ) {
        if let Some(fx) = self.seat_contact_mut(core, slot) {
            fx.shake
                .arm_with(magnitude, duration, side, &mut fx.shake_rng);
        }
    }

    /// Whether `slot`'s wreck has been squashed by the big blast: its matrix is scaled `1e-4`
    /// and state 6 never rewrites it (Pulse `0x08841e50`, Pure `0x089233f0`,
    /// `docs/ghidra/functions/psp-pure-usa/ship-destroy.md`, row 22), so it is not drawn.
    #[must_use]
    pub(crate) fn wreck_gone(&self, slot: usize) -> bool {
        self.craft_flashes.wreck_gone[slot]
    }

    /// The gaseous flash at the body, where the title throws one (`ship-destroy.md`, rows 8 and
    /// 21): on the state 5 edge and again at the big blast.
    fn throw_gaseous_flash(&mut self, core: &SimRace, at: Vec3) {
        if core.sim.destroyed.gaseous_flash.is_some() {
            self.spawn_gaseous_flash(at);
        }
    }

    pub(super) fn start_flash(&mut self, kind: oag_fx::flash::Kind, at: Vec3) {
        if let Some(flash) = &mut self.screen_flash {
            flash.start(kind, at);
        }
    }
}

#[cfg(feature = "view")]
impl RaceView {
    /// The local player was put back on the track: `Ship_SetState` case 3 and
    /// `Ship_UpdateRespawn` both start kind 6 for `craft+0x368 == 0` only.
    pub(super) fn flash_player_reset(&mut self, core: &SimRace, slot: usize) {
        if slot == core.player_slot() {
            self.start_flash(oag_fx::flash::RESET, Vec3::ZERO);
        }
    }
}
