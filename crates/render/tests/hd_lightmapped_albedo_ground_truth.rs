//! A lightmapped HD material binds a picture its variant declares, not the
//! engine-bound env slot sitting at entry 0.
//!
//! **`#[ignore]`d and never run in CI**; needs the decrypted PS3 image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'test(/^hd_lightmapped_albedo_ground_truth::/)'
//! ```
//!
//! Vineta K's `d_s_n_customr` wet floor names `ds_dualparaboloid_c` (512x256,
//! sampler `0x8365b1f3`, declared by no variant) at entry 0 and its real
//! picture `ds_floorplain_wet_cs` / `ds_floorchevron_wet_cs` (2048x512) at
//! entry 1. Binding entry 0 drew the floor black with the reflection map's
//! pattern. Dropping the declared-sampler filter in `skin::picks` fails this.

use crate::archive_cache;

use oag_mesh::mesh;

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn vinetas_wet_floor_binds_its_floor_texture_not_the_paraboloid() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let path = "/data/environments/01_vineta_k/track.vex";
    let data = archive_cache::read(&spec, path).expect("the .vex reads");
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data).expect("a sibling .rcsmodel");
    let rcs = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");
    let (model, _) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        archive_cache::read(&spec, name)
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
            archive_cache::read(&spec, &path).map(|d| (spec, d))
        }) else {
            continue;
        };
        let geometry =
            mesh::rcs::sibling_geometry(&spec, &path, &data).expect("a sibling .rcsmodel");
        let rcs = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");
        let (model, _) = mesh::rcs::build_scene(&path, &data, &geometry, &mut |name| {
            archive_cache::read(&spec, name)
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
            archive_cache::read(&spec, &path).map(|d| (spec, d))
        }) else {
            continue;
        };
        let geometry =
            mesh::rcs::sibling_geometry(&spec, &path, &data).expect("a sibling .rcsmodel");
        let rcs = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");
        let (model, _) = mesh::rcs::build_scene(&path, &data, &geometry, &mut |name| {
            archive_cache::read(&spec, name)
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
    let data = archive_cache::read(&spec, path).expect("the .vex reads");
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data).expect("a sibling .rcsmodel");
    let rcs = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");
    let (model, _) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        archive_cache::read(&spec, name)
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

/// The start line: Talon's Junction's `constantdiffuse_specular_normal` names
/// one texture, `ds_sfblend_ns`, a tangent-space normal and specular map its
/// program unpacks with `MAD 2, -1` and never samples as colour, so the
/// surface is its authored constant diffuse (`0x7611a2d8`, a dark
/// `0.037, 0.054, 0.077`) times the light. Binding the map as the picture drew
/// a purple ridged strip across the road on twelve circuits. Dropping
/// `Program::samples_as_normal` from `skin::lightmapped` fails this.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_start_line_is_its_constant_diffuse_not_its_normal_map() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA00.PSARC", image.display());
    let path = "/data/environments/talons_junction/track.vex";
    let data = archive_cache::read(&spec, path).expect("the .vex reads");
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data).expect("a sibling .rcsmodel");
    let rcs = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");
    let (model, report) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        archive_cache::read(&spec, name)
    })
    .expect("Talon's Junction builds");
    let mut checked = 0;
    for (slot, material) in rcs.materials.iter().enumerate() {
        if !material
            .name
            .ends_with("/constantdiffuse_specular_normal.rcsmaterial")
        {
            continue;
        }
        let texture = model.textures[slot]
            .as_ref()
            .expect("the authored constant diffuse is bound");
        assert_eq!(
            (texture.width, texture.height),
            (1, 1),
            "slot {slot} binds {} instead of its constant diffuse",
            texture.label
        );
        let rgba = texture.to_rgba().expect("a one-texel picture");
        assert_eq!(
            [rgba[0], rgba[1], rgba[2]],
            [57, 68, 80],
            "slot {slot}: 0.037, 0.054, 0.077 as the texel bytes the sRGB decode turns back into them"
        );
        checked += 1;
    }
    assert!(checked > 0, "Talon's Junction carries the start line slots");
    assert!(report.constant_diffuse > 0, "{}", report.describe());
}

/// Sebenco Climb's booth glass: `ds_booth_glass` samples its `Texture1` as a
/// tangent-space normal map (`MAD 2, -1` on all three channels, block #7 of
/// its `.rcsmaterial`) and never as colour, so the surface is its authored
/// constant (`0x7611a2d8`) times the light. Binding the map as the picture
/// drew the arch's glass band pure white, where the original's is dark.
/// Dropping the family from `skin::DIFFUSE_PARAMETER` fails this.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_booth_glass_is_its_constant_diffuse_not_its_normal_map() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let path = "/data/environments/10_sebenco_climb/track.vex";
    let data = archive_cache::read(&spec, path).expect("the .vex reads");
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data).expect("a sibling .rcsmodel");
    let rcs = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");
    let (model, _) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        archive_cache::read(&spec, name)
    })
    .expect("Sebenco Climb builds");
    let mut checked = 0;
    for (slot, material) in rcs.materials.iter().enumerate() {
        if !material.name.ends_with("/ds_booth_glass.rcsmaterial") {
            continue;
        }
        let texture = model.textures[slot]
            .as_ref()
            .expect("the authored constant is bound");
        assert_eq!(
            (texture.width, texture.height),
            (1, 1),
            "slot {slot} binds {} instead of its constant",
            texture.label
        );
        let rgba = texture.to_rgba().expect("a one-texel picture");
        let want = material
            .parameters
            .iter()
            .find(|p| p.hash == 0x7611_a2d8)
            .expect("the glass authors 0x7611a2d8")
            .value;
        let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        assert_eq!(
            [rgba[0], rgba[1], rgba[2]],
            [byte(want[0]), byte(want[1]), byte(want[2])],
            "slot {slot}"
        );
        checked += 1;
    }
    assert!(checked > 0, "Sebenco Climb carries booth glass slots");
}

/// Sebenco Climb's pipes: `m_diffuse_normal_constantspecular` unpacks
/// `ds_pipe_nc.gtf`'s RGB as a normal and multiplies its **alpha** by the
/// authored `0x7611a2d8` (`MUL H2.xyz, R1.w, C`) to get the diffuse. Unsampled
/// as a colour, the slot drew pure white (a tunnel's pipe and stairs, 211 where
/// the original reads 50; 24 before the product was sRGB-encoded, 46 after).
/// Every sampled texel is checked against that product.
/// Dropping `skin::ALPHA_DIFFUSE` fails this.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_pipe_diffuse_is_the_maps_alpha_times_its_constant() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let path = "/data/environments/10_sebenco_climb/track.vex";
    let data = archive_cache::read(&spec, path).expect("the .vex reads");
    let geometry = mesh::rcs::sibling_geometry(&spec, path, &data).expect("a sibling .rcsmodel");
    let rcs = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");
    let (model, _) = mesh::rcs::build_scene(path, &data, &geometry, &mut |name| {
        archive_cache::read(&spec, name)
    })
    .expect("Sebenco Climb builds");
    let mut checked = 0;
    for (slot, material) in rcs.materials.iter().enumerate() {
        if !material
            .name
            .ends_with("/m_diffuse_normal_constantspecular.rcsmaterial")
        {
            continue;
        }
        let Some(map_path) = material
            .samplers
            .iter()
            .find(|(hash, _)| *hash == 0x3bdc_0403)
            .and_then(|(_, path)| path.as_deref())
        else {
            continue;
        };
        let constant = material
            .parameters
            .iter()
            .find(|p| p.hash == 0x7611_a2d8)
            .expect("the material authors 0x7611a2d8")
            .value;
        let texture = model.textures[slot].as_ref().expect("the diffuse is bound");
        assert!(
            texture.label.starts_with("alpha diffuse"),
            "slot {slot} binds {}",
            texture.label
        );
        let blob = archive_cache::read(&spec, &format!("/{map_path}")).expect("the map reads");
        let map = mesh::ModelTexture::from_gtf(map_path, &blob).expect("the map decodes");
        let (map, got) = (
            map.to_rgba().expect("decoded texels"),
            texture.to_rgba().expect("one-texel-per-texel picture"),
        );
        assert_eq!(map.len(), got.len(), "slot {slot}: same size as its map");
        for (m, g) in map.chunks_exact(4).zip(got.chunks_exact(4)).step_by(97) {
            let alpha = f32::from(m[3]) / 255.0;
            for lane in 0..3 {
                // The raw product, bound as the byte its sRGB decode returns.
                let raw = (alpha * constant[lane]).clamp(0.0, 1.0);
                let want = (raw.powf(1.0 / 2.2) * 255.0).round() as u8;
                assert_eq!(g[lane], want, "slot {slot} lane {lane}");
            }
        }
        checked += 1;
    }
    assert!(checked > 0, "Sebenco Climb carries pipe slots");
}

/// Families whose program is a constant and nothing else sampled: `metallic`
/// (Amphiseum) is `light * 0x512f8e65 + specular`, so the slot binds that
/// constant as the texel its sRGB decode returns; `dc_constantglow` (Moa
/// Therma) is fog over `0x4d60d566` and no light, bound the same way.
/// Dropping either family from `skin::DIFFUSE_PARAMETER` fails this.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn constant_colour_families_bind_their_authored_constant() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut checked = 0;
    for (circuit, family, parameter) in [
        ("amphiseum", "metallic", 0x512f_8e65_u32),
        ("03_track", "dc_constantglow", 0x4d60_d566),
    ] {
        let path = format!("/data/environments/{circuit}/track.vex");
        let Some((spec, data)) = (0..4).find_map(|n| {
            let spec = format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", image.display());
            archive_cache::read(&spec, &path).map(|data| (spec, data))
        }) else {
            panic!("{path} is on no archive");
        };
        let geometry =
            mesh::rcs::sibling_geometry(&spec, &path, &data).expect("a sibling .rcsmodel");
        let rcs = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");
        let (model, _) = mesh::rcs::build_scene(&path, &data, &geometry, &mut |name| {
            archive_cache::read(&spec, name)
        })
        .expect("the circuit builds");
        for (slot, material) in rcs.materials.iter().enumerate() {
            if !material.name.ends_with(&format!("/{family}.rcsmaterial")) {
                continue;
            }
            let Some(constant) = material.parameters.iter().find(|p| p.hash == parameter) else {
                continue;
            };
            let texture = model.textures[slot]
                .as_ref()
                .unwrap_or_else(|| panic!("{family} slot {slot} binds its constant"));
            assert_eq!((texture.width, texture.height), (1, 1), "{family} {slot}");
            let rgba = texture.to_rgba().expect("a one-texel picture");
            for lane in 0..3 {
                let c = constant.value[lane].clamp(0.0, 1.0);
                assert_eq!(
                    rgba[lane],
                    (c.powf(1.0 / 2.2) * 255.0).round() as u8,
                    "{family} slot {slot} lane {lane}"
                );
            }
            checked += 1;
        }
    }
    assert!(checked >= 2, "both families are present, saw {checked}");
}
