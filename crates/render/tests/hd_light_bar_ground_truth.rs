//! `diffuse_normal_specular_emmissive` (start line, pit lane, rail walls) and
//! `water_test_2` (Vineta K's sea) against the capture that read them.
//!
//! **`#[ignore]`d and never run in CI**; needs the decrypted PS3 image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'test(/^hd_light_bar_ground_truth::/)'
//! ```
//!
//! The reading (`docs/formats/rcsmaterial.md`, "The start line's light bars"):
//! the emissive family's fragment program samples the `_ne` file at unit 1
//! and adds `_ne.a * 0x7611a2d8` after the light, exactly as the pad programs
//! do, and the model authors that colour per material (`(0, 0.25, 1)` on
//! `ds_pit`/`ds_wall`, the azure panels of the original's start line). The
//! scene bound none of it, so those panels drew dark.

use crate::archive_cache;

use oag_mesh::mesh::{self, slots};

const IMAGE: &str = "hdfury-ps3-eu-dec.iso";

fn archive(image: &std::path::Path, n: u32) -> String {
    format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", image.display())
}

fn build(path: &str) -> Option<(mesh::Model, mesh::rcs::Report)> {
    let image = oag_testdata::image(IMAGE)?;
    let (spec, data) = (0..4).find_map(|n| {
        let spec = archive(&image, n);
        archive_cache::read(&spec, path).map(|d| (spec, d))
    })?;
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data)?;
    mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        (0..4).find_map(|n| archive_cache::read(&archive(&image, n), name))
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

/// The sea sheet (`water_test_2`, chunks 6 and 23 each way) carries render flag
/// `0x10`: the original draws it only into its behind-the-glass target, so the
/// main view leaves it out (`hd_behind_glass_ground_truth`). Its shading, lit
/// vertex colour (`slots::ICE` with the water rate), is the reading for that
/// target and reaches no main-view vertex.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn vineta_k_sea_sheet_is_behind_the_glass_not_in_the_main_view() {
    let Some((model, report)) = build("/data/environments/01_vineta_k/track.vex") else {
        return;
    };
    // The material is still read (the count is per material); its chunks are not built.
    assert_eq!(report.water_lit_colour, 1);
    assert!(report.behind_glass > 0);
    assert!(
        model.vertices.iter().all(|v| v.slots & slots::ICE == 0),
        "no sea-sheet vertex in the main view"
    );
}

/// Counter-clockwise and clockwise triangles of the draws `model` holds for
/// `chunk`, seen from `eye` (right-handed: a triangle is counter-clockwise when
/// the eye is on the side its `(b - a) x (c - a)` normal points to).
fn facing(model: &mesh::Model, chunk: u32, eye: [f32; 3]) -> (usize, usize) {
    let mut front = 0;
    let mut back = 0;
    for draw in model
        .draws
        .iter()
        .chain(&model.alpha_tested_draws)
        .chain(&model.transparent_draws)
        .filter(|draw| draw.chunk == Some(chunk))
    {
        for triangle in model.indices[draw.range.start as usize..draw.range.end as usize].chunks(3)
        {
            let [a, b, c] = [0, 1, 2].map(|i| model.vertices[triangle[i] as usize].position);
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let normal = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            let to_eye = [eye[0] - a[0], eye[1] - a[1], eye[2] - a[2]];
            let side = normal[0] * to_eye[0] + normal[1] * to_eye[1] + normal[2] * to_eye[2];
            if side > 0.0 {
                front += 1;
            } else {
                back += 1;
            }
        }
    }
    (front, back)
}

/// The law behind per-material back-face culling (state bit 4,
/// `docs/ghidra/functions/ps3-hdfury-eu/chunk-cull.md`): Vineta K's chunk 1556,
/// the navy slab the original does not draw at the start grid, has only
/// clockwise triangles from the eye of that capture (`45.63, 40.12, -94.99`,
/// two surfaces of 20 triangles), so a culling draw shows nothing of it. The
/// controls keep the sign convention honest: chunk 1557 is 56 counter-clockwise
/// triangles from the same eye, chunks 1554 and 1555 are 12 and 12.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn vineta_ks_wall_top_slab_has_no_front_facing_triangle_from_the_start_grid() {
    let Some((model, _)) = build("/data/environments/01_vineta_k/track.vex") else {
        return;
    };
    let eye = [45.63, 40.12, -94.99];
    assert_eq!(facing(&model, 1556, eye), (0, 40), "chunk 1556 faces away");
    assert_eq!(facing(&model, 1557, eye), (56, 0), "control: 1557 faces it");
    for chunk in [1554, 1555] {
        assert_eq!(
            facing(&model, chunk, eye),
            (12, 12),
            "control: chunk {chunk}"
        );
    }
}
