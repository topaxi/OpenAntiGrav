//! Vineta K's ceiling: the tunnel glass reads the screen behind it, and the
//! arch lights add their glow layer.
//!
//! **`#[ignore]`d and never run in CI**; needs the decrypted PS3 image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_vineta_ceiling_ground_truth)'
//! ```
//!
//! `mt_tunnelrefraction` and `cl_tunnelrefraction` declare the engine-bound
//! screen grab `0x88a0df95`; drawn as ordinary opaque lit surfaces they were
//! black panels over the sea. `2uv_offset_lights` accumulates its unit-1 glow
//! behind a write to `H2.w`, which `Program::accumulates` used to read as
//! taking the sample away, so the arch strips drew the honeycomb alone.

use oag_mesh::mesh::{self, slots};

fn vineta() -> (mesh::Model, mesh::rcs::Report, oag_rcs::rcsmodel::Model) {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso").expect("the PS3 image");
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let path = "/data/environments/01_vineta_k/track.vex";
    let data = mesh::read_blob(&spec, path).expect("the .vex reads");
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data).expect("a sibling .rcsmodel");
    let rcs = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");
    let (model, report) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        mesh::read_blob(&spec, name).ok()
    })
    .expect("Vineta K builds");
    (model, report, rcs)
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_tunnel_glass_is_two_blended_passes_over_the_screen() {
    if oag_testdata::image("hdfury-ps3-eu-dec.iso").is_none() {
        return;
    }
    let (model, report, rcs) = vineta();
    assert_eq!(report.refraction_bound, 3, "{}", report.describe());
    assert_eq!(report.refraction_unread, 0);
    let glass: Vec<usize> = rcs
        .materials
        .iter()
        .enumerate()
        .filter(|(_, m)| m.name.ends_with("tunnelrefraction.rcsmaterial"))
        .map(|(slot, _)| slot)
        .collect();
    assert!(!glass.is_empty());
    let word_of = |draw: &mesh::DrawCall| {
        model.vertices[model.indices[draw.range.start as usize] as usize].slots
    };
    for (list, name) in [
        (&model.draws, "opaque"),
        (&model.alpha_tested_draws, "cutout"),
    ] {
        assert!(
            list.iter().all(|d| word_of(d) & slots::REFRACTION == 0),
            "a refraction chunk is in the {name} list"
        );
    }
    let grab: Vec<_> = model
        .transparent_draws
        .iter()
        .filter(|d| word_of(d) & slots::REFRACTION != 0)
        .collect();
    let (scale, add): (Vec<&&mesh::DrawCall>, Vec<&&mesh::DrawCall>) = grab
        .iter()
        .partition(|d| word_of(d) & slots::REFRACT_GRAB != 0);
    assert!(!scale.is_empty());
    assert_eq!(
        scale.len(),
        add.len(),
        "every chunk is drawn in both passes"
    );
    assert!(scale.iter().all(|d| {
        d.blend_state
            .is_some_and(|b| b.color.dst_factor == wgpu::BlendFactor::Src)
    }));
    assert!(add.iter().all(|d| {
        d.blend_state
            .is_some_and(|b| b.color.dst_factor == wgpu::BlendFactor::One)
    }));
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_arch_light_strips_carry_their_glow_layer() {
    if oag_testdata::image("hdfury-ps3-eu-dec.iso").is_none() {
        return;
    }
    let (model, _, rcs) = vineta();
    let slot = rcs
        .materials
        .iter()
        .position(|m| m.name.ends_with("/2uv_offset_lights.rcsmaterial"))
        .expect("Vineta K carries 2uv_offset_lights");
    let words: Vec<u32> = model
        .draws
        .iter()
        .filter(|d| d.texture == Some(slot))
        .map(|d| model.vertices[model.indices[d.range.start as usize] as usize].slots)
        .collect();
    assert!(!words.is_empty(), "slot {slot} draws nothing");
    assert!(
        words.iter().all(|w| w & slots::ADD_SECOND != 0),
        "slot {slot} lost its additive glow layer"
    );
    assert!(
        model.lightmaps[slot].is_some(),
        "slot {slot} has no glow texture bound"
    );
}
