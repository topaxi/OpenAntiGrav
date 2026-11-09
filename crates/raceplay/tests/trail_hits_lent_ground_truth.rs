//! The trail-hit burst lent to Wipeout Pulse (`graphics.trail_hits`), off real disc images:
//! Pulse's race and Wipeout HD's `WO_TRAIL_HITSHIP.POB`.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all trail_hits_lent
//! ```
//!
//! Pulse authors no `WO_TRAIL_HITSHIP` (`docs/formats/pulse-absent-effects.md`). With the setting
//! on and a source that authors it mounted, a craft posed inside a rival's engine trail plays
//! the burst; with the setting off it does not and the loader says nothing of it; on with no
//! source, nothing plays and the report names why.

use oag_raceplay as race;

const PULSE: &str = "data/images/pulse-psp-eu.chd";
const HD: &str = "data/images/hdfury-ps3-eu-dec.iso";

fn load(trail_hits: race::TrailHits) -> Option<race::Loaded> {
    let pulse = oag_testdata::image(PULSE)?;
    let loaded = race::load(&race::Options {
        source: pulse.display().to_string(),
        mode: oag_race::Mode::SingleRace,
        trail_hits,
        ..race::Options::default()
    })
    .expect("loading the Pulse race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

fn lent_from_hd() -> Option<race::TrailHits> {
    let hd = oag_testdata::image(HD)?;
    Some(race::TrailHits::On {
        lender: Some(hd.display().to_string()),
    })
}

/// Slot 1 posed eight units behind slot 0 on slot 0's heading, both flying 60 units a second,
/// and the exhaust and trail-hit history of that flight replayed - `--field-warmup`'s path.
fn tucked_in_a_rivals_trail(loaded: race::Loaded) -> race::Race {
    let mut race = race::Race::start(loaded.setup);
    assert!(
        !race.hd_trail_active(),
        "Pulse draws the PSP ribbon, not HD's tube"
    );
    let lead = race.sim.world.ships[0].physics.body;
    let forward = lead.forward();
    race.sim.world.ships[1].physics.body.orientation = lead.orientation;
    race.sim.world.ships[1].physics.body.position = lead.position - forward * 8.0;
    let velocities = [forward * 60.0; 2];
    let ticks = race.field_warm_up_ticks();
    race.warm_up_field(&velocities, ticks, None);
    race
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and data/images/hdfury-ps3-eu-dec.iso"]
fn trail_hits_lent_on_pulse_a_craft_tucked_in_a_rivals_trail_plays_hds_burst() {
    let Some(on) = lent_from_hd() else { return };
    let Some(loaded) = load(on) else { return };
    let line = |needle: &str| loaded.report.iter().find(|l| l.contains(needle)).cloned();
    let burst = line("WO_TRAIL_HITSHIP: ").expect("the loader said nothing of the lent effect");
    assert!(
        burst.contains("emitter(s)") && !burst.contains("will not be drawn"),
        "{burst}"
    );
    let lent = line("trail hits: ").expect("the report names the lending");
    assert!(lent.contains("Wipeout HD"), "{lent}");
    let race = tucked_in_a_rivals_trail(loaded);
    assert!(
        race.stage().playing_count() > 0,
        "a craft eight units inside a rival's ribbon played no burst"
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn trail_hits_lent_off_pulse_plays_no_burst_and_loads_no_effect() {
    let Some(loaded) = load(race::TrailHits::Off) else {
        return;
    };
    assert!(
        loaded
            .report
            .iter()
            .all(|l| !l.contains("WO_TRAIL_HITSHIP") && !l.contains("trail hits")),
        "the setting off added to the report: {:?}",
        loaded
            .report
            .iter()
            .filter(|l| l.contains("TRAIL_HITSHIP") || l.contains("trail hits"))
            .collect::<Vec<_>>()
    );
    let race = tucked_in_a_rivals_trail(loaded);
    assert_eq!(race.stage().playing_count(), 0);
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn trail_hits_lent_with_no_source_plays_nothing_and_the_report_names_why() {
    let Some(loaded) = load(race::TrailHits::On { lender: None }) else {
        return;
    };
    let why = loaded
        .report
        .iter()
        .find(|l| l.contains("trail hits: "))
        .expect("the report is silent about a switched-on setting that found no source");
    assert!(
        why.contains("no source that authors WO_TRAIL_HITSHIP"),
        "{why}"
    );
    let race = tucked_in_a_rivals_trail(loaded);
    assert_eq!(race.stage().playing_count(), 0);
}
