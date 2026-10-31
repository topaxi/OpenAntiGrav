//! The speed plans learned in overlapped stages ([`learn_speed_plans`]) are the
//! plans the one-after-another learner made, and a job started on the race-load
//! thread survives the launch's setters (`docs/gameplay/ai.md`, "The speed
//! plan").

use oag_gameplay::Controller;

use crate::speed_plan::{LearnedPlans, learn_speed_plans, learn_speed_plans_serially};
use crate::{Options, Race, load};

/// Pulse Outpost 7 (`07_Track`): one route, which never verifies.
const OUTPOST_7: &str = r"Data\Environments\07_Track\track.vex";

fn pulse_outpost_7() -> Option<Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = load(&Options {
        source: image.display().to_string(),
        class: "RAPIER".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(OUTPOST_7.to_string()),
        ..Options::default()
    })
    .expect("loading 07_Track");
    Some(Race::start(loaded.setup))
}

/// The two learners on the same race, every field they write compared.
fn assert_overlapped_is_serial(label: &str, race: &Race) -> LearnedPlans {
    let mut overlapped = race.sim.clone();
    let mut serial = race.sim.clone();
    let started = std::time::Instant::now();
    learn_speed_plans(&mut overlapped);
    let took = started.elapsed();
    let started = std::time::Instant::now();
    learn_speed_plans_serially(&mut serial);
    println!(
        "{label}: learning overlapped {took:?}, as before 2026-10-31 {:?}",
        started.elapsed()
    );
    let overlapped = LearnedPlans::of(overlapped);
    assert_eq!(overlapped, LearnedPlans::of(serial));
    overlapped
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn overlapped_plans_are_the_serial_plans_on_outpost_7() {
    let Some(race) = pulse_outpost_7() else {
        return;
    };
    assert!(!race.sim.routes.is_empty(), "07_Track has a fork");
    let learned = assert_overlapped_is_serial("07_Track", &race);
    assert!(learned.speed_plan().is_some(), "the ring's plan verifies");
}

/// 2048 `altima`, whose routes verify, so every hull also adapts each route's
/// plan (the third stage).
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn overlapped_plans_are_the_serial_plans_on_2048_altima() {
    let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let loaded = load(&Options {
        source: source.display().to_string(),
        class: "RAPIER".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(r"Data\art\published\environments\altima\track.vex".to_string()),
        ..Options::default()
    })
    .expect("loading altima");
    let race = Race::start(loaded.setup);
    assert!(!race.sim.routes.is_empty(), "altima has forks");
    let learned = assert_overlapped_is_serial("altima", &race);
    assert!(
        learned.route_plan_count() > 0,
        "a route verifies, so the hulls adapt it"
    );
}

/// The menus' setters as a campaign cell runs them, between the load
/// thread's start and the hand-over.
fn launch_setters(race: &mut Race) {
    race.set_autopilot(false);
    race.set_control_scheme(crate::ControlScheme::default());
    race.set_ai_tuning(oag_ai::Difficulty::tune_at_scale(
        0.5,
        &oag_ai::Tuning::default(),
    ));
    race.set_finished_thrust_skill(0.5);
    // What `set_opponent_pilot` writes into the world.
    for slot in 1..usize::from(race.sim.world.ship_count) {
        race.sim.world.ships[slot].driver.pilot ^= 0x5a5a;
    }
    assert!(
        race.use_original_ai(None, 1),
        "Pulse has the original tables"
    );
}

/// The menus' setters move no plan input, so a job started on the load thread
/// is kept and its plans are the ones the first tick of the set-up race would
/// learn. A second person seated moves one: the hand-over's call starts again
/// rather than the first tick.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_job_started_on_the_load_thread_survives_the_launch() {
    let Some(mut ahead) = pulse_outpost_7() else {
        return;
    };
    let mut lazy = pulse_outpost_7().expect("loaded once already");
    assert!(!ahead.sim.routes.is_empty(), "07_Track has a fork");
    ahead.begin_speed_plans_ahead();
    assert_eq!(ahead.speed_plan_job_fits(), Some(true));
    launch_setters(&mut ahead);
    launch_setters(&mut lazy);
    assert_eq!(
        ahead.speed_plan_job_fits(),
        Some(true),
        "no plan input moved"
    );
    ahead.begin_speed_plans();
    assert_eq!(ahead.speed_plan_job_fits(), Some(true), "kept");

    ahead.ensure_speed_plans();
    lazy.ensure_speed_plans();
    assert_eq!(ahead.speed_plan().cloned(), lazy.speed_plan().cloned());
    for slot in 0..8 {
        assert_eq!(
            ahead.plan_of(slot).cloned(),
            lazy.plan_of(slot).cloned(),
            "slot {slot}"
        );
    }
    for tick in 0..120 {
        ahead.tick(&oag_gameplay::PlayerInputs::none());
        lazy.tick(&oag_gameplay::PlayerInputs::none());
        assert_eq!(ahead.sim.state_hash(), lazy.sim.state_hash(), "tick {tick}");
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_second_person_seated_restarts_the_job_at_the_hand_over() {
    let Some(mut race) = pulse_outpost_7() else {
        return;
    };
    race.begin_speed_plans_ahead();
    race.sim.world.controllers[1] = Controller::Local;
    assert_eq!(
        race.speed_plan_job_fits(),
        Some(false),
        "a person in slot 1"
    );
    race.begin_speed_plans();
    assert_eq!(race.speed_plan_job_fits(), Some(true), "started again");
}
