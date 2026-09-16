//! The Plasma on a real circuit out of a real disc image.
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
//! `mine_ground_truth.rs` and `missile_ground_truth.rs` both give: that file is
//! at its `BASELINE` ceiling in `scripts/check-file-size.py`, so a line added
//! to it fails `just check-size`. The harness helpers below are the same
//! deliberate near-duplicates.
//!
//! # What only real data can say here
//!
//! 1. **That the disc's own Plasma block decodes at all.** The fixture tables
//!    in `crates/formats/src/weapons/tests.rs` are hand-written from the
//!    schema, so they prove the parser reads *a* document of that shape and
//!    nothing about the shipped one. The offsets are measured off
//!    `WeaponStats_ParsePlasma`; whether this build's attribute *names* match
//!    what Pulse actually authors is a different question, and only the disc
//!    answers it.
//! 2. **That the bolt survives the real collision soup.** A plasma flies the
//!    Rocket's floor-following path - a 12-unit probe along the surface normal
//!    it carries, redirected onto whatever it finds. On a hand-built fixture
//!    there is no floor, so the probe misses on every tick and the whole
//!    ride-the-track half of the model is never exercised. A circuit is the
//!    only thing that runs it.
//! 3. **That one press really is one bolt through the whole path.** The
//!    headless test pins `spend_pickup`; this pins it end to end, against a
//!    table loaded off the disc rather than a fixture, on a craft under the
//!    real force law.
//!
//! What is deliberately **not** asserted is any authored *value*. Reproducing
//! one would put shipped tuning data in the repository, which ADR-0006 does not
//! allow - so the assertions below are all relative (further than, still
//! inside, more than one) or against numbers the test itself measured.

use std::path::PathBuf;

use oag_core::math::Vec3;
use oag_game::race;
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_tables::weapons::Weapon;

/// Long enough that the craft is genuinely up to speed before a test measures
/// anything.
///
/// **Past the start-line countdown, not merely "a while".**
/// `oag_race::COUNTDOWN_TICKS` is the measured 272 ticks `RaceState` gates the
/// player's thrust through - see
/// `docs/gameplay/race-modes.md#the-countdown-is-measured`. A warm-up shorter
/// than it leaves the craft stationary, and every assertion about a *moving*
/// craft then fails for a reason that has nothing to do with the weapon. These
/// files read a flat `120` until 2026-09-02 and went red the day the countdown
/// landed.
const WARM_UP_TICKS: u64 = oag_race::COUNTDOWN_TICKS + 120;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// A single race: the one mode with weapons on, and so the only one a pickup
/// can happen in at all.
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

/// One button held down, tick after tick. See `mine_ground_truth.rs`'s twin for
/// why a fresh `Input` per call is right.
fn held(button: Button) -> oag_gameplay::InputSnapshot {
    let mut buttons = Input::new();
    buttons.begin_frame(button.bit());
    buttons.begin_frame(button.bit());
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// Every plasma bolt slot 0 has in the air.
fn bolts(race: &race::Race) -> Vec<oag_gameplay::projectile::Projectile> {
    race.sim
        .world
        .projectiles
        .slots
        .iter()
        .filter(|p| p.kind == Some(Weapon::Plasma) && p.owner == 0)
        .copied()
        .collect()
}

/// A race with the craft already up to speed, and full throttle to keep it
/// there.
fn moving() -> Option<(race::Race, oag_gameplay::InputSnapshot)> {
    let loaded = single_race()?;
    let mut race = race::Race::start(loaded.setup);
    let throttle = held(Button::Cross);
    for _ in 0..WARM_UP_TICKS {
        race.tick(&PlayerInputs::single(throttle));
    }
    Some((race, throttle))
}

/// The disc's own `<Plasma>` block decodes, and it decodes as a projectile
/// rather than as a defaulted husk.
///
/// **Shape, not values**: every assertion is "authored above zero" or "ordered
/// the way the schema says", which is what ADR-0006 leaves room for. A block
/// that failed to decode comes back `None`; one that decoded off the wrong
/// attribute names comes back all zeroes, and that is what the positives here
/// catch.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_discs_plasma_block_decodes_as_a_projectile() {
    let Some(loaded) = single_race() else { return };
    let race = race::Race::start(loaded.setup);
    let plasma = race.plasma_stats().expect("the disc authors a Plasma");

    assert!(plasma.damage > 0.0, "no authored damage: {plasma:?}");
    assert!(plasma.blastradius > 0.0, "no authored blastradius");
    assert!(plasma.blastforce > 0.0, "no authored blastforce");
    assert!(plasma.absorb > 0.0, "no authored absorb");

    // Four separately authored class speeds, ascending - which is the file's own
    // design rather than a scaling this engine applies, and is what says the
    // four offsets were read in the right order rather than all landing on one.
    let speeds: Vec<f32> = oag_tables::handling::SpeedClass::ALL
        .into_iter()
        .map(|class| plasma.speed_for(class))
        .collect();
    println!(
        "plasma class speeds: {speeds:?}, launch {}",
        plasma.launch_speed
    );
    assert!(speeds[0] > 0.0, "Venom's plasma speed is zero: {speeds:?}");
    for pair in speeds.windows(2) {
        assert!(
            pair[1] >= pair[0],
            "the class speeds are not ascending: {speeds:?} - the four offsets \
             are probably being read in the wrong order"
        );
    }

    // Harder than a rocket and slower to arrive is the shipped relationship on
    // both Pulse tables; asserting the *direction* keeps the number off the page.
    let rocket = race.rocket_stats().expect("the disc authors a Rocket");
    assert!(
        plasma.damage > rocket.damage,
        "the plasma is not the heavier hit, which is what says these are two \
         different blocks rather than one read twice"
    );
}

/// One press puts exactly **one** bolt in the air, ahead of the craft, and it
/// flies the real circuit rather than falling through it.
///
/// **One is recovered, not chosen**: `Weapon_FirePlasma` (`0x0886a868`) takes
/// one pool slot and clears its own request bit in the same breath, and the
/// `<Stats>` author no `spread`. See
/// `docs/ghidra/functions/psp-pulse-usa/plasma.md`.
///
/// The flight half is what needs the disc. `Plasma_Update`'s floor follower
/// probes 12 units along the surface normal the bolt carries and redirects onto
/// whatever it finds; with no track under it the probe misses every tick and the
/// bolt simply falls, which passes a "did it move" assertion for entirely the
/// wrong reason.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_plasma_fired_on_a_real_track_is_one_bolt_and_it_flies() {
    let Some((mut race, throttle)) = moving() else {
        return;
    };
    let plasma = race.plasma_stats().expect("the disc authors a Plasma");
    let speed = race.sim.world.ships[0]
        .physics
        .body
        .linear_velocity
        .length();
    assert!(
        speed > 10.0,
        "the craft is barely moving at {speed:.1} units/s"
    );

    let origin = race.sim.world.ships[0].physics.body.position;
    let forward = race.sim.world.ships[0].physics.body.forward();
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Plasma);

    let mut buttons = Input::new();
    buttons.begin_frame(0);
    let mut fire = oag_gameplay::InputSnapshot::new();
    fire.buttons = buttons;
    fire.buttons
        .begin_frame(Button::Square.bit() | Button::Cross.bit());
    race.tick(&PlayerInputs::single(fire));

    let launched = bolts(&race);
    assert_eq!(
        launched.len(),
        1,
        "one press put {} bolts in the air, not one",
        launched.len()
    );
    assert_eq!(
        race.ship_pickup(),
        None,
        "firing must spend the pickup on the tick the bolt leaves"
    );

    let bolt = launched[0];
    assert!(
        (bolt.position - origin).dot(forward) > 0.0,
        "the bolt spawned behind the craft"
    );
    // The class speed plus `launchSpeed`, both km/h in the file. Asserted as
    // "faster than the craft" rather than against the authored figure, which
    // ADR-0006 keeps off the page - and it is the assertion that would catch the
    // units bug `rocket-visuals.md` records, in the other direction: a bolt
    // spending km/h as units per second would come out 3.6x too fast.
    let bolt_speed = bolt.velocity.length();
    assert!(
        bolt_speed > speed,
        "the bolt is slower than the craft that fired it: {bolt_speed:.1} against \
         {speed:.1} units/s"
    );
    let authored =
        (plasma.speed_for(oag_tables::handling::SpeedClass::Venom) + plasma.launch_speed) / 3.6;
    assert!(
        (bolt_speed - authored).abs() < 1.0,
        "the bolt is doing {bolt_speed:.1} units/s where the disc's own Venom \
         figure converted from km/h is {authored:.1} - a factor of 3.6 out means \
         the conversion was dropped"
    );

    // And it rides the track. Drive it out and watch it either travel or detonate
    // on the circuit's own geometry; both are correct, and neither is "it fell
    // through the world", which is what a broken surface probe looks like.
    let start = bolt.position;
    let mut furthest: f32 = 0.0;
    let mut detonated_after = None;
    for tick in 1..=60 {
        race.tick(&PlayerInputs::single(throttle));
        match bolts(&race).first() {
            Some(live) => furthest = furthest.max((live.position - start).length()),
            None => {
                detonated_after = Some(tick);
                break;
            }
        }
    }
    println!(
        "the bolt covered {furthest:.1} units before {}",
        detonated_after.map_or_else(
            || "the run ended with it still flying".to_string(),
            |t| format!("detonating on tick {t}")
        )
    );
    assert!(
        furthest > speed / 60.0,
        "the bolt moved {furthest:.1} units, less than the craft covers in one \
         tick - it is not flying"
    );

    // Whatever happened to it, the blast numbers behind it are the Plasma's own
    // and are still reachable. A weapon that reaches an impact with no authored
    // numbers is a bug in `pickup::IMPLEMENTED`, per `projectile::blast_stats`.
    assert!(
        race.plasma_stats().is_some(),
        "the table went away mid-race"
    );
    assert!(
        plasma.blastradius > 0.0,
        "a detonating plasma would spend a zero radius and reach nobody"
    );
}

/// A plasma bolt that never hits anything detonates at ten seconds, and the
/// detonation hurts nobody standing just inside its own blast radius - even on
/// a real disc's own `<Plasma>` numbers and a real circuit's own damage law.
///
/// **Why "never hits anything" is engineered rather than found.** A bolt fired
/// straight down a real circuit hits a wall in about a second and a half -
/// measured directly on `Talons Junction` while writing this test, well
/// inside the ten-second window this test exists to reach. So this fires
/// straight up from 10,000 units above the track instead: nothing in any
/// shipped circuit's collision geometry reaches that high, so
/// `Plasma_Update`'s probe finds no floor and no wall on any tick, and the
/// only way the flight can end is the lifetime hitting zero - the ending this
/// test is actually about. `oag_gameplay::projectile::FALL_ACCELERATION`
/// pulls the bolt down at most `0.5 * 50.0 * 10.0^2 = 2500` units over the
/// full flight, so even a bolt launched with no upward velocity at all stays
/// thousands of units clear of the ground the whole time.
///
/// **The "nearby craft" is a real, active opponent, teleported alongside the
/// bolt's own position every tick before `Race::tick` runs** - at
/// `0.75 * blastradius` off to one side, inside the radius a real blast would
/// reach and outside a hull's own collision footprint, so the sentinel is a
/// blast candidate rather than a direct hit the moment it is placed.
/// `Race::tick` applies a tick's blast before `step_opponents` moves anyone
/// (`crates/game/src/race/tick.rs`), so the craft is exactly where the bolt is
/// at the moment any blast would land, on every single tick of the flight -
/// nothing here depends on predicting where a real circuit's geometry puts
/// the bolt, because the bolt never reaches any. Slot 0's own shield is not
/// asserted here: it spends the same ten seconds actually racing a real
/// circuit against real opponents, where a wall scrape or a rival's own
/// weapon can cost it shield with nothing to do with this bolt.
///
/// Confidence on the reading this asserts: **`plasma.md`**'s "the teardown
/// spends no blast on either ending" - the corresponding unit tests in
/// `oag_gameplay::projectile::plasma::tests` cover the same claim on a hand
/// fixture; this is the same claim against the shipped table and a real
/// track's own damage law.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_plasma_bolt_that_times_out_far_above_the_track_hurts_nobody_below_it() {
    let Some((mut race, throttle)) = moving() else {
        return;
    };
    let plasma = race.plasma_stats().expect("the disc authors a Plasma");
    assert!(
        plasma.blastradius > 0.0,
        "a zero radius reaches nobody anyway"
    );
    assert!(plasma.damage > 0.0, "a zero damage credits nothing anyway");

    assert!(
        race.sim.world.ship_count > 1,
        "a Single Race needs at least one opponent to stand nearby"
    );
    assert!(
        race.sim.world.ships[1].active,
        "slot 1 is not an active opponent"
    );
    let sentinel_shield_before = race.sim.world.ships[1].physics.shield;

    let above = race.sim.world.ships[0].physics.body.position + Vec3::Y * 10_000.0;
    assert!(
        race.sim
            .world
            .projectiles
            .charge_up(above, Vec3::ZERO, 0, 0.0),
        "no free slot to fire into"
    );
    assert_eq!(
        bolts(&race).len(),
        1,
        "the direct spawn did not land a Plasma in the pool"
    );

    // Just inside `blastradius` and well clear of a hull's own collision
    // footprint - close enough that the blast would reach it, and far enough
    // that the sentinel does not itself register as a direct hit the moment
    // it is placed there. A hull hit and a timeout are different endings
    // (`Impact::struck` against `None`) and this test is about the second.
    let nearby_offset = Vec3::X * (plasma.blastradius * 0.75);

    let mut ticks = 0_usize;
    let mut ended = false;
    while !ended {
        // Before the tick, so the blast this tick's `projectile::step` may
        // spend sees slot 1 sitting next to the bolt.
        if let Some(bolt) = bolts(&race).first() {
            race.sim.world.ships[1].physics.body.position = bolt.position + nearby_offset;
        }
        race.tick(&PlayerInputs::single(throttle));
        ticks += 1;
        if bolts(&race).is_empty() {
            ended = true;
        }
        assert!(ticks < 700, "the bolt never ended");
    }

    let flown = ticks as f32 / 60.0;
    println!("the bolt timed out after {ticks} ticks ({flown:.2}s)");
    assert!(
        (flown - 10.0).abs() <= 2.0 / 60.0,
        "it ended at {flown}s, not the recovered 10.0s - it may have found a \
         floor or wall this test was meant to put out of reach"
    );

    // Slot 1 alone, not the whole grid. Slot 0 spends its own ten seconds
    // actually racing a real circuit against real opponents - a wall scrape
    // or a rival's own weapon can cost it shield in that window with nothing
    // to do with the Plasma, and this test would then be asserting the wrong
    // thing. Slot 1 never touches the track for the whole flight - it is
    // teleported next to the bolt and nowhere else - so nothing but this
    // bolt's own blast can move its shield.
    assert_eq!(
        race.sim.world.ships[1].physics.shield, sentinel_shield_before,
        "the sentinel's shield moved from {} to {} - the timeout spent a blast \
         the original's teardown never reaches",
        sentinel_shield_before, race.sim.world.ships[1].physics.shield
    );
}
