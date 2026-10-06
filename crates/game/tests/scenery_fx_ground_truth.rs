//! A circuit's own placed particle effects play from the start of a real race.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test scenery_fx_ground_truth --run-ignored all
//! ```
//!
//! `PsysNode_Init` (`0x089156a0`) spawns one instance per `ParticleSystem`
//! node of the circuit's `.vex` at load, and the update slot keeps it on its
//! node for the race. Basilico (`01_Track`) places three `WO_BLUE_WELDER`, which
//! a live PPSSPP capture confirmed; circuit seven eighteen `WO_MODESTO_STEAM_A`.
//! See `docs/ghidra/functions/psp-pulse-usa/placed-particle-systems.md`.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

fn race_on(track: &str) -> Option<race::Race> {
    let image = oag_testdata::image("pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(format!(r"Data\Environments\{track}\track.vex")),
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in loaded
        .report
        .iter()
        .filter(|l| l.starts_with("placed effect"))
    {
        println!("{line}");
    }
    Some(race::Race::start(loaded.setup))
}

fn playing_after(race: &mut race::Race, ticks: u32) -> (usize, usize) {
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
    }
    let fx = race.scenery_fx();
    (fx.playing_count(), fx.stage().alive_count())
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn basilico_s_three_welders_spark_from_the_first_ticks() {
    let Some(mut race) = race_on("01_Track") else {
        return;
    };
    assert_eq!(race.scenery_fx().placed().len(), 3);
    assert!(
        race.scenery_fx()
            .placed()
            .iter()
            .all(|p| p.name == oag_title::engine_effects::BLUE_WELDER_EFFECT)
    );
    // The welder bursts every 20-60 ticks, so a few seconds always holds sparks.
    let (playing, alive) = playing_after(&mut race, 180);
    assert_eq!(playing, 3, "every placed welder is attached");
    assert!(alive > 0, "and has sparks in the air");
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn circuit_seven_s_eighteen_steam_vents_all_play() {
    let Some(mut race) = race_on("07_Track") else {
        return;
    };
    assert_eq!(race.scenery_fx().placed().len(), 18);
    let (playing, alive) = playing_after(&mut race, 120);
    assert_eq!(playing, 18, "all eighteen fit their own pool");
    assert!(alive >= 18, "every vent is emitting");
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_circuit_that_places_none_plays_none() {
    let Some(mut race) = race_on("16_Track") else {
        return;
    };
    assert!(race.scenery_fx().placed().is_empty());
    assert_eq!(playing_after(&mut race, 30), (0, 0));
}
