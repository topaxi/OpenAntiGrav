//! A snapshot shares the constant half of a [`RaceSim`] and copies the rest, and the
//! half that is filled on the first tick survives a restore across that tick in both
//! directions (`docs/architecture/adr/0087-a-replay-and-photo-mode-share-one-timeline.md`,
//! decision 2).

use super::reconcile::{two_person_race, varied};
use super::*;
use crate::{Options, load};
use oag_gameplay::{Controller, PlayerInputs};

/// The text every comparison of a whole simulation reads, with the per-tick outputs a
/// drawn tick leaves queued and a quiet one drops taken out.
fn text(race: &mut Race) -> String {
    race.drain_cues();
    race.drain_announcements();
    race.drain_class_announcements();
    format!("{:?}", race.sim)
}

/// Runs `ticks` of `inputs`, drawn or quiet, from tick `from`.
fn run(race: &mut Race, from: u32, ticks: u32, quiet: bool, inputs: &dyn Fn(u32) -> PlayerInputs) {
    for tick in from..from + ticks {
        if quiet {
            race.tick_sim_only(&inputs(tick));
        } else {
            race.tick(&inputs(tick));
        }
    }
}

/// The claim on one race, built twice by `fresh`. `ticks` runs well past the first.
fn crosses_the_first_tick(
    fresh: &dyn Fn() -> Race,
    ticks: u32,
    inputs: &dyn Fn(u32) -> PlayerInputs,
) {
    let mut drawn = fresh();
    let before_fill = drawn.snapshot_sim();
    let before_text = format!("{before_fill:?}");
    assert!(
        !before_fill.plans_built,
        "the plans are learned on the first tick"
    );

    run(&mut drawn, 0, ticks, false, inputs);
    assert!(
        drawn.sim.plans_built,
        "a race with a field learns its plans"
    );
    assert!(
        format!("{before_fill:?}") == before_text,
        "the fill reached a snapshot taken before it"
    );
    let filled = drawn.snapshot_sim();
    let filled_text = text(&mut drawn);

    // A snapshot from before the fill, put back into a race after it: no plans again, and
    // the race learns the same ones. The quiet tick and the drawn one agree on the way.
    drawn.restore_sim(&before_fill);
    assert!(
        format!("{:?}", drawn.sim) == before_text,
        "a restore across the fill is not the snapshot"
    );
    run(&mut drawn, 0, ticks, true, inputs);
    assert_eq!(filled.state_hash(), drawn.sim.state_hash());
    assert!(
        text(&mut drawn) == filled_text,
        "a pre-fill snapshot replayed quietly left the drawn run outside the hash"
    );

    // The reverse: a snapshot from after the fill, put back into a race that never ticked.
    let mut other = fresh();
    assert!(!other.sim.plans_built);
    other.restore_sim(&filled);
    assert!(other.sim.shares_constant_half_with(&filled));
    run(&mut other, ticks, ticks, true, inputs);
    run(&mut drawn, ticks, ticks, false, inputs);
    assert_eq!(drawn.sim.state_hash(), other.sim.state_hash());
    assert!(
        text(&mut drawn) == text(&mut other),
        "a post-fill snapshot in a race that never ticked left the drawn run"
    );
}

#[test]
fn a_snapshot_shares_the_constant_half_for_the_whole_race() {
    let mut race = two_person_race(Controller::Remote);
    race.tick(&varied(0));
    let early = race.snapshot_sim();
    for tick in 1..240 {
        race.tick(&varied(tick));
    }
    let late = race.snapshot_sim();
    assert!(late.shares_constant_half_with(&race.sim));
    assert!(
        early.shares_constant_half_with(&late),
        "a tick copied part of the constant half"
    );
    assert_ne!(early.state_hash(), late.state_hash());
}

/// The text is what says a restore is whole, and a shared field must stay in it: a change
/// to the constant half alone shows, and reaches only the snapshot it was made in.
#[test]
fn the_text_of_a_snapshot_reads_the_shared_half() {
    let mut race = two_person_race(Controller::Remote);
    race.tick(&varied(0));
    let before = format!("{:?}", race.sim);

    let mut order = race.snapshot_sim();
    Arc::make_mut(&mut order.ai_order)[0] ^= 1;
    let mut collision = race.snapshot_sim();
    collision.collision = Arc::new(CollisionWorld::new());
    let mut class = race.snapshot_sim();
    class.class = "ANOTHER".into();
    let mut weapons = race.snapshot_sim();
    weapons.weapons = Some(Arc::new(one_missile_table()));

    for (name, changed) in [
        ("order", order),
        ("collision", collision),
        ("class", class),
        ("weapons", weapons),
    ] {
        assert!(
            format!("{changed:?}") != before,
            "a change to {name} does not show in the text"
        );
    }
    assert!(
        format!("{:?}", race.sim) == before,
        "a snapshot's write reached the race"
    );
}

#[test]
fn a_restore_crosses_the_first_tick_both_ways() {
    crosses_the_first_tick(&|| two_person_race(Controller::Remote), 90, &varied);
}

/// The fixture's circuit has no fork, so `routes` stays empty there. These have one.
fn one_person_on_the_throttle(tick: u32) -> PlayerInputs {
    let mut buttons = oag_core::buttons::Input::new();
    buttons.begin_frame(oag_core::buttons::Button::Cross.bit());
    PlayerInputs::single(
        oag_gameplay::InputSnapshot {
            buttons,
            stick_x: ((tick as f32) * 0.05).sin() * 0.3,
            ..oag_gameplay::InputSnapshot::default()
        }
        .sanitised(),
    )
}

fn crosses_on_a_fork(source: &std::path::Path, track: &str) {
    let loaded = load(&Options {
        source: source.display().to_string(),
        class: "RAPIER".to_string(),
        mode: Mode::SingleRace,
        track: Some(track.to_string()),
        ..Options::default()
    })
    .expect("loading the circuit");
    let start = || Race::start(loaded.setup.clone());
    assert!(!start().sim.routes.is_empty(), "{track} has a fork");
    crosses_the_first_tick(&start, 240, &one_person_on_the_throttle);
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_restore_crosses_the_first_tick_on_outpost_7() {
    if let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") {
        crosses_on_a_fork(&image, r"Data\Environments\07_Track\track.vex");
    }
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_restore_crosses_the_first_tick_on_2048_altima() {
    if let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") {
        crosses_on_a_fork(&source, r"Data\art\published\environments\altima\track.vex");
    }
}
