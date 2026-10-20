//! The two HD circuits whose `Sun direction` is a degenerate triple (a length of about
//! `1e-4`) keep the rest of their authored rig.
//!
//! **`#[ignore]`d and never run in CI.** It needs `data/images/hdfury-ps3-eu-dec.iso`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^hd_sunless_rig_ground_truth::/)'
//! ```
//!
//! Before 2026-10-20 `EnvSettings::direction` answering `None` sent the whole circuit to
//! `Light::stand_in`, which dropped the ambient, the prelit curve and the specular with the
//! sun: Tech de Ra and Modesto Heights were the two worst pairs of the HD compare page
//! (mean grey difference 54.5 and 42.1). The sun is now drawn at zero (chosen, not
//! measured) and everything else is the circuit's own.

use oag_raceplay as race;

fn report(track: &str) -> Option<Vec<String>> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(loaded.report)
}

fn check(track: &str, ambient: &str, prelit: &str) {
    let Some(report) = report(track) else { return };
    assert!(
        !report.iter().any(|l| l.contains("stand-in rig")),
        "the circuit fell back to the stand-in rig: {report:#?}"
    );
    assert!(
        report.iter().any(|l| l.contains("degenerate triple")),
        "the degenerate sun is not reported: {report:#?}"
    );
    let rig = report
        .iter()
        .find(|l| l.contains("colour [0.00, 0.00, 0.00]"))
        .unwrap_or_else(|| panic!("no authored rig line with a zero sun: {report:#?}"));
    assert!(rig.contains(ambient), "{rig}");
    assert!(rig.contains(prelit), "{rig}");
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn tech_de_ra_keeps_its_ambient_and_prelit_curve() {
    check(
        "Data\\Environments\\Tech_De_Ra\\track.vex",
        "ambient [1.00, 1.00, 1.00]",
        "prelit 4.0*lightmap^2.5",
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn modesto_heights_keeps_its_ambient_and_prelit_curve() {
    check(
        "Data\\Environments\\Modesto_Heights\\track.vex",
        "ambient [0.31, 0.26, 0.71]",
        "prelit 3.0*lightmap^3.0",
    );
}
