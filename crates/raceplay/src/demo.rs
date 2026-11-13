//! An attract demo's race: the same race a player would have, with nobody in it.
//!
//! The simulation already has the pieces. `World::controllers` says who is a person, and
//! `World::primary_slot` falls back to slot 0 for a race with no person in it, which is the slot a
//! full-screen HUD and a single camera follow. That slot is stepped by the player step whatever its
//! controller says, so the operator's autopilot is what flies it
//! ([`Race::set_autopilot`]); every other slot is an opponent.

use oag_gameplay::{Controller, MAX_PLAYERS};

use crate::Race;

impl Race {
    /// Takes the race from the person who would have flown it: every slot's controller is the AI
    /// and the camera slot is autopiloted. Call it after [`Race::start`] and before the first tick;
    /// the controllers are in the state hash, so a race handed over later than that would not
    /// reproduce.
    ///
    /// The title's demo rules that reach the simulation are laid over the race too: the
    /// opponents' weapon-use odds are multiplied by [`oag_title::demo::Rules::ai_weapon_use`]
    /// (`oag_ai::weapon_ai::Law::use_scale`), and an impact shakes the camera only if
    /// [`oag_title::demo::Rules::camera_shake`] says so.
    ///
    /// Not a recorded controller change ([`Race::set_controller`] writes one): a demo records
    /// nothing.
    pub fn fly_as_demo(&mut self, demo: &oag_title::demo::Demo) {
        self.sim.world.controllers = [Controller::Ai; MAX_PLAYERS];
        self.set_autopilot(true);
        self.sim.fire_terms.use_scale = demo.rules.ai_weapon_use.value;
        self.view.camera_shake_on = demo.rules.camera_shake.value;
    }
}

#[cfg(test)]
mod tests {
    use oag_race::Mode;

    use crate::{Race, Setup};

    #[test]
    fn a_demo_has_no_person_and_every_craft_is_flown_by_the_ai() {
        let mut race = Race::start(Setup::headless(Mode::SingleRace, 1));
        assert_eq!(
            race.sim.world.human_slots().count(),
            1,
            "the default grid has a person"
        );
        race.fly_as_demo(&oag_title::demo::Demo::PULSE);
        assert_eq!(race.sim.world.human_slots().count(), 0);
        let camera_slot = race.sim.world.primary_slot();
        assert!(
            race.flown_for_the_player(camera_slot),
            "the camera slot is stepped as a player and nobody holds its pad: the autopilot must"
        );
        for slot in 0..usize::from(race.sim.world.ship_count) {
            assert!(
                slot == camera_slot || !race.sim.world.person_flown(slot),
                "slot {slot} is flown by a person"
            );
        }
    }

    #[cfg(feature = "view")]
    #[test]
    fn a_demo_camera_does_not_shake_and_a_race_camera_does() {
        let mut race = Race::start(Setup::headless(Mode::SingleRace, 1));
        race.force_shake(1.0);
        assert!(race.view.contact.shake.active());
        race.fly_as_demo(&oag_title::demo::Demo::PULSE);
        race.force_shake(1.0);
        race.tick(&oag_gameplay::PlayerInputs::default());
        assert!(!race.view.contact.shake.active());
    }

    #[test]
    fn a_demo_doubles_the_opponents_weapon_use_and_a_race_does_not() {
        let mut race = Race::start(Setup::headless(Mode::SingleRace, 1));
        assert_eq!(race.sim.fire_terms.use_scale, 1.0);
        race.fly_as_demo(&oag_title::demo::Demo::PULSE);
        assert_eq!(race.sim.fire_terms.use_scale, 2.0);
    }
}
