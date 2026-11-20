//! Wipeout: Omega Collection draws its dome as `Lighting.Sky brightness` times the texture.
//!
//! **`#[ignore]`d and never run in CI.** It needs the corrected Omega extraction
//! (`data/extracted/ps4`, or `$OAG_OMEGA_SOURCE`).
//!
//! The dome's pixel program is `texture * SkyBrightness` and nothing else, and its constant reads
//! equal to the circuit's authored key (`docs/formats/omega-status.md`, "The sky dome's pixel
//! program"). What this pins: the loader hands the dome the gain the circuit authors, on a
//! HD-lineage and on a 2048-lineage circuit, and the vertex colour is the only thing that moved.

use oag_raceplay as race;
use oag_tables::envsettings::{EnvSettings, SKY_BRIGHTNESS};

fn source() -> Option<String> {
    let path = match std::env::var_os("OAG_OMEGA_SOURCE") {
        Some(path) => oag_testdata::repo_root().join(path),
        None => oag_testdata::exact("data/extracted/ps4")?,
    };
    Some(path.display().to_string())
}

fn dome_gain_is_the_authored_brightness(track: &str, authored_at_least: f32) {
    let Some(source) = source() else { return };
    let mut archives = oag_omega::open(&source).expect("opening the Omega source");
    let name = track.replace(".vex", ".envsettings");
    let text = String::from_utf8(archives.read_name(&name).expect("the circuit's settings"))
        .expect("text");
    let authored = EnvSettings::parse(&text)
        .expect("parses")
        .scalar(SKY_BRIGHTNESS)
        .unwrap_or_else(|| panic!("{track}: no Sky brightness"));
    assert!(
        authored > authored_at_least,
        "{track}: a gain that is not the identity, {authored}"
    );
    let loaded = race::load(&race::Options {
        source: source.clone(),
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("{track}: {e:#}"));
    let sky = loaded.sky_model.as_ref().expect("the circuit's dome");
    let want = oag_gpu::formats::SceneStore::SrgbTexels.texel_gain(authored);
    assert!(want > 1.0, "{track}: {want}");
    for (k, v) in sky.vertices.iter().enumerate() {
        for channel in 0..3 {
            assert!(
                (v.colour[channel] - want).abs() < 1e-6,
                "{track}: vertex {k} channel {channel}: {} against {want}",
                v.colour[channel]
            );
        }
    }
    assert!(
        loaded
            .report
            .iter()
            .any(|l| l.contains("the circuit's sky dome") && l.contains("`Sky brightness`")),
        "{:#?}",
        loaded.report
    );
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn sebenco_climb_dome_is_scaled_by_its_authored_brightness() {
    dome_gain_is_the_authored_brightness(r"Data\Environments\10_sebenco_climb\track.vex", 1.5);
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn modesto_heights_dome_is_scaled_by_its_authored_brightness() {
    dome_gain_is_the_authored_brightness(r"Data\Environments\modesto_heights\track.vex", 2.2);
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn a_2048_lineage_dome_is_scaled_by_its_authored_brightness() {
    dome_gain_is_the_authored_brightness(r"Data\Environments2048\sol\track.vex", 1.0);
}
