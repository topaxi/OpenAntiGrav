//! Wipeout 2048's per-material back-face culling (render-state bit 4, the word HD authors at
//! `+0x10` and a Vita header at `+0x12`), on Vineta K.
//!
//! **`#[ignore]`d and never run in CI**; needs `data/extracted/vita/PCSF00007`. Remove the
//! `material_state` fill in `psp2::bind_textures` and both assertions fail.
//!
//! Vineta K's chunk 47 is the counterpart of HD's chunk 1556 (the slab that has no front-facing
//! triangle at HD's start grid): the same x and z extent, 20 triangles. From HD's eye it faces
//! the eye in 2048's file, so the slab is not culled away on 2048 at that pose; the cull bit
//! is set on it all the same.

use std::path::Path;

use oag_mesh::mesh::rcs::psp2::{self, Animation};
use oag_mesh::mesh::render_state::STATE_CULL;

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn vineta_ks_materials_carry_the_cull_bit_and_honour_cull_culls_their_draws() {
    let package = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
    if !package.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            package.display()
        );
        return;
    }
    let mut archive =
        oag_assets::psarc::Archive::open(package.to_str().expect("utf-8")).expect("opens");
    let base = "data/art/published/DLC1/environments/Vineta_K/track";
    let blob = archive
        .read_path(&format!("{base}.rcsmodel"))
        .expect("model");
    let skeleton = archive.read_path(&format!("{base}.rcsskeleton")).ok();
    let clip = archive.read_path(&format!("{base}.rcsanimclip")).ok();
    let animation = skeleton
        .as_deref()
        .and_then(|s| Animation::parse(s, clip.as_deref()).ok());
    let (mut model, _) = psp2::build("vineta", &blob, animation.as_ref(), &mut |p| {
        archive.read_path(p).ok()
    })
    .expect("builds");
    assert_eq!(model.material_state.len(), model.textures.len());
    let with_bit = model
        .material_state
        .iter()
        .filter(|s| **s & STATE_CULL != 0)
        .count();
    assert!(
        with_bit * 100 >= model.material_state.len() * 90,
        "{with_bit} of {} materials carry bit 4",
        model.material_state.len()
    );
    model.honour_cull();
    let all = || {
        model
            .draws
            .iter()
            .chain(&model.alpha_tested_draws)
            .chain(&model.transparent_draws)
    };
    let culled = all().filter(|d| d.culled).count();
    assert!(culled * 100 >= all().count() * 90, "{culled} draws culled");
    assert!(
        all().filter(|d| d.chunk == Some(47)).all(|d| d.culled),
        "chunk 47 (HD's 1556 counterpart) culls"
    );
}
