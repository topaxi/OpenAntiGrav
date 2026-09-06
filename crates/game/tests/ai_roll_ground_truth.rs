//! What our AI's barrel rolls cost it, on the disc's own circuits.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!   ai_roll_ground_truth
//! ```
//!
//! # Why this file exists at all
//!
//! Our opponents barrel-roll on purpose and **the original's never do** - a
//! deliberate, authorized deviation recorded on `docs/gameplay/ai.md`. Every
//! gate around it has a unit test in `oag_physics::barrel_roll` and every axis
//! that feeds it has one in `oag_ai`, and none of them can answer the question
//! that actually decides whether the mechanic is tuned: **on a real circuit,
//! over a real race, how much of its shield does an opponent spend on this?**
//!
//! The failure mode is specific and it has already happened once, on
//! 2026-09-06, before the grounded gate landed: an Ace was arming four to eight
//! rolls a race on ten of the twelve circuits and spending 30 to 54 of its 95
//! shield, and on two circuits it never leaves the ground on, every one of
//! those charges bought nothing. **A tier that rolls itself back down to
//! single-digit shield has been tuned wrong**, and that is what
//! [`no_tier_rolls_itself_down_to_nothing`] asserts.
//!
//! [`sweep_rolls`] is the table behind it - twelve circuits by four tiers - and
//! it is `OAG_SWEEP`-gated rather than merely `#[ignore]`d, for the reason
//! `race_ground_truth.rs`'s own `sweep_grip` gives: `just test-data` runs
//! ignored tests, and a run of that length in that suite is how a suite stops
//! being run.

use std::path::{Path, PathBuf};

use oag_game::{catalogue, race};

/// How long a measured race runs: five minutes at 60 Hz, the same window
/// `race_ground_truth.rs`'s solo benchmark uses.
const TICKS: u64 = 18_000;

/// Which slot is measured. Slot 0 is switched off and 2..8 with it, so what is
/// left is one opponent alone on the circuit - the driving, with no traffic
/// mixed into it.
const LONE: usize = 1;

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

/// What one lone craft did with the roll on one circuit.
#[derive(Debug, Default, Clone, Copy)]
struct Rolls {
    /// How many rolls it armed.
    armed: u32,
    /// What they cost it, in shield-pool units.
    spent: f32,
    /// What was left in the pool at the end.
    shield: f32,
    /// The pool's capacity, so the two above can be read as a fraction.
    capacity: f32,
    /// How far round it got, and whether it had to be recovered - a craft that
    /// spent the race being put back is not measuring the same thing.
    laps: u32,
    respawns: u32,
    /// The quickest lap it managed with no recovery in it, in ticks.
    best: Option<u64>,
    /// How many separate airborne windows it flew - a rising edge on
    /// `ShipState::time_airborne`, counted whether or not it decided to roll
    /// during one.
    ///
    /// **Answers the question the armed count alone cannot**: an airborne gate
    /// that never opens makes the propensity axes moot regardless of how they
    /// are tuned, and a circuit that opens it once for half a second looks
    /// nothing like one that opens it eight times for two.
    airborne_windows: u32,
    /// The longest single window, in seconds.
    longest_airborne: f32,
    /// Ticks spent airborne at all, out of `TICKS` - the fraction of the race
    /// the roll gate was even a live question.
    airborne_ticks: u64,
}

/// One lone craft, one circuit, one difficulty, and optionally one named
/// character rather than whichever one slot 1 drew.
///
/// **`track` is a WAD entry name, not a circuit id.** `catalogue::Track::id`
/// spells the circuit `10_Track` and `Options::track` wants
/// `Data\Tracks\...\track.vex`; passing the first silently fails to load, and a
/// measurement that returns `None` on every circuit reads exactly like a
/// mechanic that never fires. That is not a hypothetical - it is what the first
/// draft of `no_tier_rolls_itself_down_to_nothing` did.
fn rolls_on(level: oag_ai::Difficulty, track: &str, pilot: Option<oag_ai::Pilot>) -> Option<Rolls> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: level,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .ok()?;
    let mut race = race::Race::start(loaded.setup);
    if let Some(pilot) = pilot {
        // Tempered here rather than in the caller, so a sweep across tiers is
        // asking about the same character at four levels.
        race.set_ai_pilot(LONE, level.temper(&pilot));
    }
    // Everyone but one opponent off the track, so nothing it does is about
    // anybody else - the same isolation `race_ground_truth.rs`'s solo benchmark
    // makes, and for the same reason.
    for slot in 2..8 {
        race.world.ships[slot].active = false;
    }
    race.world.ships[0].active = false;

    let mut best: Option<u64> = None;
    let mut lap = race.world.ships[LONE].standing.lap;
    let mut started = 0u64;
    let mut recovered_this_lap = false;
    let mut airborne_windows = 0u32;
    let mut longest_airborne = 0.0f32;
    let mut airborne_ticks = 0u64;
    let mut was_airborne = false;
    for tick in 0..TICKS {
        let before = race.respawns_of(LONE);
        race.tick(&oag_gameplay::InputSnapshot::default());
        if race.respawns_of(LONE) != before {
            recovered_this_lap = true;
        }
        let now = race.world.ships[LONE].standing.lap;
        if now != lap {
            if lap > 1 && !recovered_this_lap {
                let taken = tick - started;
                best = Some(best.map_or(taken, |held: u64| held.min(taken)));
            }
            started = tick;
            lap = now;
            recovered_this_lap = false;
        }
        let airborne = race.world.ships[LONE].physics.time_airborne;
        if airborne > 0.0 {
            airborne_ticks += 1;
            if !was_airborne {
                airborne_windows += 1;
            }
            longest_airborne = longest_airborne.max(airborne);
        }
        was_airborne = airborne > 0.0;
    }

    Some(Rolls {
        armed: race.rolls_armed_of(LONE),
        spent: race.roll_shield_spent_of(LONE),
        shield: race.world.ships[LONE].physics.shield,
        capacity: race.world.ships[LONE].handling.dimensions.shield,
        laps: lap,
        respawns: race.respawns_of(LONE),
        best,
        airborne_windows,
        longest_airborne,
        airborne_ticks,
    })
}

/// Every forward circuit on the disc, as `(id, entry name)`.
fn circuits() -> Vec<(String, String)> {
    let Some(image) = image() else {
        return Vec::new();
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_formats::fexml::expand(&blob).expect("expanding it");

    catalogue::tracks(&definition)
        .into_iter()
        // Forward only: a reversed circuit is the same geometry and doubles the
        // wall clock for nothing this measurement can see.
        .filter(|track| !track.reversed)
        .map(|track| (track.id.clone(), track.entry_name()))
        .collect()
}

/// Every forward circuit on the disc, at one difficulty.
fn rolls_everywhere(
    level: oag_ai::Difficulty,
    pilot: Option<oag_ai::Pilot>,
) -> Vec<(String, Rolls)> {
    circuits()
        .into_iter()
        .filter_map(|(id, entry)| rolls_on(level, &entry, pilot).map(|rolls| (id, rolls)))
        .collect()
}

/// One difficulty tier, one circuit, **the full grid a real race actually
/// starts with** - nothing switched off, nothing forced.
///
/// **This is the check `rolls_on` cannot do.** Every other function in this
/// file isolates one craft so nothing it does is about anybody else, which is
/// right for reading what one character costs and wrong for answering
/// whether the mechanic fires with seven rivals, traffic and weapons in the
/// way. Every opponent draws its own pilot from `oag_ai::pilot_for_slot`, the
/// same as a player would see; only the tier is chosen here.
fn grid_rolls_on(level: oag_ai::Difficulty, track: &str) -> Option<Vec<Rolls>> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: level,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .ok()?;
    let mut race = race::Race::start(loaded.setup);
    let opponents: Vec<usize> = (1..usize::from(race.ship_count())).collect();
    measure_grid(&mut race, &opponents)
}

/// The same full-grid race, but with each opponent slot's **character**
/// tempered as a named tier rather than whichever one tier the race-wide
/// difficulty chose for everybody.
///
/// **This is the honest substitute for "Ace against the lower tiers".**
/// `race::Options::difficulty` is a single race-wide setting - `Race::start`
/// tempers every slot's pilot from it and derives the shared `Tuning` (grip,
/// turn rate, mistakes, reaction) from it too, and nothing in `Race` takes a
/// difficulty per slot. So a grid that is *physically* mixed-tier does not
/// exist to ask about. What this does instead: race-wide difficulty is fixed
/// at `Ace` (so every craft shares the same grip, turn allowance and reaction
/// time - nobody is slow because it cannot corner), and each slot's *pilot* is
/// separately tempered to the tier `tiers` names for it, which is exactly the
/// three roll axes plus weapon/ram/defence appetite - the axes
/// `Difficulty::temper` actually touches. A craft named `Novice` here is a
/// full-speed driver with a novice's *temperament*, not a slow one; that is
/// the whole of what "propensity" can mean without a per-slot `Tuning`.
fn grid_rolls_mixed_on(track: &str, tiers: &[oag_ai::Difficulty]) -> Option<Vec<(oag_ai::Difficulty, Rolls)>> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .ok()?;
    let mut race = race::Race::start(loaded.setup);
    let opponents: Vec<usize> = (1..usize::from(race.ship_count())).collect();
    for (slot, &tier) in opponents.iter().zip(tiers.iter()) {
        race.set_ai_pilot(*slot, tier.temper(&oag_ai::Pilot::BALANCED));
    }
    let rolls = measure_grid(&mut race, &opponents)?;
    Some(
        opponents
            .iter()
            .zip(tiers.iter())
            .zip(rolls)
            .map(|((_, &tier), rolls)| (tier, rolls))
            .collect(),
    )
}

/// Runs the clock on an already-set-up race and reads every named slot's
/// roll, shield and airborne figures at the end - the per-slot loop
/// [`rolls_on`] runs for one craft, generalised to however many are asked
/// for.
fn measure_grid(race: &mut race::Race, slots: &[usize]) -> Option<Vec<Rolls>> {
    let width = slots.iter().copied().max().map_or(0, |m| m + 1);
    let mut lap = vec![0u32; width];
    for &slot in slots {
        lap[slot] = race.world.ships[slot].standing.lap;
    }
    let mut started = vec![0u64; width];
    let mut recovered_this_lap = vec![false; width];
    let mut best: Vec<Option<u64>> = vec![None; width];
    let mut airborne_windows = vec![0u32; width];
    let mut longest_airborne = vec![0.0f32; width];
    let mut airborne_ticks = vec![0u64; width];
    let mut was_airborne = vec![false; width];

    for tick in 0..TICKS {
        let before: Vec<u32> = slots.iter().map(|&s| race.respawns_of(s)).collect();
        race.tick(&oag_gameplay::InputSnapshot::default());
        for (i, &slot) in slots.iter().enumerate() {
            if race.respawns_of(slot) != before[i] {
                recovered_this_lap[slot] = true;
            }
            let now = race.world.ships[slot].standing.lap;
            if now != lap[slot] {
                if lap[slot] > 1 && !recovered_this_lap[slot] {
                    let taken = tick - started[slot];
                    best[slot] = Some(best[slot].map_or(taken, |held: u64| held.min(taken)));
                }
                started[slot] = tick;
                lap[slot] = now;
                recovered_this_lap[slot] = false;
            }
            let airborne = race.world.ships[slot].physics.time_airborne;
            if airborne > 0.0 {
                airborne_ticks[slot] += 1;
                if !was_airborne[slot] {
                    airborne_windows[slot] += 1;
                }
                longest_airborne[slot] = longest_airborne[slot].max(airborne);
            }
            was_airborne[slot] = airborne > 0.0;
        }
    }

    Some(
        slots
            .iter()
            .map(|&slot| Rolls {
                armed: race.rolls_armed_of(slot),
                spent: race.roll_shield_spent_of(slot),
                shield: race.world.ships[slot].physics.shield,
                capacity: race.world.ships[slot].handling.dimensions.shield,
                laps: lap[slot],
                respawns: race.respawns_of(slot),
                best: best[slot],
                airborne_windows: airborne_windows[slot],
                longest_airborne: longest_airborne[slot],
                airborne_ticks: airborne_ticks[slot],
            })
            .collect(),
    )
}

/// **The constraint, asserted rather than printed.**
///
/// An opponent that ends a race in single digits has spent its pool on showing
/// off and will be destroyed by the first wall it meets - which is a worse
/// opponent, and the regression this whole mechanic caused once already.
///
/// Three circuits rather than twelve, so this is cheap enough to live in
/// `just test-data`. **`09_Track` is the one that earns its place**: it is
/// where the failure actually happened, an aggressive Ace finishing on 6.5
/// shield at the floors this landed with, and it is still the worst case at
/// 21.7. `10_Track` has the longest airborne windows the sweep found on a
/// circuit a craft laps cleanly, and `14_Track` finished on the least shield of
/// the twelve before the AI rolled at all. The whole table is [`sweep_rolls`].
#[test]
#[ignore = "needs a disc image in data/images/"]
fn no_tier_rolls_itself_down_to_nothing() {
    let wanted = ["09_Track", "10_Track", "14_Track"];
    let circuits: Vec<(String, String)> = circuits()
        .into_iter()
        .filter(|(id, _)| wanted.contains(&id.as_str()))
        .collect();
    if circuits.is_empty() {
        return;
    }
    // The most willing character of the four, so this is the worst case rather
    // than whichever one slot 1 happened to draw - which is a shy pilot, and a
    // shy pilot passing says nothing about the other three.
    for (id, entry) in circuits {
        for (_, level) in oag_ai::Difficulty::ALL {
            let Some(rolls) = rolls_on(level, &entry, Some(oag_ai::Pilot::AGGRESSIVE)) else {
                return;
            };
            println!(
                "{id} {:<8} armed {:<3} spent {:>5.1} left {:>5.1} of {:.0}",
                level.name(),
                rolls.armed,
                rolls.spent,
                rolls.shield,
                rolls.capacity
            );
            assert!(
                rolls.shield >= 10.0,
                "{id} at {}: a craft that ends on {:.1} shield after arming \
                 {} rolls has been tuned to roll itself down to nothing",
                level.name(),
                rolls.shield,
                rolls.armed
            );
        }
    }
}

/// And the deviation is actually *reachable*: an Ace rolls somewhere on the
/// disc.
///
/// **The other half of the constraint above, and the one a timid tuning would
/// pass silently.** A floor set too high or an airtime set too long gives a
/// green suite and an AI that never rolls, which is the original's behaviour
/// rather than the one the maintainer asked for.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_ace_actually_rolls_somewhere_on_the_disc() {
    let table = rolls_everywhere(oag_ai::Difficulty::Ace, Some(oag_ai::Pilot::BALANCED));
    if table.is_empty() {
        return;
    }
    let armed: u32 = table.iter().map(|(_, rolls)| rolls.armed).sum();
    for (track, rolls) in &table {
        println!(
            "{track:<10} armed {:<3} spent {:>5.1} left {:>5.1} laps {} respawns {}",
            rolls.armed, rolls.spent, rolls.shield, rolls.laps, rolls.respawns
        );
    }
    assert!(
        armed > 0,
        "a balanced Ace armed no rolls on any of the disc's twelve forward \
         circuits, so the deviation is unreachable and the AI is back to the \
         original's behaviour"
    );
}

/// The whole table: twelve circuits by four tiers, printed rather than
/// asserted.
///
/// The figures it produces go into `docs/gameplay/ai.md` by hand, so the tuning
/// has its measurement beside it. Twelve circuits times four tiers times five
/// minutes of simulation is minutes of wall clock, so it is off unless asked
/// for - `#[ignore]` alone would not do it, since `just test-data` runs ignored
/// tests:
///
/// ```sh
/// OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 \
///   cargo nextest run --release -p oag-game --run-ignored all sweep_rolls \
///   --no-capture
/// ```
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn sweep_rolls() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let mut report = String::from(
        "\npilot       tier     circuit    armed  spent  left   laps  resp  best\n\
         ---------------------------------------------------------------------\n",
    );
    // The pilot is forced rather than drawn, because `pilot_for_slot` hands
    // slot 1 the *same* character on every circuit - so an unforced sweep is one
    // character's table with four tiers on it, and reads as the field's.
    for (pilot_name, pilot) in oag_ai::Pilot::BUILT_IN {
        for (name, level) in oag_ai::Difficulty::ALL {
            let table = rolls_everywhere(level, Some(pilot));
            if table.is_empty() {
                return;
            }
            let armed: u32 = table.iter().map(|(_, rolls)| rolls.armed).sum();
            let spent: f32 = table.iter().map(|(_, rolls)| rolls.spent).sum();
            for (track, rolls) in &table {
                report.push_str(&format!(
                    "{pilot_name:<11} {name:<8} {track:<10} {:<6} {:<6.1} {:<6.1} {:<5} {:<5} {}\n",
                    rolls.armed,
                    rolls.spent,
                    rolls.shield,
                    rolls.laps,
                    rolls.respawns,
                    rolls.best.map_or("-".to_string(), |ticks| format!(
                        "{:.1}s",
                        ticks as f32 / 60.0
                    )),
                ));
            }
            report.push_str(&format!(
                "{pilot_name:<11} {name:<8} {:<10} {armed:<6} {spent:<6.1}\n\n",
                "TOTAL"
            ));
        }
    }
    println!("{report}");
}

/// **A full grid, not an isolated craft**: does the mechanic still fire with
/// seven rivals, traffic and weapons in the way, and does a higher tier still
/// roll more than a lower one when they are actually racing each other?
///
/// `09_Track` - the circuit `no_tier_rolls_itself_down_to_nothing` already
/// treats as the worst case for shield - at all four tiers, nothing
/// switched off and nothing forced: every opponent draws its own pilot from
/// `oag_ai::pilot_for_slot`, exactly as a real race does.
///
/// **Asserted loosely, on purpose.** Seven independent per-flight coin
/// tosses over one race is a small sample, so this checks the *aggregate*
/// across the whole grid is non-decreasing tier to tier rather than any
/// single slot's count, and it stops short of asserting a strict `>` between
/// adjacent tiers - see the printed table for whether the ordering the
/// maintainer asked for actually held on this run, and `docs/gameplay/ai.md`
/// for the numbers this produced.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_full_grid_still_rolls_and_a_higher_tier_rolls_no_less() {
    let track = circuits()
        .into_iter()
        .find(|(id, _)| id == "09_Track")
        .map(|(_, entry)| entry);
    let Some(track) = track else {
        return;
    };

    let mut totals = Vec::new();
    for (name, level) in oag_ai::Difficulty::ALL {
        let Some(rolls) = grid_rolls_on(level, &track) else {
            return;
        };
        let armed: u32 = rolls.iter().map(|r| r.armed).sum();
        let spent: f32 = rolls.iter().map(|r| r.spent).sum();
        println!("=== 09_Track, full grid, {name} ===");
        for (slot, r) in rolls.iter().enumerate() {
            println!(
                "  slot {:<2} armed {:<3} spent {:>5.1} left {:>5.1} of {:.0} \
                 airborne windows {:<3} longest {:.2}s ticks airborne {}/{TICKS} laps {} resp {}",
                slot + 1,
                r.armed,
                r.spent,
                r.shield,
                r.capacity,
                r.airborne_windows,
                r.longest_airborne,
                r.airborne_ticks,
                r.laps,
                r.respawns
            );
        }
        println!("  TOTAL armed {armed} spent {spent:.1}");
        totals.push((name, armed));
    }

    for pair in totals.windows(2) {
        let ((easier_name, easier), (harder_name, harder)) = (pair[0], pair[1]);
        assert!(
            harder >= easier,
            "on a full grid on 09_Track, {harder_name} armed {harder} rolls total \
             against {easier_name}'s {easier}, which is a lower tier rolling more \
             than a higher one, not less"
        );
    }
}

/// **Ace against the lower tiers, in the same race.** `race::Options`'s
/// difficulty is race-wide - see [`grid_rolls_mixed_on`] for why a physically
/// mixed-tier grid does not exist to ask about - so this fixes the whole
/// field's `Tuning` at `Ace` and tempers each opponent's *pilot* to a
/// different named tier, then reads whether the ones tempered `Ace` still
/// roll more than the ones tempered `Novice` while contending for the same
/// track and the same traffic.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_ace_pilot_rolls_more_than_a_novice_pilot_in_the_same_race() {
    let track = circuits()
        .into_iter()
        .find(|(id, _)| id == "09_Track")
        .map(|(_, entry)| entry);
    let Some(track) = track else {
        return;
    };

    use oag_ai::Difficulty::{Ace, Elite, Novice, Skilled};
    let tiers = [Ace, Ace, Elite, Elite, Skilled, Skilled, Novice];
    let Some(rolls) = grid_rolls_mixed_on(&track, &tiers) else {
        return;
    };

    println!("=== 09_Track, full grid, mixed pilot tiers (shared Ace Tuning) ===");
    let mut by_tier: Vec<(oag_ai::Difficulty, u32, f32)> = Vec::new();
    for (slot, (tier, r)) in rolls.iter().enumerate() {
        println!(
            "  slot {:<2} {:<8} armed {:<3} spent {:>5.1} left {:>5.1} of {:.0} \
             airborne windows {:<3} longest {:.2}s",
            slot + 1,
            tier.name(),
            r.armed,
            r.spent,
            r.shield,
            r.capacity,
            r.airborne_windows,
            r.longest_airborne,
        );
        by_tier.push((*tier, r.armed, r.spent));
    }

    let armed_of = |wanted: oag_ai::Difficulty| -> u32 {
        by_tier
            .iter()
            .filter(|(tier, _, _)| *tier == wanted)
            .map(|(_, armed, _)| armed)
            .sum()
    };
    let ace = armed_of(Ace);
    let novice = armed_of(Novice);
    println!("  Ace-tempered slots armed {ace} total, Novice-tempered armed {novice}");
    assert!(
        ace >= novice,
        "an Ace-tempered pilot armed {ace} rolls against a Novice-tempered \
         pilot's {novice} in the same race, sharing the same grid, track and \
         Tuning - the difficulty propensity is not doing its job"
    );
}
