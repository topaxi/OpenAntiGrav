//! The PS2 lock-on reticle's art, off the real PS2 disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! A PS2 `.vex` embeds an empty texture block; the HUD's sight models took
//! their picture from the model alone, so the reticle was drawn from nothing.

use oag_raceplay as race;

/// Every sight model the Pulse layout names decodes into the HUD sheet on the
/// PS2 pressing, from the texture set in the archive entry before it.
#[test]
#[ignore = "needs a disc image"]
fn the_ps2_sight_models_decode_into_the_sheet() {
    let Some(image) = oag_testdata::image("data/images/pulse-ps2-eu.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the race");
    for stem in [
        "missile_sight_outer",
        "missile_sight_inner",
        "leachbeam_sight",
    ] {
        let prefix = format!(r"HUD model Data\HUD\{stem}.vex: ");
        assert!(
            loaded.report.iter().any(|line| line.starts_with(&prefix)),
            "{stem}: no decoded-model line in the loader report:\n{}",
            loaded.report.join("\n")
        );
    }
}

/// Every team's `shipwreck.vex` loads on the PS2 pressing, textured from the
/// entry before it, as it does on the PSP.
#[test]
#[ignore = "needs a disc image"]
fn every_ps2_team_has_a_textured_wreck() {
    let Some(image) = oag_testdata::image("data/images/pulse-ps2-eu.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the race");
    assert!(!loaded.liveries.is_empty(), "no grid was fielded");
    for livery in &loaded.liveries {
        let wreck = livery
            .wreck
            .as_ref()
            .unwrap_or_else(|| panic!("{}: no wreck loaded", livery.team));
        assert!(
            !wreck.model.textures.is_empty() && wreck.model.textures.iter().all(Option::is_some),
            "{}: the wreck's texture did not decode",
            livery.team
        );
    }
}

/// A PS2 circuit's own placed particle effects, weather anchors and `Camera`
/// nodes play as they do on the PSP: circuit 1 places three welders and eight
/// cameras on both pressings.
#[test]
#[ignore = "needs a disc image"]
fn a_ps2_circuit_plays_its_placed_effects_and_cuts_to_its_cameras() {
    let Some(image) = oag_testdata::image("data/images/pulse-ps2-eu.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(r"Data\Environments\01_Track\track.vex".to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the race");
    let has = |text: &str| loaded.report.iter().any(|line| line.contains(text));
    assert!(
        has("placed effect: 3 x WO_BLUE_WELDER, played from load on its node"),
        "{}",
        loaded.report.join("\n")
    );
    assert!(has("destroy camera: 8 authored Camera node(s)"));
}
