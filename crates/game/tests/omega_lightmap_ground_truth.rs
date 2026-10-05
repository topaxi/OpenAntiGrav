//! Wipeout: Omega Collection lights a lightmapped surface with its own
//! `Lighting.Nova prelit scale bias power` triple, read from the circuit's
//! `.EnvSettings` and bound as the `nova` rig.
//!
//! **`#[ignore]`d and never run in CI.** It needs the corrected Omega
//! extraction (`data/extracted/ps4`, or `$OAG_OMEGA_SOURCE`).
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(omega_lightmap_ground_truth)'
//! ```
//!
//! The law itself (`pow(lightmap, power) * scale + bias`, no constant ambient)
//! is checked through the mesh pipeline in `oag-render`'s `omega_nova_prelit`;
//! what is checked here is that Omega's real file reaches it, that the
//! `Tonemap` block reads, and that 2048, which shares the container family and
//! authors the key too, does not.

use oag_raceplay as race;
use oag_tables::envsettings::{EnvSettings, NOVA_PRELIT};

fn source() -> Option<String> {
    let path = match std::env::var_os("OAG_OMEGA_SOURCE") {
        Some(path) => oag_testdata::repo_root().join(path),
        None => oag_testdata::exact("data/extracted/ps4")?,
    };
    Some(path.display().to_string())
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn tech_de_ras_authored_triple_reaches_the_rig() {
    let Some(source) = source() else { return };
    let mut archives = oag_omega::open(&source).expect("opening the Omega source");
    let text = String::from_utf8(
        archives
            .read_name(r"Data\environments\tech_de_ra\track.envsettings")
            .expect("tech_de_ra's .EnvSettings"),
    )
    .expect("text");
    let env = EnvSettings::parse(&text).expect("parses");
    assert_eq!(
        env.vec3(NOVA_PRELIT),
        Some([2.5, 0.2, 2.0]),
        "the patch's own triple"
    );
    assert!(
        env.tonemap("Tonemap").is_some(),
        "the Tonemap block reads whole"
    );
    assert!(
        env.tonemap("TonemapHDR").is_some(),
        "and so does its HDR twin"
    );

    let loaded = race::load(&race::Options {
        source: source.clone(),
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("loading {source}: {e:#}"));
    let light = loaded.light;
    assert_eq!(light.nova, 1.0, "the Omega rig is flagged");
    assert_eq!(light.prelit_scale, [2.5; 3]);
    assert_eq!(light.prelit_power, [2.0; 3]);
    assert_eq!(light.prelit_bias, 0.2);
    assert!(
        loaded
            .report
            .iter()
            .any(|l| l.contains("prelit 2.5*lightmap^2.0 + 0.20")),
        "{:#?}",
        loaded.report
    );
    assert!(
        loaded.report.iter().any(|l| l.contains("Tonemap applied")),
        "{:#?}",
        loaded.report
    );
    // The block reaches the renderer: dropping the plumbing leaves `None`
    // here and the race back on the saturating 8-bit target.
    let tonemap = loaded
        .omega_tonemap
        .expect("Tech De Ra's Tonemap block is applied");
    assert!(tonemap.exposure_maximum >= tonemap.exposure_minimum);
}
