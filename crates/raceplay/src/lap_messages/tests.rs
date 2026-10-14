use super::*;

fn pulse(stored: Option<u32>) -> LapWatch {
    LapWatch::new(oag_pulse::hud::ART.lap_hud.expect("read"), stored)
}

fn lap(lap: u32, ticks: u32, best: u32) -> LapEdge {
    LapEdge {
        completed: true,
        finished: false,
        lap,
        laps_target: Some(3),
        lap_ticks: Some(ticks),
        best_lap_ticks: Some(best),
    }
}

#[test]
fn a_clean_lap_is_perfect_and_a_scrape_spoils_it() {
    let mut watch = pulse(None);
    assert_eq!(
        watch.observe(false, false, lap(2, 3600, 3600)),
        ["IG_HUD_PLAP"]
    );
    watch.observe(true, false, LapEdge::default());
    assert_eq!(
        watch.observe(false, false, lap(3, 3600, 3600)),
        ["IG_HUD_FIN_LAP"]
    );
}

#[test]
fn a_scrape_before_the_first_crossing_taints_lap_one() {
    let mut watch = pulse(None);
    watch.observe(true, false, LapEdge::default());
    let first = LapEdge {
        completed: false,
        ..lap(1, 0, 0)
    };
    assert!(watch.observe(false, false, first).is_empty());
    assert!(watch.observe(false, false, lap(2, 3600, 3600)).is_empty());
}

#[test]
fn the_final_lap_line_comes_last_and_the_finish_says_nothing() {
    let mut watch = pulse(Some(4000));
    assert_eq!(
        watch.observe(false, false, lap(3, 3600, 3600)),
        ["IG_HUD_PLAP", "IG_HUD_NEWLAP_REC", "IG_HUD_FIN_LAP"]
    );
    let mut done = lap(4, 3500, 3500);
    done.finished = true;
    assert!(watch.observe(false, false, done).is_empty());
}

#[test]
fn a_record_needs_a_stored_one_to_beat_and_the_best_lap_of_the_race() {
    let mut none = pulse(None);
    assert!(
        !none
            .observe(true, false, lap(2, 100, 100))
            .contains(&"IG_HUD_NEWLAP_REC")
    );
    let mut slower = pulse(Some(3000));
    assert!(
        !slower
            .observe(true, false, lap(2, 3600, 3600))
            .contains(&"IG_HUD_NEWLAP_REC")
    );
    let mut worse_than_race_best = pulse(Some(4000));
    assert!(
        !worse_than_race_best
            .observe(true, false, lap(3, 3600, 3000))
            .contains(&"IG_HUD_NEWLAP_REC")
    );
}

/// HD raises no new-lap-record line in Time Trial (its `0x91d20` skips it in
/// mode 5 while the profile byte is clear); Pulse and Pure do.
#[test]
fn only_hd_skips_the_record_line_in_time_trial() {
    let beaten = lap(2, 3000, 3000);
    for (art, expected) in [
        (oag_hd::hud::ART, false),
        (oag_pulse::hud::ART, true),
        (oag_pure::hud::ART, true),
    ] {
        let hud = art.lap_hud.expect("read");
        let mut watch = LapWatch::new(hud, Some(4000));
        let in_time_trial = watch.observe(true, true, beaten);
        assert_eq!(in_time_trial.contains(&hud.new_lap_record), expected);
        let mut watch = LapWatch::new(hud, Some(4000));
        let elsewhere = watch.observe(true, false, beaten);
        assert!(elsewhere.contains(&hud.new_lap_record), "any other mode");
    }
}

/// Each title raises its own phrase ids, not Pulse's.
#[test]
fn pure_raises_its_own_keys() {
    let hud = oag_pure::hud::ART.lap_hud.expect("read");
    let mut watch = LapWatch::new(hud, Some(4000));
    assert_eq!(
        watch.observe(false, false, lap(3, 3600, 3600)),
        ["HUD_Perfect Lap!", "HUD_New Lap Record", "HUD_Final Lap!"]
    );
}

mod race_wiring {
    use super::*;
    use crate::Race;
    use crate::tests::race_with_a_grid;
    use oag_gameplay::PlayerInputs;

    fn contact() -> oag_physics::wall::WallContact {
        oag_physics::wall::WallContact {
            point: oag_core::math::Vec3::ZERO,
            normal: oag_core::math::Vec3::Y,
            depth: 0.1,
            surface: oag_physics::Surface::Wall,
            collider: 0,
        }
    }

    fn watched() -> (Race, oag_physics::Evaluated) {
        let mut race = race_with_a_grid();
        let evaluated = *race.tick(&PlayerInputs::none()).primary();
        for hud in &mut race.view.huds {
            hud.lap_watch = Some(pulse(None));
        }
        (race, evaluated)
    }

    fn dirty_of(race: &Race, slot: usize) -> bool {
        race.view.huds[slot]
            .lap_watch
            .as_ref()
            .expect("watched")
            .dirty
    }

    fn dirty(race: &Race) -> bool {
        dirty_of(race, 0)
    }

    /// The original's `0x20` bit is set by every positive-friction contact
    /// record, not only an inbound one: a sustained scrape spoils the lap.
    #[test]
    fn a_scrape_that_is_not_an_inbound_hit_spoils_the_lap() {
        let (mut race, mut evaluated) = watched();
        evaluated.wall.resolved = Some(contact());
        evaluated.wall.impact = false;
        race.observe_lap(0, &oag_race::Outcome::default(), &evaluated);
        assert!(dirty(&race));
    }

    #[test]
    fn a_shielded_contact_and_no_contact_leave_it_clean() {
        let (mut race, mut evaluated) = watched();
        race.observe_lap(0, &oag_race::Outcome::default(), &evaluated);
        assert!(!dirty(&race), "no contact");
        evaluated.wall.resolved = Some(contact());
        evaluated.shield.absorbed = true;
        race.observe_lap(0, &oag_race::Outcome::default(), &evaluated);
        assert!(!dirty(&race), "absorbed by a shield");
    }

    /// A network client flying seat 1 grades seat 1's laps, not slot 0's.
    #[test]
    fn the_watch_follows_this_machines_seat() {
        let (mut race, mut evaluated) = watched();
        race.set_seat(Some(1));
        evaluated.wall.resolved = Some(contact());
        race.observe_lap(0, &oag_race::Outcome::default(), &evaluated);
        assert!(!dirty(&race), "slot 0 is someone else's craft");
        race.observe_lap(1, &oag_race::Outcome::default(), &evaluated);
        assert!(dirty_of(&race, 1));
    }

    /// Two people on one machine: each seat's wall scrape spoils only its own
    /// lap, and each seat's lines land in its own view's board.
    #[test]
    fn each_local_seat_grades_its_own_laps_into_its_own_view() {
        let (mut race, mut evaluated) = watched();
        race.sim.world.controllers[1] = oag_gameplay::Controller::Local;
        let clean = evaluated;
        evaluated.wall.resolved = Some(contact());
        let done = oag_race::Outcome {
            lap_completed: true,
            lap_ticks: Some(3600),
            ..Default::default()
        };
        race.sim.world.race[0].lap = 2;
        race.sim.world.race[1].lap = 2;
        race.observe_lap(0, &oag_race::Outcome::default(), &clean);
        race.observe_lap(1, &oag_race::Outcome::default(), &evaluated);
        assert!(!dirty_of(&race, 0));
        assert!(dirty_of(&race, 1), "seat 1 scraped");
        race.observe_lap(0, &done, &clean);
        race.observe_lap(1, &done, &clean);
        let has = |slot: usize| {
            let mut board = race.view.huds[slot].messages.clone();
            board.advance();
            board.lines().iter().any(Option::is_some)
        };
        assert!(has(0), "seat 0's clean lap announces in seat 0's view");
        assert!(!has(1), "seat 1's scraped lap announces nothing");
    }

    fn overlap_slots_0_and_1(race: &mut Race) {
        let at = race.sim.world.ships[0].physics.body.position;
        race.sim.world.ships[1].physics.body.position =
            at + oag_core::math::Vec3::new(0.0, 0.0, 1.0);
        race.sim.world.ships[0].physics.body.linear_velocity =
            oag_core::math::Vec3::new(0.0, 0.0, 8.0);
        race.sim.world.ships[1].physics.body.linear_velocity =
            oag_core::math::Vec3::new(0.0, 0.0, -8.0);
    }

    /// `Body_ResolveContactPair` appends a positive-friction record to both
    /// bodies' contact rings, and the loop that reads them sets the `0x20` bit
    /// for each: bumping a rival spoils both craft's laps.
    #[test]
    fn a_craft_to_craft_contact_spoils_both_laps() {
        let (mut race, clean) = watched();
        race.sim.world.controllers[1] = oag_gameplay::Controller::Local;
        overlap_slots_0_and_1(&mut race);
        race.resolve_craft_pairs();
        race.observe_lap(0, &oag_race::Outcome::default(), &clean);
        race.observe_lap(1, &oag_race::Outcome::default(), &clean);
        assert!(dirty_of(&race, 0) && dirty_of(&race, 1));
    }

    #[test]
    fn a_shielded_craft_keeps_its_lap_through_a_craft_contact() {
        let (mut race, clean) = watched();
        race.sim.world.controllers[1] = oag_gameplay::Controller::Local;
        race.sim.world.ships[1].physics.shield_pickup_timer = 1.0;
        overlap_slots_0_and_1(&mut race);
        race.resolve_craft_pairs();
        race.observe_lap(0, &oag_race::Outcome::default(), &clean);
        race.observe_lap(1, &oag_race::Outcome::default(), &clean);
        assert!(dirty_of(&race, 0) && !dirty_of(&race, 1));
    }

    /// A reconciliation replay re-runs ticks the prediction already watched.
    #[test]
    fn a_replayed_tick_does_not_feed_the_watch() {
        let (mut race, mut evaluated) = watched();
        evaluated.wall.resolved = Some(contact());
        race.quiet = true;
        race.observe_lap(0, &oag_race::Outcome::default(), &evaluated);
        assert!(!dirty(&race));
    }
}
