//! Wipeout Pure's Time Trial clock counts down to a medal, not up to `TOTAL`.
//!
//! Measured on PPSSPP (`pure-fidelity`, 2026-10-22): four Time Trial frames with Feisar's Vector
//! on a fresh profile read `BRONZE TIME` and a countdown to the circuit's gold time for the class
//! plus the file's `Bronze` offset. `PlayerStatus_Update` (`0x0892195c`) names the medal
//! (`docs/ghidra/functions/psp-pure-usa/hud-time-ladder.md`). Pure's `stats.xml` authors no
//! `<LapTimes>`, which made the whole file fail to parse until this lane.
//!
//! **`#[ignore]`d and never run in CI**: it needs `pure-psp-usa.chd`.

use oag_hud::{PaceTier, RecordTarget, pace_for};

fn ladder(track: &str) -> Option<RecordTarget> {
    let image = oag_testdata::image("pure-psp-usa.chd")?;
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VECTOR".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(track.to_string()),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading a Pure circuit");
    let stats = loaded
        .track_stats
        .expect("Pure's stats.xml reads without a <LapTimes>");
    assert_eq!(stats.lap_times, None, "Pure authors no lap figures");
    Some(
        RecordTarget::new(
            oag_race::Mode::TimeTrial,
            "VECTOR",
            Some(&stats),
            None,
            None,
        )
        .expect("a Time Trial has a record target"),
    )
}

/// `[gold, silver, bronze]` in centiseconds and the countdown at 1,000 ticks (16.66 s) of clock.
fn check(circuit: &str, limits: [i64; 3]) {
    let track = format!(r"Data\Environments\{circuit}\track.vex");
    let Some(target) = ladder(&track) else { return };
    assert_eq!(target.medal_ladder, Some(limits), "{circuit}");
    let pace = pace_for(oag_race::Mode::TimeTrial, 1000, 1000, None, Some(&target))
        .expect("a fresh profile has a pace");
    assert_eq!(
        pace.tier,
        PaceTier::Bronze,
        "no best: the first medal to chase"
    );
    assert!(!pace.missed);
    // The countdown is the limit less the clock, to a tick.
    let remaining = i64::from(pace.remaining_ticks) * 100 / 60;
    assert!(
        (remaining - (limits[2] - 1666)).abs() <= 2,
        "{circuit}: {remaining}"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn vineta_k_chases_bronze_at_130_seconds() {
    check("01_Vineta_K", [12_400, 12_700, 13_000]);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn chenghou_project_chases_bronze_at_160_seconds() {
    check("04_Chenghou_Project", [15_400, 15_700, 16_000]);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn blue_ridge_chases_bronze_at_158_seconds() {
    check("07_Blue_Ridge", [15_200, 15_500, 15_800]);
}
