//! `diffuse_normal_specular_emmissive` (start line, pit lane, rail walls) and
//! `water_test_2` (Vineta K's sea) against the capture that read them.
//!
//! **`#[ignore]`d and never run in CI**; needs the decrypted PS3 image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_light_bar_ground_truth)'
//! ```
//!
//! The reading (`docs/formats/rcsmaterial.md`, "The start line's light bars"):
//! the emissive family's fragment program samples the `_ne` file at unit 1
//! and adds `_ne.a * 0x7611a2d8` after the light, exactly as the pad programs
//! do, and the model authors that colour per material (`(0, 0.25, 1)` on
//! `ds_pit`/`ds_wall`, the azure panels of the original's start line). The
//! scene bound none of it, so those panels drew dark.

use oag_mesh::mesh::{self, slots};

const IMAGE: &str = "hdfury-ps3-eu-dec.iso";

fn archive(image: &std::path::Path, n: u32) -> String {
    format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", image.display())
}

fn build(path: &str) -> Option<(mesh::Model, mesh::rcs::Report)> {
    let image = oag_testdata::image(IMAGE)?;
    let (spec, data) = (0..4).find_map(|n| {
        let spec = archive(&image, n);
        mesh::read_blob(&spec, path).ok().map(|d| (spec, d))
    })?;
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data)?;
    mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        (0..4).find_map(|n| mesh::read_blob(&archive(&image, n), name).ok())
    })
    .ok()
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn vineta_k_start_line_binds_its_light_bars_in_the_authored_azure() {
    let Some((model, report)) = build("/data/environments/01_vineta_k/track.vex") else {
        return;
    };
    assert!(
        report.pad_ne_bound >= 13,
        "the 13 emissive-family slots bind their mask, got {}",
        report.pad_ne_bound
    );
    let bars: Vec<_> = model
        .vertices
        .iter()
        .filter(|v| v.slots & slots::PAD_NE != 0)
        .collect();
    assert!(!bars.is_empty(), "vertices carry the PAD_NE role");
    let azure = bars.iter().any(|v| {
        let index = slots::material_index(v.slots) as usize;
        index > 0
            && model
                .emissive
                .get(index - 1)
                .is_some_and(|e| e.tint == [0.0, 0.25, 1.0])
    });
    assert!(
        azure,
        "ds_pit's authored (0, 0.25, 1) colour reaches a vertex"
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn vineta_k_sea_sheet_is_lit_vertex_colour_not_a_white_picture() {
    let Some((model, report)) = build("/data/environments/01_vineta_k/track.vex") else {
        return;
    };
    assert_eq!(report.water_lit_colour, 1);
    let water: Vec<_> = model
        .vertices
        .iter()
        .filter(|v| v.slots & slots::ICE != 0)
        .collect();
    assert!(!water.is_empty(), "the sea sheet carries the ICE role bit");
    assert!(
        water.iter().all(|v| {
            let index = slots::material_index(v.slots) as usize;
            model.emissive.get(index - 1).is_some_and(|e| e.rate == 2.0)
        }),
        "and every such vertex is flagged as water, not as the ice pool"
    );
}
