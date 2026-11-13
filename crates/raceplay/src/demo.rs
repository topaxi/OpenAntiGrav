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
    /// [`oag_title::demo::Rules::camera_shake`] says so. A title whose demo watches through the
    /// spectator director gets it from the first tick when the circuit authors stations for it
    /// ([`crate::finish_camera`]); without, the chase camera stays.
    ///
    /// Not a recorded controller change ([`Race::set_controller`] writes one): a demo records
    /// nothing.
    pub fn fly_as_demo(&mut self, demo: &oag_title::demo::Demo) {
        self.sim.world.controllers = [Controller::Ai; MAX_PLAYERS];
        self.set_autopilot(true);
        self.sim.fire_terms.use_scale = demo.rules.ai_weapon_use.value;
        self.view.camera_shake_on = demo.rules.camera_shake.value;
        #[cfg(feature = "view")]
        if demo.camera.value == oag_title::demo::Camera::Spectator
            && let Some(director) = &mut self.view.finish_camera
        {
            director.reseed(self.sim.race_seed);
            self.view.spectates_from_start = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use oag_core::math::Vec3;
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
    fn a_demo_is_watched_through_the_spectator_director_from_its_first_tick() {
        use crate::finish_camera::{FinishCamera, SPECTATOR_SEED, ViewMode};
        let station = |x: f32| oag_render::camera::destroy::Station {
            eye: Vec3::new(x, 30.0, 0.0),
            aim: Vec3::new(x - 390.0, 0.0, 0.0),
        };
        let mut race = Race::start(Setup::headless(Mode::SingleRace, 1));
        race.view.finish_camera = Some(FinishCamera::new(
            vec![station(400.0), station(1400.0)],
            0.5,
            SPECTATOR_SEED,
        ));
        race.tick(&oag_gameplay::PlayerInputs::default());
        assert_eq!(race.spectator_mode(), None, "a race has no director yet");
        race.fly_as_demo(&oag_title::demo::Demo::PULSE);
        race.tick(&oag_gameplay::PlayerInputs::default());
        assert_eq!(
            race.spectator_mode(),
            Some(ViewMode::Track),
            "the director starts in the constructor's mode, on the first tick, with no finish"
        );
        let mut chase = Race::start(Setup::headless(Mode::SingleRace, 1));
        let mut director = FinishCamera::new(vec![station(400.0)], 0.5, SPECTATOR_SEED);
        director.reseed(1);
        chase.view.finish_camera = Some(director);
        let mut other = oag_title::demo::Demo::PULSE;
        other.camera = oag_title::pre_race::Sourced::chosen(oag_title::demo::Camera::Chase);
        chase.fly_as_demo(&other);
        chase.tick(&oag_gameplay::PlayerInputs::default());
        assert_eq!(chase.spectator_mode(), None, "the chase camera stays");
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
