//! The Mine, on a real circuit out of a real disc image.
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
//! 2. **That the mines end up behind the craft.** `craft+0xa0` being the rear
//!    anchor is recovered, but which way this engine's own `body.forward()`
//!    points through `hull_extent` is not something a hand-written pose proves -
//!    a sign error there lays the cluster out of the nose, which is a different
//!    weapon and would still detonate on somebody.
//! 3. **That the authored fuse outlives the drop.** `timetodie` is seven seconds
//!    and the cluster takes half of one; a fuse read from the wrong offset would
//!    very likely be some other weapon's number, and the ones adjacent to it in
//!    the struct are radii and forces rather than seconds. A cluster whose first
//!    mine has expired before its last is laid is what that looks like.

use std::path::{Path, PathBuf};

use oag_formats::weapons::Weapon;
use oag_game::race;
use oag_gameplay::input::{Button, Input};
use oag_physics::SpeedClass;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// A single race: the one mode with weapons on, and so the only one a pickup can
/// happen in at all.
fn single_race() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: SpeedClass::Venom,
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

/// Every mine slot 0 has laid, in the order the array holds them.
fn laid(race: &race::Race) -> Vec<oag_gameplay::projectile::Projectile> {
    race.world
        .projectiles
        .slots
        .iter()
        .filter(|p| p.kind == Some(Weapon::Mine) && p.owner == 0)
        .copied()
        .collect()
}

/// One press lays the whole cluster, spread along the track, and the pickup
/// survives until the last one is out.
///
/// **Three claims in one run, because they are three views of one mechanism**
/// and separating them would mean driving the same thirty ticks three times:
///
/// - The count matches `mine::CLUSTER`. That is the invented number, so what is
///   being pinned is that the machinery lays *all* of it - a reload that reset
///   to the wrong value, or a counter decremented twice, shows up here whatever
///   the constant is later measured to be.
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
    // every gap assertion below passes for the wrong reason.
    let throttle = held(Button::Cross);
    for _ in 0..120 {
        race.tick(&throttle);
    }

    let stats = race.mine_stats().expect("the disc authors a Mine");
    let speed = race.world.ships[0].physics.body.linear_velocity.length();
    assert!(
        speed > 10.0,
        "the craft is barely moving at {speed:.1} units/s, so a cluster could not \
         spread even if the drop worked"
    );

    let before = race.world.ships[0].physics.body.position;
    race.world.ships[0].pickup.weapon = Some(Weapon::Mine);
    race.world.ships[0]
        .pickup
        .begin_drop(oag_gameplay::projectile::mine::CLUSTER);

    // The whole cluster takes `(CLUSTER - 1) * DROP_INTERVAL` plus the tick the
    // first one leaves on. Drive well past that.
    let mut still_held_after_first = None;
    for tick in 0..60 {
        race.tick(&throttle);
        if still_held_after_first.is_none() && !laid(&race).is_empty() {
            still_held_after_first = Some(race.ship_pickup());
            assert!(tick < 5, "the first mine took {tick} ticks to leave");
        }
    }

    let mines = laid(&race);
    println!(
        "laid {} mines over {:.1} units of track at {speed:.1} units/s",
        mines.len(),
        (race.world.ships[0].physics.body.position - before).length()
    );
    assert_eq!(
        mines.len(),
        usize::from(oag_gameplay::projectile::mine::CLUSTER),
        "the drop laid {} of {} mines",
        mines.len(),
        oag_gameplay::projectile::mine::CLUSTER
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
    let expected = speed * oag_gameplay::projectile::mine::DROP_INTERVAL;
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

/// The cluster ends up **behind** the craft that laid it.
///
/// The rear anchor is recovered; that this engine's own `hull_extent` is being
/// asked for it in the right direction is not, and a sign error there is silent -
/// mines out of the nose still detonate on somebody. Measured along the craft's
/// forward axis at the moment of the drop, so it does not depend on the circuit's
/// shape.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_cluster_is_laid_behind_the_craft() {
    let Some(loaded) = single_race() else { return };
    let mut race = race::Race::start(loaded.setup);
    let throttle = held(Button::Cross);
    for _ in 0..120 {
        race.tick(&throttle);
    }
    race.mine_stats().expect("the disc authors a Mine");

    let origin = race.world.ships[0].physics.body.position;
    let forward = race.world.ships[0].physics.body.forward();
    race.world.ships[0].pickup.weapon = Some(Weapon::Mine);
    race.world.ships[0]
        .pickup
        .begin_drop(oag_gameplay::projectile::mine::CLUSTER);
    // One tick, so exactly the first mine is out and the craft has barely moved -
    // measuring after the whole cluster would fold the craft's own travel into
    // the answer and make a nose-mounted drop look rearward.
    race.tick(&throttle);

    let mines = laid(&race);
    assert_eq!(
        mines.len(),
        1,
        "expected exactly the first mine of the drop"
    );
    let along = (mines[0].position - origin).dot(forward);
    println!("the first mine sits {along:.2} units along the craft's forward axis");
    assert!(
        along < 0.0,
        "the mine was laid {along:.2} units *ahead* of the craft - the drop point \
         is coming out of the nose"
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
    for _ in 0..120 {
        race.tick(&throttle);
    }
    let stats = race.mine_stats().expect("the disc authors a Mine");

    race.world.ships[0].pickup.weapon = Some(Weapon::Mine);
    race.world.ships[0]
        .pickup
        .begin_drop(oag_gameplay::projectile::mine::CLUSTER);
    for _ in 0..40 {
        race.tick(&throttle);
    }
    let mines = laid(&race);
    assert!(
        !mines.is_empty(),
        "nothing was laid, so nothing can be tripped"
    );
    let dropper_shield = race.world.ships[0].physics.shield;

    // Put a rival on top of one of them. Slot 1 rather than slot 0, and its
    // position is written directly: driving a craft onto a mine takes a lap and
    // this is a test about the trip, not about the AI's line.
    let victim = 1;
    assert!(
        race.world.ships[victim].active,
        "the grid has no second craft to trip a mine with"
    );
    let before = race.world.ships[victim].physics.shield;
    race.world.ships[victim].physics.body.position = mines[0].position;
    race.tick(&throttle);

    let after = race.world.ships[victim].physics.shield;
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
        race.world.ships[0].physics.shield, dropper_shield,
        "the craft that laid the cluster was hurt by a mine it drove away from"
    );
}
