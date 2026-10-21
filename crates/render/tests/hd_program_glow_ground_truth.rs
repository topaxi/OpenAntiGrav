//! Wipeout HD's glow mask is what each material's fragment program writes, read
//! off the disc's own microcode.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'test(/^hd_program_glow_ground_truth::/)'
//! ```
//!
//! The reference is the original running: a live RPCS3 frame of Metropia's
//! start (2026-10-21) has its scene target's alpha at **255 on the neon sign's
//! pixels, 122 on the sky and 0 on the lit road and walls**, and the programs
//! behind those pixels are `cf_constantcolourglow` (`MOV H0.w, 1`),
//! the sky draw (`COL0.w`) and `lambertzeroalpha`/`diffuse_specular`
//! (`MOV H0.w, 0`). The loader carries each material's value per slot in
//! `Model::material_glow`; dropping the reading leaves every slot at 0.

use std::path::{Path, PathBuf};

use oag_mesh::mesh;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

fn build(image: &Path) -> Option<mesh::Model> {
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let path = "/data/environments/02_track/track.vex";
    let data = mesh::read_blob(&spec, path).ok()?;
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data)?;
    let (model, _) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        mesh::read_blob(&spec, name).ok()
    })
    .expect("the scene builds");
    Some(model)
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn metropia_carries_its_programs_glow_masks() {
    let Some(image) = image() else {
        return;
    };
    let Some(model) = build(&image) else {
        return;
    };
    assert!(
        model.glow_from_program,
        "HD's scene writes its mask by program"
    );
    let glow = &model.material_glow;
    assert_eq!(
        glow.len(),
        model.textures.len(),
        "one value per material slot"
    );
    let constants = glow.iter().filter(|&&g| g == 1.0).count();
    let texels = glow.iter().filter(|&&g| g < 0.0).count();
    let dark = glow.iter().filter(|&&g| g == 0.0).count();
    assert!(
        constants >= 1,
        "`cf_constantcolourglow` (the neon sign) writes alpha 1"
    );
    assert!(
        texels >= 1,
        "the emissive pads and lights write a texture's alpha"
    );
    assert!(
        dark * 2 > glow.len(),
        "most of a circuit is lit surface, which writes no mask ({dark} of {})",
        glow.len()
    );
    // A constant is a value in [0, 1] and a texel code is below -1.
    assert!(glow.iter().all(|&g| g <= 1.0), "no constant above 1");
}
