//! A craft hit by a weapon slows down, on a real circuit out of a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this crate:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all
//! ```
//!
//! Its own file rather than a section of `race_ground_truth.rs`, for the reason
//! `plasma_ground_truth.rs` gives: that file is at its `BASELINE` ceiling in
//! `scripts/check-file-size.py`. The harness helpers below are the same
//! deliberate near-duplicates.
//!
//! # Why this test exists at all
//!
//! The mechanic was **missing rather than mistuned** - reported from play on
//! 2026-09-06 as "a craft hit by a mine, rocket or missile does not slow down",
//! and it did not, because nothing armed the timer. `oag_physics::slowdown`
//! holds the recovered law; this is the end of it that a player feels.
//!
//! # What only real data can say here
//!
//! 1. **That the disc's own `slowdown_time` and `slowdown_limit` reach the
//!    simulation.** Both decode against hand-written fixtures already. What no
//!    fixture can show is that the figures the *shipped* table authors are
//!    routed all the way from `<WeaponStats>` through
//!    `projectile::blast_stats` and `slowdown::drain` into a craft's timer,
//!    with nothing dropped or defaulted on the way.
//! 2. **That the Plasma really is the weapon that saturates the cap.** The
//!    whole cap assertion below rests on it, and it is a property of the
//!    shipped tables rather than of the schema - so it is asserted as a
//!    relation between two numbers read off the disc, never as either number.
//! 3. **That a slowed craft is slower under the real force law.** The engine
//!    effect is an early return, not a multiplier: what actually costs speed is
//!    the drag and the throttle ramp that follow it, on a real circuit with
//!    real handling constants. On a flat fixture there is no track to be slowed
//!    *along*.
//!
//! What is deliberately **not** asserted is any authored value. Every assertion
//! is a relation - slower than, at or under, longer than a tick - or is against
//! a number this test measured itself in the same run.
//!
//! # The control run
//!
//! Speed is asserted against **the same race run twice from one `Setup`**, one
//! copy hit and one not, rather than against a threshold or against another
//! craft. The simulation is deterministic, so the two runs are bit-identical up
//! to the detonation and every difference afterwards is the blast - which is
//! the only comparison that is not swamped by where on the circuit the craft
//! happens to be.

use std::path::PathBuf;

use oag_core::TickRate;
use oag_game::race;
use oag_gameplay::input::{Button, Input};
use oag_gameplay::projectile::{BlastStats, blast};

/// Long enough that the craft is genuinely up to speed before anything is
/// measured, and past the start-line countdown. See `plasma_ground_truth.rs`,
/// which explains the `COUNTDOWN_TICKS` term at length.
const WARM_UP_TICKS: u64 = oag_race::COUNTDOWN_TICKS + 120;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// A single race: the one mode with weapons on, and so the only one whose
/// `<WeaponStats>` a craft can ever meet.
fn single_race() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// One button held down, tick after tick.
fn held(button: Button) -> oag_gameplay::InputSnapshot {
    let mut buttons = Input::new();
    buttons.begin_frame(button.bit());
    buttons.begin_frame(button.bit());
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// The Plasma's blast, straight off the player's disc.
fn plasma_blast(race: &race::Race) -> BlastStats {
    let plasma = race.plasma_stats().expect("the disc authors a Plasma");
    BlastStats {
        radius: plasma.blastradius,
        damage: plasma.damage,
        force: plasma.blastforce,
        slowdown_time: plasma.slowdown_time,
    }
}

/// Slot 0's forward speed.
fn speed(race: &race::Race) -> f32 {
    let body = &race.sim.world.ships[0].physics.body;
    body.linear_velocity.dot(body.forward())
}

/// Detonates one real Plasma blast **at the rim of slot 0's hull, behind it**.
///
/// Two deliberate choices, both about keeping the impulse out of the way of the
/// speed measurement:
///
/// - `blastforce` falls off as `1.0 - d / blastradius`, so a detonation at
///   `0.999` of the radius spends a thousandth of it. `slowdown_time` and
///   `damage` have **no** falloff, so both land in full - which is the shape
///   this test wants and is the recovered rule, not a convenience.
/// - What is left of the impulse points **forward**, along the craft's own
///   heading, because the blast is behind it. So the craft is nudged *faster*
///   by the hit, and a speed drop afterwards is a drop despite a shove rather
///   than because of one.
///
/// A real `projectile::step` detonation would put the blast wherever the bolt
/// stopped; nothing about the arithmetic differs, and aiming a bolt at a
/// specific craft on a real circuit is a flight test rather than a slowdown
/// one. `plasma_ground_truth.rs` covers the flight.
fn detonate_behind_the_player(race: &mut race::Race, stats: &BlastStats) {
    let body = &race.sim.world.ships[0].physics.body;
    let point = body.position - body.forward() * (stats.radius * 0.999);
    let count = race.sim.world.ship_count as usize;
    let rules = oag_gameplay::damage_rules(race.sim.world.race.mode);
    let reached = blast(
        &mut race.sim.world.ships[..count],
        point,
        stats,
        rules,
        &mut [],
    );
    assert!(reached > 0, "the blast reached nobody, including the craft");
}

/// Tops slot 0's energy back up, so the blast below cannot destroy the craft
/// and turn the whole measurement into one about a wreck.
fn refill_energy(race: &mut race::Race) {
    let max = race.sim.world.ships[0].handling.dimensions.shield;
    race.sim.world.ships[0].physics.shield = max;
}

/// A hit Plasma costs the craft its engine, and the craft gets it back.
///
/// The assertions, in order: the credit lands, the drain arms the timer on the
/// next tick and empties the slot, the engine produces **nothing at all** while
/// the timer runs, the craft is slower than its own untouched twin for it, and
/// once the timer expires it accelerates again.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_plasma_hit_costs_a_craft_its_engine_and_it_recovers() {
    let Some(loaded) = single_race() else { return };
    let throttle = held(Button::Cross);

    // One `Setup`, two races. Everything is identical until the detonation.
    let mut hit = race::Race::start(loaded.setup.clone());
    let mut control = race::Race::start(loaded.setup);
    for _ in 0..WARM_UP_TICKS {
        hit.tick(&throttle);
        control.tick(&throttle);
    }
    refill_energy(&mut hit);
    refill_energy(&mut control);

    let before = speed(&hit);
    assert!(before > 10.0, "the craft is barely moving at {before:.1}");
    assert_eq!(
        before,
        speed(&control),
        "the two runs diverged before the blast, so nothing below measures it"
    );

    let stats = plasma_blast(&hit);
    detonate_behind_the_player(&mut hit, &stats);
    assert!(
        hit.sim.world.ships[0].pending_slowdown > 0.0,
        "the blast credited no slowdown, so the disc's `slowdown_time` is not \
         reaching `projectile::blast`"
    );

    // The drain, on the next tick.
    let evaluated = hit.tick(&throttle);
    control.tick(&throttle);
    assert_eq!(
        hit.sim.world.ships[0].pending_slowdown, 0.0,
        "the pending slot was not drained"
    );
    let armed = hit.sim.world.ships[0].physics.slowdown_timer;
    assert!(armed > 0.0, "the timer was not armed: {armed}");
    assert_eq!(
        hit.sim.world.ships[0].physics.craft_state,
        oag_physics::CraftState::Racing,
        "the blast destroyed the craft, so what follows measures a wreck"
    );

    // **No thrust at all**, which is an early return rather than a multiplier.
    assert_eq!(
        evaluated.engine,
        oag_physics::engine::EngineForce::default(),
        "a slowed craft still produced thrust or lift"
    );
    assert_eq!(
        evaluated.lateral_grip,
        oag_core::math::Vec3::ZERO,
        "a slowed craft still had lateral grip"
    );

    // Run the window out. The timer counts down by `dt` a tick and lands
    // *negative* rather than at zero, which is the original's own shape - so
    // the exit test is `> 0.0` and the assertion afterwards is `<= 0.0`.
    let mut ticks_slowed = 1;
    while hit.sim.world.ships[0].physics.slowdown_timer > 0.0 {
        hit.tick(&throttle);
        control.tick(&throttle);
        ticks_slowed += 1;
        assert!(ticks_slowed < 600, "the timer never expired");
    }
    assert!(
        hit.sim.world.ships[0].physics.slowdown_timer <= 0.0,
        "the loop exited with the timer still running"
    );
    assert!(
        ticks_slowed > 1,
        "the whole slowdown lasted one tick, which no authored figure should"
    );

    let slowed = speed(&hit);
    let untouched = speed(&control);
    println!(
        "slowed for {ticks_slowed} ticks: {slowed:.2} against the control's \
         {untouched:.2} units/s, from {before:.2}"
    );
    assert!(
        slowed < untouched,
        "the hit craft is doing {slowed:.2} units/s and its untouched twin \
         {untouched:.2} - the slowdown cost it nothing, and it was nudged \
         *forward* by what was left of the blast impulse"
    );

    // And it recovers: with the timer gone the engine is back, so the craft
    // accelerates out of the hole. Its own speed over its own next second,
    // rather than the control's - the two are on different parts of the
    // circuit by now and no longer comparable.
    let recovering_from = speed(&hit);
    for _ in 0..60 {
        hit.tick(&throttle);
    }
    let recovered = speed(&hit);
    println!("recovered to {recovered:.2} units/s from {recovering_from:.2}");
    assert!(
        recovered > recovering_from,
        "the craft is not accelerating again after the timer expired: \
         {recovered:.2} against {recovering_from:.2} units/s"
    );
}

/// A second Plasma inside the window does not extend the timer past the cap.
///
/// `slowdown_limit` is a ceiling on **seconds of slowdown outstanding**, and
/// the mistake this test exists to catch is reading it as anything else - a
/// per-hit cap, a speed floor, or a duration counted from the first hit. Under
/// the wrong reading, two hits give roughly twice the timer; under the right
/// one they give the ceiling both times.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_second_plasma_inside_the_window_does_not_extend_the_timer() {
    let Some(loaded) = single_race() else { return };
    let throttle = held(Button::Cross);
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..WARM_UP_TICKS {
        race.tick(&throttle);
    }
    refill_energy(&mut race);

    let limit = race
        .slowdown_limit()
        .expect("the disc authors a <Global slowdown_limit>");
    let stats = plasma_blast(&race);
    assert!(limit > 0.0, "the authored ceiling is zero: {limit}");
    assert!(
        stats.slowdown_time > 0.0,
        "the Plasma authors no slowdown_time"
    );
    // **The Plasma is the weapon that saturates the cap**, which is what makes
    // it the right one to test the ceiling with. Asserted as a relation between
    // two figures off the disc, so neither is written down here.
    assert!(
        stats.slowdown_time >= limit,
        "the Plasma no longer saturates the cap, so this test is measuring a \
         weapon that never reaches the ceiling and the rest of it proves \
         nothing"
    );

    let dt = TickRate::DEFAULT.dt();

    detonate_behind_the_player(&mut race, &stats);
    race.tick(&throttle);
    let after_one = race.sim.world.ships[0].physics.slowdown_timer;
    assert!(
        after_one <= limit,
        "one hit put {after_one} on the timer, past the authored ceiling"
    );
    assert!(
        after_one > limit - 2.0 * dt,
        "one Plasma left {after_one} against a ceiling of {limit} - it should \
         have saturated it, less this tick's own decay"
    );

    // A second hit while the first is still running.
    refill_energy(&mut race);
    detonate_behind_the_player(&mut race, &stats);
    race.tick(&throttle);
    let after_two = race.sim.world.ships[0].physics.slowdown_timer;
    println!("one hit left {after_one}, two left {after_two}, ceiling {limit}");
    assert!(
        after_two <= limit,
        "two hits put {after_two} on the timer against a ceiling of {limit} - \
         the clamp in `oag_physics::slowdown::add` is not being applied, or \
         the limit is being read as something other than seconds outstanding"
    );
    // The tell-tale of the wrong reading, stated separately: stacking would put
    // this near twice the first figure rather than back at the same one.
    assert!(
        after_two < after_one * 2.0 - dt,
        "the second hit stacked onto the first: {after_two} against \
         {after_one}"
    );
}

/// Every weapon the disc authors sits at or under the ceiling, and the mechanic
/// is reachable from more than the one weapon this file fires.
///
/// A shape assertion over the whole table rather than over the Plasma alone: a
/// weapon authoring more than the cap would be silently truncated by
/// `oag_physics::slowdown::add` and would never show up in play.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_decoded_weapon_authors_a_slowdown_inside_the_ceiling() {
    let Some(loaded) = single_race() else { return };
    let race = race::Race::start(loaded.setup);
    let limit = race
        .slowdown_limit()
        .expect("the disc authors a <Global slowdown_limit>");

    let authored: Vec<(&str, f32)> = [
        ("Rocket", race.rocket_stats().map(|s| s.slowdown_time)),
        ("Missile", race.missile_stats().map(|s| s.slowdown_time)),
        ("Plasma", race.plasma_stats().map(|s| s.slowdown_time)),
        ("Mine", race.mine_stats().map(|s| s.slowdown_time)),
        ("Bomb", race.bomb_stats().map(|s| s.slowdown_time)),
        ("Shuriken", race.shuriken_stats().map(|s| s.slowdown_time)),
    ]
    .into_iter()
    .filter_map(|(name, value)| value.map(|v| (name, v)))
    .collect();

    assert!(
        authored.len() >= 4,
        "only {} weapon blocks decoded, so the table did not load properly",
        authored.len()
    );
    for (name, seconds) in &authored {
        assert!(*seconds > 0.0, "{name} authors no slowdown at all");
        assert!(
            *seconds <= limit,
            "{name} authors more slowdown than the global ceiling, which \
             `slowdown::add` would silently truncate"
        );
    }
}
