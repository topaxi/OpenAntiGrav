//! What an opponent does with a pickup it is handed **once**, on a real
//! circuit out of a real disc image.
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
//! # Why "once" is the whole point of this file
//!
//! `race_ground_truth.rs`'s `an_opponent_fires_at_a_craft_ahead_on_a_real_circuit`
//! already asserts that an opponent fires, and it is a good test of the thing it
//! set out to measure - whether `wants_to_fire`'s **aiming** gates ever open on
//! real geometry, which a synthetic straight cannot answer. To do that it hands
//! every empty craft a fresh Rocket **every tick**, and says so.
//!
//! That refill is also what hid a bug for months. `Race::spend_opponent_pickup`
//! ran its weapon arms as `weapon == X && self.fire_x(..)`, and `fire_x` returns
//! `false` for the ordinary "not this tick" answer - the trigger is a *rate*, so
//! it declines about nineteen ticks in twenty. A `false` right-hand side made
//! the whole condition false and fell through to the **absorb** branch, which
//! paid the weapon's energy into the pool and emptied the slot. Under a per-tick
//! refill that is invisible: the craft is handed another one immediately and
//! eventually rolls a success. Under a real race it means an opponent cashes in
//! every weapon on the tick it collects it and never fires one.
//!
//! So this file hands a craft **one** pickup and then leaves it alone, which is
//! what a `Weapon Pad` does. That is the only way to see the difference between
//! "declines a shot" and "loses the weapon".
//!
//! **Measured against the code it was written for**: with the `&&`-chain
//! restored, this reports `Rocket: 0 of 7 accounted for` - every opponent on the
//! grid lost its weapon inside two seconds - against `7 of 7` once the arms
//! return instead of falling through.

use std::path::{Path, PathBuf};

use oag_formats::weapons::Weapon;
use oag_game::race;
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

/// Hands every opponent one `weapon` and drives `ticks` ticks without ever
/// handing out another.
///
/// Returns `(opponents, accounted_for)` - a craft is accounted for when it
/// either put one of that weapon in the air or is still holding the one it was
/// given. **Anything else is the weapon having evaporated**, which is the defect
/// this file exists for.
///
/// The measurement is deliberately *not* "did the energy pool go up". That was
/// the first attempt and it reported zero against code that was losing every
/// pickup: a craft at full shield absorbs into a clamp, so the pool does not
/// move and the absorb leaves no trace. Counting the weapon itself has no such
/// blind spot.
fn hand_out_once(weapon: Weapon, ticks: usize) -> Option<(usize, usize)> {
    let loaded = single_race()?;
    let mut race = race::Race::start(loaded.setup);
    // A few ticks so the grid settles and the standings take a first fix - the
    // drivers' own gates read `Standing::progress`.
    for _ in 0..60 {
        race.tick(&oag_gameplay::InputSnapshot::default());
    }

    let opponents = 1..usize::from(race.ship_count());
    let count = opponents.len();
    for slot in opponents.clone() {
        race.world.ships[slot].pickup.weapon = Some(weapon);
    }

    // Watched every tick rather than only at the end, because a projectile is
    // gone from the array the moment it detonates and a laid mine's whole life
    // can fit inside this window.
    let mut fired = std::collections::BTreeSet::new();
    for _ in 0..ticks {
        race.tick(&oag_gameplay::InputSnapshot::default());
        for projectile in &race.world.projectiles.slots {
            if projectile.kind == Some(weapon) && projectile.owner != 0 {
                fired.insert(usize::from(projectile.owner));
            }
        }
    }

    let accounted = opponents
        .filter(|&slot| {
            fired.contains(&slot) || race.world.ships[slot].pickup.weapon == Some(weapon)
        })
        .count();
    Some((count, accounted))
}

/// **An opponent handed one weapon either fires it or is still holding it.** It
/// does not turn into energy in between.
///
/// Driven for two seconds, which at a `TRIGGER_RATE` of `0.05` is long enough
/// for a craft to take a shot and nowhere near long enough for one to have any
/// legitimate reason to be empty-handed.
///
/// The assertion is deliberately not "they all fired": whether a given craft
/// finds a target in two seconds depends on where the field is on the circuit,
/// and demanding a shot would make this a test of the racing line. What it
/// demands is only that the weapon is *accounted for* - fired or held - because
/// the third outcome is the bug.
///
/// All four armed weapons, not just one, because the chain had a separate arm
/// per weapon and each was independently able to fall through.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_opponent_handed_one_weapon_keeps_it_until_it_fires_it() {
    for weapon in [Weapon::Rocket, Weapon::Missile, Weapon::Mine, Weapon::Bomb] {
        let Some((count, accounted)) = hand_out_once(weapon, 120) else {
            return;
        };
        println!("{weapon:?}: {accounted} of {count} accounted for");
        assert_eq!(
            accounted,
            count,
            "{weapon:?}: {} of {count} opponents lost the weapon they were handed \
             without ever firing it - `spend_opponent_pickup` is falling through \
             to its absorb branch on a declined shot",
            count - accounted
        );
    }
}
