//! Wipeout HD's Time Trial `RECORD` readout counts down from the circuit's own
//! `stats.xml`, not from nothing.
//!
//! Measured on RPCS3 (2026-10-21, `hd-go-board-hud`): on all thirteen Time Trial
//! pairs the original's top-right panel reads `RECORD` and a time that, added
//! to the race clock, is 599.0 s (Ubermall `9.46.1` at `0.12.9`, Modesto Heights
//! `9.46.2` at `0.12.7`); in RAM the HUD record's `+0x3c` read 53,040 at a race
//! clock of 68.6 s, `59,900` centiseconds in all. HD's circuits author
//! `<RaceTimes Venom="599" .../>`, and until this lane the loader read the file
//! on Pulse alone, so an HD race drew `TOTAL`.
//!
//! **`#[ignore]`d and never run in CI**: it needs `hdfury-ps3-eu-dec.iso`.

use oag_hud::{PaceTier, RecordTarget, pace_for};

fn load(track: &str) -> Option<oag_raceplay::Loaded> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    Some(
        oag_raceplay::load(&oag_raceplay::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::TimeTrial,
            track: Some(track.to_string()),
            seed: Some(1),
            ..oag_raceplay::Options::default()
        })
        .expect("loading an HD circuit"),
    )
}

/// The race's loaded stats make a `RECORD` pace that is 599.0 s minus the clock.
fn pace_at(track: &str, seconds: u64) -> Option<(PaceTier, u32)> {
    let loaded = load(track)?;
    let stats = loaded
        .track_stats
        .expect("the circuit's stats.xml reads out of HD's archives");
    assert_eq!(stats.race_times[0], 599.0, "Venom's <RaceTimes>");
    let target = RecordTarget::new(oag_race::Mode::TimeTrial, "VENOM", Some(&stats), None, None)
        .expect("a Time Trial has a record target");
    assert_eq!(target.authored_centis, Some(59_900));
    let pace = pace_for(
        oag_race::Mode::TimeTrial,
        seconds * 60,
        seconds * 60,
        None,
        Some(&target),
    )?;
    Some((pace.tier, pace.remaining_ticks))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn ubermall_counts_down_from_599_seconds() {
    // The pair's own spelling of the circuit.
    let Some((tier, remaining)) = pace_at(r"Data\Environments\05_Ubermall\track.vex", 13) else {
        return;
    };
    assert_eq!(tier, PaceTier::Record);
    assert_eq!(remaining, (599 - 13) * 60);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_archive_spelling_of_a_circuit_reads_too() {
    // `oag_hd::race::DEFAULT_TRACK`: forward slashes, a leading one, lower case.
    let Some((tier, remaining)) = pace_at(oag_hd::race::DEFAULT_TRACK, 13) else {
        return;
    };
    assert_eq!(tier, PaceTier::Record);
    assert_eq!(remaining, (599 - 13) * 60);
}
