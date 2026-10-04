//! Wipeout 2048's scrolling surfaces, read off each material's own uniforms.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run it with `just test-data`.
//!
//! `arena` ships three materials with HD's glow layer
//! (`Emissive_UV_Offset`/`Scale`, `TimeScaler`, `GlowTint`: the `fc09`/`fc10`
//! `VScroll_LambertAlpha_Emissive` pair) and two with a plain
//! `speed_multipliaer` scroll. The test holds what the build does with them:
//! the layer draws carry `ADD_SECOND` with their emissive texture bound beside
//! their diffuse and a table entry whose rate is the authored `TimeScaler`; the
//! scroll draws carry a track that moves `v` by the authored speed per second.
//! Remove `glow::plan`'s call and every assertion here fails.

use std::path::{Path, PathBuf};

use oag_render::mesh::rcs::psp2::{self, Animation};
use oag_render::mesh::{Model, slots};
use oag_render::mesh_render::TexAnims;

fn package() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn build(circuit: &str) -> Option<(Model, psp2::Report)> {
    let package = package()?;
    let mut archive =
        oag_assets::psarc::Archive::open(package.to_str().expect("utf-8")).expect("opens");
    let base = format!("data/art/published/environments/{circuit}/track");
    let model = archive
        .read_path(&format!("{base}.rcsmodel"))
        .expect("model");
    let skeleton = archive.read_path(&format!("{base}.rcsskeleton")).ok();
    let clip = archive.read_path(&format!("{base}.rcsanimclip")).ok();
    let animation = skeleton
        .as_deref()
        .and_then(|s| Animation::parse(s, clip.as_deref()).ok());
    Some(
        psp2::build(circuit, &model, animation.as_ref(), &mut |p| {
            archive.read_path(p).ok()
        })
        .expect("builds"),
    )
}

/// The first vertex each non-empty draw's indices reach.
fn first_vertices(
    model: &Model,
) -> impl Iterator<Item = (&oag_render::mesh::DrawCall, oag_render::mesh::GpuVertex)> {
    model.draws.iter().filter(|d| !d.range.is_empty()).map(|d| {
        (
            d,
            model.vertices[model.indices[d.range.start as usize] as usize],
        )
    })
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn arenas_glow_layers_carry_their_authored_rates_and_a_bound_emissive_texture() {
    let Some((model, report)) = build("arena") else {
        return;
    };
    assert_eq!(report.glow_layers, 3, "{}", report.describe());
    let layered: Vec<_> = first_vertices(&model)
        .filter(|(_, v)| v.slots & slots::ADD_SECOND != 0)
        .collect();
    assert!(layered.len() > 40, "only {} layered draws", layered.len());
    for (draw, vertex) in &layered {
        let texture = draw.texture.expect("a layered draw is textured");
        assert!(
            model.lightmaps.get(texture).is_some_and(Option::is_some),
            "layered draw has no emissive texture bound"
        );
        let slot = (vertex.slots >> slots::MATERIAL_SHIFT) as usize;
        assert!((1..=model.emissive.len()).contains(&slot));
    }
    // `TimeScaler` as authored on the three: 0.05, 1.0 and 3.0.
    let mut rates: Vec<f32> = model.emissive.iter().map(|e| e.rate).collect();
    rates.sort_by(f32::total_cmp);
    rates.dedup();
    assert_eq!(rates, vec![0.05, 1.0, 3.0], "{:?}", model.emissive);
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn arenas_plain_scrolls_move_v_by_the_authored_speed() {
    let Some((model, report)) = build("arena") else {
        return;
    };
    assert_eq!(report.scrolling_materials, 2, "{}", report.describe());
    let scrolling: Vec<_> = first_vertices(&model)
        .filter(|(_, v)| v.anim != 0)
        .collect();
    assert!(!scrolling.is_empty());
    let at_zero = TexAnims::sample(&model, 0.0);
    let at_one = TexAnims::sample(&model, 1.0);
    for (_, vertex) in scrolling {
        let slot = vertex.anim as usize;
        assert_eq!(at_zero.transform[slot], [1.0, 1.0, 0.0, 0.0]);
        let moved = at_one.transform[slot];
        assert_eq!(&moved[..3], &[1.0, 1.0, 0.0]);
        // `speed_multipliaer` is 0.3 on both of arena's scrolling materials.
        assert!((moved[3] - 0.3).abs() < 1e-5, "{moved:?}");
    }
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn altima_scrolls_its_plain_materials_and_draws_the_rest_still() {
    let Some((model, report)) = build("altima") else {
        return;
    };
    assert_eq!(report.glow_layers, 0, "{}", report.describe());
    assert_eq!(report.scrolling_materials, 12, "{}", report.describe());
    assert!(model.emissive.is_empty());
}
