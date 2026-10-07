//! A lightmapped HD material binds a picture its variant declares, not the
//! engine-bound env slot sitting at entry 0.
//!
//! **`#[ignore]`d and never run in CI**; needs the decrypted PS3 image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_lightmapped_albedo_ground_truth)'
//! ```
//!
//! Vineta K's `d_s_n_customr` wet floor names `ds_dualparaboloid_c` (512x256,
//! sampler `0x8365b1f3`, declared by no variant) at entry 0 and its real
//! picture `ds_floorplain_wet_cs` / `ds_floorchevron_wet_cs` (2048x512) at
//! entry 1. Binding entry 0 drew the floor black with the reflection map's
//! pattern. Dropping the declared-sampler filter in `skin::picks` fails this.

use oag_mesh::mesh;

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn vinetas_wet_floor_binds_its_floor_texture_not_the_paraboloid() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let path = "/data/environments/01_vineta_k/track.vex";
    let data = mesh::read_blob(&spec, path).expect("the .vex reads");
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data).expect("a sibling .rcsmodel");
    let rcs = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");
    let (model, _) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        mesh::read_blob(&spec, name).ok()
    })
    .expect("Vineta K builds");

    let mut checked = 0;
    for (slot, material) in rcs.materials.iter().enumerate() {
        if !material.name.ends_with("/d_s_n_customr.rcsmaterial") {
            continue;
        }
        let texture = model.textures[slot].as_ref().expect("an albedo is bound");
        println!(
            "slot {slot}: {} {}x{}",
            texture.label, texture.width, texture.height
        );
        assert_eq!(
            (texture.width, texture.height),
            (2048, 512),
            "slot {slot} binds {} instead of its wet floor",
            texture.label
        );
        checked += 1;
    }
    assert!(checked > 0, "Vineta K should carry d_s_n_customr slots");
}
