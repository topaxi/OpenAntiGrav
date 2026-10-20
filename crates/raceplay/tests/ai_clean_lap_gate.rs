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
//! the same public `oag_raceplay` surface: three fields instead of eleven,
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
//!   `Eliminated`, asserted **exactly** - not as a ceiling (until 2026-10-20; a
//!   floor since, see the last bullet). `Died` joined on
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
//!   **Since 2026-10-20 it is a floor on the median of five perturbed runs**
//!   (`tests/perturb`), the worst median status over nine offsets: a row that
//!   is a coin between two categories cannot be asserted exactly, so nothing
//!   reads as "improved" any more, and a row moving up is a cue to regenerate.
//! - **`lap_ticks`** (`Option<u64>`): the fastest clean lap, as a **ceiling**
//!   with [`LAP_TOLERANCE_TICKS`] of slack, asserted whenever the baseline
//!   recorded a time - which includes some `Eliminated` rows, because a craft
//!   can lap cleanly once and still die later in the run (`07_Track` at VENOM
//!   is exactly this: `Eliminated`, `lap_ticks` still `Some`). A row regressing
//!   from `Some` to `None` already fails on `status` above; this assertion is
//!   only reached when both sides have a time to compare.
//! - **`contact_ticks`** (`u32`): ticks spent touching a wall while still
//!   `Racing`, also a ceiling, with [`CONTACT_TOLERANCE_TICKS`] of slack. Rows
//!   that baseline at **zero** - four before the speed plan, 38 after it - are
//!   the rows this gate exists to protect most: any wall contact on them is a
//!   regression from perfect, not a matter of degree.
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
//! **Repeat runs have no spread; perturbed runs do.** Since 2026-10-20 the
//! bounds come from the spread of the median over small perturbations of the
//! airborne gravity ([`BASELINE`]'s own doc comment), and the tolerances below
//! sit on top of that.
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
//!   cargo nextest run -p oag-raceplay --run-ignored all \
//!   --no-capture print_baseline_source
//! ```
//!
//! **No `--release`, deliberately** - `just test-data` itself never passes it,
//! the workspace's `[profile.dev.package.*]` already carries `opt-level = 2` for
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
//! **Tightened 2026-10-03 again, lane `pulse-ai-05`**: `05_Track` forward's
//! speed plan verifies now (its line leaves a magstrip on the first jump's
//! run-up), so its four rows drive the plan: contact 68 / 140 / 171 / 232 to
//! zero at every class, laps 2,354 / 2,136 / 1,884 / 1,737 to 2,091 / 1,833 /
//! 1,554 / 1,329 ticks. Every other row regenerated identical.
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
//! **Regenerated 2026-10-03 for the AI speed plan** (`oag_ai::SpeedPlan`,
//! `docs/gameplay/ai.md`, "The speed plan"): the drivers follow a plan learned
//! in our own physics instead of the corner model, wherever the plan verified
//! clean. `Eliminated` rows 11 to 1 (`07_Track` at every class, `01`, `04`,
//! `10` and `13` at RAPIER or PHANTOM are now `CleanLap`), contact ticks
//! summed over the 48 rows 8,828 to 1,243, 38 rows at zero contact. One row
//! laps slower: `10_Track` FLASH 2,050 to 2,160 ticks, with its contact 66 to
//! 0 - the plan brakes where the corner model ground the wall.
//!
//! **Tightened 2026-10-12, lane `ai-rapier-adopt`**: `06_Track` RAPIER takes
//! the hardened trough plan (`docs/gameplay/ai.md`, "Where only the trough plan
//! verifies"), 2301 to 1728 ticks and contact 173 to 0. Only that row was
//! rewritten. The other 47 were left as committed: `ai_speed_plan_sweep`'s lone
//! slices are byte-identical on every other row with and without the change,
//! though a generator run on this change's tree already differs from the table
//! on many of them inside the tolerances.
//!
//! **Regenerated 2026-10-21 for the magstrip escape** (lane `deviations-to-original`): a
//! probe inside a `MagFloor` escapes, as Pulse's `Ship_HoverTwoPoint` gate allows
//! (`oag_physics::hover::MagFloorEscape::Always`). By the noise method over
//! `perturb::BOUND_OFFSETS`, failures of the 100 Pulse AI-gate tests per offset: law off
//! `0 1 0 1 1 2 1 1 1` (median 1, max 2), law on `1 1 1 2 1 2 1 2 1` (median 1, max 2), inside
//! the bar (median and max each at most two above law off). One row carries it: `06_Track`
//! RAPIER, worst median contact `154` to `419`, still `CleanLap`. That is our magnet hold, not
//! our driver: one run of that row escaped off a magstrip 489 times at full blend, with probe
//! heights down to `-0.25` and the craft pinned near `(745, 365, -1)`, where the original's hold
//! kept every probe at least `2.19` above the strip on Talon's Junction
//! (`docs/physics/airborne-ground-truth.md`). Every other row moved inside the tolerances.
//!
//! Before that, [`BASELINE`] was generated this way on commit `8f5070ef` ("Merge: the
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

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_raceplay::catalogue;

use crate::in_parallel;
use crate::perturb;

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
///
/// Loaded once and raced once per scale in [`perturb::GATE`]
/// (`tests/perturb/mod.rs`), in that order: the gate judges the median, not
/// one chaotic run.
fn solos_on(track: &str, class: &str, offset: f32) -> Option<Vec<Solo>> {
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
    Some(
        perturb::GATE
            .iter()
            .map(|&eps| {
                let mut setup = loaded.setup.clone();
                perturb::apply_at(&mut setup, eps, offset);
                solo(setup)
            })
            .collect(),
    )
}

/// One lone race from `setup`.
fn solo(setup: race::Setup) -> Solo {
    let mut race = race::Race::start(setup);
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

    Solo {
        best,
        contact_ticks: race.wall_contact_ticks_of(LONE),
        state: race.sim.world.ships[LONE].physics.craft_state,
        deaths: race.sim.world.ships[LONE].standing.deaths,
    }
}

/// The three columns of one row over its runs, each the median of its own
/// column: the status by rank, the clean lap with a missing one ranked
/// slowest, the contact ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Median {
    status: Status,
    lap_ticks: Option<u64>,
    contact_ticks: u32,
}

impl Median {
    fn of(solos: &[Solo]) -> Self {
        let statuses: Vec<Status> = solos.iter().map(Status::of).collect();
        let laps: Vec<u64> = solos.iter().map(|s| s.best.unwrap_or(u64::MAX)).collect();
        let contacts: Vec<u32> = solos.iter().map(|s| s.contact_ticks).collect();
        let lap = perturb::median(&laps);
        Self {
            status: perturb::median(&statuses),
            lap_ticks: (lap != u64::MAX).then_some(lap),
            contact_ticks: perturb::median(&contacts),
        }
    }
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

/// **13 rows regenerated 2026-10-13 for per-team opponent handling** (lane
/// `hd-opponent-variant-2`): the lone craft is slot 1, which on the default
/// grid now flies Triakis (the slowest `accelcap` and turn, a 6.0 wide hull)
/// instead of the player's hull, on the player's plan re-learned for it
/// (`oag_ai::SpeedPlan::adapt`). One reason per row; "its own plan" is the plan
/// searched from scratch on Triakis, measured on this branch and left to plans
/// computed ahead of the race:
/// - `02`, `03_Track` PHANTOM and `03_Track` RAPIER, slower only: the hull.
///   Its own plan laps them in 1,624, 1,780 and 1,946 ticks.
/// - `07_Track`, all four classes, slower (FLASH and PHANTOM 2 contact ticks):
///   the hull, on the adapted plan, since its own fails on the centre jump at
///   FLASH and PHANTOM. Its own plan laps VENOM in 2,879 and RAPIER in 2,448.
/// - `14_Track` PHANTOM, `Eliminated` as before, slower with more contact: its
///   own plan reads the same, 2,227 ticks and 492.
/// - `02_Track` FLASH, 0 to 266 contact ticks: Triakis scrapes samples
///   1614-1636 on every plan tried (its own 280, adapted 266, adapted with no
///   line slack at all 87); the race driver drifts wide at full lock there
///   where the plan's verification did not. A driver matter, not a plan one.
/// - `02_Track` RAPIER, 0 to 290: the adapted plan; its own plan is clean.
/// - `06_Track` RAPIER, 0 to 43, mostly the standing lap at samples 816-877:
///   the adapted plan (its own search does not verify).
/// - `13_Track` PHANTOM, 0 to 13: 2-3 ticks a lap at samples 1010-1015 on the
///   adapted plan.
/// - `14_Track` RAPIER, 0 to 141: the adapted ring plan at samples 3561-3596
///   and the adapted route plan at 2668-2703 (its own ring plan: 24).
///
/// **3 rows regenerated 2026-10-19 for the absorb half of
/// `WeaponAi_DecideFireOrAbsorb`** (lane `ai-absorb-turbo`): a lone craft now
/// absorbs the Rocket it cannot aim (on one noise schedule, so at tick 1994 on
/// most circuits) and picks up what comes next. One reason per row:
/// - `14_Track` PHANTOM, `Eliminated` to `CleanLap`: the absorbed energy.
///   Shield at the start of laps 2 and 6 110 and 65, against 99 and 37 without
///   it; 17.5 at the end against 0. Contact 492 to 521 is the inversion note
///   above: it drives on.
/// - `02_Track` FLASH, 266 to 306, and `06_Track` RAPIER, 43 to 95: each takes
///   a Turbo and fires it with no wall contact for 600 (02) and 3,400 (06)
///   ticks; the extra contact is new scrapes later in a run that has
///   diverged (02 at ticks 2755-2960, 06 at 14147-14237), not the boost.
///
/// The other 45 rows held inside their ceilings; 30 of them read faster.
///
/// **Rows regenerated for the measured air laws (2026-10-20, lane
/// `driver-vs-new-physics`), each only when the law's distribution showed no
/// shift.** This board flips on noise: with every law off, a constant airborne
/// pitch torque of `1e-5` to `1e-2` (the bias is `8`) regresses 0 to 7 of the 58
/// AI-gate tests. A law may move rows here when, over the same nine
/// perturbations, its regression count has a median at most laws-off + 2 and a
/// max at most laws-off max + 2 (`docs/physics/airborne-ground-truth.md`, "Our AI
/// against the air laws"). Rows moved:
/// - `01_Track` PHANTOM, `CleanLap` to `NoCleanLap`: Pulse's in-air
///   `weight_distribution` torque (with `oag_ai::air_trim`). The same row
///   regresses under 3 of the 9 perturbations with every law off.
/// - Pulse's world-axis inertia diagonal, on top: `01_Track` PHANTOM back to
///   `CleanLap` (1,472 ticks, 27 contact); `06_Track` RAPIER 95 to 156 contact
///   ticks (lap 1,772); `07_Track` RAPIER 0 to 36 (lap 2,467). `06_Track` RAPIER
///   regresses under 4 of the 9 perturbations with every law off.
///
/// **Regenerated 2026-10-20 from a distribution (lane `ai-gate-robustness`).**
/// Each row is now raced once per scale of `perturb::GATE` (every craft's
/// airborne gravity times `1 + eps`, `eps` in `0, 1e-3, -1e-4, 1e-5, -1e-6`;
/// `tests/perturb/mod.rs`) and asserted on the median of its five runs. A
/// median is itself a draw, so each bound is the **worst** of nine medians, one
/// per offset of `perturb::BOUND_OFFSETS` (`0, 2e-3, -2e-5, 3e-6, -5e-4, 5e-5,
/// -3e-6, 7e-4, -7e-6` on top of every run); the trailing comment lists those
/// nine medians as `lap/contact` in that order. Over the nine scales of
/// `perturb::SCALES` the single-run gates had regressed 0, 3, 3, 4, 5, 6, 7, 7
/// and 8 of the 58 AI-gate tests (the force-term sweep in
/// `docs/physics/airborne-ground-truth.md` read median 6, max 8). Against the
/// single-run ceilings before it, 28 rows are no looser and these 20 are
/// (lap, contact, old -> new): `07_Track` VENOM 2956 -> 2964, 0 -> 1; `03_Track`
/// FLASH 2114 -> 2185; `04_Track` FLASH 2064 -> 2074; `06_Track` FLASH
/// 1924 -> 1979, 0 -> 89 (one offset's median); `07_Track` FLASH 2836 -> 2833,
/// 2 -> 13; `03_Track` RAPIER 1940 -> 1952; `04_Track` RAPIER 1768 -> 1789;
/// `14_Track` RAPIER 1959 -> 1961, 141 -> 57; `01_Track` RAPIER 1462 -> 1551,
/// 0 -> 1; `13_Track` RAPIER 2013 -> 1873, 0 -> 1; `06_Track` RAPIER
/// 1772 -> 2693, 156 -> 154 (the plan-selection amplifier: its plan verifies in
/// 4 of the 9 medians, lapping about 1,750, and not in the rest, about 2,690);
/// `07_Track` RAPIER 2467 -> 2777, 36 -> 19; `16_Track` PHANTOM 1636 -> 1664;
/// `03_Track` PHANTOM 1792 -> 1803; `10_Track` PHANTOM 0 -> 1 contact; `05_Track`
/// PHANTOM 1329 -> 1326, 0 -> 4; `04_Track` PHANTOM 1570 -> 1608; `01_Track`
/// PHANTOM 1472 -> 1495, 27 -> 32; `06_Track` PHANTOM 1424 -> 1460, 0 -> 53;
/// `07_Track` PHANTOM 2453 -> 2486, 2 -> 10. The status column is every row's
/// `CleanLap`, now a floor. **Chosen, not measured**: the method is ours.
///
/// The frozen board: 12 circuits x 4 classes, generated the way this file's
/// own doc comment describes. **May only improve** - see that doc comment for
/// what "improve" means per field and how each is asserted.
#[rustfmt::skip]
const BASELINE: &[Row] = &[
    Row { circuit: "16_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2327), contact_ticks: 0 }, // 2327/0 2324/0 2327/0 2325/0 2324/0 2325/0 2325/0 2327/0 2324/0
    Row { circuit: "03_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2325), contact_ticks: 0 }, // 2324/0 2325/0 2325/0 2324/0 2323/0 2325/0 2325/0 2324/0 2325/0
    Row { circuit: "02_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2102), contact_ticks: 0 }, // 2102/0 2100/0 2101/0 2101/0 2102/0 2101/0 2101/0 2101/0 2101/0
    Row { circuit: "10_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2356), contact_ticks: 0 }, // 2355/0 2350/0 2351/0 2350/0 2299/0 2353/0 2353/0 2356/0 2354/0
    Row { circuit: "05_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2007), contact_ticks: 0 }, // 2005/0 2004/0 2004/0 2003/0 2002/0 2007/0 1996/0 2005/0 2004/0
    Row { circuit: "04_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2237), contact_ticks: 0 }, // 2234/0 2237/0 2234/0 2234/0 2233/0 2234/0 2235/0 2233/0 2235/0
    Row { circuit: "09_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2688), contact_ticks: 0 }, // 2685/0 2681/0 2685/0 2683/0 2682/0 2688/0 2685/0 2684/0 2683/0
    Row { circuit: "14_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2530), contact_ticks: 0 }, // 2520/0 2530/0 2524/0 2523/0 2523/0 2522/0 2521/0 2521/0 2521/0
    Row { circuit: "01_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(1917), contact_ticks: 0 }, // 1904/0 1891/0 1904/0 1904/0 1906/0 1905/0 1902/0 1901/0 1917/0
    Row { circuit: "13_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2216), contact_ticks: 0 }, // 2208/0 2209/0 2207/0 2207/0 2213/0 2216/0 2212/0 2212/0 2212/0
    Row { circuit: "06_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2116), contact_ticks: 4 }, // 2096/0 2098/0 2116/0 2110/0 2109/0 2089/4 2099/0 2089/0 2093/0
    Row { circuit: "07_Track", class: "VENOM", status: Status::CleanLap, lap_ticks: Some(2958), contact_ticks: 0 }, // 2949/0 2950/0 2958/0 2941/0 2862/0 2950/0 2954/0 2942/0 2938/0
    Row { circuit: "16_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2083), contact_ticks: 0 }, // 2081/0 2082/0 2083/0 2081/0 2081/0 2081/0 2079/0 2080/0 2082/0
    Row { circuit: "03_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2185), contact_ticks: 0 }, // 2177/0 2185/0 2183/0 2183/0 2180/0 2178/0 2184/0 2183/0 2176/0
    Row { circuit: "02_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(1913), contact_ticks: 30 }, // 1907/19 1910/29 1913/30 1910/25 1909/27 1911/25 1899/28 1909/28 1902/29
    Row { circuit: "10_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2139), contact_ticks: 0 }, // 2139/0 2136/0 2134/0 2137/0 2134/0 2134/0 2134/0 2134/0 2132/0
    Row { circuit: "05_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(1760), contact_ticks: 0 }, // 1760/0 1760/0 1760/0 1759/0 1758/0 1758/0 1758/0 1760/0 1758/0
    Row { circuit: "04_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2074), contact_ticks: 0 }, // 2069/0 2068/0 2069/0 2069/0 2070/0 2069/0 2069/0 2074/0 2070/0
    Row { circuit: "09_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2383), contact_ticks: 0 }, // 2379/0 2374/0 2383/0 2381/0 2377/0 2380/0 2382/0 2379/0 2383/0
    Row { circuit: "14_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2202), contact_ticks: 0 }, // 2202/0 2195/0 2201/0 2201/0 2201/0 2200/0 2202/0 2200/0 2200/0
    Row { circuit: "01_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(1668), contact_ticks: 3 }, // 1666/2 1667/2 1666/3 1666/1 1665/0 1667/1 1666/2 1668/1 1665/1
    Row { circuit: "13_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2003), contact_ticks: 0 }, // 1997/0 2002/0 1998/0 2001/0 2003/0 1991/0 1998/0 1998/0 2001/0
    Row { circuit: "06_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(1930), contact_ticks: 0 }, // 1913/0 1930/0 1914/0 1918/0 1909/0 1922/0 1929/0 1918/0 1923/0
    Row { circuit: "07_Track", class: "FLASH", status: Status::CleanLap, lap_ticks: Some(2837), contact_ticks: 7 }, // 2837/2 2802/0 2828/3 2834/0 2819/0 2818/2 2828/7 2823/5 2824/1
    Row { circuit: "16_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(1789), contact_ticks: 0 }, // 1787/0 1789/0 1787/0 1786/0 1785/0 1785/0 1785/0 1784/0 1781/0
    Row { circuit: "03_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(1952), contact_ticks: 0 }, // 1952/0 1944/0 1952/0 1950/0 1945/0 1947/0 1949/0 1948/0 1947/0
    Row { circuit: "02_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(1696), contact_ticks: 0 }, // 1692/0 1688/0 1685/0 1685/0 1688/0 1696/0 1693/0 1695/0 1693/0
    Row { circuit: "10_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(1882), contact_ticks: 0 }, // 1870/0 1882/0 1870/0 1878/0 1882/0 1877/0 1881/0 1880/0 1876/0
    Row { circuit: "05_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(1504), contact_ticks: 11 }, // 1503/1 1502/0 1503/0 1503/0 1503/11 1504/0 1504/1 1502/0 1504/1
    Row { circuit: "04_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(1789), contact_ticks: 0 }, // 1787/0 1788/0 1786/0 1789/0 1787/0 1788/0 1787/0 1786/0 1788/0
    Row { circuit: "09_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(2059), contact_ticks: 0 }, // 2055/0 2055/0 2059/0 2059/0 2055/0 2055/0 2058/0 2059/0 2056/0
    Row { circuit: "14_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(1961), contact_ticks: 57 }, // 1931/48 1928/28 1930/24 1928/30 1930/25 1931/53 1930/28 1961/57 1929/19
    Row { circuit: "01_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(1550), contact_ticks: 2 }, // 1445/1 1446/1 1455/0 1455/1 1447/1 1446/0 1449/0 1550/2 1443/2
    Row { circuit: "13_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(1873), contact_ticks: 2 }, // 1871/0 1864/0 1869/0 1864/2 1873/0 1871/0 1868/1 1869/0 1808/0
    Row { circuit: "06_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(2693), contact_ticks: 419 }, // 1767/303 1850/419 1877/327 1882/324 1787/122 1764/304 2693/147 2686/146 1849/160
    Row { circuit: "07_Track", class: "RAPIER", status: Status::CleanLap, lap_ticks: Some(2776), contact_ticks: 21 }, // 2750/17 2776/3 2459/21 2766/16 2468/2 2470/18 2756/16 2762/16 2485/19
    Row { circuit: "16_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(1667), contact_ticks: 0 }, // 1653/0 1660/0 1667/0 1657/0 1664/0 1656/0 1655/0 1654/0 1661/0
    Row { circuit: "03_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(1803), contact_ticks: 0 }, // 1793/0 1796/0 1796/0 1795/0 1803/0 1790/0 1791/0 1789/0 1792/0
    Row { circuit: "02_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(1608), contact_ticks: 0 }, // 1595/0 1601/0 1603/0 1597/0 1593/0 1599/0 1608/0 1599/0 1597/0
    Row { circuit: "10_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(1778), contact_ticks: 1 }, // 1777/0 1777/0 1778/0 1776/1 1775/1 1775/1 1774/1 1778/0 1772/1
    Row { circuit: "05_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(1327), contact_ticks: 4 }, // 1325/2 1327/3 1326/0 1326/0 1326/0 1325/0 1326/2 1326/4 1326/3
    Row { circuit: "04_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(1608), contact_ticks: 0 }, // 1581/0 1581/0 1594/0 1600/0 1585/0 1587/0 1603/0 1608/0 1586/0
    Row { circuit: "09_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(1833), contact_ticks: 0 }, // 1832/0 1827/0 1833/0 1830/0 1833/0 1831/0 1828/0 1829/0 1830/0
    Row { circuit: "14_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(2001), contact_ticks: 272 }, // 1978/267 1978/263 1991/245 2001/266 1975/269 1999/0 1980/272 1983/252 1981/256
    Row { circuit: "01_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(1466), contact_ticks: 31 }, // 1450/13 1306/6 1466/31 1315/6 1310/22 1314/8 1419/6 1306/6 1313/3
    Row { circuit: "13_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(1841), contact_ticks: 0 }, // 1836/0 1838/0 1841/0 1839/0 1838/0 1838/0 1839/0 1834/0 1835/0
    Row { circuit: "06_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(1463), contact_ticks: 33 }, // 1453/1 1463/0 1452/3 1460/21 1449/0 1451/0 1458/0 1460/2 1457/33
    Row { circuit: "07_Track", class: "PHANTOM", status: Status::CleanLap, lap_ticks: Some(2483), contact_ticks: 14 }, // 2467/1 2468/5 2463/6 2470/9 2468/11 2475/14 2456/3 2483/9 2466/11
];

/// Runs one row over [`perturb::GATE`] and asserts its [`Median`] against its
/// frozen [`BASELINE`] entry.
///
/// `entry` is the WAD entry name [`solos_on`] needs - see its own doc comment
/// for why this takes one instead of resolving `baseline.circuit` itself.
///
/// Three assertions, in the order described in the file doc comment: `status`
/// exactly, `lap_ticks` as a ceiling when the baseline has one, `contact_ticks`
/// always as a ceiling, each on the median of the runs. A failure lists every
/// run, so a row at the edge of its distribution reads as one.
fn check_row(baseline: &Row, entry: &str) {
    let Some(solos) = solos_on(entry, baseline.class, perturb::offset()) else {
        return;
    };
    let median = Median::of(&solos);
    let runs = runs_text(&solos);
    let status = median.status;
    assert!(
        status >= baseline.status,
        "{} {}: median status {status:?}, worse than the baseline's {:?}; runs {runs}",
        baseline.circuit,
        baseline.class,
        baseline.status
    );

    if let Some(ceiling) = baseline.lap_ticks {
        let actual = median.lap_ticks.unwrap_or_else(|| {
            panic!(
                "{} {}: status matched {status:?} but the median run has no clean lap, \
                 though the baseline carries one; runs {runs}",
                baseline.circuit, baseline.class
            )
        });
        assert!(
            actual <= ceiling + LAP_TOLERANCE_TICKS,
            "{} {}: median clean lap {actual} ticks ({:.1}s), baseline {ceiling} ticks \
             ({:.1}s) + {LAP_TOLERANCE_TICKS} tolerance; runs {runs}",
            baseline.circuit,
            baseline.class,
            actual as f32 / 60.0,
            ceiling as f32 / 60.0
        );
    }

    assert!(
        median.contact_ticks <= baseline.contact_ticks + CONTACT_TOLERANCE_TICKS,
        "{} {}: median {} contact ticks, baseline {} + {CONTACT_TOLERANCE_TICKS} tolerance{}; \
         runs {runs}",
        baseline.circuit,
        baseline.class,
        median.contact_ticks,
        baseline.contact_ticks,
        if baseline.contact_ticks == 0 {
            " - this was a zero-contact row"
        } else {
            ""
        }
    );
}

/// Every run of a row as `status/lap/contact`, in [`perturb::GATE`] order.
fn runs_text(solos: &[Solo]) -> String {
    solos
        .iter()
        .map(|solo| {
            format!(
                "{:?}/{}/{}",
                Status::of(solo),
                solo.best.map_or("-".to_string(), |t| t.to_string()),
                solo.contact_ticks
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Looks a row up in [`BASELINE`] by name, resolves its WAD entry name off
/// the disc's own catalogue, and runs [`check_row`] against it.
///
/// Resolving through `circuits()` rather than reconstructing the entry name
/// from the id is deliberate: it is the same lookup `check_named_row`'s own
/// membership assertion already needs, and it means a track renamed or
/// removed upstream fails loudly here instead of `solos_on` silently loading
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

/// Regenerates [`BASELINE`] from a real run: every row's [`Median`] over
/// [`perturb::GATE`], with its runs as a trailing comment. Never run in
/// `just test-data` - `#[ignore]`d and gated on `OAG_SWEEP`, the same contract
/// every scratch sweep in this crate uses. See the file doc comment for the
/// command. Rows run side by side (`in_parallel`), twelve at a time.
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
    let cells: Vec<(&str, &String, &String)> = CLASSES
        .iter()
        .flat_map(|&class| circuits.iter().map(move |(id, entry)| (class, id, entry)))
        .collect();
    let jobs: Vec<(usize, f32)> = (0..cells.len())
        .flat_map(|cell| {
            perturb::BOUND_OFFSETS
                .iter()
                .map(move |&offset| (cell, offset))
        })
        .collect();
    let medians = in_parallel::map(&jobs, 16, |&(cell, offset)| {
        let (class, _, entry) = cells[cell];
        Median::of(&solos_on(entry, class, offset).expect("the image is present"))
    });
    println!("#[rustfmt::skip]\nconst BASELINE: &[Row] = &[");
    for (k, (class, id, _)) in cells.iter().enumerate() {
        let row = &medians[k * perturb::BOUND_OFFSETS.len()..][..perturb::BOUND_OFFSETS.len()];
        let worst = Median {
            status: row.iter().map(|m| m.status).min().expect("offsets"),
            lap_ticks: row
                .iter()
                .map(|m| m.lap_ticks.unwrap_or(u64::MAX))
                .max()
                .filter(|&t| t != u64::MAX),
            contact_ticks: row.iter().map(|m| m.contact_ticks).max().expect("offsets"),
        };
        let status_src = match worst.status {
            Status::Eliminated => "Status::Eliminated",
            Status::Died => "Status::Died",
            Status::NoCleanLap => "Status::NoCleanLap",
            Status::CleanLap => "Status::CleanLap",
        };
        let lap_src = worst
            .lap_ticks
            .map_or_else(|| "None".to_string(), |t| format!("Some({t})"));
        let spread: Vec<String> = row
            .iter()
            .map(|m| {
                format!(
                    "{}/{}",
                    m.lap_ticks.map_or("-".to_string(), |t| t.to_string()),
                    m.contact_ticks
                )
            })
            .collect();
        println!(
            "    Row {{ circuit: \"{id}\", class: \"{class}\", status: {status_src}, \
             lap_ticks: {lap_src}, contact_ticks: {} }}, // {}",
            worst.contact_ticks,
            spread.join(" ")
        );
    }
    println!("];");
}
