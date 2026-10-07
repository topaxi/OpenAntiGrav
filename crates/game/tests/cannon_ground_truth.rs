//! The Cannon on a real circuit out of a real disc image.
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
//! Its own file rather than a section of `race_ground_truth.rs`, for the
//! reason `mine_ground_truth.rs` and `missile_ground_truth.rs` both give: that
//! file is at its `BASELINE` ceiling in `scripts/check-file-size.py`.
//!
//! # What only real data can say here
//!
//! 1. **That the disc's own Cannon block decodes at all.** `absorb`, `rounds`,
//!    `rate` and `damage_per_bullet` are the four this weapon authors that no
//!    earlier weapon does under those exact names.
//! 2. **That rounds leave while `SQUARE` is held, and none leaves while it is
//!    not.** This is the weapon's whole distinguishing shape - it is driven by
//!    the *held* half of the fire button rather than the press edge every
//!    other weapon consumes, see
//!    `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md` - and a
//!    hand-built fixture cannot exercise `Race::advance_cannons` running
//!    against a real per-craft reload countdown the way a loaded race does.
//!
//!    **This assertion used to be its own negation**, and that is worth
//!    recording rather than quietly fixing: until 2026-09-07 this file
//!    asserted that a round leaves with `SQUARE` never pressed, because the
//!    countdown's gate had been read as a track weapon-pad flag. A player
//!    reported the weapon as unfireable, the gate turned out to be the fire
//!    button, and the test had been holding the bug in place.
//! 3. **That the round survives the real collision soup**, flying straight and
//!    reflecting off a floor as `Cannon_UpdateRound` does (it reads no surface
//!    normal and rides nothing).
//!
//! What is deliberately **not** asserted is any authored *value*, per
//! ADR-0006 - every assertion here is relative or against a number the test
//! itself measured.

use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;
use oag_tables::weapons::Weapon;

/// Past the start-line countdown, not merely "a while" - see
/// `plasma_ground_truth.rs`'s twin for why a flat tick count is the wrong
/// warm-up.
const WARM_UP_TICKS: u64 = oag_race::COUNTDOWN_TICKS + 120;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

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

/// One button held down, tick after tick. See `mine_ground_truth.rs`'s twin
/// for why a fresh `Input` per call is right.
fn held(button: Button) -> oag_gameplay::InputSnapshot {
    let mut buttons = Input::new();
    buttons.begin_frame(button.bit());
    buttons.begin_frame(button.bit());
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// Every Cannon round slot 0 has fired and still has in the air.
fn rounds(race: &race::Race) -> Vec<oag_weapons::projectile::Projectile> {
    race.sim
        .world
        .projectiles
        .slots
        .iter()
        .filter(|p| p.kind == Some(Weapon::Cannon) && p.owner == 0)
        .copied()
        .collect()
}

fn moving() -> Option<(race::Race, oag_gameplay::InputSnapshot)> {
    let loaded = single_race()?;
    let mut race = race::Race::start(loaded.setup);
    let throttle = held(Button::Cross);
    for _ in 0..WARM_UP_TICKS {
        race.tick(&PlayerInputs::single(throttle));
    }
    Some((race, throttle))
}

/// The disc's own `<Cannon>` block decodes, and it decodes as this weapon's
/// own shape rather than as a defaulted husk or another weapon's block read
/// twice.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_discs_cannon_block_decodes() {
    let Some(loaded) = single_race() else { return };
    let race = race::Race::start(loaded.setup);
    let cannon = race.cannon_stats().expect("the disc authors a Cannon");

    assert!(cannon.absorb > 0.0, "no authored absorb: {cannon:?}");
    assert!(cannon.rounds > 0.0, "no authored rounds: {cannon:?}");
    assert!(cannon.rate > 0.0, "no authored rate: {cannon:?}");
    assert!(
        cannon.damage_per_bullet > 0.0,
        "no authored damage_per_bullet: {cannon:?}"
    );

    // The schema's own shape: no speed of any kind, unlike every other
    // projectile weapon here - see `oag_tables::weapons::CannonStats`'s own
    // doc comment. Nothing to assert positively; recorded so a future reader
    // of this file does not go looking for a `speed_for` this weapon has no
    // reason to carry.
    println!("cannon: {cannon:?}");
}

/// A picked-up Cannon fires while `SQUARE` is held and stays silent while it
/// is not - the whole point of this weapon, per its own evidence page.
///
/// **The silent half runs first, and it is the half that used to be
/// inverted.** `Race::spend_pickup`'s Cannon arm is still a recovered
/// non-event, so a press must not spend the pickup; but the *held* button is
/// what advances the countdown, so a long stretch with fire up must produce
/// nothing at all.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_held_fire_button_is_what_fires_a_cannon() {
    let Some((mut race, throttle)) = moving() else {
        return;
    };
    let cannon = race.cannon_stats().expect("the disc authors a Cannon");
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Cannon);

    // Long enough that a self-firing countdown would have gone off many times
    // over, with fire never held. Nothing may leave the barrel. A flat
    // 600 ticks rather than a multiple of `rate`, because `rate` is now the
    // reciprocal the disc authors - about a sixtieth of a second - and three
    // of those is not a window anything could be caught in.
    let quiet = 600u64;
    for _ in 0..quiet {
        race.tick(&PlayerInputs::single(throttle));
        assert!(
            rounds(&race).is_empty(),
            "a Cannon round left with SQUARE never held - the countdown is \
             running unpressed, which is the 2026-09-07 bug"
        );
    }
    assert_eq!(
        race.ship_pickup(),
        Some(Weapon::Cannon),
        "an unfired Cannon must still be in the slot after {quiet} ticks"
    );

    // Now hold it. `Cross` rides along so the craft keeps accelerating while
    // fire is down, the same shape `plasma_ground_truth.rs`'s own combined
    // press takes.
    let mut fire = oag_gameplay::InputSnapshot::new();
    let mut buttons = Input::new();
    let mask = Button::Square.bit() | Button::Cross.bit();
    buttons.begin_frame(mask);
    buttons.begin_frame(mask);
    fire.buttons = buttons;

    let ticks = (cannon.rate * 60.0).ceil() as u64 + 120;
    println!(
        "cannon rate {:.4}s per round, magazine {}",
        cannon.rate, cannon.rounds
    );
    let mut ever_fired = false;
    for _ in 0..ticks {
        race.tick(&PlayerInputs::single(fire));
        if !rounds(&race).is_empty() {
            ever_fired = true;
            break;
        }
    }
    assert!(
        ever_fired,
        "no Cannon round appeared in {ticks} ticks with SQUARE held down - \
         the reload countdown is not advancing on a held button"
    );

    let round = rounds(&race)[0];
    let speed = race.sim.world.ships[0]
        .physics
        .body
        .linear_velocity
        .length();
    assert!(
        round.velocity.length() > speed,
        "the round is slower than the craft that fired it: {:.1} against \
         {speed:.1} units/s - it should carry the craft's own speed plus a \
         base, never less than the craft alone",
        round.velocity.length()
    );

    // And it flies on through real geometry: straight, reflecting off a floor,
    // ending on a wall.
    let start = round.position;
    let mut furthest: f32 = 0.0;
    let mut detonated_after = None;
    for tick in 1..=120 {
        race.tick(&PlayerInputs::single(throttle));
        match rounds(&race).first() {
            Some(live) => furthest = furthest.max((live.position - start).length()),
            None => {
                detonated_after = Some(tick);
                break;
            }
        }
    }
    println!(
        "the round covered {furthest:.1} units before {}",
        detonated_after.map_or_else(
            || "the run ended with it still flying".to_string(),
            |t| format!("detonating/expiring on tick {t}")
        )
    );
}

/// An opponent holding a Cannon fires it, with no button pressed anywhere -
/// and only while it has somebody in front to fire at.
///
/// **The mechanism is measured; what the gate reads is not.**
/// `Cannon_UpdateReload` gates on the fire-held byte of the firing craft's
/// control record, which for an opponent is `Ai + 0x1e`. Nothing in the image
/// writes that byte at any width, and `Ai_Construct`'s object is allocated
/// without a zero-fill, so on the real hardware it is whatever the heap block
/// held. There is no design there to be faithful to, and opponent behaviour is
/// a design axis on this project rather than a fidelity one - so the trigger is
/// **chosen, not measured, with no confidence score**: an opponent holds fire
/// while `oag_ai::Driver::holds_fire` says there is a target ahead. See
/// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
///
/// **What this pins is what a player should be able to see**: rounds leaving an
/// opponent's barrel with the pad untouched, aimed up the road rather than
/// sprayed at nothing. Every opponent is armed rather than just slot 1, because
/// which craft happens to have a rival in its cone at any moment is a property
/// of the race, not of the weapon - arming one slot and hoping would be a
/// flaky test of a real behaviour.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_opponent_holding_a_cannon_fires_it_with_no_button_anywhere() {
    let Some((mut race, throttle)) = moving() else {
        return;
    };
    let count = usize::from(race.sim.world.ship_count);
    assert!(count > 1, "a single race should grid more than one craft");
    let cannon = race.cannon_stats().expect("the disc authors a Cannon");
    for slot in 1..count {
        race.sim.world.ships[slot].pickup.weapon = Some(Weapon::Cannon);
    }

    // `throttle` holds CROSS and never SQUARE, so nothing the player does can
    // account for a round appearing. Long enough for the field to string out
    // and put somebody in somebody else's cone; the magazine itself is a second
    // and a half.
    let ticks = (cannon.rate * 60.0).ceil() as u64 + 900;
    let mut shooters: Vec<u8> = Vec::new();
    for _ in 0..ticks {
        race.tick(&PlayerInputs::single(throttle));
        for round in race
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .filter(|p| p.kind == Some(Weapon::Cannon) && p.owner != 0)
        {
            if !shooters.contains(&round.owner) {
                shooters.push(round.owner);
            }
        }
        if !shooters.is_empty() {
            break;
        }
    }
    assert!(
        !shooters.is_empty(),
        "no opponent Cannon round appeared in {ticks} ticks with every opponent \
         armed - either `Race::advance_cannons` is not running the other slots, \
         or `Driver::holds_fire` never finds a target on this circuit"
    );
    println!("opponent slots that fired: {shooters:?}");

    // And the player, who is holding no fire button, fired nothing.
    assert!(
        rounds(&race).is_empty(),
        "slot 0 fired a Cannon with SQUARE never held"
    );
}

/// An opponent with nobody in front of it does **not** fire.
///
/// The other half of the tuned trigger, and the half that would otherwise pass
/// silently: a gate that always returns `true` satisfies the test above just as
/// well as the real one. The leader has no rival ahead by construction, so its
/// magazine must still be full after a long stretch.
///
/// Chosen, not measured - see the test above.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_opponent_with_nobody_ahead_does_not_fire_its_cannon() {
    let Some((mut race, throttle)) = moving() else {
        return;
    };
    let count = usize::from(race.sim.world.ship_count);
    // Whoever is out front right now: `Field::ahead` is empty for it, so
    // `Driver::holds_fire` has nothing to aim at.
    let places = race.places();
    let leader = (1..count)
        .min_by_key(|&slot| places[slot])
        .expect("a grid of more than one");
    race.sim.world.ships[leader].pickup.weapon = Some(Weapon::Cannon);

    for _ in 0..300 {
        race.tick(&PlayerInputs::single(throttle));
        // Stop as soon as it is no longer leading - the premise has gone and
        // the rest of the run would be measuring something else.
        if race.places()[leader] != 1 {
            break;
        }
        assert!(
            race.sim
                .world
                .projectiles
                .slots
                .iter()
                .all(|p| p.kind != Some(Weapon::Cannon) || p.owner != leader as u8),
            "the craft in front fired a Cannon at nobody - `Driver::holds_fire` \
             is not gating on a target"
        );
    }
}

/// The disc carries the round's own model, and it decodes to real geometry.
///
/// **The acceptance test for objective 2's asset half.** `Cannon_Construct`
/// (`0x088651d8`) loads `CANNON_MODEL_ENTRY` into every round instance's scene
/// node; this asserts that entry resolves in `Data.wad` and parses to
/// something with vertices, so `Race::cannon_model_matrices` has a mesh to
/// drive. Without it the rounds fall back to nothing at all - which is the
/// deliberate choice, but it must be *visible* which of the two happened.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_discs_cannon_round_carries_its_own_model() {
    let Some(loaded) = single_race() else { return };
    let model = loaded
        .cannon_model
        .expect("the disc carries Data\\Weapons\\pulse_muzzleflash.vex");

    assert!(
        !model.vertices.is_empty() && !model.indices.is_empty(),
        "the round's model decoded to no geometry: {} vertices, {} indices",
        model.vertices.len(),
        model.indices.len()
    );

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for vertex in &model.vertices {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex.position[axis]);
            max[axis] = max[axis].max(vertex.position[axis]);
        }
    }
    let span = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    println!(
        "pulse_muzzleflash.vex: {} vertices, {} indices, spans x {:.3}, y {:.3}, z {:.3}",
        model.vertices.len(),
        model.indices.len(),
        span[0],
        span[1],
        span[2]
    );

    // The same assertion `the_rocket_model_is_longest_along_the_axis_it_is_flown_down`
    // makes, and for the same reason: `projectile_model_matrices` builds
    // `Mat4::from_cols(side, up, forward, position)`, mapping the model's own
    // **+Z** onto the direction of travel. Nothing in a unit test can check
    // that +Z is where this mesh actually points; only the real file can.
    //
    // It is also the evidence that this mesh is the *bolt* despite being named
    // `pulse_muzzleflash`: a 3.6-long, 1.2-square dart is a round in flight, not
    // a flat flash at a barrel.
    let longest = (0..3).max_by(|a, b| span[*a].total_cmp(&span[*b])).unwrap();
    assert_eq!(
        longest,
        2,
        "the round is longest along {} but `cannon_model_matrices` aims +Z down \
         the velocity, so it would be drawn broadside; spans are {span:?}",
        ["x", "y", "z"][longest]
    );
}
