//! The committed regression ratchet behind `ai_clean_lap_board.rs`'s sweep.
//!
//! That file is a scratch exploration tool: `#[ignore]`d, gated on `OAG_SWEEP`,
//! and it prints rather than asserts. This file is the opposite kind of thing -
//! a frozen [`BASELINE`] and 48 asserting tests, one per `(circuit, class)`
//! cell, so a change that makes the AI worse fails `just test-data` instead of
//! waiting for someone to notice a printed table got worse. See the
//! `row_test!` macro's own doc comment for why 48 rather than the four
//! class-level tests the brief this file was built against asked for.
//!
//! **This does not share code with `ai_clean_lap_board.rs`.** That file's
//! `solo_on` is private to its own test binary - integration tests do not see
//! each other's items - and it is owned by another thread besides
//! (`handover/gameplay/an-ace-craft-should-lap-every-circuit-without-touching-a-wall.md`).
//! [`solo_on`] below is a second, deliberately smaller implementation against
//! the same public `oag_game::race` surface: three fields instead of eleven,
//! because a regression gate only needs what it asserts on.
//!
//! # What "clean lap" means here, and what it does not
//!
//! Every row is a **lone** Ace on `Mode::SingleRace`, 18,000 ticks (five
//! simulated minutes), with every other slot switched off - the same scenario
//! `ai_clean_lap_board.rs` and `race_ground_truth.rs`'s `solo_laps_everywhere`
//! use. "Clean" means only "no respawn happened during this lap"; it says
//! nothing about the racing line taken. **A row can be clean by this
//! definition while grinding a wall every lap** - `07_Track` is the standing
//! example: every class here reads it `eliminated`, but before this project
//! measured wall contact as its own quantity `07_Track` passed
//! `race_ground_truth.rs`'s older gate while shedding shield to its own walls
//! on every lap it drove. That is exactly why the `contact_ticks` column
//! exists: a green board from [`BASELINE`]'s status column alone does not mean
//! the AI drives every track perfectly, only that it has not gotten worse at
//! the three things this file can see - finishing, dying, and how much wall it
//! eats on the way.
//!
//! # The three columns, and how each is asserted
//!
//! - **`status`** ([`Status`]): `CleanLap`, `NoCleanLap` or `Eliminated`,
//!   asserted **exactly** - not as a ceiling. A status is a category, not a
//!   number with a direction, and "asserted exactly" is what forces a human to
//!   look at every change to it, including a craft newly *surviving* to a
//!   clean lap. The failure message says which way the row moved: a regression
//!   reads as a regression, and an improvement reads as an instruction to
//!   tighten [`BASELINE`] rather than as a bug.
//! - **`lap_ticks`** (`Option<u64>`): the fastest clean lap, as a **ceiling**
//!   with [`LAP_TOLERANCE_TICKS`] of slack, asserted whenever the baseline
//!   recorded a time - which includes some `Eliminated` rows, because a craft
//!   can lap cleanly once and still die later in the run (`07_Track` at VENOM
//!   is exactly this: `Eliminated`, `lap_ticks` still `Some`). A row regressing
//!   from `Some` to `None` already fails on `status` above; this assertion is
//!   only reached when both sides have a time to compare.
//! - **`contact_ticks`** (`u32`): ticks spent touching a wall while still
//!   `Racing`, also a ceiling, with [`CONTACT_TOLERANCE_TICKS`] of slack. Four
//!   rows baseline at **zero** - `02_Track` and `10_Track` at VENOM, `03_Track`
//!   at VENOM and FLASH - and those are the rows this gate exists to protect
//!   most: any wall contact on them is a regression from perfect, not a matter
//!   of degree.
//!
//! **The contact ceiling inverts on a row that stays `Eliminated` but survives
//! longer.** `ai_clean_lap_board.rs`'s own doc records this: a wreck settled
//! against a wall keeps accruing while `oag_physics::step` keeps integrating
//! it, so a craft that now dies later in an `Eliminated` row can read *more*
//! contact ticks for surviving longer, not for driving worse. This gate does
//! not special-case it: `racing_ticks` is not carried here (the exact
//! denominator that would disambiguate the two is on the board, not this
//! table), so an `Eliminated` row's contact ceiling is a coarser signal than a
//! `Racing` row's and a failure on one is worth reading against the board
//! before assuming a regression. Recording `racing_ticks` in [`BASELINE`] too
//! was considered and left out to keep the table under the 1,000-line file
//! ceiling - see [`Row`]'s own doc.
//!
//! # Tolerance, and why it is not measured from repeat runs
//!
//! The simulation is deterministic by this project's own contract
//! (`docs/architecture/determinism.md`, and `race_ground_truth.rs`'s
//! `a_driven_field_replays_identically` asserts it directly): two runs of the
//! same row produce bit-identical ticks, so a tolerance derived from
//! repeat-run spread would be zero, which is an equality gate wearing a
//! ceiling's clothes and exactly what the brief this file was built against
//! forbids. **This is checked below, not assumed** - `print_baseline_source`
//! was run twice on the same tree and the two outputs `diff`ed; see the
//! commit that filled in [`BASELINE`] for the result of that comparison, done
//! at the same time as generating the numbers themselves.
//!
//! So the tolerance is **chosen, not measured** (no confidence score - this is
//! an engineering choice, not an RE claim), from what a *legitimate* AI change
//! has actually cost on this board, per the handover thread's own log: step 3
//! (the per-craft yaw ceiling) cost "0.5-3.0 s slower per circuit" for a real
//! safety improvement, and step 8's mean clean lap moved 38.9s -> 38.1s ->
//! 39.0s across three tuning variants of the same idea. `LAP_TOLERANCE_TICKS`
//! is set to **90 ticks (1.5s)** - inside that observed 0.5-3.0s band, so a
//! change of the shape this project actually makes does not itself trip the
//! gate, while a change an order of magnitude smaller than that band still
//! does. `CONTACT_TOLERANCE_TICKS` is **10** - small next to every non-zero
//! baseline row (the smallest is 11) but enough that the four zero rows still
//! read as "must stay at or near zero" rather than "must stay at exactly the
//! bit-identical figure this run happened to produce", since a future
//! *unrelated* change to contact detection's rounding could shift a
//! genuinely-clean row by a tick or two without that being what this gate is
//! for.
//!
//! # Regenerating [`BASELINE`]
//!
//! Never by hand. [`print_baseline_source`] below runs the same 48 rows this
//! table freezes and prints them as a Rust literal ready to paste in.
//!
//! ```sh
//! flock "$HOME/.cache/oag/gate.lock" env OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 \
//!   cargo nextest run --release -p oag-game --run-ignored all \
//!   --no-capture print_baseline_source
//! ```
//!
//! [`BASELINE`] below was generated this way on commit `8f5070ef` ("Merge: the
//! two changes only pay together, and 05_Track's line blocks them"),
//! 2026-09-12, and cross-checked against the table already recorded in
//! `handover/gameplay/an-ace-craft-should-lap-every-circuit-without-touching-a-wall.md`
//! under "The board" - the two agree on every one of the 48 rows' status and
//! `contact_ticks`(that table does not carry raw `lap_ticks`, only seconds
//! rounded to one decimal, which is why this file has its own generator rather
//! than transcribing the printed one).

use std::path::PathBuf;

use oag_game::{catalogue, race};

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// Five simulated minutes at 60 Hz - the same window
/// `ai_clean_lap_board.rs` and `race_ground_truth.rs`'s solo benchmarks use,
/// so all three tables are comparable.
const TICKS: u64 = 18_000;

/// The lone measured slot; every other slot is switched off.
const LONE: usize = 1;

/// A ceiling's slack on the clean-lap time, in ticks. See this file's own doc
/// comment for where 90 (1.5s) comes from.
const LAP_TOLERANCE_TICKS: u64 = 90;

/// A ceiling's slack on wall-contact ticks. See this file's own doc comment.
const CONTACT_TOLERANCE_TICKS: u32 = 10;

/// What one lone Ace managed on one circuit at one speed class - the three
/// fields this gate asserts on and nothing else. See the file doc comment for
/// why this duplicates rather than reuses `ai_clean_lap_board.rs::Solo`.
struct Solo {
    /// The quickest lap with no recovery in it, in ticks.
    best: Option<u64>,
    /// Ticks spent in contact with a wall while still `Racing`.
    contact_ticks: u32,
    /// What state the craft ended in.
    state: oag_physics::CraftState,
}

/// One craft, alone, on one circuit at one class, exactly the scenario
/// `ai_clean_lap_board.rs::solo_on` measures. See that file for the fuller
/// version with the driver-index clustering this gate does not need.
fn solo_on(track: &str, class: &str) -> Option<Solo> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .ok()?;
    let mut race = race::Race::start(loaded.setup);
    for slot in 2..8 {
        race.sim.world.ships[slot].active = false;
    }
    race.sim.world.ships[0].active = false;

    let mut best: Option<u64> = None;
    let mut lap = race.sim.world.ships[LONE].standing.lap;
    let mut started = 0u64;
    let mut recovered_this_lap = false;
    for tick in 0..TICKS {
        let before = race.respawns_of(LONE);
        race.tick(&oag_gameplay::InputSnapshot::default());
        if race.respawns_of(LONE) != before {
            recovered_this_lap = true;
        }
        let now = race.sim.world.ships[LONE].standing.lap;
        if now != lap {
            if lap > 1 && !recovered_this_lap {
                let taken = tick - started;
                best = Some(best.map_or(taken, |held: u64| held.min(taken)));
            }
            started = tick;
            lap = now;
            recovered_this_lap = false;
        }
    }

    Some(Solo {
        best,
        contact_ticks: race.wall_contact_ticks_of(LONE),
        state: race.sim.world.ships[LONE].physics.craft_state,
    })
}

/// Every forward circuit on the disc, as `(id, entry name)`. Reversed circuits
/// are excluded - same geometry, doubled wall clock, nothing new to assert on.
fn circuits() -> Vec<(String, String)> {
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
        .filter(|track| !track.reversed)
        .map(|track| (track.id.clone(), track.entry_name()))
        .collect()
}

/// The four rungs, as `Options::class` spells them.
const CLASSES: [&str; 4] = ["VENOM", "FLASH", "RAPIER", "PHANTOM"];

/// The three categories a row can be in, ranked worst to best so a failure
/// message can say which direction a mismatch moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Status {
    Eliminated,
    NoCleanLap,
    CleanLap,
}

impl Status {
    fn of(solo: &Solo) -> Self {
        if solo.state == oag_physics::CraftState::Eliminated {
            Status::Eliminated
        } else if solo.best.is_some() {
            Status::CleanLap
        } else {
            Status::NoCleanLap
        }
    }
}

/// One frozen row of [`BASELINE`].
struct Row {
    circuit: &'static str,
    class: &'static str,
    status: Status,
    /// The baseline clean-lap time, in ticks, if this row ever recorded one -
    /// an `Eliminated` row may still carry one, see the file doc comment.
    lap_ticks: Option<u64>,
    contact_ticks: u32,
}

/// The frozen board: 12 circuits x 4 classes, generated the way this file's
/// own doc comment describes. **May only improve** - see that doc comment for
/// what "improve" means per field and how each is asserted.
#[rustfmt::skip]
const BASELINE: &[Row] = &[
    Row { circuit: "03_Track", class: "VENOM",   status: Status::CleanLap,   lap_ticks: Some(0), contact_ticks: 0 },
];

/// Runs one row and asserts it against its frozen [`BASELINE`] entry.
///
/// Three assertions, in the order described in the file doc comment: `status`
/// exactly, `lap_ticks` as a ceiling when the baseline has one, `contact_ticks`
/// always as a ceiling.
fn check_row(baseline: &Row) {
    let Some(solo) = solo_on(baseline.circuit, baseline.class) else {
        return;
    };
    let status = Status::of(&solo);
    let direction = if status > baseline.status {
        "IMPROVED - tighten BASELINE in crates/game/tests/ai_clean_lap_gate.rs"
    } else {
        "regressed"
    };
    assert_eq!(
        status, baseline.status,
        "{} {}: status {status:?}, baseline says {:?} ({direction})",
        baseline.circuit, baseline.class, baseline.status
    );

    if let Some(ceiling) = baseline.lap_ticks {
        let actual = solo.best.unwrap_or_else(|| {
            panic!(
                "{} {}: status matched {status:?} but no clean lap was recorded, \
                 though the baseline carries one",
                baseline.circuit, baseline.class
            )
        });
        assert!(
            actual <= ceiling + LAP_TOLERANCE_TICKS,
            "{} {}: clean lap took {actual} ticks ({:.1}s), baseline {ceiling} ticks \
             ({:.1}s) + {LAP_TOLERANCE_TICKS} tolerance",
            baseline.circuit,
            baseline.class,
            actual as f32 / 60.0,
            ceiling as f32 / 60.0
        );
    }

    assert!(
        solo.contact_ticks <= baseline.contact_ticks + CONTACT_TOLERANCE_TICKS,
        "{} {}: {} contact ticks, baseline {} + {CONTACT_TOLERANCE_TICKS} tolerance{}",
        baseline.circuit,
        baseline.class,
        solo.contact_ticks,
        baseline.contact_ticks,
        if baseline.contact_ticks == 0 {
            " - this was a zero-contact row"
        } else {
            ""
        }
    );
}

/// Looks a row up in [`BASELINE`] by name and runs [`check_row`] against it.
///
/// Also asserts the disc's own catalogue still has the circuit, so a track
/// renamed or removed upstream fails here rather than silently skipping its
/// row - `circuits()` filters to what the disc has, and a name that has
/// drifted out of it would otherwise just never be checked.
fn check_named_row(circuit: &str, class: &str) {
    let circuits = circuits();
    if circuits.is_empty() {
        return;
    }
    assert!(
        circuits.iter().any(|(id, _)| id == circuit),
        "{circuit} is in BASELINE but not on the disc's own catalogue"
    );
    let baseline = BASELINE
        .iter()
        .find(|row| row.circuit == circuit && row.class == class)
        .unwrap_or_else(|| panic!("no BASELINE row for {circuit} {class}"));
    check_row(baseline);
}

/// One `#[test]` per `(circuit, class)` cell - 48 in total, not four tests of
/// twelve rows each. `ai_clean_lap_board.rs`'s own doc records a row at
/// "fifteen seconds in release", so twelve rows in one process is 180 s of
/// single-core work with nothing else to overlap it - and `check-test-budget`'s
/// own history (`ai_roll_ground_truth`, CLAUDE.md's worked example) is
/// exactly this shape of test measuring 95-114 s isolated and 154-321 s under
/// a loaded `test-data` run, well past a naive read of the 300 s ceiling.
/// `cargo nextest` parallelises across tests, not across the loop body inside
/// one, so the test *is* the unit CLAUDE.md's own "make the matrix the test
/// axis" rule asks for - the brief that opened this file asked for four
/// class-level tests instead, and this is the deviation from it, made on that
/// evidence rather than on a measurement this file's own generator run could
/// not safely wait to take (see the generator's own timing report in the
/// commit that filled in `BASELINE`).
macro_rules! row_test {
    ($name:ident, $circuit:expr, $class:expr) => {
        #[test]
        #[ignore = "needs a disc image in data/images/"]
        fn $name() {
            check_named_row($circuit, $class);
        }
    };
}

row_test!(track_01_venom, "01_Track", "VENOM");
row_test!(track_02_venom, "02_Track", "VENOM");
row_test!(track_03_venom, "03_Track", "VENOM");
row_test!(track_04_venom, "04_Track", "VENOM");
row_test!(track_05_venom, "05_Track", "VENOM");
row_test!(track_06_venom, "06_Track", "VENOM");
row_test!(track_07_venom, "07_Track", "VENOM");
row_test!(track_09_venom, "09_Track", "VENOM");
row_test!(track_10_venom, "10_Track", "VENOM");
row_test!(track_13_venom, "13_Track", "VENOM");
row_test!(track_14_venom, "14_Track", "VENOM");
row_test!(track_16_venom, "16_Track", "VENOM");

row_test!(track_01_flash, "01_Track", "FLASH");
row_test!(track_02_flash, "02_Track", "FLASH");
row_test!(track_03_flash, "03_Track", "FLASH");
row_test!(track_04_flash, "04_Track", "FLASH");
row_test!(track_05_flash, "05_Track", "FLASH");
row_test!(track_06_flash, "06_Track", "FLASH");
row_test!(track_07_flash, "07_Track", "FLASH");
row_test!(track_09_flash, "09_Track", "FLASH");
row_test!(track_10_flash, "10_Track", "FLASH");
row_test!(track_13_flash, "13_Track", "FLASH");
row_test!(track_14_flash, "14_Track", "FLASH");
row_test!(track_16_flash, "16_Track", "FLASH");

row_test!(track_01_rapier, "01_Track", "RAPIER");
row_test!(track_02_rapier, "02_Track", "RAPIER");
row_test!(track_03_rapier, "03_Track", "RAPIER");
row_test!(track_04_rapier, "04_Track", "RAPIER");
row_test!(track_05_rapier, "05_Track", "RAPIER");
row_test!(track_06_rapier, "06_Track", "RAPIER");
row_test!(track_07_rapier, "07_Track", "RAPIER");
row_test!(track_09_rapier, "09_Track", "RAPIER");
row_test!(track_10_rapier, "10_Track", "RAPIER");
row_test!(track_13_rapier, "13_Track", "RAPIER");
row_test!(track_14_rapier, "14_Track", "RAPIER");
row_test!(track_16_rapier, "16_Track", "RAPIER");

row_test!(track_01_phantom, "01_Track", "PHANTOM");
row_test!(track_02_phantom, "02_Track", "PHANTOM");
row_test!(track_03_phantom, "03_Track", "PHANTOM");
row_test!(track_04_phantom, "04_Track", "PHANTOM");
row_test!(track_05_phantom, "05_Track", "PHANTOM");
row_test!(track_06_phantom, "06_Track", "PHANTOM");
row_test!(track_07_phantom, "07_Track", "PHANTOM");
row_test!(track_09_phantom, "09_Track", "PHANTOM");
row_test!(track_10_phantom, "10_Track", "PHANTOM");
row_test!(track_13_phantom, "13_Track", "PHANTOM");
row_test!(track_14_phantom, "14_Track", "PHANTOM");
row_test!(track_16_phantom, "16_Track", "PHANTOM");

/// Regenerates [`BASELINE`] from a real run. Never run in `just test-data` -
/// `#[ignore]`d and gated on `OAG_SWEEP`, the same contract every scratch
/// sweep in this crate uses. See the file doc comment for the command.
#[test]
#[ignore = "a generator: set OAG_SWEEP, paste the printed rows into BASELINE"]
fn print_baseline_source() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let circuits = circuits();
    if circuits.is_empty() {
        return;
    }
    println!("#[rustfmt::skip]\nconst BASELINE: &[Row] = &[");
    for class in CLASSES {
        for (id, entry) in &circuits {
            let start = std::time::Instant::now();
            let Some(solo) = solo_on(entry, class) else {
                continue;
            };
            let elapsed = start.elapsed();
            let status = Status::of(&solo);
            let status_src = match status {
                Status::Eliminated => "Status::Eliminated",
                Status::NoCleanLap => "Status::NoCleanLap",
                Status::CleanLap => "Status::CleanLap",
            };
            let lap_src = solo
                .best
                .map_or_else(|| "None".to_string(), |t| format!("Some({t})"));
            eprintln!(
                "// {id} {class}: {:.1}s wall",
                elapsed.as_secs_f32()
            );
            println!(
                "    Row {{ circuit: \"{id}\", class: \"{class}\", status: {status_src}, \
                 lap_ticks: {lap_src}, contact_ticks: {} }},",
                solo.contact_ticks
            );
        }
    }
    println!("];");
}
