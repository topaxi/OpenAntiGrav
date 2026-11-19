//! Basilico's start laser is drawn under the circuit's own GE light rig.
//!
//! One frame of the original on Basilico's grid replays a lighting list (ambient `0x7a7c90`,
//! two directional lights) just before the laser strip's draw: the strip carries normals and
//! no colour, so the rig is what colours it, and it sums to white across the strip. The
//! track pass lit it with the stand-in, which left it at a little over half the brightness
//! (`docs/tools/compare-page.md`, "basilico-black"). The loader marks the strip's and the
//! pulse's colourless vertices `lit` 2.0, which the track pass reads as "this rig".

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
    let road_unmarked = model
        .draws
        .iter()
        .flat_map(|draw| draw.range.clone())
        .all(|at| model.vertices[model.indices[at as usize] as usize].lit != HullLights::TRACK_LIT);
    assert!(road_unmarked, "an opaque batch is never marked");
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.starts_with("start laser: ") && !line.starts_with("start laser: 0 ")),
        "the loader reports the marking: {:?}",
        loaded.report
    );
}
