//! The two rear weapons - the Mine and the Bomb - on a real circuit out of a
//! real disc image.
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
//! `missile_ground_truth.rs` gives at length: that file is at its `BASELINE`
//! ceiling in `scripts/check-file-size.py`, so a line added to it fails
//! `just check-size`. The harness helpers below are the same deliberate
//! near-duplicates.
//!
//! # What only real data can say here
//!
//! 1. **That a drop actually lays a trail rather than a heap.** The spread comes
//!    entirely from the craft *moving* between drops - nothing scatters a mine
//!    sideways - so on a static fixture five mines land on top of each other and
//!    every assertion about a cluster passes vacuously. It needs a craft under
//!    the real force law at a real speed.
//! 2. **That the mines are laid at the craft's own position.** Measured on the
//!    original (`mine.md`, 2026-10-01); a drop point still pushed back by the
//!    hull, or out of the nose, is silent - mines still detonate on somebody.
//! 3. **That the authored fuse outlives the drop.** `timetodie` is seven seconds
//!    and the cluster takes half of one; a fuse read from the wrong offset would
//!    very likely be some other weapon's number, and the ones adjacent to it in
//!    the struct are radii and forces rather than seconds. A cluster whose first
//!    mine has expired before its last is laid is what that looks like.

use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;
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

/// A single race: the one mode with weapons on, and so the only one a pickup can
/// happen in at all.
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
///
/// A fresh `Input` each call rather than one carried through the run: the
/// throttle is read with `is_held` rather than as an edge, so a snapshot built
/// this way is indistinguishable from one whose button has been down for a
/// minute - and a shared one would need threading through every helper here.
fn held(button: Button) -> oag_gameplay::InputSnapshot {
    let mut buttons = Input::new();
    buttons.begin_frame(button.bit());
    buttons.begin_frame(button.bit());
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// Every charge of one kind slot 0 has laid, in the order the array holds them.
fn laid_of(race: &race::Race, kind: Weapon) -> Vec<oag_weapons::projectile::Projectile> {
    race.sim
        .world
        .projectiles
        .slots
        .iter()
        .filter(|p| p.kind == Some(kind) && p.owner == 0)
        .copied()
        .collect()
}

/// Every mine slot 0 has laid, in the order the array holds them.
fn laid(race: &race::Race) -> Vec<oag_weapons::projectile::Projectile> {
    laid_of(race, Weapon::Mine)
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

/// One press lays the whole cluster, spread along the track, and the pickup
/// survives until the last one is out.
///
/// **Three claims in one run, because they are three views of one mechanism**
/// and separating them would mean driving the same thirty ticks three times:
///
/// - The count matches `mine::CLUSTER`. That number was invented when this
///   test was written and was measured at five on the running original on
///   2026-09-15 (`docs/ghidra/functions/psp-pulse-usa/mine.md`), so what is
///   being pinned is that the machinery lays *all* of it - a reload that reset
///   to the wrong value, or a counter decremented twice, shows up here.
/// - They are spread out rather than stacked. The gap asserted against is the
///   craft's own travel, not a chosen distance.
/// - The craft keeps the pickup until the last one leaves, which is the
///   recovered coupling in `Weapon_DropMines`'s zero branch.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn one_press_lays_a_cluster_spread_along_the_track() {
    let Some(loaded) = single_race() else { return };
    let mut race = race::Race::start(loaded.setup);
    // Enough ticks under full throttle that the craft is genuinely moving before
    // the drop starts - a stationary craft lays five mines in one place and
    // every gap assertion below passes for the wrong reason. See
    // [`WARM_UP_TICKS`] for why the count is what it is.
    let throttle = held(Button::Cross);
    for _ in 0..WARM_UP_TICKS {
        race.tick(&PlayerInputs::single(throttle));
    }

    let stats = race.mine_stats().expect("the disc authors a Mine");
    let speed = race.sim.world.ships[0]
        .physics
        .body
        .linear_velocity
        .length();
    assert!(
        speed > 10.0,
        "the craft is barely moving at {speed:.1} units/s, so a cluster could not \
         spread even if the drop worked"
    );

    let before = race.sim.world.ships[0].physics.body.position;
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Mine);
    race.sim.world.ships[0]
        .pickup
        .begin_drop(oag_weapons::projectile::mine::CLUSTER);

    // The whole cluster takes `(CLUSTER - 1) * DROP_INTERVAL` plus the tick the
    // first one leaves on. Drive well past that.
    let mut still_held_after_first = None;
    for tick in 0..60 {
        race.tick(&PlayerInputs::single(throttle));
        if still_held_after_first.is_none() && !laid(&race).is_empty() {
            still_held_after_first = Some(race.ship_pickup());
            assert!(tick < 5, "the first mine took {tick} ticks to leave");
        }
    }

    let mines = laid(&race);
    println!(
        "laid {} mines over {:.1} units of track at {speed:.1} units/s",
        mines.len(),
        (race.sim.world.ships[0].physics.body.position - before).length()
    );
    assert_eq!(
        mines.len(),
        usize::from(oag_weapons::projectile::mine::CLUSTER),
        "the drop laid {} of {} mines",
        mines.len(),
        oag_weapons::projectile::mine::CLUSTER
    );
    assert_eq!(
        still_held_after_first,
        Some(Some(Weapon::Mine)),
        "the pickup was spent on the first mine of the cluster rather than the last"
    );
    assert_eq!(
        race.ship_pickup(),
        None,
        "the craft is still holding a Mine after the whole cluster came out"
    );

    // Spread, measured against the craft's own travel rather than a number: two
    // mines dropped `DROP_INTERVAL` apart are about `speed * DROP_INTERVAL`
    // apart, and anything above a hull length says they are not stacked.
    let mut furthest: f32 = 0.0;
    for (i, a) in mines.iter().enumerate() {
        for b in &mines[i + 1..] {
            furthest = furthest.max((a.position - b.position).length());
        }
    }
    let expected = speed * oag_weapons::projectile::mine::DROP_INTERVAL;
    assert!(
        furthest > expected,
        "the furthest two mines are {furthest:.1} apart where one drop interval at \
         {speed:.1} units/s is {expected:.1} - the cluster is stacked, not laid"
    );

    // And the fuse is the authored one, still running on every mine. Seven
    // seconds against a drop that took under one.
    for mine in &mines {
        assert!(
            mine.lifetime > 0.0 && mine.lifetime <= stats.timetodie,
            "a mine carries {:.2} s of fuse against an authored {:.2}",
            mine.lifetime,
            stats.timetodie
        );
    }
}

/// The first mine of a cluster is laid **at the craft's own position**.
///
/// Measured on the running original 2026-10-01: a stationary craft's mines carry
/// its body position to the hundredth, and `Bomb_Init`'s drop point equals the
/// body position to the last bit, stationary and at 106 u/s. The charge starts
/// inside the hull and the craft's own motion leaves it behind; this engine used
/// to push it back by `hull_extent`, which was chosen and is not the law. The
/// first tick of a drop, so the craft has not yet moved and "where it was" is
/// the position read before the tick.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_mine_is_laid_at_the_crafts_own_position() {
    let Some(loaded) = single_race() else { return };
    let mut race = race::Race::start(loaded.setup);
    let throttle = held(Button::Cross);
    for _ in 0..WARM_UP_TICKS {
        race.tick(&PlayerInputs::single(throttle));
    }
    race.mine_stats().expect("the disc authors a Mine");

    let origin = race.sim.world.ships[0].physics.body.position;
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Mine);
    race.sim.world.ships[0]
        .pickup
        .begin_drop(oag_weapons::projectile::mine::CLUSTER);
    // One tick, so exactly the first mine is out and the craft has barely moved -
    // measuring after the whole cluster would fold the craft's own travel into
    // the answer and make a nose-mounted drop look rearward.
    race.tick(&PlayerInputs::single(throttle));

    let mines = laid(&race);
    assert_eq!(
        mines.len(),
        1,
        "expected exactly the first mine of the drop"
    );
    let off = (mines[0].position - origin).length();
    println!("the first mine sits {off:.4} units from where the craft was");
    assert!(
        off < 1e-3,
        "the mine was laid {off:.4} units from the craft's own position"
    );
}

/// A craft that drives into somebody else's cluster sets one off and takes the
/// blast; the craft that laid it does not trip its own on the way out.
///
/// **Both halves in one run on purpose**: the second is only interesting while
/// the first is true. A trip radius read as zero makes the cluster harmless and
/// passes any "the dropper survived" assertion on its own.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_cluster_trips_on_a_rival_and_not_on_its_own_dropper() {
    let Some(loaded) = single_race() else { return };
    let mut race = race::Race::start(loaded.setup);
    let throttle = held(Button::Cross);
    for _ in 0..WARM_UP_TICKS {
        race.tick(&PlayerInputs::single(throttle));
    }
    let stats = race.mine_stats().expect("the disc authors a Mine");

    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Mine);
    race.sim.world.ships[0]
        .pickup
        .begin_drop(oag_weapons::projectile::mine::CLUSTER);
    for _ in 0..40 {
        race.tick(&PlayerInputs::single(throttle));
    }
    let mines = laid(&race);
    assert!(
        !mines.is_empty(),
        "nothing was laid, so nothing can be tripped"
    );
    let dropper_shield = race.sim.world.ships[0].physics.shield;

    // Put a rival on top of one of them. Slot 1 rather than slot 0, and its
    // position is written directly: driving a craft onto a mine takes a lap and
    // this is a test about the trip, not about the AI's line.
    let victim = 1;
    assert!(
        race.sim.world.ships[victim].active,
        "the grid has no second craft to trip a mine with"
    );
    let before = race.sim.world.ships[victim].physics.shield;
    race.sim.world.ships[victim].physics.body.position = mines[0].position;
    race.tick(&PlayerInputs::single(throttle));

    let after = race.sim.world.ships[victim].physics.shield;
    println!(
        "slot {victim} took {:.1} energy off an authored {:.1} damage inside a \
         {:.1} trigger radius",
        before - after,
        stats.damage,
        stats.trigger_radius
    );
    assert!(
        after < before,
        "a craft sitting exactly on a mine took no damage - the trip radius or the \
         blast lookup is not being spent"
    );
    assert!(
        laid(&race).len() < mines.len(),
        "the mine that went off is still in the array"
    );
    assert_eq!(
        race.sim.world.ships[0].physics.shield, dropper_shield,
        "the craft that laid the cluster was hurt by a mine it drove away from"
    );
}

/// A Quake wave rippling under a laid mine sets it off, and the mine hurts
/// nobody doing so - `Mine_SweepCraftTrigger`'s closing `Quake_SpanIntensityAt`
/// test, which only raises the destroy bit.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_quake_wave_under_a_mine_sets_it_off_quietly() {
    let Some(loaded) = single_race() else { return };
    let mut race = race::Race::start(loaded.setup);
    let throttle = held(Button::Cross);
    for _ in 0..WARM_UP_TICKS {
        race.tick(&PlayerInputs::single(throttle));
    }
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Mine);
    race.sim.world.ships[0]
        .pickup
        .begin_drop(oag_weapons::projectile::mine::CLUSTER);
    for _ in 0..40 {
        race.tick(&PlayerInputs::single(throttle));
    }
    let mines = laid(&race);
    assert!(!mines.is_empty(), "nothing was laid");
    let shields: Vec<f32> = race
        .sim
        .world
        .ships
        .iter()
        .map(|s| s.physics.shield)
        .collect();

    // Launch a wave from where the first mine lies, travelling forward, so the
    // mine sits inside its radius on the very next tick.
    let stats = race.quake_stats().expect("the disc authors a Quake");
    let progress = race
        .course()
        .expect("a course")
        .locate(mines[0].position, None)
        .expect("the mine is on the ring")
        .progress;
    race.sim.world.quake = Some(oag_weapons::projectile::quake::Wave::launch(
        7, progress, 1.0, &stats,
    ));
    race.tick(&PlayerInputs::single(throttle));

    assert!(
        laid(&race).len() < mines.len(),
        "the mine under the wave is still in the array"
    );
    // The wave itself hits craft in its radius - that is its own law, tested
    // elsewhere - but the mine it set off must add nothing. Nobody stood on
    // the mine, so any energy lost here is the wave's, and it is bounded by the
    // wave's own damage; a mine's damage on top would exceed it.
    for (slot, ship) in race.sim.world.ships.iter().enumerate() {
        assert!(
            shields[slot] - ship.physics.shield <= stats.damage + 1e-3,
            "slot {slot} lost {:.1}, more than the wave alone can take - the mine it \
             set off spent a blast",
            shields[slot] - ship.physics.shield
        );
    }
}

/// **A Bomb press lays exactly one, and it is a Mine one size up in every way
/// the engine models.**
///
/// The Bomb shares every line of `oag_weapons::projectile::mine` with the Mine
/// except a count, so what a real-data test can say that a unit test cannot is
/// that sharing it did not quietly make one of them the other. Four claims, all
/// against the *disc's* numbers rather than chosen ones:
///
/// - One charge, not [`CLUSTER`](oag_weapons::projectile::mine::CLUSTER). A
///   Bomb wired through the Mine's `Drop` with the wrong count is the single
///   most likely way this refactor goes wrong, and it is invisible to the type
///   system.
/// - It is at the craft's own position, like the Mine's.
/// - It does not move. Asserted over sixty ticks of the craft driving away, so
///   a bomb that inherited any of the launcher's velocity - which is what the
///   original's own negated-forward spawn direction might have meant - fails.
/// - Its fuse is the Bomb's own and outlasts the Mine's, which is what says the
///   two `<Stats>` blocks did not get transposed on the way through `Drop`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_bomb_is_one_static_charge_at_the_crafts_own_position() {
    let Some((mut race, throttle)) = moving() else {
        return;
    };
    let bomb = race.mine_stats().expect("a Mine");
    let stats = race.bomb_stats().expect("the disc authors a Bomb");
    // Pulse's Bomb has a fuse; Pure's does not (`BombStats::timetodie`), and
    // this test drives the Pulse disc.
    let fuse = stats.timetodie.expect("Pulse's Bomb authors a timetodie");

    let origin = race.sim.world.ships[0].physics.body.position;
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Bomb);
    let mut buttons = oag_gameplay::input::Input::new();
    buttons.begin_frame(Button::Cross.bit());
    let press = oag_gameplay::InputSnapshot {
        buttons: {
            let mut b = buttons;
            b.begin_frame(Button::Cross.bit() | Button::Square.bit());
            b
        },
        ..oag_gameplay::InputSnapshot::new()
    };
    race.tick(&PlayerInputs::single(press));

    let charges = laid_of(&race, Weapon::Bomb);
    assert_eq!(
        charges.len(),
        1,
        "a Bomb press laid {} charges - it is wired through the Mine's cluster \
         count",
        charges.len()
    );
    assert_eq!(
        race.ship_pickup(),
        None,
        "the pickup survived a drop of one, so the counter did not reach zero"
    );

    let off = (charges[0].position - origin).length();
    println!(
        "the bomb sits {off:.4} units from where the craft was, fuse {:.1} s \
         against the mine's {:.1}",
        charges[0].lifetime, bomb.timetodie
    );
    assert!(
        off < 1e-3,
        "the bomb was laid {off:.4} units from the craft's own position"
    );
    assert!(
        charges[0].lifetime > bomb.timetodie,
        "the bomb's fuse is {:.1} s against the mine's {:.1} - the two <Stats> \
         blocks are transposed",
        charges[0].lifetime,
        bomb.timetodie
    );
    assert!(
        charges[0].lifetime <= fuse,
        "the bomb carries {:.1} s against an authored {:.1}",
        charges[0].lifetime,
        fuse
    );

    // Static, over a full second of the craft driving away from it.
    let placed = charges[0].position;
    for _ in 0..60 {
        race.tick(&PlayerInputs::single(throttle));
    }
    let still = laid_of(&race, Weapon::Bomb);
    assert_eq!(still.len(), 1, "the bomb went off with nobody near it");
    let moved = (still[0].position - placed).length();
    let travelled = (race.sim.world.ships[0].physics.body.position - placed).length();
    println!("after a second the craft is {travelled:.1} away and the bomb moved {moved:.3}");
    assert!(
        moved < 1e-3,
        "the bomb drifted {moved:.3} units while the craft drove {travelled:.1} away"
    );
}

/// A Bomb hurts more than a Mine, on the disc's own numbers, against the same
/// craft in the same place.
///
/// The comparison is the assertion. Both charges are laid at the same point and
/// the same rival is put on each in turn, so the only difference between the two
/// measurements is which `<Stats>` block was spent - which is what a blast
/// lookup wired to the wrong weapon would get wrong while still doing damage.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_bomb_hits_harder_than_a_mine_at_the_same_spot() {
    let Some((mut race, throttle)) = moving() else {
        return;
    };
    race.bomb_stats().expect("the disc authors a Bomb");
    let victim = 1;
    assert!(
        race.sim.world.ships[victim].active,
        "no second craft on the grid"
    );

    let mut taken = Vec::new();
    for kind in [Weapon::Mine, Weapon::Bomb] {
        race.sim.world.ships[0].pickup.weapon = Some(kind);
        race.sim.world.ships[0].pickup.begin_drop(1);
        race.tick(&PlayerInputs::single(throttle));

        let charges = laid_of(&race, kind);
        assert!(!charges.is_empty(), "{kind:?}: nothing was laid");
        let before = race.sim.world.ships[victim].physics.shield;
        race.sim.world.ships[victim].physics.body.position = charges[0].position;
        race.tick(&PlayerInputs::single(throttle));
        taken.push(before - race.sim.world.ships[victim].physics.shield);

        // Put the victim back out of the way and heal it, so the second
        // measurement starts from the same place the first did.
        race.sim.world.ships[victim].physics.shield =
            race.sim.world.ships[victim].handling.dimensions.shield;
        race.sim.world.ships[victim].physics.body.position =
            race.sim.world.ships[0].physics.body.position + forward_of(&race, 0) * 500.0;
        race.sim.world.ships[0].pickup.weapon = None;
        race.sim.world.ships[0].pickup.begin_drop(0);
    }

    println!("mine took {:.1}, bomb took {:.1}", taken[0], taken[1]);
    assert!(
        taken[0] > 0.0,
        "the mine did no damage, so the comparison says nothing"
    );
    assert!(
        taken[1] > taken[0],
        "the bomb took {:.1} against the mine's {:.1} - the blast lookup is \
         spending the wrong weapon's block",
        taken[1],
        taken[0]
    );
}

/// One craft's forward axis, for the arithmetic above.
fn forward_of(race: &race::Race, slot: usize) -> oag_core::math::Vec3 {
    race.sim.world.ships[slot].physics.body.forward()
}
