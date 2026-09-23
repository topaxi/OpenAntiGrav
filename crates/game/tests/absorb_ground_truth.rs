//! The weapon-absorb burst on a real race out of a real disc image.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test absorb_ground_truth --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! That the burst reaches the screen at all, which it did on no title before
//! 2026-09-23 - the sound played and nothing drew. So each test absorbs a
//! pickup on a real race and asserts what `oag_game::race::absorb` promises
//! for that title: one `WO_WEAPON_ABSORB` instance per locator the hull
//! authors, started on the title's own stagger and playing on the stage. The
//! locator counts are the discs' own - Pulse's Assegai carries six `Ship
//! Collision Fx` nodes and HD's Feisar six `absorb` ones - so a loader that
//! read the wrong class, or the wrong file, comes back with a count of zero
//! and fails here rather than drawing nothing quietly.

use std::path::PathBuf;

use oag_game::race;
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_tables::weapons::Weapon;

/// Past the start-line countdown - see `shuriken_ground_truth.rs`'s twin.
const WARM_UP_TICKS: u64 = oag_race::COUNTDOWN_TICKS + 60;

fn single_race(image: &str, team: Option<&str>) -> Option<race::Loaded> {
    let image: PathBuf = oag_testdata::image(image)?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        team: team.map(str::to_string),
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

fn throttle(extra: u32) -> oag_gameplay::InputSnapshot {
    let mut buttons = Input::new();
    buttons.begin_frame(Button::Cross.bit());
    buttons.begin_frame(Button::Cross.bit() | extra);
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// Absorbs a Mine on a moving craft and returns how many absorb bursts had
/// started by every tick of the following second and a half, starting with
/// the absorb tick itself - the burst's own count rather than the stage's,
/// since the rest of the field is racing, and firing, the whole time.
fn absorb_and_watch(loaded: race::Loaded) -> Vec<u32> {
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..WARM_UP_TICKS {
        race.tick(&PlayerInputs::single(throttle(0)));
    }
    let before = race.absorb_started_for_tests();
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Mine);
    race.tick(&PlayerInputs::single(throttle(Button::Circle.bit())));
    assert_eq!(
        race.ship_pickup(),
        None,
        "the Mine was not absorbed - the press never reached the absorb arm"
    );
    let mut started = vec![race.absorb_started_for_tests() - before];
    for _ in 0..90 {
        race.tick(&PlayerInputs::single(throttle(0)));
        started.push(race.absorb_started_for_tests() - before);
    }
    started
}

/// The first tick at which more than `n` bursts had started.
fn first_past(started: &[u32], n: u32) -> Option<usize> {
    started.iter().position(|&count| count > n)
}

/// Pulse: one burst per `Ship Collision Fx` node, a tenth of a second apart,
/// all six of Assegai's under way within six tenths.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn pulse_plays_one_burst_per_collision_fx_node_a_tenth_apart() {
    let Some(loaded) = single_race("data/images/pulse-psp-usa.chd", Some("Assegai")) else {
        return;
    };
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("6 Ship Collision Fx locator(s)")),
        "Assegai's hull no longer reports its six locators"
    );
    let started = absorb_and_watch(loaded);
    println!("started by each tick after the absorb: {started:?}");
    assert_eq!(
        started[0], 1,
        "node 0 has no delay and starts on the absorb tick"
    );
    // Node 1 is due 0.1 s later - six ticks of countdown, then the start.
    let second = first_past(&started, 1).expect("the second burst never started");
    assert!(
        (6..=8).contains(&second),
        "the second burst started on tick {second}, not a tenth of a second in"
    );
    assert_eq!(
        started.last().copied(),
        Some(6),
        "Assegai carries six Ship Collision Fx nodes"
    );
}

/// Wipeout HD: the six `absorb` locators, in three mirrored pairs 0.2 s apart.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_plays_three_mirrored_pairs_on_its_absorb_locators() {
    let Some(loaded) = single_race("data/images/hdfury-ps3-eu-dec.iso", Some("Feisar")) else {
        return;
    };
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("6 absorb locator(s)")),
        "Feisar's Locators.vex no longer reports its six absorb locators"
    );
    let started = absorb_and_watch(loaded);
    println!("started by each tick after the absorb: {started:?}");
    assert_eq!(
        started[0], 2,
        "the outer pair starts together on the absorb tick"
    );
    let middle = first_past(&started, 2).expect("the middle pair never started");
    assert!(
        (12..=14).contains(&middle),
        "the middle pair started on tick {middle}, not 0.2 s in"
    );
    assert_eq!(started[middle], 4, "the middle pair did not start together");
    let inner = first_past(&started, 4).expect("the inner pair never started");
    assert!(
        (24..=26).contains(&inner),
        "the inner pair started on tick {inner}, not 0.4 s in"
    );
    assert_eq!(
        started.last().copied(),
        Some(6),
        "Feisar carries six absorb locators"
    );
}

/// Pulse's hull overlay: built for the grid's hulls, started by a pickup
/// absorb, and run for its one-second window - the pulse climbing one frame's
/// `2 * dt` a tick to `2.0`, and gone after.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn pulse_lights_the_hull_for_one_second_after_a_pickup_absorb() {
    let Some(loaded) = single_race("data/images/pulse-psp-usa.chd", Some("Assegai")) else {
        return;
    };
    let overlays = loaded
        .report
        .iter()
        .filter(|line| line.contains("absorb overlay over"))
        .count();
    assert!(
        overlays >= 1,
        "no hull reported an absorb overlay - the texture or the scale did not resolve"
    );
    assert!(
        !loaded
            .report
            .iter()
            .any(|line| line.contains("no one batch scale")),
        "a Pulse PSP hull has batches at more than one scale"
    );
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..WARM_UP_TICKS {
        race.tick(&PlayerInputs::single(throttle(0)));
    }
    assert_eq!(race.absorb_overlay_pulse(0), None, "lit before any absorb");
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Mine);
    race.tick(&PlayerInputs::single(throttle(Button::Circle.bit())));
    let mut pulses = vec![race.absorb_overlay_pulse(0)];
    for _ in 0..70 {
        race.tick(&PlayerInputs::single(throttle(0)));
        pulses.push(race.absorb_overlay_pulse(0));
    }
    println!("pulse per tick after the absorb: {pulses:?}");
    let lit = pulses.iter().filter(|pulse| pulse.is_some()).count();
    assert!(
        (58..=61).contains(&lit),
        "the overlay was lit for {lit} ticks, not one second"
    );
    let peak = pulses.iter().flatten().copied().fold(0.0f32, f32::max);
    assert!(
        peak > 1.9,
        "the pulse never reached the end of its window ({peak})"
    );
    assert_eq!(pulses[70], None, "still lit past the window");
}
