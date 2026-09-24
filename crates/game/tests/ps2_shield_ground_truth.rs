//! The PS2 shield shell, off the real PS2 disc: textured and additive.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(ps2_shield_ground_truth)'
//! ```
//!
//! Reported from play on 2026-09-17: on the PS2 source the Shield pickup's
//! shell "renders solid and not animated". Two causes, both on the PS2 path
//! alone. The shell's one `Texture` node carries no pixels on PS2, so it bound
//! the white 1x1 until `livery::shield_model` learned the plume's external
//! texture set; and its batches carry no `0x0700` blend class, so they drew
//! opaque until `livery::blend_additively` put them on the additive class the
//! PSP shell's own batches name. See that function for the evidence and its
//! confidence.

use std::path::PathBuf;

use oag_game::race;
use oag_vex::vex::BlendClass;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-ps2-eu.chd")
}

fn load() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// Every team's shell has its texture decoded, and not one of its draws is
/// left in the opaque or cutout lists, where it would hide the craft.
#[test]
#[ignore = "needs a disc image"]
fn every_ps2_shell_is_textured_and_additive() {
    let Some(loaded) = load() else {
        return;
    };
    assert!(!loaded.liveries.is_empty(), "no grid was fielded");
    for livery in &loaded.liveries {
        let shell = livery
            .shield
            .as_ref()
            .unwrap_or_else(|| panic!("{}: no shield shell", livery.team));
        assert!(
            !shell.textures.is_empty() && shell.textures.iter().all(Option::is_some),
            "{}: the shell's texture did not decode, so it binds the white 1x1",
            livery.team
        );
        assert!(
            shell.draws.is_empty() && shell.alpha_tested_draws.is_empty(),
            "{}: {} opaque and {} cutout draw(s) left, so the shell hides the craft",
            livery.team,
            shell.draws.len(),
            shell.alpha_tested_draws.len()
        );
        assert!(
            !shell.transparent_draws.is_empty()
                && shell
                    .transparent_draws
                    .iter()
                    .all(|draw| draw.blend == Some(BlendClass::Additive)),
            "{}: a shell draw is not on the additive class",
            livery.team
        );
    }
}

/// The cockpit sphere already carries `0x200` on PS2, so it only needed its
/// texture - and must come out of the load untouched otherwise.
#[test]
#[ignore = "needs a disc image"]
fn the_ps2_cockpit_sphere_is_textured() {
    let Some(loaded) = load() else {
        return;
    };
    let sphere = loaded
        .shield_cockpit
        .as_ref()
        .expect("vr_shield_cockpit.vex is on the PS2 disc");
    assert!(
        !sphere.textures.is_empty() && sphere.textures.iter().all(Option::is_some),
        "the cockpit sphere's texture did not decode"
    );
    assert!(sphere.draws.is_empty() && sphere.alpha_tested_draws.is_empty());
    assert!(
        sphere
            .transparent_draws
            .iter()
            .all(|draw| draw.blend == Some(BlendClass::Additive))
    );
}
