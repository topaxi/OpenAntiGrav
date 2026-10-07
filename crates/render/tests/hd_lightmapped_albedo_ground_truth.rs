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

/// Names the picture each lightmapped slot of `material` binds on Sebenco Climb,
/// forward and reversed, and asserts `ok` of its label.
fn sebenco_slots(material: &str, ok: impl Fn(&str) -> bool, why: &str) {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut checked = 0;
    for track in ["track.vex", "track_reversed.vex"] {
        let path = format!("/data/environments/10_sebenco_climb/{track}");
        let Some((spec, data)) = (0..4).find_map(|n| {
            let spec = format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", image.display());
            mesh::read_blob(&spec, &path).ok().map(|d| (spec, d))
        }) else {
            continue;
        };
        let geometry =
            mesh::rcs::sibling_geometry(&spec, &path, &data).expect("a sibling .rcsmodel");
        let rcs = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");
        let (model, _) = mesh::rcs::build_scene(&path, &data, &geometry, &mut |name| {
            mesh::read_blob(&spec, name).ok()
        })
        .expect("Sebenco Climb builds");
        for (slot, m) in rcs.materials.iter().enumerate() {
            if !m.name.ends_with(material) || m.lightmap_entry().is_none() {
                continue;
            }
            let Some(texture) = model.textures[slot].as_ref() else {
                continue;
            };
            assert!(
                ok(&texture.label.to_ascii_lowercase()),
                "{track} slot {slot} binds {}: {why}",
                texture.label
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "no {material} slot drew");
}

/// `sebenco_ice` reads `and_ice1` (unit 0) as `TEX H1.x`, a specular power,
/// and `and_snow3alpha` (unit 2) as `TEX H3.xyz`. The pool drew purple with
/// the scalar map as its colour. Dropping `Program::samples_colour` from
/// `skin::lightmapped::albedo` fails this.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn sebencos_ice_pool_binds_the_map_its_program_reads_as_colour() {
    sebenco_slots(
        "/sebenco_ice.rcsmaterial",
        |label| label.contains("and_snow3alpha"),
        "a scalar map, not the picture",
    );
}

/// Sebenco Climb's `track_coloured_specular_alpha4glow` solar wall names its
/// normal map (`ds_solarwall_n`, sampler `0x48f37f5a`, declared at unit 1) at
/// entry 0 and its picture (`ds_solarwall_c_withglow`, `0x3bdc0403`, unit 0)
/// at entry 2. Binding entry 0 drew the wall blue-violet with the normal map's
/// stripes. Dropping `0x48f37f5a` from `skin::NOT_A_PICTURE` fails this.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn sebencos_solar_wall_binds_its_picture_not_its_normal_map() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut checked = 0;
    for track in ["track.vex", "track_reversed.vex"] {
        let path = format!("/data/environments/10_sebenco_climb/{track}");
        let Some((spec, data)) = (0..4).find_map(|n| {
            let spec = format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", image.display());
            mesh::read_blob(&spec, &path).ok().map(|d| (spec, d))
        }) else {
            continue;
        };
        let geometry =
            mesh::rcs::sibling_geometry(&spec, &path, &data).expect("a sibling .rcsmodel");
        let rcs = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");
        let (model, _) = mesh::rcs::build_scene(&path, &data, &geometry, &mut |name| {
            mesh::read_blob(&spec, name).ok()
        })
        .expect("Sebenco Climb builds");
        for (slot, material) in rcs.materials.iter().enumerate() {
            if !material
                .name
                .ends_with("/track_coloured_specular_alpha4glow.rcsmaterial")
            {
                continue;
            }
            let Some(texture) = model.textures[slot].as_ref() else {
                continue;
            };
            assert!(
                !texture.label.to_ascii_lowercase().contains("_n."),
                "{track} slot {slot} binds the normal map {}",
                texture.label
            );
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "Sebenco Climb should carry the solar wall slots"
    );
}

/// Vineta K's backdrop terrain `and_rocktosand` lists `j_rockblend5` first, a
/// red-channel blend mask read one lane wide at unit 2, ahead of the sand
/// (unit 0) and rock (unit 1) it blends. Its colour lane is `Mixed`, so the
/// "first non-lookup entry" fallback painted the mask: the red shape behind the
/// tunnel glass. Dropping `declared_picture` from `skin::picks` fails this.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn vinetas_backdrop_terrain_binds_sand_and_rock_not_its_blend_mask() {
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
        if !material.name.ends_with("/and_rocktosand.rcsmaterial") {
            continue;
        }
        let Some(texture) = model.textures[slot].as_ref() else {
            continue;
        };
        assert!(
            !texture.label.contains("rockblend"),
            "slot {slot} binds the blend mask {}",
            texture.label
        );
        checked += 1;
    }
    assert!(checked > 0, "Vineta K should draw an and_rocktosand slot");
}
