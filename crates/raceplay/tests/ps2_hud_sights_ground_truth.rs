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
