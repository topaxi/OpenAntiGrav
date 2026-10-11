//! Every title's opponents fire on Pulse's law, on a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(fire_law_inherit_ground_truth)'
//! ```
//!
//! The maintainer's rule (2026-10-03): a title with no measured law of its
//! own runs Pulse's, never this project's chosen fallback. HD, Omega and 2048
//! each ship a `WeaponAIstats.xml` (`docs/gameplay/ai.md`), so each runs
//! `oag_ai::weapon_ai` on its own odds: **inherited from Pulse, unmeasured on
//! that title.** Dropping a title's `Weapons::ai` falls back to
//! `FireLaw::Ours` and fails here.

use oag_raceplay as race;
use oag_raceplay::FireLaw;

fn fire_law(source: String, mode: oag_race::Mode) -> FireLaw {
    let loaded = race::load(&race::Options {
        source,
        mode,
        seed: Some(1),
        ..race::Options::default()
    })
    .expect("loading the race");
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("opponents fire on its odds")),
        "no weapon-AI line in the loader report: {:#?}",
        loaded.report
    );
    race::Race::start(loaded.setup).fire_law()
}

/// One title, one mode: a race load each, which is most of a 2048 test's time, so the
/// modes are the test axis rather than a loop.
fn fires_original(source: String, mode: oag_race::Mode) {
    assert_eq!(fire_law(source, mode), FireLaw::Original, "{mode:?}");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_hd_single_race_fires_on_the_original_law() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    fires_original(image.display().to_string(), oag_race::Mode::SingleRace);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_hd_eliminator_fires_on_the_original_law() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    fires_original(image.display().to_string(), oag_race::Mode::Eliminator);
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_2048_single_race_fires_on_the_original_law() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    fires_original(path.display().to_string(), oag_race::Mode::SingleRace);
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_2048_eliminator_fires_on_the_original_law() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    fires_original(path.display().to_string(), oag_race::Mode::Eliminator);
}

#[test]
#[ignore = "needs the extracted Omega packages in data/extracted/ps4/"]
fn an_omega_single_race_fires_on_the_original_law() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    fires_original(path.display().to_string(), oag_race::Mode::SingleRace);
}

#[test]
#[ignore = "needs the extracted Omega packages in data/extracted/ps4/"]
fn an_omega_eliminator_fires_on_the_original_law() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    fires_original(path.display().to_string(), oag_race::Mode::Eliminator);
}
