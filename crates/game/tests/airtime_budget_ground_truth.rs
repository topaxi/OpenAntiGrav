//! How much airtime each circuit actually offers, against what each
//! difficulty tier currently requires before it will consider a barrel roll.
//!
//! **`#[ignore]`d and `OAG_SWEEP`-gated, never run in CI or `just test-data`.**
//! It needs game content, which this project does not ship (see
//! `docs/architecture/adr/0006-no-copyrighted-content.md`), and 24
//! circuit-directions at up to four difficulty tiers over an 18,000-tick race
//! each is minutes of wall clock, the same reason `ai_roll_ground_truth.rs`'s
//! own `sweep_rolls` is gated the same way rather than merely `#[ignore]`d.
//!
//! ```sh
//! OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 \
//!   cargo nextest run --release -p oag-game --run-ignored all \
//!   airtime_budget_ground_truth --no-capture
//! ```
//!
//! # Why this file exists at all
//!
//! `ai_roll_ground_truth.rs` established that `09_Track` locks Novice,
//! Skilled and Elite out of the roll entirely, because every jump there lands
//! in 0.47-0.62s and `roll_caution` raises the threshold above that for every
//! tier but Ace. The maintainer's response to "loosen `roll_caution`" was to
//! rule it out as premature: it is one constant applied across sixteen
//! circuits with completely different airtime budgets, and no measurement
//! existed yet of what those budgets actually are. This file is that
//! measurement, and it tunes nothing - every test here prints rather than
//! asserts.
//!
//! # What counts as a "jump" here, and what does not
//!
//! `ShipState::time_airborne` climbs for as long as a craft's wheels are off
//! the ground, and it does not distinguish a driven jump from a fall off the
//! circuit that ends in a respawn relaunch - `ai_roll_ground_truth.rs` already
//! hit this once, as a single 5.07s outlier on `09_Track`. Left in, a
//! circuit's own respawns would inflate its measured airtime rather than
//! reporting it, and a Novice-grip pathology like `13_Track`'s (221-349
//! respawns a race, see `docs/gameplay/ai.md`) would make that circuit's row
//! read as generous when the truth is closer to unmeasurable. So every
//! airborne window here is checked against the respawn count at the time it
//! opened and closed, and one is dropped rather than counted if a respawn
//! landed nearby - see [`RESPAWN_LEAD_TICKS`], [`RESPAWN_TRAIL_TICKS`] and
//! [`MAX_PLAUSIBLE_TICKS`] for exactly how nearby (and how long is too long
//! regardless), and [`AirtimeSample::discarded`] for how many were dropped.
//! A trailing window still open when the measurement window ends is dropped
//! for the same reason a respawn-contaminated one is: it is a truncated
//! duration, not a real one.

use std::path::PathBuf;

use oag_game::{catalogue, race};

/// How long a measured race runs: five minutes at 60 Hz, the same window
/// `ai_roll_ground_truth.rs`'s own benchmark uses.
const TICKS: u64 = 18_000;

/// Which slot is measured, alone on the circuit - see `ai_roll_ground_truth.rs`
/// for why isolating one craft is the right read for "what does this circuit
/// offer", as opposed to "does the mechanic fire with seven rivals in the way".
const LONE: usize = 1;

/// How many ticks *before* an airborne window's rising edge a respawn still
/// counts as having produced it - the relaunch drop rather than a driven
/// jump. **Chosen, not measured.** Two seconds at 60 Hz.
const RESPAWN_LEAD_TICKS: u64 = 120;

/// How many ticks *after* an airborne window's falling edge a respawn still
/// counts as contaminating it - the fall completing into a respawn some time
/// after the ground would otherwise have been under it.
///
/// **Chosen, not measured, but not arbitrary either**: `oag_race::recovery`'s own
/// `RESCUE_TICKS` (90) is how long a craft must dwell off its line before a
/// respawn fires at all, so a fall's landing and the respawn it triggers can
/// legitimately be 90 ticks apart with nothing wrong. An initial 30-tick
/// margin under-caught this - `05_Track`/`skilled` kept a 67.83s "airborne
/// window" that was really an unrecovered fall - so this is `RESCUE_TICKS`
/// plus a full second of slack, not a bare guess.
const RESPAWN_TRAIL_TICKS: u64 = 150;

/// The longest window this sweep counts as a driven flight at all, regardless
/// of respawn proximity.
///
/// **Chosen, not measured, off the same precedent `ai_roll_ground_truth.rs`'s
/// own module docs already used**: its one respawn-relaunch outlier ran
/// 5.07s. The longest window this sweep kept on a row with **zero** respawns
/// anywhere in the whole race - so nothing in that race to be contaminated
/// by - was 1.72-1.73s (`25_Track`, every tier), well under this cutoff.
/// `17_Track`/Ace's 2.93s sits on a row with nine respawns in the same race,
/// so it is not independent evidence the way `25_Track`'s is. A raw window
/// past 5s with **no** nearby respawn is not a longer jump - the near-respawn
/// filter above cannot catch a fall that never re-triggers one nearby, only
/// one that does, and this is what caught it: `05_Track`/`skilled` flew a
/// 67.83s "window" with the nearest respawn nowhere near it, plainly a
/// collision/stuck state rather
/// than a driven flight.
const MAX_PLAUSIBLE_TICKS: u64 = 300;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// One `PI_Track` entry: its id, the WAD entry name a race loads, the
/// environment directory it shares with its own reverse, and whether this is
/// that reverse.
struct Circuit {
    id: String,
    entry: String,
    location: String,
    reversed: bool,
}

/// Every `PI_Track` entry on the disc, forward and reversed both - unlike
/// `ai_roll_ground_truth.rs`'s `circuits()`, which filters to forward only,
/// this file's whole point is comparing a circuit against its own reverse.
///
/// **Keeps `location` rather than just `id`/`entry`**, so a caller can group
/// a circuit with its reverse - the two do not sit at adjacent ids in plugin
/// order (`16_Track`/`32_Track` do; `21_Track` and `05_Track` do not, and
/// behave nothing alike), so pairing by list position would be wrong.
fn circuits_all() -> Vec<Circuit> {
    let Some(image) = image() else {
        return Vec::new();
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");

    catalogue::tracks(&definition)
        .into_iter()
        .map(|track| Circuit {
            id: track.id.clone(),
            entry: track.entry_name(),
            location: track.location.clone(),
            reversed: track.reversed,
        })
        .collect()
}

/// One raw airborne window, before the respawn filter runs on it.
struct RawWindow {
    /// First tick `time_airborne` was seen above zero.
    start: u64,
    /// First tick it was seen back at zero, i.e. one past the last airborne
    /// tick - so `end - start` is the window's length in ticks.
    end: u64,
}

/// What one lone craft's airborne windows looked like on one circuit, at one
/// difficulty and speed class, over one race.
#[derive(Debug, Default)]
struct AirtimeSample {
    /// Kept (driven, uncontaminated) window durations, in ticks, in flight
    /// order.
    episodes_ticks: Vec<u64>,
    /// Raw windows thrown out because a respawn landed near their start or
    /// end - see [`RESPAWN_LEAD_TICKS`]/[`RESPAWN_TRAIL_TICKS`] - or because
    /// the craft was still airborne when the measurement window closed.
    discarded: u32,
    /// Laps completed with no respawn anywhere in them - the denominator a
    /// "windows per lap" figure should divide by, and the read on whether this
    /// row is trustworthy at all.
    clean_laps: u32,
    /// Laps reached at all, clean or not.
    laps: u32,
    /// Total respawns over the race.
    respawns: u32,
}

impl AirtimeSample {
    /// Kept durations, sorted ascending, in ticks.
    fn sorted_ticks(&self) -> Vec<u64> {
        let mut v = self.episodes_ticks.clone();
        v.sort_unstable();
        v
    }

    /// `(min, median, max)` over the kept durations, in ticks - `None` if
    /// nothing was kept.
    fn stats_ticks(&self) -> Option<(u64, u64, u64)> {
        let v = self.sorted_ticks();
        let min = *v.first()?;
        let max = *v.last()?;
        let median = v[v.len() / 2];
        Some((min, median, max))
    }
}

/// Ticks to seconds, at the simulation's fixed 60 Hz.
fn secs(ticks: u64) -> f32 {
    ticks as f32 / 60.0
}

/// One lone craft, one circuit, one difficulty and speed class - every
/// airborne window it flies, split into kept and discarded by the respawn
/// filter described in the module docs.
///
/// **`track` is a WAD entry name, not a circuit id** - see
/// `ai_roll_ground_truth.rs::rolls_on` for why that distinction is load-bearing
/// here too.
fn measure_airtime(
    level: oag_ai::Difficulty,
    track: &str,
    pilot: Option<oag_ai::Pilot>,
    class: &str,
) -> Option<AirtimeSample> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: level,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .ok()?;
    let mut race = race::Race::start(loaded.setup);
    if let Some(pilot) = pilot {
        // Tempered here, so a sweep across tiers is asking about the same
        // character at four levels rather than whichever one slot 1 drew.
        race.set_ai_pilot(LONE, level.temper(&pilot));
    }
    // Everyone but one opponent off the track, so every airborne window
    // measured is this craft's own driving, with no traffic in the way of it.
    for slot in 2..8 {
        race.sim.world.ships[slot].active = false;
    }
    race.sim.world.ships[0].active = false;

    let mut lap = race.sim.world.ships[LONE].standing.lap;
    let mut recovered_this_lap = false;
    let mut clean_laps = 0u32;

    let mut respawn_ticks: Vec<u64> = Vec::new();
    let mut prev_respawns = race.respawns_of(LONE);

    let mut raw: Vec<RawWindow> = Vec::new();
    let mut was_airborne = false;
    let mut start_tick = 0u64;

    for tick in 0..TICKS {
        race.tick(&oag_gameplay::InputSnapshot::default());

        let now_respawns = race.respawns_of(LONE);
        if now_respawns != prev_respawns {
            respawn_ticks.push(tick);
            prev_respawns = now_respawns;
            recovered_this_lap = true;
        }

        let now_lap = race.sim.world.ships[LONE].standing.lap;
        if now_lap != lap {
            // The same convention `ai_roll_ground_truth.rs::rolls_on` uses for
            // its own "best lap" figure: the first lap segment is the run-up
            // off the grid, not a full lap, so only `lap > 1` counts one.
            if lap > 1 && !recovered_this_lap {
                clean_laps += 1;
            }
            lap = now_lap;
            recovered_this_lap = false;
        }

        let airborne = race.sim.world.ships[LONE].physics.time_airborne;
        if airborne > 0.0 {
            if !was_airborne {
                start_tick = tick;
            }
            was_airborne = true;
        } else if was_airborne {
            raw.push(RawWindow {
                start: start_tick,
                end: tick,
            });
            was_airborne = false;
        }
    }
    // A window still open when the clock ran out is a truncated duration, not
    // a real one - dropped, and counted as discarded rather than silently lost.
    let truncated_tail = was_airborne;

    let mut episodes_ticks: Vec<u64> = Vec::new();
    let mut discarded: u32 = u32::from(truncated_tail);
    for window in &raw {
        let duration = window.end - window.start;
        let lo = window.start.saturating_sub(RESPAWN_LEAD_TICKS);
        let hi = window.end + RESPAWN_TRAIL_TICKS;
        let near_a_respawn = respawn_ticks.iter().any(|&r| r >= lo && r <= hi);
        // Some falls never trigger a nearby respawn at all - the craft comes
        // down somewhere the collision mesh still counts as "on the line"
        // eventually, tens of seconds later - so proximity to a respawn tick
        // cannot be the only test. `MAX_PLAUSIBLE_TICKS` catches those.
        let implausibly_long = duration > MAX_PLAUSIBLE_TICKS;
        if near_a_respawn || implausibly_long {
            discarded += 1;
        } else {
            episodes_ticks.push(duration);
        }
    }

    Some(AirtimeSample {
        episodes_ticks,
        discarded,
        clean_laps,
        laps: lap,
        respawns: race.respawns_of(LONE),
    })
}

/// Prints one [`AirtimeSample`] row in the shared column layout every test in
/// this file uses.
fn print_row(label: &str, sample: &AirtimeSample) {
    let kept = sample.episodes_ticks.len();
    // A row where the respawn/plausibility filter threw out at least as much
    // as it kept is a row about this circuit's respawn pathology, not its
    // airtime - `13_Track`/novice (0 kept, 350 discarded) and
    // `29_Track`/novice (1 kept, 226 discarded) are both this, and printing a
    // distribution for either would read as a measurement it is not.
    // A clean zero (0 kept, 0 discarded, `resp 0`) is the strongest claim
    // this file can make - "this circuit offers nothing at all" - and must
    // print as one, not get folded into the same bucket as a circuit whose
    // respawn count ate the whole race. Only *discarding at least as much as
    // was kept* is the pathology signal.
    let swamped = sample.discarded > 0 && sample.discarded >= kept as u32;
    if swamped {
        println!(
            "{label:<28} NOT MEASURABLE - kept {:<3} discarded {:<3} clean_laps {:<2} \
             laps {:<2} resp {:<3} (respawn pathology swamps this row)",
            kept, sample.discarded, sample.clean_laps, sample.laps, sample.respawns
        );
        return;
    }
    match sample.stats_ticks() {
        Some((min, median, max)) => {
            println!(
                "{label:<28} kept {:<3} discarded {:<3} clean_laps {:<2} laps {:<2} \
                 resp {:<3} min {:>5.2}s/{:<4}t med {:>5.2}s/{:<4}t max {:>5.2}s/{:<4}t",
                kept,
                sample.discarded,
                sample.clean_laps,
                sample.laps,
                sample.respawns,
                secs(min),
                min,
                secs(median),
                median,
                secs(max),
                max,
            );
        }
        None => {
            println!(
                "{label:<28} kept 0   discarded {:<3} clean_laps {:<2} laps {:<2} resp {:<3} \
                 - no driven airborne window at all",
                sample.discarded, sample.clean_laps, sample.laps, sample.respawns
            );
        }
    }
}

/// **Step 1 of the task: does tier change the airborne-window distribution at
/// all, or only the threshold it is checked against?**
///
/// Two circuits, four tiers each, `VENOM`, `BALANCED` forced: `03_Track`
/// (Moa Therma, the maintainer's named example) and `10_Track` (the longest
/// airborne windows `ai_roll_ground_truth.rs`'s sweep found on a circuit a
/// craft laps cleanly). If the kept-window distribution barely moves tier to
/// tier here, sweeping all 24 circuit-directions at every tier is measuring
/// the same thing four times over for 4x the wall clock, and
/// `airtime_budget_sweep` below sweeps at `Ace` alone instead, on that
/// evidence - printed here rather than assumed.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn tier_dependence_probe() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let wanted = ["03_Track", "10_Track"];
    let circuits: Vec<Circuit> = circuits_all()
        .into_iter()
        .filter(|c| wanted.contains(&c.id.as_str()))
        .collect();
    if circuits.is_empty() {
        return;
    }
    for circuit in circuits {
        for (name, level) in oag_ai::Difficulty::ALL {
            let Some(sample) = measure_airtime(
                level,
                &circuit.entry,
                Some(oag_ai::Pilot::BALANCED),
                "VENOM",
            ) else {
                return;
            };
            print_row(&format!("{} {name}", circuit.id), &sample);
        }
    }
}

/// **Step 3's class check.** The maintainer's "very difficult if possible at
/// all" is a human playing whatever class they raced, not necessarily
/// `VENOM` - the slowest class and the one every other ground-truth test in
/// this crate uses. Airtime scales with speed, so this checks whether a
/// faster class changes the answer for the rows that matter most:
/// `03_Track` (Moa Therma, the maintainer's named example) plus `16_Track`
/// and `07_Track` - the two circuits `airtime_budget_sweep` found flying
/// **zero** airborne windows at every tier at `VENOM`, so "cannot roll here"
/// is the strongest claim in the whole table and the one most worth checking
/// against a faster class before it goes in the report.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn zero_window_circuits_at_the_fastest_class() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let wanted = ["03_Track", "16_Track", "07_Track"];
    let circuits: Vec<Circuit> = circuits_all()
        .into_iter()
        .filter(|c| wanted.contains(&c.id.as_str()))
        .collect();
    if circuits.is_empty() {
        return;
    }
    for circuit in circuits {
        for (name, level) in oag_ai::Difficulty::ALL {
            let Some(sample) = measure_airtime(
                level,
                &circuit.entry,
                Some(oag_ai::Pilot::BALANCED),
                "PHANTOM",
            ) else {
                return;
            };
            print_row(&format!("{} PHANTOM {name}", circuit.id), &sample);
        }
    }
}

/// **The table nobody has**: every circuit, forward and reversed, its
/// airborne-window distribution, and both readings of "what a roll currently
/// requires" beside it -
///
/// - **can-it-ever-happen**: `AGGRESSIVE`'s own `roll_airtime` low end (the
///   least demanding of the four built-in pilots) times that tier's
///   `roll_caution` - the absolute floor for that tier, no pilot lower.
/// - **typical pilot**: `BALANCED`'s tempered span - what the field's most
///   common character actually asks for at that tier.
///
/// Swept at `Ace` alone if [`tier_dependence_probe`] found the airborne
/// distribution itself does not move tier to tier (only the threshold checked
/// against it does) - in which case one circuit's row answers the question
/// for every tier at once, and the four-tier thresholds are simply printed
/// alongside the one measured distribution. See the doc comment above this
/// function once the probe has run for which of the two this file actually
/// did.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn airtime_budget_sweep() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    println!("\n-- thresholds, seconds/ticks, by tier --");
    for (name, level) in oag_ai::Difficulty::ALL {
        let caution = level.roll_caution();
        let floor = oag_ai::Pilot::AGGRESSIVE.roll_airtime.low * caution;
        let balanced = level.temper(&oag_ai::Pilot::BALANCED).roll_airtime;
        println!(
            "{name:<8} can-it-ever-happen(aggressive low) {:>5.2}s/{:<4} \
             typical(balanced) {:.2}-{:.2}s / {}-{}t",
            floor,
            (floor * 60.0).round() as u64,
            balanced.low,
            balanced.high,
            (balanced.low * 60.0).round() as u64,
            (balanced.high * 60.0).round() as u64,
        );
    }

    println!("\n-- circuits, VENOM, BALANCED forced --");
    let mut circuits = circuits_all();
    // Grouped by `location` rather than left in plugin-definition order, so a
    // circuit's row sits directly above its own reverse's - `16_Track` and
    // `32_Track` already happen to be adjacent that way, but `21_Track` and
    // `05_Track` do not, and behave nothing alike.
    circuits.sort_by(|a, b| {
        a.location
            .cmp(&b.location)
            .then(a.reversed.cmp(&b.reversed))
    });
    for circuit in circuits {
        let label = if circuit.reversed {
            format!("{} (rev)", circuit.id)
        } else {
            circuit.id.clone()
        };
        for (name, level) in oag_ai::Difficulty::ALL {
            let Some(sample) = measure_airtime(
                level,
                &circuit.entry,
                Some(oag_ai::Pilot::BALANCED),
                "VENOM",
            ) else {
                return;
            };
            print_row(&format!("{label} {name}"), &sample);
        }
    }
}

// **This pass tunes nothing.** No test above compares a measured figure to
// `roll_caution`, `roll_airtime` or any other pilot axis and fails on the
// result - every one prints rather than asserts, exactly like
// `ai_roll_ground_truth.rs::sweep_rolls`, because the brief this file answers
// is "what does each circuit offer", not "is the gate right", and the second
// question is the maintainer's to answer once this table exists.
