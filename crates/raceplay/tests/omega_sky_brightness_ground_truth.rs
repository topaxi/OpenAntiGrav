//! Wipeout: Omega Collection draws its dome as `Lighting.Sky brightness` times the texture.
//!
//! **`#[ignore]`d and never run in CI.** It needs the corrected Omega extraction
//! (`data/extracted/ps4`, or `$OAG_OMEGA_SOURCE`).
//!
//! The dome's pixel program is `texture * SkyBrightness` and nothing else, and its constant reads
//! equal to the circuit's authored key (`docs/formats/omega-status.md`, "The sky dome's pixel
//! program"). What this pins: the loader hands the dome the gain the circuit authors, on a
//! HD-lineage and on a 2048-lineage circuit, and the vertex colour is the only thing that moved.
//!
//! Sebenco Climb's load also carries the snow check (`snow_is_drawn_by_its_own_program`): the
//! circuit is loaded once and both claims read it ("The snow program" in the same page).

use oag_raceplay as race;
use oag_tables::envsettings::{EnvSettings, SKY_BRIGHTNESS};

fn source() -> Option<String> {
    let path = match std::env::var_os("OAG_OMEGA_SOURCE") {
        Some(path) => oag_testdata::repo_root().join(path),
        None => oag_testdata::exact("data/extracted/ps4")?,
    };
    Some(path.display().to_string())
}

fn dome_gain_is_the_authored_brightness(track: &str, authored_at_least: f32) -> Option<race::Loaded> {
    let source = source()?;
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
    Some(loaded)
}

/// `RT_snow1` is drawn by its pixel program: the track model carries the mode's three entries
/// with the material's own constants, and the snow submeshes' vertices name them and carry the
/// baked light (`colour.a` 1, `sun_mask` a mask in 0..1, the drift mask in the second pair).
fn snow_is_drawn_by_its_own_program(loaded: &race::Loaded) {
    use oag_mesh::mesh::PROGRAM_OMEGA_SNOW;
    let source = source().expect("the source the load used");
    let mut archives = oag_omega::open(&source).expect("opening the Omega source");
    let blob = archives
        .read_name(r"Data\Environments\10_sebenco_climb\track.final.rcsmodel")
        .expect("the circuit's model");
    let model = oag_rcs::rcsmodel::psp2::parse(&blob).expect("parses");
    let material = model
        .materials
        .iter()
        .find(|m| m.name.to_ascii_lowercase().ends_with("/rt_snow1.rcsmaterial"))
        .expect("the circuit's snow material");
    let authored = |name: &str| {
        material
            .param(oag_rcs::rcsmaterial::name_hash(name))
            .unwrap_or_else(|| panic!("{name} is authored"))
    };
    let table = &loaded.track_model.emissive;
    let first = table
        .iter()
        .position(|e| e.program[0] == PROGRAM_OMEGA_SNOW)
        .expect("a snow entry in the glow table");
    let [fresnel, spec, blend] = [&table[first], &table[first + 1], &table[first + 2]];
    let close = |a: f32, b: f32, what: &str| assert!((a - b).abs() < 1e-5, "{what}: {a} against {b}");
    close(fresnel.offset, authored("FresnelMin")[0], "FresnelMin");
    close(fresnel.scale, authored("FresnelScale")[0], "FresnelScale");
    close(fresnel.program[1], authored("DiffuseWrap")[0], "DiffuseWrap");
    close(
        fresnel.tint[1],
        authored("FresnelStrength")[0] * authored("FresnelColour")[1],
        "FresnelStrength * FresnelColour",
    );
    close(spec.rate, authored("FresnelPower")[0], "FresnelPower");
    close(spec.offset, authored("SpecPower")[0], "SpecPower");
    close(spec.scale, authored("SpecTwinkleScale")[0], "SpecTwinkleScale");
    close(blend.tint[0], authored("snownormDetailStrength")[0], "snownormDetailStrength");
    close(blend.tint[1], authored("snownormDetailScale")[0], "snownormDetailScale");
    close(blend.tint[2], authored("snownormDetailLargeScale")[0], "snownormDetailLargeScale");
    let slot = u32::try_from(first + 1).expect("a small index");
    let named: Vec<_> = loaded
        .track_model
        .vertices
        .iter()
        .filter(|v| oag_mesh::mesh::slots::material_index(v.slots) == slot)
        .collect();
    assert!(named.len() > 10_000, "{} vertices name the snow entry", named.len());
    for v in &named {
        assert_eq!(v.colour[3], 1.0);
        assert!((0.0..=1.0).contains(&v.sun_mask), "{}", v.sun_mask);
        assert!((0.0..=1.0).contains(&v.texcoord2[1]), "{}", v.texcoord2[1]);
    }
    assert!(
        loaded.report.iter().any(|l| l.contains("snow material(s) lit by their own pixel program")),
        "{:#?}",
        loaded.report
    );
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn sebenco_climb_dome_is_scaled_by_its_authored_brightness() {
    let Some(loaded) =
        dome_gain_is_the_authored_brightness(r"Data\Environments\10_sebenco_climb\track.vex", 1.5)
    else {
        return;
    };
    snow_is_drawn_by_its_own_program(&loaded);
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn a_2048_lineage_dome_is_scaled_by_its_authored_brightness() {
    dome_gain_is_the_authored_brightness(r"Data\Environments2048\sol\track.vex", 1.0);
}
