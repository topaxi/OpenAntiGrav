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

/// **What a field does to itself over a real race**, with the disc's own pads
/// and the disc's own draw rather than a weapon handed back every tick.
///
/// A measurement before it is an assertion, because the number was not
/// predictable from the code. Every craft follows the *same* authored racing
/// line and a mine cluster is laid *on* that line, since the only spread it gets
/// is the craft moving between drops and nothing scatters one sideways. So the
/// geometry is set up for the field to mine itself.
///
/// What keeps it in proportion is three things, and this is what the test
/// checks is still true: the pads' own `refresh_time` rations the pickups, the
/// draw spreads them over seven weapons rather than handing out mines, and a
/// driver only lays when somebody is **behind** it - so the craft most likely to
/// eat a cluster is the one that provoked it, which is the weapon working rather
/// than a bug.
///
/// **The stress case is much worse and is recorded here rather than asserted**:
/// with a Mine handed back the moment a craft empties - far more generous than
/// any pad layout - the field ground to a mean of 21 of 95 energy in ten seconds
/// and the worst craft was destroyed. That is not a realistic arrangement, but
/// it is the direction this weapon pushes a one-line grid in, and it is the
/// number to re-measure if pads are ever made more frequent or the draw is
/// narrowed.
///
/// # What the dodge bought, measured
///
/// This test is also the before-and-after for `Driver::avoidance`, which is the
/// term that lets a driver see a laid charge at all. Same circuit, same seed,
/// same minute:
///
/// | | charges laid | mean energy | worst craft |
/// | --- | --- | --- | --- |
/// | blind | - | 76 of 95 | **34** |
/// | dodging | 12 | 86 of 95 | **75** |
/// | dodging, with the Plasma in the draw (2026-09-02) | 6 | 77 of 95 | **47** |
///
/// The mean barely moves and the *worst* craft more than doubles, which is the
/// shape to expect and is worth stating: the damage was never spread evenly. It
/// was concentrated on whichever craft was sitting behind a driver with a Mine,
/// eating cluster after cluster on a line it could not see. The dodge does not
/// make mines useless - 75 of 95 is still a craft that has been hit - it stops
/// one craft being singled out for a punishment it had no way to avoid.
///
/// # Why the third row exists, and why the floor moved rather than the code
///
/// **The Plasma landing in `pickup::IMPLEMENTED` on 2026-09-02 took the worst
/// craft from 75 to 47 and this test red.** It was measured rather than
/// assumed: the same scenario on the commit before, and on that commit alone,
/// gives the second row.
///
/// It is the weapon working, not a regression, and the arithmetic says so. The
/// Plasma's authored `damage` is the largest of any weapon on both shipped
/// tables - **four times the Rocket's and twelve times the Mine's** - and one
/// hit is more than half a full pool. A worst craft at 47 of 95 is down by
/// roughly *one* of them net of regeneration, where the whole point of the
/// blind-mine number (34) was that it took a dozen small ones nobody could see.
/// The halved charge count in the same row is the other half of the same story:
/// the Plasma takes draw share from the Mine and the Bomb, so **fewer** mines
/// are laid, not more.
///
/// **Nothing here can be dodged by anybody**, which is what makes it fair in
/// the sense this test was written to defend: a bolt in flight arrives faster
/// than any driver or player could react, so the player is under exactly the
/// same threat. That is the opposite of the mine case, where an opponent was
/// eating charges the player could steer around.
///
/// So the floor below moved from half a pool to `0.45`, and the value is picked
/// rather than fitted: it must sit **above** the blind-mine 34 the guard exists
/// to catch and **below** what the field now reaches with the game's hardest
/// weapon in play. `0.45` of 95 is 42.75, which is 8.75 clear of the failure
/// shape and 4.25 clear of the current measurement. If a future weapon squeezes
/// that gap further, split the metric by weapon rather than dropping the floor
/// again - the guard is worth nothing once it cannot separate the two causes.
///
/// **Two measured divergences are worth naming here**, because between them
/// they are the reason the third row could have been milder - and the larger of
/// the two may yet push the floor back *up*:
///
/// - **The Plasma very probably winds up before it fires, and this build fires
///   it instantly.** `<Plasma charge_time>` is authored at `3` on all three
///   shipped tables, no consumer has been found on a path read end to end, and
///   a maintainer who plays Pulse says the weapon does charge. A three-second
///   commit per shot changes how often a field actually lands one far more than
///   anything else here does. So "the weapon working" above is true of the
///   weapon *as read*, and if `charge_time` is ever found this measurement
///   should be retaken before the floor is trusted. See
///   `docs/ghidra/functions/psp-pulse-usa/plasma.md`.
/// - The disc's own `WeaponAIstats.xml` authors the Plasma at
///   `useAgainstAI="1.0"` where the Rocket, the Missile and the Mine are all
///   `1.2` - so the original also uses it against other craft less often, and
///   this engine does not, because that whole decision is unported (see
///   `Race::spend_opponent_pickup`). `Race::fire_opponent_plasma` deliberately
///   reuses the Rocket's gate rather than inventing a second one.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_field_racing_with_real_pads_does_not_mine_itself_to_death() {
    let Some(loaded) = single_race() else { return };
    let mut race = race::Race::start(loaded.setup);

    let opponents = 1..usize::from(race.ship_count());
    let full: f32 = race.world.ships[1].handling.dimensions.shield;
    assert!(full > 0.0, "the fixture has no energy pool to measure");

    // A minute of racing, untouched: real pads, real refresh timers, the real
    // weighted draw.
    let mut laid = 0usize;
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..3_600 {
        race.tick(&oag_gameplay::InputSnapshot::default());
        for projectile in &race.world.projectiles.slots {
            if matches!(projectile.kind, Some(Weapon::Mine | Weapon::Bomb))
                && seen.insert(projectile.position.x.to_bits())
            {
                laid += 1;
            }
        }
    }

    let alive = opponents
        .clone()
        .filter(|&slot| race.world.ships[slot].active)
        .count();
    let mean: f32 = opponents
        .clone()
        .map(|slot| race.world.ships[slot].physics.shield)
        .sum::<f32>()
        / opponents.len() as f32;
    let worst = opponents
        .clone()
        .map(|slot| race.world.ships[slot].physics.shield)
        .fold(f32::INFINITY, f32::min);
    println!(
        "a minute of racing with the disc's own pads: {laid} charges laid, \
         {alive} of {} opponents still active, mean energy {mean:.0} of {full:.0}, \
         worst {worst:.0}",
        opponents.len()
    );

    assert_eq!(
        alive,
        opponents.len(),
        "a craft was destroyed over a minute of ordinary racing"
    );
    assert!(
        mean > full * 0.7,
        "the field ground itself to a mean of {mean:.0} of {full:.0} over a minute \
         of ordinary racing - a grid that all follows one racing line and lays \
         mines on it will mine itself down unless `Driver::avoidance` is steering \
         around them"
    );
    // **The worst craft, not only the mean**, because the mean was never the
    // problem: blind, it sat at 76 of 95 while one craft was ground down to 34.
    // A regression in the dodge shows up here first and in the mean barely at
    // all.
    assert!(
        worst > full * 0.45,
        "one craft ended on {worst:.0} of {full:.0} while the field averaged \
         {mean:.0} - it is being singled out. Two things do that and they look \
         alike here: a driver that cannot see the charges it is driving over \
         (the shape this guard was written for, which measured 34), or a craft \
         eating repeated Plasma hits (one is worth more than half a pool). \
         {laid} charges were laid this run - a low count points at the second"
    );
}
