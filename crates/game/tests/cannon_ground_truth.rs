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
//! 3. **That the round survives the real collision soup**, riding the
//!    floor-follower every other unread projectile weapon here shares as a
//!    placeholder.
//!
//! What is deliberately **not** asserted is any authored *value*, per
//! ADR-0006 - every assertion here is relative or against a number the test
//! itself measured.

use std::path::{Path, PathBuf};

use oag_formats::weapons::Weapon;
use oag_game::race;
use oag_gameplay::input::{Button, Input};

/// Past the start-line countdown, not merely "a while" - see
/// `plasma_ground_truth.rs`'s twin for why a flat tick count is the wrong
/// warm-up.
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
fn rounds(race: &race::Race) -> Vec<oag_gameplay::projectile::Projectile> {
    race.world
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
        race.tick(&throttle);
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
    // projectile weapon here - see `oag_formats::weapons::CannonStats`'s own
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
    race.world.ships[0].pickup.weapon = Some(Weapon::Cannon);

    // Long enough that a self-firing countdown would have gone off many times
    // over, with fire never held. Nothing may leave the barrel. A flat
    // 600 ticks rather than a multiple of `rate`, because `rate` is now the
    // reciprocal the disc authors - about a sixtieth of a second - and three
    // of those is not a window anything could be caught in.
    let quiet = 600u64;
    for _ in 0..quiet {
        race.tick(&throttle);
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
        race.tick(&fire);
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
    let speed = race.world.ships[0].physics.body.linear_velocity.length();
    assert!(
        round.velocity.length() > speed,
        "the round is slower than the craft that fired it: {:.1} against \
         {speed:.1} units/s - it should carry the craft's own speed plus a \
         base, never less than the craft alone",
        round.velocity.length()
    );

    // And it rides the track - the shared floor-follower every unread
    // projectile weapon here gets, exercised against real geometry.
    let start = round.position;
    let mut furthest: f32 = 0.0;
    let mut detonated_after = None;
    for tick in 1..=120 {
        race.tick(&throttle);
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
