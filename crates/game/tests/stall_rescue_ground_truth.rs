//! A craft that stops on a real circuit gets picked up - or did, until the
//! *sustained* version of the stop this file drove stopped reproducing. A
//! different-shaped version of it has not: read "What is still open" below
//! before assuming the disc is clean.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! cargo nextest run -p oag-game --run-ignored all -E 'binary(stall_rescue_ground_truth)'
//! ```
//!
//! # Why this needed a disc, and why it needed *this* circuit
//!
//! [`race::RESCUE_HALF_WIDTHS`] recovers a craft that has left the circuit. It
//! cannot recover one that is still on it and going nowhere, because the question
//! it asks - is this craft far from its line - has the same answer for a craft
//! beached against the scenery as for one driving well.
//!
//! The failure was not reachable synthetically. It happened where a circuit's
//! authored racing line ran above its own collision surface, which was a
//! property of shipped track data: `docs/gameplay/ai.md` measured `05_Track` as
//! having 134 line samples with nothing under them, in runs at 161-211 and
//! 811-893, the worst on the disc. A craft came off there, wedged, and sat.
//! Measured before the rescue existed: a lone novice opponent spent **3,634
//! consecutive ticks** - just over a minute - below one unit per second with the
//! throttle held down, and completed one lap in five minutes.
//!
//! # One shape of the premise expired; a different shape of the same failure did not
//!
//! `docs/gameplay/ai.md`'s "the last three, with a mechanism the original does
//! not have" fixed a real cause: a craft was falling through the track between
//! one hover-probe test and the next, and `oag_physics::hover::sweep` now
//! catches the crossing mid-tick instead of only testing contact discretely.
//! That closed the *sustained*, `3,634`-tick-shaped version of this file's
//! original measurement - swept 2026-09-02, all twelve circuits at all four
//! difficulties, Venom class, 12,000 ticks each: **no cell reaches
//! [`race::STALL_TICKS`] (120) any more**, the closest being `07_Track` at
//! novice, 46.
//!
//! **It did not close the bounce-in-place gap the reversed-grid work found on
//! the way past `05_Track`** - `docs/ghidra/functions/psp-pulse-usa/grid.md`,
//! "the stall rescue does not catch a craft bouncing in place": a craft can
//! stay physically pinned in one place while its speed keeps flicking back
//! above `STALL_SPEED`, so the *sustained* dwell this file measures never
//! reaches its threshold even though the craft is making no progress at all.
//! **Confirmed still open, traced 2026-09-02**: a lone novice opponent on
//! `05_Track` reaches `pos (-717.0, 19.3, -512.3)` by tick ~5,000 and is still
//! within a few units of it at tick 12,000 - 7,000+ ticks, effectively
//! stationary - while its per-200-tick speed window ranges from as low as
//! 0.00-0.04 up to 11-13 units/s throughout, so `stalled_ticks` keeps resetting
//! and the rescue never fires. One lap completed in the whole 12,000-tick run,
//! `respawns` zero. The same run at every other difficulty on `05_Track`, and
//! novice on every other circuit but `07_Track` (3 of 4 laps, same shape,
//! milder), completes cleanly - so this is `05_Track`-and-`07_Track`-at-novice
//! specific, not a difficulty-wide or circuit-wide regression.
//!
//! So what moved is narrower than it first looked: the *sustained*-stall
//! reading this file used to demonstrate is gone, and that half of the
//! regression guard below is real. The bounce-in-place gap is not, and
//! nothing here closes it - `crates/game/src/race/tests/respawn.rs`'s
//! `a_stopped_opponent_is_flagged_after_the_dwell` proves the dwell counter
//! mechanism works exactly as designed; it was never the counter that was
//! wrong, it is that a bouncing craft does not describe the state the counter
//! watches for.

use std::path::{Path, PathBuf};

use oag_game::race;

/// The disc, or `None` on a checkout without one.
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
    None
}

/// What one lone opponent did over a run.
#[derive(Debug, Default)]
struct Solo {
    /// The longest unbroken run of ticks meeting the stall condition exactly as
    /// [`race::Race::stalled`] tests it: stopped, asking to move, not finished.
    longest_stall: u32,
    /// Laps reached.
    lap: u32,
    /// Times it was put back, by any of the three triggers.
    respawns: u32,
}

/// Drives one opponent alone round `track` and watches for stalls.
///
/// Alone, and with the player deactivated, so nothing it does is about anybody
/// else - the same isolation `race_ground_truth`'s solo benchmark uses.
fn solo(level: oag_ai::Difficulty, track: &str, ticks: u64) -> Option<Solo> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: level,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    for slot in 2..oag_gameplay::MAX_SHIPS {
        race.world.ships[slot].active = false;
    }
    race.world.ships[0].active = false;

    let mut solo = Solo::default();
    let mut run = 0u32;
    for tick in 0..ticks {
        race.tick(&oag_gameplay::InputSnapshot::default());
        let ship = &race.world.ships[1];
        if ship.physics.craft_state != oag_physics::CraftState::Racing {
            continue;
        }
        // The same three terms `Race::stalled` applies, restated here rather than
        // reached through it: a test that called the private predicate would pass
        // whatever that predicate did, including nothing.
        //
        // `tick != oag_race::COUNTDOWN_TICKS` excludes one specific, understood
        // tick: the countdown hold releases every opponent's throttle from zero
        // to full in a single tick, so at `tick == COUNTDOWN_TICKS` `thrust` has
        // just gone positive while `linear_velocity` has not risen off ~0 yet -
        // which reads as `stalled` by this predicate's own three terms even
        // though the craft is not stuck. Confirmed by instrumenting
        // `(tick, thrust, speed)` around the release: on all of `16_Track`,
        // `03_Track` and `06_Track` at ace, `thrust` reads 100 and `speed`
        // ~0.57 units/s at exactly `COUNTDOWN_TICKS`, and >1.0 the very next
        // tick - identically on all three, so this is a property of the
        // countdown gate itself, not of any one circuit. Excluded here rather
        // than by loosening the zero-tolerance assertion below, the same way
        // `shuriken_ground_truth::WARM_UP_TICKS` skips past its own known
        // window rather than starting exactly at `COUNTDOWN_TICKS`.
        let stalled = ship.physics.thrust > 0.0
            && !ship.standing.finished()
            && ship.physics.body.linear_velocity.length() < race::STALL_SPEED
            && tick != oag_race::COUNTDOWN_TICKS;
        run = if stalled { run + 1 } else { 0 };
        solo.longest_stall = solo.longest_stall.max(run);
    }
    solo.lap = race.world.ships[1].standing.lap;
    solo.respawns = race.respawns_of(1);
    Some(solo)
}

/// **The regression guard for the sustained-stall reading only**: no circuit
/// on the disc sits below `STALL_SPEED` for `STALL_TICKS` consecutive ticks -
/// which is a narrower claim than "no craft gets stuck". A craft can still
/// make effectively no progress while never meeting that exact condition; see
/// this file's module doc for `05_Track` and `07_Track` at novice, which do.
///
/// This used to drive `05_Track` at novice and assert the rescue fired,
/// because that cell was the disc's worst *sustained* beaching - see this
/// file's module doc for why that stopped being true. What is left to assert
/// against real disc data is the negative: every circuit, at every
/// difficulty, stays under [`race::STALL_TICKS`] on this one measure. The
/// mechanism itself - that the dwell counter fires at the threshold and puts
/// a craft back - is proven synthetically instead, in
/// `crates/game/src/race/tests/respawn.rs`. Laps are printed rather than
/// asserted on, precisely because a low count here (`05_Track`, `07_Track`,
/// both at novice) is not this test's business to hide.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn no_circuit_sustains_a_stall_past_the_rescue_threshold() {
    let tracks = [
        "01_Track", "02_Track", "03_Track", "04_Track", "05_Track", "06_Track", "07_Track",
        "09_Track", "10_Track", "13_Track", "14_Track", "16_Track",
    ];
    let difficulties = [
        oag_ai::Difficulty::Novice,
        oag_ai::Difficulty::Skilled,
        oag_ai::Difficulty::Elite,
        oag_ai::Difficulty::Ace,
    ];

    let mut worst: Option<(oag_ai::Difficulty, &str, u32)> = None;
    for track in tracks {
        for difficulty in difficulties {
            let entry = format!("Data\\Environments\\{track}\\track.vex");
            let Some(solo) = solo(difficulty, &entry, 12_000) else {
                return;
            };
            println!(
                "{difficulty:?} {track}: longest stall {} ticks, {} lap(s), {} respawn(s)",
                solo.longest_stall, solo.lap, solo.respawns
            );
            if worst.is_none_or(|(_, _, longest)| solo.longest_stall > longest) {
                worst = Some((difficulty, track, solo.longest_stall));
            }
            assert!(
                solo.longest_stall < race::STALL_TICKS,
                "{difficulty:?} {track}: a craft sat stopped for {} ticks, past the {} \
                 the rescue promises - a real sustained beaching is back",
                solo.longest_stall,
                race::STALL_TICKS
            );
        }
    }
    let (difficulty, track, longest) = worst.expect("the track list is not empty");
    println!(
        "worst cell by sustained dwell: {difficulty:?} {track}, longest stall {longest} ticks"
    );
}

/// **The control: the rescue fires nowhere it is not needed.**
///
/// The measurement this threshold was chosen from is that no healthy craft spends
/// a *single* consecutive tick below [`race::STALL_SPEED`] - not a small number,
/// zero - across all twelve circuits at all four difficulties. That is what buys
/// the room between the healthy case and the beached one, and it is the property
/// that would break first if the constant were ever raised.
///
/// One tick is excluded from that count, not exempted from the guarantee: the
/// countdown hold's release, at exactly `COUNTDOWN_TICKS`, necessarily reads as
/// `stalled` for one tick by [`solo`]'s own predicate (thrust has just gone
/// positive, speed has not risen off ~0 yet) on every circuit, since the hold
/// mechanism itself is circuit-agnostic - see the comment in [`solo`] for the
/// instrumented proof. `solo` does not count that specific tick, so "zero"
/// below still means zero *genuine* stalls; a real one, anywhere else in the
/// run, still fails this test exactly as before.
///
/// A stall rescue that fired on a craft merely cornering slowly would launder bad
/// driving into completed laps, which is exactly what `race_ground_truth`'s solo
/// benchmark exists to catch - it counts a lap driven with a recovery in it as no
/// lap at all.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_healthy_craft_never_looks_stalled_for_a_single_tick() {
    for track in ["16_Track", "03_Track", "06_Track"] {
        let entry = format!("Data\\Environments\\{track}\\track.vex");
        let Some(solo) = solo(oag_ai::Difficulty::Ace, &entry, 6_000) else {
            return;
        };
        println!(
            "ace {track}: longest stall {} ticks, laps {}, respawns {}",
            solo.longest_stall, solo.lap, solo.respawns
        );
        assert_eq!(
            solo.longest_stall, 0,
            "a healthy craft on {track} looked stalled for {} consecutive ticks, \
             which is the margin this threshold rests on",
            solo.longest_stall
        );
        assert_eq!(
            solo.respawns, 0,
            "{track} recovered a craft that did not need it"
        );
    }
}
