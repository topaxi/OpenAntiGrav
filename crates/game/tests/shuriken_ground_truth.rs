//! The Shuriken on a real circuit out of a real disc image.
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
//! Its own file for the reason `plasma_ground_truth.rs` and
//! `mine_ground_truth.rs` give; the harness helpers below are the same
//! deliberate near-duplicates.
//!
//! # What only real data can say here
//!
//! 1. **That the disc's own Shuriken block decodes**, with its two damage pairs
//!    read the right way round. The fixture in
//!    `crates/formats/src/weapons/tests.rs` proves the parser reads *a*
//!    document of that shape; only the disc says the shipped one matches.
//! 2. **That a blade actually bounces.** This is the whole weapon, and it is
//!    the one thing no synthetic fixture can produce: a bounce needs a wall the
//!    blade runs *into*, at an angle, which means real track geometry. The
//!    headless tests can only pin the launch.
//! 3. **That the fuse ends it rather than a wall.** A blade with the disc's own
//!    `fuse` should still be alive after several bounces and gone once the fuse
//!    runs out - the ordering that says the bounce path is not quietly
//!    detonating it.
//!
//! What is deliberately **not** asserted is any authored *value*. Reproducing
//! one would put shipped tuning data in the repository, which ADR-0006 does not
//! allow - so the assertions below are all relative (further than, still
//! inside, more than one) or against numbers the test itself measured.

use std::path::{Path, PathBuf};

use oag_formats::weapons::Weapon;
use oag_game::race;
use oag_gameplay::input::{Button, Input};

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

/// Every blade slot 0 has in the air.
fn blades(race: &race::Race) -> Vec<oag_gameplay::projectile::Projectile> {
    race.world
        .projectiles
        .slots
        .iter()
        .filter(|p| p.kind == Some(Weapon::Shuriken) && p.owner == 0)
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
        race.tick(&throttle);
    }
    Some((race, throttle))
}

/// The disc's own `<Shuriken>` block decodes, and reads the blast pair rather
/// than the ricochet pair.
///
/// **Shape, not values**, per ADR-0006: every assertion is "authored above
/// zero", "ordered the way the schema says", or a comparison between two of the
/// disc's own numbers. A block that failed to decode comes back `None`; one
/// that read the wrong attribute names comes back zeroed.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_discs_shuriken_block_decodes_with_its_blast_pair() {
    let Some(loaded) = single_race() else { return };
    let race = race::Race::start(loaded.setup);
    let shuriken = race.shuriken_stats().expect("the disc authors a Shuriken");

    assert!(shuriken.blastdamage > 0.0, "no blastdamage: {shuriken:?}");
    assert!(shuriken.blastradius > 0.0, "no blastradius");
    assert!(shuriken.blastforce > 0.0, "no blastForce");
    assert!(shuriken.absorb > 0.0, "no absorb");
    assert!(
        shuriken.fuse > 0.0,
        "a zero fuse would reap a blade on the tick it was thrown"
    );

    let speeds: Vec<f32> = oag_formats::handling::SpeedClass::ALL
        .into_iter()
        .map(|class| shuriken.speed_for(class))
        .collect();
    println!(
        "shuriken class speeds: {speeds:?}, launch {}, fuse {}",
        shuriken.launch_speed, shuriken.fuse
    );
    assert!(speeds[0] > 0.0, "Venom's blade speed is zero: {speeds:?}");
    for pair in speeds.windows(2) {
        assert!(
            pair[1] >= pair[0],
            "the class speeds are not ascending: {speeds:?} - the four offsets \
             are probably being read in the wrong order"
        );
    }
}

/// One press throws **one** blade, off to one side, and it **bounces off the
/// circuit's walls** rather than dying on the first one.
///
/// The bounce is the weapon, and it is the half that needs a real track: it
/// takes geometry the blade runs into at an angle, which no hand-built fixture
/// has. The counter is `Projectile::bounces`, which the Shuriken raises without
/// limit - unlike the Missile, which stops at `MAX_BOUNCES`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_thrown_blade_bounces_off_a_real_circuit_and_dies_on_its_fuse() {
    let Some((mut race, throttle)) = moving() else {
        return;
    };
    let shuriken = race.shuriken_stats().expect("the disc authors a Shuriken");
    let speed = race.world.ships[0].physics.body.linear_velocity.length();
    assert!(
        speed > 10.0,
        "the craft is barely moving at {speed:.1} units/s"
    );

    let forward = race.world.ships[0].physics.body.forward();
    let up = race.world.ships[0].physics.body.up();
    let right = forward.cross(up);
    race.world.ships[0].pickup.weapon = Some(Weapon::Shuriken);

    let mut buttons = Input::new();
    buttons.begin_frame(0);
    let mut fire = oag_gameplay::InputSnapshot::new();
    fire.buttons = buttons;
    fire.buttons
        .begin_frame(Button::Square.bit() | Button::Cross.bit());
    race.tick(&fire);

    let thrown: Vec<_> = blades(&race);
    assert_eq!(thrown.len(), 1, "one press threw {} blades", thrown.len());
    assert_eq!(race.ship_pickup(), None, "throwing must spend the pickup");

    // Off to one side, at the recovered angle's magnitude - the side itself is
    // the coin's and is deliberately not asserted here.
    let direction = thrown[0].velocity.normalize();
    let angle = direction.dot(right).atan2(direction.dot(forward)).abs();
    assert!(
        (angle - oag_gameplay::projectile::shuriken::LAUNCH_ANGLE).abs() < 1e-3,
        "the blade left at {angle:.4} rad, not the recovered launch angle"
    );

    // Fly it out. A blade should glance off the circuit's own walls and still be
    // alive doing it - the fuse, not the first wall, is what ends it.
    let mut peak_bounces = 0u8;
    let mut alive_ticks = 0u32;
    let fuse_ticks = (shuriken.fuse * 60.0).ceil() as u32;
    for _ in 0..fuse_ticks + 30 {
        race.tick(&throttle);
        match blades(&race).first() {
            Some(live) => {
                peak_bounces = peak_bounces.max(live.bounces);
                alive_ticks += 1;
            }
            None => break,
        }
    }
    println!(
        "the blade lived {alive_ticks} tick(s) of a {fuse_ticks}-tick fuse and \
         bounced {peak_bounces} time(s)"
    );

    // It flew for something like its fuse rather than dying instantly. Not an
    // equality: a blade that finds a craft or a dead end legitimately ends
    // early, so this is the "it is not detonating on the first wall" assertion.
    assert!(
        alive_ticks > fuse_ticks / 4,
        "the blade lasted {alive_ticks} of {fuse_ticks} ticks - it is dying on \
         contact rather than glancing off"
    );
    assert!(
        alive_ticks <= fuse_ticks + 1,
        "the blade outlived its own {fuse_ticks}-tick fuse by {} tick(s)",
        alive_ticks.saturating_sub(fuse_ticks)
    );
}
