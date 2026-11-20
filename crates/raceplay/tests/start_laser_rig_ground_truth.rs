//! Basilico's start laser is drawn under the circuit's own GE light rig.
//!
//! One frame of the original on Basilico's grid replays a lighting list (ambient `0x7a7c90`,
//! two directional lights) just before the laser strip's draw: the strip carries normals and
//! no colour, so the rig is what colours it, and it sums to white across the strip. The
//! track pass lit it with the stand-in, which left it at a little over half the brightness
//! (`docs/tools/compare-page.md`, "basilico-black"). The loader marks the strip's and the
//! pulse's colourless vertices `lit` 2.0, which the track pass reads as "this rig".
//!
//! The rule is the batch's, not the texture's: every vertex with normals and no colour takes the
//! rig (one frame of each of six circuits' grids: every such batch bar the hull's extra pass is
//! drawn with lighting on and the circuit's lights, `docs/ghidra/functions/psp-pulse-usa/scene-light.md`,
//! "Which models take the rig"). Only the animated flash sprites keep the stand-in.

use oag_mesh::mesh_render::HullLights;
use oag_raceplay as race;

const STRIP: &str = "startline_laser_ADD_GLOW.tga";

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_laser_strip_takes_the_circuits_rig_and_the_road_does_not() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        track: Some(r"Data\Environments\01_Track\track.vex".to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let model = &loaded.track_model;
    let strip = model
        .transparent_draws
        .iter()
        .find(|draw| {
            draw.texture
                .and_then(|t| model.textures.get(t))
                .and_then(Option::as_ref)
                .is_some_and(|t| t.label.eq_ignore_ascii_case(STRIP))
        })
        .expect("Basilico authors the laser strip as a blended batch");
    let lit: Vec<f32> = strip
        .range
        .clone()
        .map(|at| model.vertices[model.indices[at as usize] as usize].lit)
        .collect();
    assert_eq!(
        lit.len(),
        63,
        "the strip is 21 triangles (23 strip vertices)"
    );
    assert!(
        lit.iter().all(|&lit| lit == HullLights::TRACK_LIT),
        "every strip vertex is marked for the rig: {lit:?}"
    );
    assert!(
        loaded.light.hull.enabled(),
        "the circuit authors the rig the strip is lit by"
    );
    let drawn = |model: &oag_mesh::mesh::Model| {
        model
            .draws
            .iter()
            .chain(&model.alpha_tested_draws)
            .chain(&model.transparent_draws)
            .map(|draw| {
                let label = draw
                    .texture
                    .and_then(|t| model.textures.get(t))
                    .and_then(Option::as_ref)
                    .map(|t| t.label.to_ascii_lowercase())
                    .unwrap_or_default();
                let lits: Vec<f32> = draw
                    .range
                    .clone()
                    .map(|at| model.vertices[model.indices[at as usize] as usize].lit)
                    .collect();
                (label, lits)
            })
            .collect::<Vec<_>>()
    };
    let draws = drawn(model);
    assert!(
        draws
            .iter()
            .all(|(_, lits)| lits.iter().all(|&lit| lit != 1.0)),
        "no colourless vertex with normals is left on the stand-in on Basilico"
    );
    let marked: usize = draws
        .iter()
        .map(|(_, lits)| {
            lits.iter()
                .filter(|&&lit| lit == HullLights::TRACK_LIT)
                .count()
        })
        .sum();
    assert!(
        marked > lit.len(),
        "more than the laser strip is marked: {marked} vertices"
    );
    assert!(
        draws
            .iter()
            .filter(|(_, lits)| lits.iter().all(|&lit| lit == 0.0))
            .count()
            > 100,
        "vertex-coloured batches stay unmarked"
    );
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.starts_with("track rig: ") && !line.starts_with("track rig: 0 ")),
        "the loader reports the marking: {:?}",
        loaded.report
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_flash_sprites_keep_the_stand_in() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        track: Some(r"Data\Environments\05_Track\track.vex".to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let model = &loaded.track_model;
    let flash: Vec<f32> = model
        .transparent_draws
        .iter()
        .chain(&model.draws)
        .chain(&model.alpha_tested_draws)
        .filter(|draw| {
            draw.texture
                .and_then(|t| model.textures.get(t))
                .and_then(Option::as_ref)
                .is_some_and(|t| t.label.eq_ignore_ascii_case("cameraflash_glow_add.tga"))
        })
        .flat_map(|draw| draw.range.clone())
        .map(|at| model.vertices[model.indices[at as usize] as usize].lit)
        .collect();
    assert!(!flash.is_empty(), "De Konstruct authors flash sprites");
    assert!(
        flash.iter().all(|&lit| lit == 1.0),
        "the flash sprites are held back from the rig"
    );
}
