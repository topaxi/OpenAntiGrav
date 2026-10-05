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
//!
//! # And the guard that a lit-pixel count alone does not give
//!
//! The count above is one-sided, and the second failure this file now pins
//! walked straight past it: `mesh.wgsl` discarded at `shaded.a <
//! alpha_test_ref`, which is `GEQUAL` and not the `GU_GREATER` the GE
//! programs. That is invisible while the only reference is Wipeout HD's
//! `0.5`, and fatal at the `0` Pure's pad asks for - `a < 0.0` is true of no
//! fragment, so the alpha test became a no-op and the pad painted its
//! texture's transparent background as a solid plate. **A square instead of a
//! pad, and `lit > 100` passed harder than before**, because a filled quad is
//! strictly more lit pixels than a cutout of it.
//!
//! So the pad is captured twice: once as its file asks, and once with the
//! test forced off. A cutout that keeps every fragment is the regression, and
//! only the comparison sees it.

use std::path::Path;

use oag_assets::Archive;
use oag_mesh::mesh;
use oag_mesh::mesh_render::{self, Anisotropy};

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
        .as_chunks::<4>()
        .0
        .iter()
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

    let mut model = mesh::build_pads(name, &blob, None).unwrap_or_else(|e| panic!("{name}: {e}"));
    // **This guard is about the discard operator, not about level selection.**
    // The orbit framing below is a viewer's, hundreds of units from every pad,
    // where the GE's slope rule would pick a coarse authored level - and a
    // coarse level of the glow texture has alpha above 0 nearly everywhere, so
    // `GU_GREATER` 0 keeps the plate whole and the two captures below stop
    // differing for a reason that is not the operator. Base level only keeps
    // the comparison about what it names. `psp_slope_lod.rs` owns the selection.
    for slot in &mut model.textures {
        if let Some(texture) = slot
            && let mesh::Texels::Chain(levels) = &texture.texels
        {
            *slot = Some(std::sync::Arc::new(mesh::ModelTexture::rgba8(
                texture.label.clone(),
                texture.width,
                texture.height,
                levels[0].clone(),
                None,
            )));
        }
    }
    assert!(
        !model.indices.is_empty(),
        "{name} authors no Speedup Pad geometry, so this test is not exercising the \
         regression - re-check the track name against oag-pure's own circuit list"
    );

    let pixels = capture(&model);
    let lit = lit_pixel_count(&pixels);
    println!("{name}: {lit} lit pixel(s) of {}", pixels.len() / 4);
    // Measured at this exact framing: 0 before the `ALPHA_TEST_THRESHOLD` fix,
    // 310 after. The bound below sits well under that with headroom for a
    // different adapter's rasterization, and well over the handful of stray
    // pixels a bug unrelated to this one could produce.
    assert!(
        lit > 100,
        "{name}: only {lit} lit pixel(s) of 1,048,576 (a real pad draws ~310 here) - the \
         pad geometry decoded but drew (almost) nothing, which is exactly the \
         `ALPHA_TEST_THRESHOLD` regression this test exists to catch. See mesh.wgsl's \
         `ALPHA_TEST_THRESHOLD` doc comment."
    );

    // The same model with every cutout draw's reference below any alpha a
    // texel can carry, so `fs_main_alpha_test` keeps every fragment: what the
    // pad looks like when the alpha test does nothing. 425 lit pixels here
    // against the 310 above, the difference being the glow texture's
    // transparent background painted as a solid plate.
    let mut untested = model.clone();
    for draw in &mut untested.alpha_tested_draws {
        draw.alpha_test_ref = Some(-1.0);
    }
    let unfiltered = lit_pixel_count(&capture(&untested));
    println!("{name}: {unfiltered} lit pixel(s) with the alpha test forced off");
    assert!(
        (lit as f32) < 0.9 * unfiltered as f32,
        "{name}: the cutout pipeline kept {lit} of the {unfiltered} lit pixel(s) it draws \
         with the alpha test forced off, so the test is discarding (almost) nothing. At \
         the reference this pad's own file asks for - `0` - that is what a discard \
         written `shaded.a < alpha_test_ref` does instead of the GE's `GU_GREATER`, and \
         the pad renders as a square plate. See mesh.wgsl's `alpha_test_ref`."
    );
}

/// The framing the Pure counts above are measured at.
fn capture(model: &mesh::Model) -> Vec<u8> {
    mesh_render::capture_pixels_from(model, 1024, 1024, 0.9, 1.4, Anisotropy::default(), 0.0)
        .expect("capturing the pad model")
}

/// **The same guard for Wipeout HD, and for a second reason.**
///
/// A pad's `Model` says nothing about whether it draws. Pure's proved that in
/// 2026-08-17 with a decode that reported 952 correct triangles and rendered
/// zero pixels, and HD's pads reached the same cliff from the other side on
/// 2026-09-03: `mesh::rcs::pads` had been forcing both pad models off the
/// authored lighting path (`vertex_colour_is_light = false` and every vertex's
/// `lit` swept to `0.0`) so an invented flat tint would be visible, and
/// putting them back means a pad's brightness is now *entirely* albedo times
/// lightmap with no ambient floor - vertex light is a flat `0,0,0` on all
/// eighteen chunks and `NO_AMBIENT` is set. If the lightmap did not reach
/// them, or their `lightmap_texcoord` fell back to `[0.0, 0.0]` (which
/// `mesh::rcs::emit` does silently), HD's pads would have gone from flat blue
/// to near-black - and every attribute assertion in
/// `crates/game/tests/hd_pad_illumination_ground_truth.rs` would still pass.
///
/// Measured at this framing when the guard was written: **559 lit pixels**
/// for the whole `12_sol_2` `Speedup Pad` model - ten plates seen from far
/// enough away to fit all ten, which is why the count is the same order as
/// Pure's 321 rather than larger. Framed close, what is on screen is a grey
/// plate with a blue chevron outline, painted by `ds_speedup_cs.gtf`; see
/// `docs/rendering/pads.md`.
#[test]
#[ignore = "needs a GPU adapter and a disc image in data/images/"]
fn an_hd_speedup_pad_draws_visible_pixels() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/hdfury-ps3-eu-dec.iso");
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        println!("skipping: {} not present", path.display());
        return;
    }
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", path.display());
    let name = "/data/environments/12_sol_2/track.vex";
    let data = mesh::read_blob(&spec, name).expect("reading 12_sol_2's track.vex");
    let geometry = mesh::rcs::sibling_geometry(&spec, name, &data)
        .expect("12_sol_2 ships a .rcsmodel beside its .vex");
    let (model, report) = mesh::rcs::build_pads(name, &data, &geometry, &mut |path| {
        mesh::read_blob(&spec, path).ok()
    })
    .expect("building 12_sol_2's Speedup Pad geometry");
    println!("{}", report.describe());
    assert!(
        !model.indices.is_empty(),
        "12_sol_2 authors no Speedup Pad geometry - see \
         talons_junction_speedup_pad_chunks_are_an_unbaked_content_donor for why this \
         circuit and not the default one"
    );

    let pixels =
        mesh_render::capture_pixels_from(&model, 1024, 1024, 0.9, 1.4, Anisotropy::default(), 0.0)
            .expect("capturing the pad model");
    let lit = lit_pixel_count(&pixels);
    println!("{name}: {lit} lit pixel(s) of {}", pixels.len() / 4);
    assert!(
        lit > 250,
        "12_sol_2's Speedup Pad model drew only {lit} lit pixel(s) of 1,048,576 (it drew \
         559 when this guard was written). The geometry decoded and the picture is \
         (almost) empty - check that the circuit's lightmap still reaches these chunks, \
         since their own vertex light is 0,0,0 and NO_AMBIENT is set. See \
         docs/rendering/pads.md."
    );
}
