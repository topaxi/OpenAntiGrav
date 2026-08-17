//! A craft that stops on a real circuit gets picked up.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! cargo nextest run -p oag-game --run-ignored all -E 'binary(stall_rescue_ground_truth)'
//! ```
//!
//! # Why this needs a disc, and why it needs *this* circuit
//!
//! [`race::RESCUE_HALF_WIDTHS`] recovers a craft that has left the circuit. It
//! cannot recover one that is still on it and going nowhere, because the question
//! it asks - is this craft far from its line - has the same answer for a craft
//! beached against the scenery as for one driving well.
//!
//! The failure is not reachable synthetically. It happens where a circuit's
//! authored racing line runs above its own collision surface, which is a property
//! of shipped track data: `docs/gameplay/ai.md` measures `05_Track` as having 134
//! line samples with nothing under them, in runs at 161-211 and 811-893, the worst
//! on the disc. A craft comes off there, wedges, and sits. Measured before this
//! rescue existed: a lone novice opponent spent **3,634 consecutive ticks** - just
//! over a minute - below one unit per second with the throttle held down, and
//! completed one lap in five minutes.
//!
//! So the assertion here is the bound the mechanism guarantees, on the circuit
//! that produces the failure, plus a control on a circuit that does not.

use std::path::{Path, PathBuf};

use oag_game::race;
use oag_physics::SpeedClass;

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
        class: SpeedClass::Venom,
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
    for _ in 0..ticks {
        race.tick(&oag_gameplay::InputSnapshot::default());
        let ship = &race.world.ships[1];
        if ship.physics.craft_state != oag_physics::CraftState::Racing {
            continue;
        }
        // The same three terms `Race::stalled` applies, restated here rather than
        // reached through it: a test that called the private predicate would pass
        // whatever that predicate did, including nothing.
        let stalled = ship.physics.thrust > 0.0
            && !ship.standing.finished()
            && ship.physics.body.linear_velocity.length() < race::STALL_SPEED;
        run = if stalled { run + 1 } else { 0 };
        solo.longest_stall = solo.longest_stall.max(run);
    }
    solo.lap = race.world.ships[1].standing.lap;
    solo.respawns = race.respawns_of(1);
    Some(solo)
}

/// **A craft that stops is picked up, and within the time the constant promises.**
///
/// The bound is what the mechanism guarantees rather than a fitted number: the
/// dwell counter reaches [`race::STALL_TICKS`] and the craft is put back, which
/// resets both the counter and the craft's own state, so no stall can outlast the
/// threshold. Before this existed the same run measured 3,634.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_craft_that_stops_on_the_disc_is_put_back() {
    let Some(solo) = solo(
        oag_ai::Difficulty::Novice,
        "Data\\Environments\\05_Track\\track.vex",
        12_000,
    ) else {
        return;
    };
    println!(
        "novice 05_Track: longest stall {} ticks ({:.1}s), laps {}, respawns {}",
        solo.longest_stall,
        solo.longest_stall as f32 / 60.0,
        solo.lap,
        solo.respawns
    );

    assert!(
        solo.longest_stall < race::STALL_TICKS,
        "a craft sat stopped for {} ticks, past the {} the rescue promises",
        solo.longest_stall,
        race::STALL_TICKS
    );
    // It was the rescue that did it, not luck: a run this circuit could not
    // complete without being put back at all.
    assert!(
        solo.respawns > 0,
        "nothing recovered the craft, so the bound above proves nothing"
    );
    // And the craft is in the race rather than parked in the scenery. One lap in
    // five minutes was the measurement before; this is the honest floor, not a
    // target - `05_Track`'s racing line running above its surface is a separate
    // open thread and this does not close it.
    assert!(
        solo.lap >= 2,
        "the craft completed {} lap(s), so it is still stuck",
        solo.lap
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
