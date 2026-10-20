//! The two HD circuits whose `Sun direction` is a tiny triple (a length of about `1e-4`)
//! are lit by their own authored rig, sun included.
//!
//! **`#[ignore]`d and never run in CI.** It needs `data/images/hdfury-ps3-eu-dec.iso`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^hd_tiny_sun_direction_ground_truth::/)'
//! ```
//!
//! Before 2026-10-20 a length under `1e-3` was read as an off switch, which sent the whole
//! circuit to `Light::stand_in` and dropped the ambient, the prelit curve and the specular
//! with the sun: Tech de Ra and Modesto Heights were the two worst pairs of the HD compare
//! page (mean grey difference 54.5 and 42.1). The original normalises the triple like any
//! other (`Scene_PrepareFrame`), and so does `EnvSettings::direction` now.

use oag_raceplay as race;

fn rig_line(track: &str) -> Option<String> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    assert!(
        !loaded.report.iter().any(|l| l.contains("stand-in rig")),
        "the circuit fell back to the stand-in rig: {:#?}",
        loaded.report
    );
    let line = loaded.report.iter().find(|l| l.contains("sun [")).cloned();
    assert!(line.is_some(), "no authored rig line: {:#?}", loaded.report);
    line
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn tech_de_ra_is_lit_by_its_own_rig() {
    let Some(rig) = rig_line("Data\\Environments\\Tech_De_Ra\\track.vex") else {
        return;
    };
    // (0.00004, 0.00004, -0.00005) normalised.
    assert!(rig.contains("sun [0.53, 0.53, -0.66]"), "{rig}");
    assert!(rig.contains("colour [4.00, 3.47, 2.04]"), "{rig}");
    assert!(rig.contains("ambient [1.00, 1.00, 1.00]"), "{rig}");
    assert!(rig.contains("prelit 4.0*lightmap^2.5"), "{rig}");
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn modesto_heights_is_lit_by_its_own_rig() {
    let Some(rig) = rig_line("Data\\Environments\\Modesto_Heights\\track.vex") else {
        return;
    };
    // (-0.00003, 0.00004, -0.00006) normalised.
    assert!(rig.contains("sun [-0.38, 0.51, -0.77]"), "{rig}");
    assert!(rig.contains("colour [4.00, 3.61, 2.16]"), "{rig}");
    assert!(rig.contains("ambient [0.31, 0.26, 0.71]"), "{rig}");
    assert!(rig.contains("prelit 3.0*lightmap^3.0"), "{rig}");
}
