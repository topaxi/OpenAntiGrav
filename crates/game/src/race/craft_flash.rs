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
//! - the player's own state-5 wash is not started, see the comment in
//!   [`Race::advance_craft_flashes`]: the original's active camera is 168.7
//!   units away by then (measured live), this port's is 11.6;
//! - every [`Race::respawn`] of the player flashes kind 6, including the
//!   invented off-track rescue, because a teleport is what the player sees;
//! - the big explosion's flash is armed at the state 5 edge and fires
//!   [`BLOWUP_DELAY`] later whether or not the craft has been put back by
//!   then, at the position it was wrecked.
//!
//! The craft's explosion *particles* (`WO_SHIP_FXNODE_EXPLO`,
//! `WO_SHIP_DEATH_SPARKS`, `WO_SHIP_EXPLOSION`) are not played by this port
//! yet; only the wash is, so a destruction shows the flash without the fire.

use oag_core::math::Vec3;
use oag_gameplay::MAX_SHIPS;
use oag_physics::CraftState;

use super::Race;

/// Seconds from the state 5 edge to the big explosion: `Ship_SetState` case
/// 5 writes `1.5` to the state timer and `Ship_UpdateDestroyed` counts it down.
pub(super) const BLOWUP_DELAY: f32 = super::eliminator::DESTROYED_DWELL;

/// Per-craft edge state for [`Race::advance_craft_flashes`].
#[derive(Debug, Clone, Default)]
pub(super) struct CraftFlashes {
    previous: [CraftState; MAX_SHIPS],
    /// Seconds left before the big explosion, and where the craft was wrecked.
    blowup: [Option<(f32, Vec3)>; MAX_SHIPS],
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
            if out_of_the_race_now {
                // **Not the player's own.** Kind 0 falls off with distance
                // from the *active* camera, and by state 5 the original has
                // put the player in its destroy camera (`Camera_SetMode(5)`),
                // measured live at 168.7 units from the wreck: a falloff of
                // 0.25, so a faint tint. This port keeps the chase camera,
                // 11.6 units away, where the same flash would be four times
                // as strong. Skipped until that camera is ported.
                if slot != player {
                    self.start_flash(oag_render::flash::BLAST, position);
                }
                self.view.craft_flashes.blowup[slot] = Some((BLOWUP_DELAY, position));
            }
            if let Some((left, at)) = &mut self.view.craft_flashes.blowup[slot] {
                // The edge tick's own `dt` is not counted: the timer is armed
                // by the case-5 write, and the first subtraction is next frame's.
                if !out_of_the_race_now {
                    *left -= dt;
                }
                if *left <= 0.0 {
                    let at = *at;
                    self.view.craft_flashes.blowup[slot] = None;
                    let kind = if slot == player {
                        oag_render::flash::PLAYER_DESTROYED
                    } else {
                        oag_render::flash::BLAST
                    };
                    self.start_flash(kind, at);
                }
            }
        }
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
