//! `paraboloidReflectionTex` is the circuit's sky, and scenery glass reads it.
//!
//! **`#[ignore]`d and never run in CI**; needs the decrypted PS3 image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'test(/^hd_sky_reflection_ground_truth::/)'
//! ```
//!
//! The reference is a cold-boot read of the engine's 512x256 reflection target (`0xc4065380`)
//! on RPCS3 with `Write Color Buffers` on, Moa Therma, Time Trial, 2026-10-21. The ten texels
//! below are that read; `mesh::sky_paraboloid` rebuilds them from `sky.gtf` to within a few
//! bytes (mean absolute error 1.0 over the whole disc, 2.4 rms). Changing the law (the `z` sign,
//! the disc layout, a gamma step or the sky's rotation) moves them by tens.

use crate::archive_cache;

use oag_mesh::mesh;

/// `(x, y, [r, g, b])` of the original's map.
const READ: [(usize, usize, [u8; 3]); 10] = [
    (128, 40, [130, 155, 157]),
    (100, 100, [140, 184, 181]),
    (60, 120, [201, 214, 192]),
    (384, 40, [81, 83, 76]),
    (420, 110, [139, 163, 133]),
    (300, 125, [162, 175, 171]),
    (128, 200, [0, 56, 49]),
    (384, 200, [0, 56, 49]),
    (200, 60, [108, 135, 131]),
    (330, 80, [84, 124, 109]),
];

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn moas_sky_rebuilds_the_original_reflection_target() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let blob = archive_cache::read(&spec, "/data/environments/03_track/sky.gtf").expect("sky.gtf");
    let sky = mesh::sky_cube::build("sky", &blob, 0.0, [1.0; 3], 0.0).expect("a cubemap");
    let map = mesh::sky_paraboloid::from_sky(&sky).expect("six faces");
    let (width, height) = mesh::sky_paraboloid::SIZE;
    assert_eq!(map.len(), (width * height * 4) as usize);
    for (x, y, want) in READ {
        let at = (y * width as usize + x) * 4;
        for channel in 0..3 {
            let got = map[at + channel];
            assert!(
                got.abs_diff(want[channel]) <= 8,
                "({x}, {y}) channel {channel}: built {got}, the original's map holds {}",
                want[channel]
            );
        }
    }
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn moas_window_glass_is_tagged_to_read_the_sky() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let path = "/data/environments/03_track/track.vex";
    let data = archive_cache::read(&spec, path).expect("the .vex reads");
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data).expect("a sibling .rcsmodel");
    let (model, report) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        archive_cache::read(&spec, name)
    })
    .expect("Moa Therma builds");
    assert!(report.window_glass_bound > 0, "{}", report.describe());
    let weights: Vec<f32> = model
        .emissive
        .iter()
        .filter(|e| e.rate == mesh::WINDOW_REFLECTION_RATE)
        .map(|e| e.tint[0])
        .collect();
    assert!(
        !weights.is_empty() && weights.iter().all(|&w| (w - 0.8).abs() < 1e-6),
        "the glass weight (parameter 0x5e5b1937) is 0.8 on every instance: {weights:?}"
    );
    trees_draw_among_the_cutouts_and_bit_2_blends_write_depth(model);
}

/// The same build, honouring HD's render-state rule
/// (`oag_title::Looks::material_render_state`): every `cf_tree` draw (`0xbd`,
/// state bit 7) leaves the blended list for the cutout list, unblended, as all
/// 31 tree draws of a live Moa Therma frame are issued; the `0x3d`/`0x2d`
/// blends that frame draws with the depth mask on write depth, and the `0x39`
/// glass in front of which the crowns were cut off does not.
/// `docs/ghidra/functions/ps3-hdfury-eu/material-state.md`, "The race pass sets
/// pass bits 3, 5 and 6".
fn trees_draw_among_the_cutouts_and_bit_2_blends_write_depth(mut model: mesh::Model) {
    let state = |model: &mesh::Model, draw: &mesh::DrawCall| model.state_of(draw.texture);
    let trees_before = model
        .transparent_draws
        .iter()
        .filter(|d| state(&model, d) == 0xbd)
        .count();
    assert!(
        trees_before > 0,
        "Moa Therma's cf_tree chunks are blended in the file"
    );
    let cutouts_before = model.alpha_tested_draws.len();

    let moved = model.honour_render_state();

    assert_eq!(moved, trees_before, "every 0xbd draw moves");
    assert_eq!(model.alpha_tested_draws.len(), cutouts_before + moved);
    assert!(
        model
            .transparent_draws
            .iter()
            .all(|d| state(&model, d) & mesh::render_state::STATE_COVERAGE == 0),
        "no coverage draw stays blended"
    );
    for tree in &model.alpha_tested_draws[cutouts_before..] {
        assert_eq!(state(&model, tree), 0xbd);
        assert_eq!(tree.blend_state, None);
        assert_eq!(
            tree.alpha_test_ref,
            Some(mesh::render_state::COVERAGE_REFERENCE)
        );
    }
    let writes = |word: u32| {
        model
            .transparent_draws
            .iter()
            .filter(|d| state(&model, d) == word)
            .map(|d| d.writes_depth)
            .collect::<Vec<_>>()
    };
    for word in [0x3d, 0x2d] {
        let w = writes(word);
        assert!(!w.is_empty() && w.iter().all(|&w| w), "{word:#x}: {w:?}");
    }
    let glass = writes(0x39);
    assert!(
        !glass.is_empty() && glass.iter().all(|&w| !w),
        "0x39: {glass:?}"
    );
}
