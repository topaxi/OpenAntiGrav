//! Wipeout Pure's `Speedup Pad` geometry decodes fine and used to draw nothing.
//!
//! `#[ignore]`d because it needs both a GPU adapter and a disc image, neither
//! of which CI has. Run it by hand:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(pad_alpha_test_ground_truth)'
//! ```
//!
//! # The regression this pins
//!
//! `vex::classes::V4` recovered Pure's `Speedup Pad`/`Weapon Pad` class ids
//! 2026-08-13, and `mesh::build_pads` is title-agnostic through that table -
//! so `Data\Environments\01_Vineta_K\track.vex` decoded 14 `Speedup Pad`
//! nodes, 952 triangles, correctly. **Every one of them still rendered as
//! zero visible pixels**, because `mesh.wgsl`'s `ALPHA_TEST_THRESHOLD` was an
//! invented `0.5` and Pure's `speedup_GLOW_KEY.tga` tops out at alpha
//! `58/255` (`0: background, 57..58: glow interior`, measured directly off
//! the decoded texture). A cutout batch discards below the threshold, so
//! every fragment of the pad discarded and the geometry decode looked
//! successful while the picture was empty.
//!
//! `docs/formats/vex.md` and `docs/overview/roadmap.md` carry the fuller
//! writeup; this file is the guard that keeps the fix from silently reverting
//! - a wrong threshold does not fail to compile, it fails to draw.

use std::path::Path;

use oag_assets::Archive;
use oag_render::mesh;
use oag_render::mesh_render::{self, Anisotropy};

/// `<image>:<archive-path>` for the Pure Data archive, or `None` if the disc
/// image is not present.
fn archive_spec() -> Option<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pure-psp-eu.chd");
    if path.exists() {
        return Some(format!("{}:PSP_GAME/USRDIR/Data.wad", path.display()));
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// Pixels whose combined channel sum clears the render's own clear colour
/// (`mesh_render`'s clear is `(0.06, 0.07, 0.09)`, summing to 56/255) by a
/// wide margin - wide enough that antialiasing fuzz at a triangle edge does
/// not count, only a fragment the pad's cutout pipeline actually kept.
fn lit_pixel_count(pixels: &[u8]) -> usize {
    pixels
        .chunks_exact(4)
        .filter(|px| u32::from(px[0]) + u32::from(px[1]) + u32::from(px[2]) > 100)
        .count()
}

#[test]
#[ignore = "needs a GPU adapter and a disc image in data/images/"]
fn a_pure_speedup_pad_draws_visible_pixels() {
    let Some(spec) = archive_spec() else {
        return;
    };
    let mut archive = Archive::open(&spec).expect("opening Pure's Data.wad");
    let name = r"Data\Environments\01_Vineta_K\track.vex";
    let blob = archive
        .read_name(name)
        .unwrap_or_else(|e| panic!("reading {name}: {e}"));

    let model = mesh::build_pads(name, &blob, None).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(
        !model.indices.is_empty(),
        "{name} authors no Speedup Pad geometry, so this test is not exercising the \
         regression - re-check the track name against oag-pure's own circuit list"
    );

    let pixels =
        mesh_render::capture_pixels_from(&model, 1024, 1024, 0.9, 1.4, Anisotropy::default(), 0.0)
            .expect("capturing the pad model");

    let lit = lit_pixel_count(&pixels);
    println!("{name}: {lit} lit pixel(s) of {}", pixels.len() / 4);
    // Measured at this exact framing: 0 before the `ALPHA_TEST_THRESHOLD` fix,
    // 321 after. The bound below sits well under that with headroom for a
    // different adapter's rasterization, and well over the handful of stray
    // pixels a bug unrelated to this one could produce.
    assert!(
        lit > 100,
        "{name}: only {lit} lit pixel(s) of 1,048,576 (a real pad draws ~321 here) - the \
         pad geometry decoded but drew (almost) nothing, which is exactly the \
         `ALPHA_TEST_THRESHOLD` regression this test exists to catch. See mesh.wgsl's \
         `ALPHA_TEST_THRESHOLD` doc comment."
    );
}
