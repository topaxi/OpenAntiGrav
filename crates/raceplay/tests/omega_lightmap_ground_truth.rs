//! Wipeout: Omega Collection lights a lightmapped surface with its own
//! `Lighting.Nova prelit scale bias power` triple, read from the circuit's
//! `.EnvSettings` and bound as the `nova` rig.
//!
//! **`#[ignore]`d and never run in CI.** It needs the corrected Omega
//! extraction (`data/extracted/ps4`, or `$OAG_OMEGA_SOURCE`).
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^omega_lightmap_ground_truth::/)'
//! ```
//!
//! The law itself (`pow(lightmap, power) * scale + bias`, no constant ambient)
//! is checked through the mesh pipeline in `oag-mesh`'s `omega_nova_prelit`;
//! what is checked here is that Omega's real file reaches it, that the
//! `Tonemap` block reads, and that 2048, which shares the container family and
//! authors the key too, does not.

use oag_raceplay as race;
use oag_tables::envsettings::{
    EnvSettings, NOVA_PRELIT, OMEGA_SUN_DIFFUSE_INTENSITY, PSP2_SUN_DIFFUSE_COLOUR,
};

fn source() -> Option<String> {
    let path = match std::env::var_os("OAG_OMEGA_SOURCE") {
        Some(path) => oag_testdata::repo_root().join(path),
        None => oag_testdata::exact("data/extracted/ps4")?,
    };
    Some(path.display().to_string())
}

/// Every circuit's own file, through the reader the race uses so the patch's copy wins,
/// carries the five bright-pass scalars and four blend factors (a census of 38 files;
/// `tonemap.md`, "The bloom chain"). The Bloom keys differ by circuit, and one circuit's
/// threshold sits above the curve's `1.0`, so its frame term never fires.
fn every_circuit_authors_a_whole_bloom_block(archives: &mut oag_assets::Archives) {
    const HD_LINEAGE: [&str; 14] = [
        "01_vineta_k",
        "02_metropia",
        "03_moa_therma",
        "04_chenghou_project",
        "05_ubermall",
        "10_sebenco_climb",
        "12_sol_2",
        "15_anulpha_pass",
        "amphiseum",
        "modesto_heights",
        "talons_junction",
        "tech_de_ra",
        "zone_1",
        "zone_2",
    ];
    const LINEAGE_2048: [&str; 10] = [
        "altima",
        "arena",
        "bridge",
        "cathedral",
        "mall",
        "park",
        "sol",
        "square",
        "subway",
        "tower",
    ];
    let mut names: Vec<String> = HD_LINEAGE
        .iter()
        .map(|c| format!(r"Data\environments\{c}\track.envsettings"))
        .collect();
    names.extend(
        HD_LINEAGE[..12]
            .iter()
            .map(|c| format!(r"Data\environments\{c}\track_reversed.envsettings")),
    );
    names.extend(["zone_3", "zone_4"].map(|c| format!(r"Data\environments\{c}\track.envsettings")));
    names.extend(
        LINEAGE_2048
            .iter()
            .map(|c| format!(r"Data\environments2048\{c}\track.envsettings")),
    );
    assert_eq!(names.len(), 38);
    let mut thresholds = Vec::new();
    for name in &names {
        let bytes = archives
            .read_name(name)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let text = String::from_utf8(bytes).expect("text");
        let env = EnvSettings::parse(&text).expect("parses");
        let block = env
            .bright_pass("Bloom")
            .unwrap_or_else(|| panic!("{name} lacks a whole Bloom block"));
        assert!(block.blend_factors.iter().all(|f| *f > 0.0), "{name}");
        thresholds.push(block.threshold);
    }
    assert!(thresholds.contains(&1.252), "Sol 2's own number");
    assert!(thresholds.contains(&0.784), "Vineta K's own number");
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
    every_circuit_authors_a_whole_bloom_block(&mut archives);
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
    // The diffuse sun is the authored colour times the authored intensity
    // (`vbroadcastss [0x019404b4]` times `[0x01e3de70]` in the executable);
    // dropping the multiply leaves the bare colour here.
    let intensity = env
        .scalar(OMEGA_SUN_DIFFUSE_INTENSITY)
        .expect("Omega authors an intensity");
    let colour = env.vec3(PSP2_SUN_DIFFUSE_COLOUR).expect("and a colour");
    assert!(intensity > 1.5, "a factor that is not the identity");
    for (i, c) in colour.iter().enumerate() {
        assert_eq!(light.sun[i], c * intensity, "channel {i}");
    }
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
    // The circuit's own `Vignette` keys reach the composite, and the rest values
    // of the other two terms (`colorscale.md`) are what it draws with them.
    let composite = tonemap.composite;
    assert_eq!(
        env.scalar("Vignette.Inner radius"),
        Some(composite.vignette_inner_radius)
    );
    assert_eq!(
        env.scalar("Vignette.Outer radius intensity"),
        Some(composite.vignette_outer_intensity)
    );
    assert!(
        (0.0..1.0).contains(&composite.vignette_outer_intensity),
        "a vignette darkens: {composite:?}"
    );
    assert_eq!(composite.color_scale, [1.0; 3]);
    assert_eq!(composite.screen_tint, [0.0; 3]);
    // The circuit's `Bloom` keys reach the composite; dropping them draws no bloom.
    let bloom = composite.bloom.expect("Tech De Ra's Bloom block");
    let authored = env.bright_pass("Bloom").expect("the block reads whole");
    assert_eq!(bloom.threshold, authored.threshold);
    assert_eq!(bloom.offset, authored.offset);
    assert_eq!(bloom.scale, authored.scale);
    assert_eq!(bloom.power, authored.power);
    assert_eq!(bloom.alpha_scale, authored.alpha_scale);
    assert_eq!(bloom.blend_factors, authored.blend_factors);
    assert!(
        loaded
            .report
            .iter()
            .any(|l| l.contains("composite vignette inner radius")),
        "{:#?}",
        loaded.report
    );
    // Omega registers 2048's `wo_*` programs and binds none: it ends in
    // `Tonemap`, so it never builds 2048's composite (checked, differs).
    assert!(loaded.wo_composite.is_none());
}
