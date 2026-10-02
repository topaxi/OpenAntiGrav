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
//! each other's items - and it is owned by another, still-open thread.
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
//! example: every class here reads it `Eliminated` (`Died`, 2026-09-16 to
//! 2026-10-01, while a wreck came back), but before this project
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
//! - **`status`** ([`Status`]): `CleanLap`, `NoCleanLap`, `Died` or
//!   `Eliminated`, asserted **exactly** - not as a ceiling. `Died` joined on
//!   2026-09-16, when a wrecked opponent started coming back after the
//!   original's own 2.3 s: every row that read `Eliminated` before then reads
//!   `Died` now, for the same driving, and `Eliminated` is reserved for a
//!   craft dead at the run's end. **Reverted 2026-10-02**: the original's wrecked
//!   opponent does not come back (state 6, measured live), so the ten `Died`
//!   rows read `Eliminated` again, for the same driving - a craft that dies now
//!   stays dead, and `Died` is no longer reachable outside the Eliminator. The same regeneration
//!   (`print_baseline_source`) dropped `contact_ticks` on eleven rows, from 283-937 to 179-602, because a craft
//!   that dies early no longer drives on to touch more wall - read the inversion note below before
//!   treating one of those ceilings as a bar a *surviving* craft must stay under. A status is a category, not a
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
//! forbids. **Checked, not assumed**: `print_baseline_source` was run twice on
//! commit `8f5070ef` (158.9s, then 157.7s) and the two 48-row outputs `diff`
//! byte-for-byte identical - every `lap_ticks` and every `contact_ticks`, not
//! only the ones that made it into [`BASELINE`]. Spread was zero.
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
//! **Regenerated 2026-09-16**, when a wrecked opponent started coming back:
//! eleven rows moved from `Eliminated` to `Died`, two from `NoCleanLap` to
//! `Died` (`01_Track` at RAPIER and PHANTOM - those craft had been dying all
//! along and the stall rescue was quietly reviving them at an empty pool,
//! which `Race::step_opponents` no longer does for a wreck), and the `Died`
//! rows' `contact_ticks` grew because a craft that comes back keeps driving,
//! and grinding, for the rest of the five minutes. Eight clean-lap times
//! moved by one tick either way against a baseline cut nine days earlier,
//! inside the tolerance, and are re-pinned here at their current values.
//!
//! **Regenerated 2026-09-29**, when the grid moved onto the AI corridor's
//! midpoint (`docs/ghidra/functions/psp-pulse-usa/grid.md`): the lone craft's
//! slot-1 start moved about 1.8 units laterally on most circuits (30 on
//! `01_Track`, where it used to start past the corridor edge and fall), so
//! every outcome sensitive to the start moved with it. Status changes, and what
//! was traced of each (`lone_craft_respawns` in `grid_stagger_ground_truth.rs`):
//!
//! - `01_Track` FLASH `CleanLap` to `NoCleanLap`: a `ResetZone` at spline index
//!   42, five times - the slow crossing of the `01_Track` hole
//!   `docs/gameplay/leaving-the-track.md` already documents (index ~31-42, a
//!   slower class falls through it, Ace at speed clears it).
//! - `05_Track` VENOM `CleanLap` to `Died`: `LostCircuit` at index 524 and 108,
//!   then a wreck at 620. RAPIER `CleanLap` to `Died`: `LostCircuit` at 114,
//!   `Stalled` at 652, a wreck at 2401. FLASH `Died` to `CleanLap`: one
//!   `LostCircuit` at 73. The mechanism behind any of them is **not traced**;
//!   `05_Track` is the circuit with 134 racing-line samples over no collision
//!   (`docs/gameplay/ai.md`).
//! - `13_Track` RAPIER (`Died`, lap time `Some` to `None`) and PHANTOM (`Died`,
//!   `None` to `Some`): repeated `LostCircuit` at index 1286-1565 in both, as
//!   before the change. **Not traced.**
//!
//! Nothing here is a claim that the driving got better or worse; it is the same
//! driver from a start that is now where the original puts it. The other rows
//! moved by a few ticks.
//!
//! **Regenerated 2026-09-30**, for a physics fix rather than a driver change:
//! `oag_physics::airbrake::evaluate`'s forward `drag` term read the raw
//! `steerX` on `-1..=1` where the original holds it on `+/-100`, so it ran
//! 100x weak (commit `b60d7505`; `engine.md`, "The raw steerX is on the 100
//! scale too"). A craft braking one side into a corner now keeps the forward
//! push the original gives it, and the Ace, tuned against the weak term,
//! arrives at corners faster. Status changes: `05_Track` FLASH and VENOM and
//! `14_Track` RAPIER `CleanLap` to `Died`; `13_Track` RAPIER stays `Died` and
//! loses its clean lap. Contact ticks rose on eleven rows. **Not traced
//! further**: the physics moved toward the capture, and retuning the driver
//! to it is the AI lane's work.
//!
//! **Regenerated again 2026-09-30, for a driver change**: `Line::curvature`
//! no longer reads the foot of a hill as a corner (`bend_angle`; `docs/gameplay/
//! ai.md`, "A valley is not a corner"). The three rows the airbrake re-record
//! above lost come back - `05_Track` VENOM and FLASH and `14_Track` RAPIER
//! `Died` to `CleanLap` - because the Ace no longer brakes 7 units/s at the
//! foot of `05_Track`'s hill and arrives at its crest lip above the ~70 units/s
//! that clears it. One row reads worse: `07_Track` FLASH `Died` to
//! `Eliminated` (contact 864 to 869), which is **a window artefact, traced**:
//! both trees wreck once mid-run (ticks 9539 and 9266) and the new one wrecks a
//! second time at tick 17834, 166 ticks before the 18,000-tick window closes,
//! and a respawn takes 167. It is the same drive with the wreck landing on the
//! wrong side of the cut. Clean-lap times fell by 1-2 s on the circuits with a valley in
//! them (`06_Track` and `09_Track` by up to 130 ticks), and `13_Track` RAPIER
//! and PHANTOM took more contact (1129 to 1555, 641 to 1347; both `Died`
//! before and after). Totals over the 48 rows: contact 17357 to 13834
//! (13340 before the airbrake change), `Died` rows 14 to 10 (11 before it).
//!
//! # Regenerating [`BASELINE`]
//!
//! Never by hand. [`print_baseline_source`] below runs the same 48 rows this
//! table freezes and prints them as a Rust literal ready to paste in.
//!
//! ```sh
//! flock "$HOME/.cache/oag/gate.lock" env OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 \
//!   cargo nextest run -p oag-game --run-ignored all \
//!   --no-capture print_baseline_source
//! ```
//!
//! **No `--release`, deliberately** - `just test-data` itself never passes it,
//! `oag-game`'s `[profile.dev.package.*]` already carries `opt-level = 2` for
//! every simulation crate this reaches, and generating under the same profile
//! the gate actually runs under means no separate dev-vs-release cross-check
//! is needed. It also turned out to be fast: all 48 rows in **158.9s**
//! (`~3.3s` a row on average) on this run, well under the "fifteen seconds [a
//! row] in release" `ai_clean_lap_board.rs`'s own doc comment states - either
//! `opt-level = 2` without a full release build's LTO is close enough to
//! release for this workload, or the machine's load happened to be lower for
//! this particular run; not resolved further since it does not change any
//! decision here.
//!
//! **Regenerated 2026-10-01 for the grid state** (`docs/physics/grid-state.md`):
//! a craft on the grid no longer gets the bank-to-yaw coupling, so it holds its
//! heading through the countdown as the original's does, and each Pulse PSP grid
//! slot takes the track's own frame instead of the node's heading. Every row's
//! start moved by a fraction of a degree and a lone Ace is a chaotic system, so
//! the board moved both ways: total contact `13834` to `12302` (`13_Track` RAPIER
//! `1555` to `886`, PHANTOM `1347` to `631`), `07_Track` FLASH `Eliminated` to
//! `Died` (better), and **one row regressed: `04_Track` RAPIER `CleanLap` to
//! `Died`**, a single death at tick 17714 of 18000 (lap 5, stopped, shield `95`)
//! after four clean laps, with `lap_ticks` still `Some`. Not tuned around; read
//! against `race_ground_truth`'s twelve-circuit gate, which stays clean.
//!
//! **Regenerated 2026-10-01 for the launch boost** (`docs/physics/launch-boost.md`),
//! for the grid state ending one tick before thrust is released, and for a respawn
//! carrying the launch state across (`Ship::place_at`): every craft, the AI included,
//! now gets the thrust multiplier its first-thrust time earns for the first second (an
//! AI thrusting at GO is graded stall, `1.2x`), so a lone Ace leaves the line faster
//! and all 48 rows moved by a few ticks either way. Net against the previous table:
//! contact `12302` to `12349` over the 48 rows; **`04_Track` RAPIER `Died` to `CleanLap`**
//! (the row the grid-state regeneration regressed comes back); **`07_Track` FLASH `Died`
//! to `Eliminated`** (contact `842` to `871`, `lap_ticks` still `Some`: dead at the end
//! of the window instead of recovered, which is `Eliminated`'s own definition and the
//! window-artefact shape the grid-state note describes - not traced this time);
//! `13_Track` PHANTOM `lap_ticks` `None` to `Some(1965)` (`Died` both). Not tuned.
//!
//! **Regenerated 2026-10-03 for de Konstruct Black** (`05_Track`, the forward
//! layout): the crest a craft launches off, on the run-up to a 40+-sample gap, no
//! longer reads as a corner (`Line::curvature`, yaw only there) and the caution lift
//! is skipped on it. Only the four `05_Track` rows moved, every other row
//! reproduced exactly: FLASH `Eliminated` to `CleanLap` (1,700 contact ticks to 140),
//! VENOM, RAPIER and PHANTOM laps 26-35 ticks quicker, and **PHANTOM's contact
//! up 203 to 232**, a row that is clean either way and not tuned around.
//!
//! **Regenerated 2026-10-02 for the grid walk** (`docs/physics/grid-state.md`,
//! `oag_gameplay::grid_walk`): every Pulse PSP slot now sits where
//! `Race_ComputeGridLayout` puts it - up to 1.7 units from where the sawtooth put it - and
//! faces the way the track's edges run, so a lone Ace starts a fraction of a unit and of a
//! degree somewhere else on every row. Twelve rows failed their old numbers; the other 36
//! moved by a few ticks inside the tolerances. **Three rows regressed in status or
//! lap time and are not tuned around**: `05_Track` FLASH `CleanLap` to `Eliminated` (dead
//! at the end of the window, `lap_ticks` still `Some`, and its wreck settled against a
//! wall for 1,700 contact ticks, which is `1591` of the table's `+1342`),
//! and `13_Track` RAPIER and PHANTOM, `Eliminated` before and after but with `lap_ticks`
//! `Some(1903)`/`Some(1965)` to `None` (no clean lap this time). Net of those three the other
//! 45 rows' contact moved by `-47` in total and the twelve VENOM rows (eleven `CleanLap`
//! and `07_Track` `Eliminated`, before and after) by `-29`: a different start, not a worse one, in the sense of the two
//! earlier regenerations above (`04_Track` RAPIER, the same chaotic shape). Read against
//! `race_ground_truth`'s twelve-circuit gate, which stays clean.
//!
//! [`BASELINE`] below was generated this way on commit `8f5070ef` ("Merge: the
//! two changes only pay together, and 05_Track's line blocks them"),
//! 2026-09-12, and cross-checked field by field against a board printed
//! separately by `ai_clean_lap_board.rs`'s own sweep on an ancestor of the
//! same commit, recorded in the open thread that owns that file - **every
//! one of the 48 rows agrees**: status, `contact_ticks` exactly, and
//! `lap_ticks` to the second once divided by 60 (that board only prints
//! seconds to one decimal, which is why this file has its own generator
//! rather than transcribing the printed one). Two independent
//! implementations of the same scenario landing on identical numbers is
//! itself evidence the reimplementation in this file is measuring the same
//! thing the existing board does, not a stronger determinism claim - that
//! one is checked below in the tolerance section.

use std::path::PathBuf;

use oag_game::{catalogue, race};
use oag_gameplay::PlayerInputs;

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
    /// How many times it was destroyed and came back.
    deaths: u32,
}

/// One craft, alone, on one circuit at one class, exactly the scenario
/// `ai_clean_lap_board.rs::solo_on` measures. See that file for the fuller
/// version with the driver-index clustering this gate does not need.
///
/// `track` must be a WAD entry name (`circuits()`'s second tuple element),
/// **not** a bare circuit id - the same trap `ai_clean_lap_board.rs::solo_on`
/// warns about. Passing a bare id used to fail `race::load` silently here:
/// `.ok()?` turned that failure into the same `None` the "no disc image"
/// skip returns, so every `row_test!` passed by never actually asserting
/// anything - caught by running the tests, not by reading the code. `race::load`
/// now panics loudly on any failure other than a missing image, so a load
/// error can never again read as a green, empty test again.
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
    .unwrap_or_else(|error| panic!("loading {track} at {class}: {error}"));
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
        race.tick(&PlayerInputs::none());
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
        deaths: race.sim.world.ships[LONE].standing.deaths,
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
    /// Dead when the five minutes ran out.
    Eliminated,
    /// Destroyed at least once and back on the track. **Unreachable outside
    /// the Eliminator since 2026-10-02**: a wrecked opponent stays down (the
    /// original's state 6, measured live), so a death now reads `Eliminated`.
    /// It was what `Eliminated` used to catch from 2026-09-16 to 2026-10-01,
    /// when a wreck came back after 2.3 s. Ranked below a craft that never
    /// died, whatever its laps: dying is the thing this gate most wants to see.
    Died,
    NoCleanLap,
    CleanLap,
}

impl Status {
    fn of(solo: &Solo) -> Self {
        if solo.state == oag_physics::CraftState::Eliminated {
            Status::Eliminated
        } else if solo.deaths > 0 {
            Status::Died
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
    Row { circuit: "16_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2506), contact_ticks: 52 },
    Row { circuit: "03_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2572), contact_ticks: 0 },
    Row { circuit: "02_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2585), contact_ticks: 3 },
    Row { circuit: "10_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2332), contact_ticks: 1 },
    Row { circuit: "05_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2354), contact_ticks: 68 },
    Row { circuit: "04_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2349), contact_ticks: 76 },
    Row { circuit: "09_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2996), contact_ticks: 21 },
    Row { circuit: "14_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2744), contact_ticks: 8 },
    Row { circuit: "01_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2225), contact_ticks: 182 },
    Row { circuit: "13_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2283), contact_ticks: 317 },
    Row { circuit: "06_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2642), contact_ticks: 97 },
    Row { circuit: "07_Track", class: "VENOM", status: Status::Eliminated, lap_ticks: Some(2978), contact_ticks: 399 },
    Row { circuit: "16_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2277), contact_ticks: 96 },
    Row { circuit: "03_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2326), contact_ticks: 0 },
    Row { circuit: "02_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2377), contact_ticks: 23 },
    Row { circuit: "10_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2050), contact_ticks: 66 },
    Row { circuit: "05_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2136), contact_ticks: 140 },
    Row { circuit: "04_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2107), contact_ticks: 94 },
    Row { circuit: "09_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2718), contact_ticks: 28 },
    Row { circuit: "14_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2431), contact_ticks: 77 },
    Row { circuit: "01_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(1989), contact_ticks: 310 },
    Row { circuit: "13_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2069), contact_ticks: 494 },
    Row { circuit: "06_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2440), contact_ticks: 110 },
    Row { circuit: "07_Track", class: "FLASH", status: Status::Eliminated, lap_ticks: Some(2837), contact_ticks: 441 },
    Row { circuit: "16_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(2061), contact_ticks: 200 },
    Row { circuit: "03_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(2113), contact_ticks: 4 },
    Row { circuit: "02_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(2207), contact_ticks: 116 },
    Row { circuit: "10_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(1842), contact_ticks: 205 },
    Row { circuit: "05_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(1884), contact_ticks: 171 },
    Row { circuit: "04_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(1870), contact_ticks: 300 },
    Row { circuit: "09_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(2478), contact_ticks: 228 },
    Row { circuit: "14_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(2140), contact_ticks: 380 },
    Row { circuit: "01_Track", class: "RAPIER", status: Status::Eliminated, lap_ticks: Some(1734), contact_ticks: 316 },
    Row { circuit: "13_Track", class: "RAPIER", status: Status::Eliminated, lap_ticks: None, contact_ticks: 350 },
    Row { circuit: "06_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(2301), contact_ticks: 173 },
    Row { circuit: "07_Track", class: "RAPIER", status: Status::Eliminated, lap_ticks: Some(2629), contact_ticks: 343 },
    Row { circuit: "16_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(1950), contact_ticks: 220 },
    Row { circuit: "03_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(2033), contact_ticks: 67 },
    Row { circuit: "02_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(2099), contact_ticks: 135 },
    Row { circuit: "10_Track", class: "PHANTOM", status: Status::Eliminated, lap_ticks: Some(1790), contact_ticks: 225 },
    Row { circuit: "05_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(1737), contact_ticks: 232 },
    Row { circuit: "04_Track", class: "PHANTOM", status: Status::Eliminated, lap_ticks: Some(1771), contact_ticks: 296 },
    Row { circuit: "09_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(2336), contact_ticks: 331 },
    Row { circuit: "14_Track", class: "PHANTOM", status: Status::Eliminated, lap_ticks: Some(1989), contact_ticks: 336 },
    Row { circuit: "01_Track", class: "PHANTOM", status: Status::Eliminated, lap_ticks: Some(1636), contact_ticks: 276 },
    Row { circuit: "13_Track", class: "PHANTOM", status: Status::Eliminated, lap_ticks: None, contact_ticks: 274 },
    Row { circuit: "06_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(2261), contact_ticks: 226 },
    Row { circuit: "07_Track", class: "PHANTOM", status: Status::Eliminated, lap_ticks: Some(2547), contact_ticks: 321 },
];

/// Runs one row and asserts it against its frozen [`BASELINE`] entry.
///
/// `entry` is the WAD entry name [`solo_on`] needs - see its own doc comment
/// for why this takes one instead of resolving `baseline.circuit` itself.
///
/// Three assertions, in the order described in the file doc comment: `status`
/// exactly, `lap_ticks` as a ceiling when the baseline has one, `contact_ticks`
/// always as a ceiling.
fn check_row(baseline: &Row, entry: &str) {
    let Some(solo) = solo_on(entry, baseline.class) else {
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

/// Looks a row up in [`BASELINE`] by name, resolves its WAD entry name off
/// the disc's own catalogue, and runs [`check_row`] against it.
///
/// Resolving through `circuits()` rather than reconstructing the entry name
/// from the id is deliberate: it is the same lookup `check_named_row`'s own
/// membership assertion already needs, and it means a track renamed or
/// removed upstream fails loudly here instead of `solo_on` silently loading
/// nothing for a path that no longer exists.
fn check_named_row(circuit: &str, class: &str) {
    let circuits = circuits();
    if circuits.is_empty() {
        return;
    }
    let entry = circuits
        .iter()
        .find(|(id, _)| id == circuit)
        .unwrap_or_else(|| panic!("{circuit} is in BASELINE but not on the disc's own catalogue"))
        .1
        .clone();
    let baseline = BASELINE
        .iter()
        .find(|row| row.circuit == circuit && row.class == class)
        .unwrap_or_else(|| panic!("no BASELINE row for {circuit} {class}"));
    check_row(baseline, &entry);
}

/// One `#[test]` per `(circuit, class)` cell - 48 in total, not the four
/// class-level tests (twelve rows each) the brief that opened this file
/// asked for. The decision predates the measurement below and was made on
/// `ai_clean_lap_board.rs`'s own doc comment - a row at "fifteen seconds in
/// release" makes twelve rows in one process 180s of single-core work with
/// nothing else to overlap it, and `scripts/check-test-budget.py`'s own
/// history (`ai_roll_ground_truth`, CLAUDE.md's worked example) is exactly
/// this failure shape: 95-114s isolated measuring 154-321s under a loaded
/// `test-data` run. `cargo nextest` parallelises across tests, not across a
/// loop body inside one, so the test *is* the unit CLAUDE.md's "make the
/// matrix the test axis" rule asks for.
///
/// **The generator's own measurement, taken afterward, says a class-level
/// design would also have fit**: 48 rows in one process took 158.9s and then
/// 157.7s on repeat (`print_baseline_source`'s doc comment above), so a
/// twelve-row class test would cost roughly a quarter of that - about 40s,
/// comfortably under 300s even at the 2-3x load inflation `ai_roll` measured.
/// The 48-test split is kept anyway: it is the design CLAUDE.md's own worked
/// example asks for regardless of margin, and a failing row now names its own
/// circuit and class instead of failing eleven unrelated ones alongside it.
/// Reported here rather than silently overridden, since it means the
/// decision no longer rests only on the pre-measurement reasoning above.
///
/// **A second measurement, of the 48 tests actually run this way**: all 48
/// plus the generator, under `cargo nextest run`'s own default concurrency
/// (no `--no-capture` forcing serial execution) on this machine at the time,
/// individual rows landed at 9.4-12.5s each - three to four times the
/// generator's serial 3.3s/row, which is contention between the 48 tests
/// themselves rather than anything outside this file, and the total wall
/// clock was 33.5s. Both numbers sit far under the ceilings that matter
/// (300s a test, 450s the suite), with enough margin that neither the
/// per-test contention seen here nor the further load `ai_roll_ground_truth`
/// records elsewhere should threaten them.
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
                Status::Died => "Status::Died",
                Status::NoCleanLap => "Status::NoCleanLap",
                Status::CleanLap => "Status::CleanLap",
            };
            let lap_src = solo
                .best
                .map_or_else(|| "None".to_string(), |t| format!("Some({t})"));
            eprintln!("// {id} {class}: {:.1}s wall", elapsed.as_secs_f32());
            println!(
                "    Row {{ circuit: \"{id}\", class: \"{class}\", status: {status_src}, \
                 lap_ticks: {lap_src}, contact_ticks: {} }},",
                solo.contact_ticks
            );
        }
    }
    println!("];");
}
