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

use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_tables::weapons::Weapon;

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
        race.tick(&PlayerInputs::none());
    }

    let opponents = 1..usize::from(race.ship_count());
    let count = opponents.len();
    for slot in opponents.clone() {
        race.sim.world.ships[slot].pickup.weapon = Some(weapon);
    }

    // Watched every tick rather than only at the end, because a projectile is
    // gone from the array the moment it detonates and a laid mine's whole life
    // can fit inside this window.
    let mut fired = std::collections::BTreeSet::new();
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
        for projectile in &race.sim.world.projectiles.slots {
            if projectile.kind == Some(weapon) && projectile.owner != 0 {
                fired.insert(usize::from(projectile.owner));
            }
        }
    }

    let accounted = opponents
        .filter(|&slot| {
            fired.contains(&slot) || race.sim.world.ships[slot].pickup.weapon == Some(weapon)
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
/// What keeps it in proportion is three things: the pads' own `refresh_time`
/// rations the pickups, the draw spreads them over seven weapons rather than
/// handing out mines, and a driver only lays when somebody is **behind** it -
/// so the craft most likely to eat a cluster is the one that provoked it,
/// which is the weapon working rather than a bug.
///
/// **The stress case is much worse and is recorded here rather than asserted**:
/// with a Mine handed back the moment a craft empties - far more generous than
/// any pad layout - the field ground to a mean of 21 of 95 energy in ten seconds
/// and the worst craft was destroyed. That is not a realistic arrangement, but
/// it is the direction this weapon pushes a one-line grid in, and it is the
/// number to re-measure if pads are ever made more frequent or the draw is
/// narrowed.
///
/// # This test used to be a single-seed worst-craft floor, and the floor broke
///
/// Up to 2026-09-06 this asserted `worst > full * 0.45` on one race, one seed
/// (the project's fixed default, `oag_raceplay::SEED = 1`) - see the git
/// history for that version and its own "What the dodge bought" table
/// (blind 34, dodging 75, dodging-with-Plasma 47). **`oag_physics::pair::overlap`
/// was corrected the same day against a live, instruction-level read of
/// `Collision_BoxAgainstBox`** (confidence 92), and that
/// correction is not being touched or reverted here: it changes which axis wins
/// a narrowphase tie on an already-detected overlap, and eight craft over 3,600
/// ticks is a chaotic enough system that the *first* differently-resolved
/// contact sends every later position, and every later RNG draw it gates, down
/// a different trajectory. At `SEED = 1` specifically, that cascade now seats
/// one opponent inside a Plasma blast (60 of 95 in one hit) that it did not
/// stand in before - `worst` reads `0.00` at the exact tuning
/// (`Tuning::look_speed = 0.30`) this floor was written to pass. That craft's
/// hits, in order: a Bomb, a Missile and a Plasma, no mine contact at all.
///
/// **Measured directly, rather than assumed, that a single seed cannot carry
/// this floor any more**: `crates/game/tests/weapon_floor_sweep.rs`'s
/// `sweep_worst_shield_over_seeds`, run at `look_speed` `0.30` (the shipped
/// default), `0.22` and `0.20` (both values `Tuning::look_speed`'s own doc
/// comment already measured as genuinely wrecking the field, pre-correction),
/// 16 seeds each, same track, same 3,600-tick window:
///
/// | `look_speed` | mean-of-means | mean-of-worsts | seeds <= 0.45 worst | seeds with a depleted craft |
/// | --- | --- | --- | --- | --- |
/// | **0.30 (shipped)** | 0.68 | 0.32 | 12/16 | 2/16 |
/// | 0.22 | 0.72 | 0.38 | 9/16 | 2/16 |
/// | 0.20 | 0.71 | 0.30 | 11/16 | 3/16 |
///
/// **0.22 scores *better* than the shipped default on every column but one**,
/// and 0.20's numbers sit inside the same noise band as 0.30's rather than
/// below it. `sweep_lap_completion_over_seeds` in the same file - opponents
/// reaching lap 2 inside a real time budget, which touches no weapon RNG at
/// all - tells the same story: `0.30` and `0.22` both post 2 short seeds of 16
/// and 0/16 untimed laps; `0.20` posts 3 of 16. None of mean shield, worst
/// shield, depleted-craft count or lap completion separates the values this
/// floor was meant to catch from the one it was meant to pass, once averaged
/// over enough seeds to wash out which craft a corrected contact normal
/// happens to redirect into which blast radius.
///
/// **So this is no longer a `look_speed` (or any other AI tuning) guard, and
/// it does not try to be one.** That job now belongs entirely to
/// `crates/game/tests/ai_look_sweep.rs`'s *solo* board - a lone craft never
/// exercises `oag_physics::pair::overlap` at all, so the correction above
/// cannot have moved it, and a direct re-run confirms it: every solo-board
/// number in `Tuning::look_speed`'s doc comment reproduces to the digit on
/// today's tree. What this test still owns is coarser and does not need a
/// single seed to be fragile about: **did something wreck the field**, not
/// **did the dice put one craft in a bad spot**. The maintainer has ruled that
/// a craft dying to weapons in a real race is fine on its own; what would not
/// be fine is most of the field going down, or the field's shield collapsing
/// far below anything a real pad layout has ever produced.
///
/// So the assertions below run eight seeds (`0..8`, chosen to include the old
/// pinned default at index 1) rather than one, and gate on the **aggregate**:
/// mean-of-means above `0.5` (comfortably below the `0.68`-`0.72` band every
/// tuning above measures at, comfortably above the blind-mine catastrophe's
/// `0.221`) and total depleted-craft count across all eight races no worse than
/// one per race on average (measured baseline at the shipped tuning across 16
/// seeds: `3` depleted-craft-instances, `0.19` per race - this floor allows
/// `8`, over four times that). **Both numbers are chosen, not measured, and
/// carry no confidence score** - they are picked with headroom from the
/// measured healthy band on one side and the measured catastrophe on the
/// other, the same way the retired `0.45` floor once was, and the same
/// re-sweep-if-squeezed rule applies: if a future weapon or pad change closes
/// this gap, split the metric rather than dropping the floor again.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_field_racing_with_real_pads_does_not_mine_itself_to_death() {
    const SEEDS: u64 = 8;
    /// Chosen, not measured, no confidence score - see this function's own
    /// doc comment for the measured band it sits below.
    const MEAN_FLOOR: f32 = 0.5;
    /// Chosen, not measured, no confidence score - see this function's own
    /// doc comment for the measured baseline it sits above.
    const DEPLETED_CEILING: usize = SEEDS as usize;

    let Some(image) = image() else { return };

    let mut means = Vec::new();
    let mut depleted_total = 0usize;
    for seed in 0..SEEDS {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            seed: Some(seed),
            ..race::Options::default()
        })
        .expect("loading the race");
        let mut race = race::Race::start(loaded.setup);

        let opponents = 1..usize::from(race.ship_count());
        let full: f32 = race.sim.world.ships[1].handling.dimensions.shield;
        assert!(full > 0.0, "the fixture has no energy pool to measure");

        // A minute of racing, untouched: real pads, real refresh timers, the
        // real weighted draw.
        let mut laid = 0usize;
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..3_600 {
            race.tick(&PlayerInputs::none());
            for projectile in &race.sim.world.projectiles.slots {
                if matches!(projectile.kind, Some(Weapon::Mine | Weapon::Bomb))
                    && seen.insert(projectile.position.x.to_bits())
                {
                    laid += 1;
                }
            }
        }

        let depleted = opponents
            .clone()
            .filter(|&slot| race.sim.world.ships[slot].physics.shield <= 0.0)
            .count();
        let mean: f32 = opponents
            .clone()
            .map(|slot| race.sim.world.ships[slot].physics.shield)
            .sum::<f32>()
            / opponents.len() as f32;
        let worst = opponents
            .map(|slot| race.sim.world.ships[slot].physics.shield)
            .fold(f32::INFINITY, f32::min);
        println!(
            "seed {seed}: {laid} charges laid, mean energy {mean:.0} of {full:.0}, \
             worst {worst:.0}, {depleted} depleted"
        );
        means.push(mean / full);
        depleted_total += depleted;
    }

    let mean_of_means = means.iter().sum::<f32>() / means.len() as f32;
    println!(
        "over {SEEDS} seeds: mean-of-means {mean_of_means:.2} (floor {MEAN_FLOOR}), \
         {depleted_total} depleted-craft-instances total (ceiling {DEPLETED_CEILING})"
    );

    assert!(
        mean_of_means > MEAN_FLOOR,
        "the field's mean shield collapsed to {mean_of_means:.2} of a full pool, \
         averaged over {SEEDS} seeds - below the blind-mine catastrophe's own \
         0.221 with no headroom left. A single seed's mean is expected to swing \
         with the corrected craft-pair physics (see this function's own doc \
         comment); an aggregate this low over {SEEDS} seeds is not that swing"
    );
    assert!(
        depleted_total <= DEPLETED_CEILING,
        "{depleted_total} craft ended a race destroyed, summed over {SEEDS} seeds - \
         more than one per race on average. A craft dying to weapons in a race is \
         fine on its own (the maintainer's own ruling); this many is the field \
         going down, not one craft's bad luck"
    );
}
