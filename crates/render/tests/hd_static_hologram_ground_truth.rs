//! HD's `dc_hologramwithstatic`, from the material record to the vertex.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run with `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'test(/^hd_static_hologram_ground_truth::/)'
//! ```
//!
//! The program is on `docs/rendering/hd-track-tone.md`, "The Amphiseum hologram board": the
//! advert and the static scrolled by the record's own rates, the scanline mask as the alpha.
//! `hd_static_hologram_lit_path.rs` checks the shader's combine; this checks the real circuit
//! reaches it with its three pictures and the record's three numbers, and that the sibling
//! `dc_hologramwithstatic2`, a different program, is left to the generic reading.

use crate::archive_cache;

use oag_mesh::mesh::{self, PROGRAM_STATIC_HOLOGRAM, slots};

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn amphiseums_start_gantry_board_scrolls_its_advert_and_static_under_the_scan_mask() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", image.display());
    let path = "/data/environments/amphiseum/track.vex";
    let Some(data) = archive_cache::read(&spec, path) else {
        return;
    };
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data).expect("the sibling geometry");
    let (model, report) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        archive_cache::read(&spec, name)
    })
    .expect("the scene builds");
    assert_eq!(
        (report.static_hologram_bound, report.static_hologram_unread),
        (1, 0),
        "the board's one material slot (637) binds"
    );
    let mut bound = Vec::new();
    for draw in model.draws.iter().chain(&model.transparent_draws) {
        let Some(slot) = draw.texture else { continue };
        let first = model.indices[draw.range.start as usize] as usize;
        let index = slots::material_index(model.vertices[first].slots) as usize;
        if index != 0 && model.emissive[index - 1].program[0] == PROGRAM_STATIC_HOLOGRAM {
            bound.push((slot, model.emissive[index - 1].tint));
        }
    }
    bound.sort_by_key(|(slot, _)| *slot);
    bound.dedup();
    assert_eq!(bound.len(), 1, "{bound:?}");
    let (slot, tint) = bound[0];
    assert_eq!(slot, 637);
    // The record's `0x31182e0d`, `0xb6573513` and `0xeedee991`, read, not typed.
    assert_eq!(tint, [1.0, 4.5384, 0.73802]);
    let label = |slots: &mesh::TextureSlots| {
        slots[slot]
            .as_ref()
            .map_or_else(String::new, |t| t.label.to_ascii_lowercase())
    };
    assert!(label(&model.textures).contains("dc_scanlines"));
    assert!(label(&model.lightmaps).contains("advert_a"));
    assert!(label(&model.pad_masks).contains("dc_gradient_horizontal"));
    assert!(
        model
            .vertices
            .iter()
            .filter(|v| v.slots & slots::ADD_SECOND != 0)
            .count()
            > 0,
        "dc_hologramwithstatic2 (slot 340) keeps the generic additive second texture"
    );
}
