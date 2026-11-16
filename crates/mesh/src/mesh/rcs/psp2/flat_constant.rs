//! Omega materials whose GCN pixel programs sample no colour texture and are a constant:
//! `metallic`, `dc_metallic` and `diffuse_spec_constant` end `light * diffuse + specular`
//! with the material's own constant, `emissive` is its `EmissiveColour` mixed with the fog
//! and nothing else (`gcn_dump` on Amphiseum's and Moa Therma's material files: no sampler,
//! one constant, the 45- and 43-instruction ambient programs; `docs/formats/rcsmaterial.md`,
//! "Omega's constant diffuse families").
//!
//! The diffuse parameters are [`crate::mesh::rcs::skin`]'s table, the ones HD's microcode
//! reads. Its other rows (`dc_constantglow`, `cf_constantcolourglow`, `dc_glowlightband`,
//! `mesh_colour_constant`) stay unwired here until their GCN programs are read.

use crate::mesh::ModelTexture;
use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmodel::psp2::{lighting_only, material::Material};

const DIFFUSE_FAMILIES: [&str; 3] = ["metallic", "dc_metallic", "diffuse_spec_constant"];

/// The one-texel picture of `material`'s constant diffuse, encoded for Omega's scene: its
/// `.gnf` texels decode with the sRGB curve, so a program-computed colour is bound as the
/// curve's inverse, as `skin::constant_picture` does for HD.
pub(super) fn diffuse(material: &Material) -> Option<ModelTexture> {
    let family = lighting_only::family(material);
    if !DIFFUSE_FAMILIES.contains(&family.as_str()) {
        return None;
    }
    let hash = crate::mesh::rcs::skin::diffuse_hash(&family, false)?;
    let value = material.param(hash)?;
    let channel = |i: usize| value.get(i).copied();
    Some(crate::mesh::rcs::skin::constant_picture(
        hash,
        [channel(0)?, channel(1)?, channel(2)?],
        oag_gpu::formats::SceneStore::SrgbTexels,
    ))
}

/// `EmissiveColour` of an `emissive` material: drawn unlit, as the program takes no light.
pub(super) fn emissive_colour(material: &Material) -> Option<[f32; 3]> {
    if lighting_only::family(material) != "emissive" {
        return None;
    }
    let v = material.param(name_hash("EmissiveColour"))?;
    Some([*v.first()?, *v.get(1)?, *v.get(2)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material(name: &str, params: &[(&str, [f32; 3])]) -> Material {
        Material {
            name: format!("data/environments/amphiseum/materials/{name}.rcsmaterial"),
            technique: None,
            textures: Vec::new(),
            lightmap: None,
            params: params
                .iter()
                .map(|(n, v)| oag_rcs::rcsmodel::psp2::material::Param {
                    hash: name_hash(n),
                    bits: v.map(f32::to_bits).to_vec(),
                })
                .collect(),
            samplers: Vec::new(),
            state: Some(124),
        }
    }

    #[test]
    fn the_hashes_are_the_ones_the_material_files_carry() {
        assert_eq!(name_hash("EmissiveColour"), 0x6223_f485);
        assert_eq!(name_hash("DiffuseColour"), 0x512f_8e65);
        assert_eq!(name_hash("diffuse"), 0x515e_298e);
    }

    #[test]
    fn a_constant_diffuse_is_bound_encoded_and_only_for_the_read_families() {
        let m = material(
            "diffuse_spec_constant",
            &[("diffuse", [0.079_665, 0.121, 0.141])],
        );
        let picture = diffuse(&m).expect("bound");
        let texels = picture.to_rgba().expect("rgba");
        let encoded = |c: f32| {
            (oag_gpu::formats::SceneStore::SrgbTexels.texel_encode(c) * 255.0).round() as u8
        };
        assert_eq!(
            &texels[..3],
            &[encoded(0.079_665), encoded(0.121), encoded(0.141)]
        );
        assert!(
            texels[0] > 40,
            "encoded, not the raw byte 20: {}",
            texels[0]
        );
        let other = material("dc_constantglow", &[("diffuse", [0.5, 0.5, 0.5])]);
        assert!(diffuse(&other).is_none());
    }

    #[test]
    fn emissive_is_its_colour_and_nothing_else_is() {
        let m = material("emissive", &[("EmissiveColour", [0.5566, 0.5566, 0.5566])]);
        assert_eq!(emissive_colour(&m), Some([0.5566; 3]));
        assert_eq!(emissive_colour(&material("metallic", &[])), None);
    }
}
