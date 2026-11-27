//! A mode that loads data of its own is offered only where that data resolves (ADR-0098):
//! Detonator on Pulse with the official mod `oag.detonator-pulse` on, not with it off, and on
//! HD, 2048 and Omega off their own discs whatever the mods.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content and the official-mods submodule.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all mode_data_ground_truth::
//! ```

use oag_source::layers;

/// The modes `image`'s title offers with the mods registered now, as the RACE page's MODE row
/// lists them (`boot::RaceSetup::modes`).
fn offered(image: &str) -> Option<(&'static oag_title::Title, Vec<oag_race::Mode>)> {
    let path = oag_testdata::image(image)?;
    let opened =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let held = oag_game::boot::mode_data_held(&opened.archives, opened.title);
    Some((
        opened.title,
        oag_game::boot::offered_modes(opened.title, &held),
    ))
}

fn official_mod() -> Option<std::path::PathBuf> {
    let manifest = oag_testdata::exact("assets/mods/official/detonator-pulse/oag-mod.toml")?;
    Some(manifest.parent()?.to_path_buf())
}

#[test]
#[ignore = "needs a disc image in data/images/ and the official-mods submodule"]
fn pulse_offers_detonator_only_with_its_mod_on() {
    let Some(root) = official_mod() else {
        return;
    };
    for image in [
        "data/images/pulse-psp-eu.chd",
        "data/images/pulse-ps2-eu.chd",
    ] {
        layers::register_mods(Vec::new());
        let Some((title, off)) = offered(image) else {
            continue;
        };
        assert_eq!(title.name, oag_pulse::TITLE.name, "{image}");
        assert!(
            !off.contains(&oag_race::Mode::Detonator),
            "{image}: {off:?}"
        );
        assert!(off.contains(&oag_race::Mode::Zone), "{image}: {off:?}");
        layers::register_mod_folder(&root, "oag.detonator-pulse");
        let (_, on) = offered(image).expect("opened once already");
        assert!(on.contains(&oag_race::Mode::Detonator), "{image}: {on:?}");
        assert_eq!(
            on.len(),
            off.len() + 1,
            "{image}: only Detonator comes and goes"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_hd_family_offers_detonator_off_its_own_disc_with_no_mod() {
    layers::register_mods(Vec::new());
    for image in [
        "data/images/hdfury-ps3-eu-dec.iso",
        "data/extracted/vita/PCSF00007",
        "data/images/omega-ps4-eu.pkg",
    ] {
        let Some((title, modes)) = offered(image) else {
            continue;
        };
        assert_eq!(
            modes,
            oag_race::Mode::ALL,
            "{image} ({}): every mode, Detonator's data from the disc",
            title.name
        );
    }
}
