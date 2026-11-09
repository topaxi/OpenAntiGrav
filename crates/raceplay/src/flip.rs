//! The Eliminator flip's place in the tick: the press, the gates and the turn, ahead of the
//! force law. `docs/ghidra/functions/ps3-hdfury-eu/eliminator-flip.md`.

use oag_gameplay::input::Button;

use oag_gameplay::MAX_PLAYERS;

use super::{Race, RaceState};

impl Race {
    /// Starts and advances each person's flip. A craft out of the race or on the grid may not
    /// start one; steering is dropped while one runs.
    pub(super) fn tick_flips(
        &mut self,
        inputs: &oag_gameplay::PlayerInputs,
        controls: &mut [oag_physics::ShipControls; MAX_PLAYERS],
    ) {
        let Some(law) = self.sim.flip_law else {
            return;
        };
        let dt = self.sim.dt;
        let racing = !RaceState::thrust_gated(self.sim.world.tick);
        for (slot, slot_controls) in controls.iter_mut().enumerate() {
            if !self.sim.world.person_flown(slot) {
                continue;
            }
            let pressed = inputs.get(slot).buttons.is_pressed(Button::Flip);
            let may_flip = racing && self.may_use_pickup(slot);
            let physics = &mut self.sim.world.ships[slot].physics;
            if pressed && may_flip {
                let lean = physics.steer_lean_follower;
                physics.flip.start(law, &physics.body, lean);
            }
            if physics.flip.active {
                physics.flip.advance(&mut physics.body, dt);
                slot_controls.steer_x = 0.0;
            }
        }
    }
}
