//! `m_diffuse_normal_constantspecular` (and its `New`): the colour is
//! `Constant1` times the map's **alpha**; the map's rgb is a tangent normal.
//!
//! Read off both compiles of the shader (`docs/formats/gxp.md`,
//! "`m_diffuse_normal_constantspecular` colours by `Texture1`'s alpha"): 2048's
//! ambient-only USSE program is `mix(fog, Constant1 * ambient * T.a, v)` and its
//! lit one multiplies the light sum by `Constant1 * T.a`; Omega's GCN program
//! fetches the alpha channel alone (`dmask:0x8`). HD's RSX block reads the same
//! (`skin::ALPHA_DIFFUSE`, `AlphaUse::Product`). So the picture bound is the
//! product per texel, opaque, and the map's rgb is never drawn as colour.

use crate::mesh::ModelTexture;
use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmodel::psp2::material::Material;

/// The family's file stems, lower case, on 2048 and Omega: 2048's Chenghou
/// names the `New` file, Omega names the plain one and ships PS4 Pro compiles
/// as `_1`/`_2` (`docs/formats/gcn.md`).
fn is_family(material: &Material) -> bool {
    let stem = material
        .name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&material.name)
        .to_ascii_lowercase();
    let stem = stem.trim_suffix(".rcsmaterial");
    let stem = stem
        .strip_suffix("_1")
        .or_else(|| stem.strip_suffix("_2"))
        .unwrap_or(stem);
    let stem = stem.trim_suffix("new");
    stem == "m_diffuse_normal_constantspecular"
}

/// The map whose alpha carries the grey and the constant it scales, for a
/// material of the family that names both: `Texture1` (`Norm` on `New`) and
/// `Constant1`.
pub(super) fn source(material: &Material) -> Option<(&str, [f32; 3])> {
    if !is_family(material) {
        return None;
    }
    let path = ["Texture1", "Norm"]
        .iter()
        .find_map(|name| material.sampler(name_hash(name)))?;
    let value = material.param(name_hash("Constant1"))?;
    let channel = |i: usize| value.get(i).copied().unwrap_or(1.0);
    Some((path, [channel(0), channel(1), channel(2)]))
}

/// `map`'s alpha times `colour` per texel, opaque, the map's rgb dropped.
///
/// Unorm bytes with no sRGB step, the way this module's `constant_colour`
/// binds the other 2048 constants: **chosen, not measured** (HD encodes the
/// same product with `skin::raw_to_texel`; no 2048 frame decides it).
pub(super) fn picture(map: &ModelTexture, colour: [f32; 3]) -> Option<ModelTexture> {
    let texels = map.to_rgba()?;
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    let mut out = Vec::with_capacity(texels.len());
    for texel in texels.as_chunks::<4>().0 {
        let alpha = f32::from(texel[3]) / 255.0;
        out.extend(colour.map(|c| byte(alpha * c)));
        out.push(255);
    }
    Some(ModelTexture::rgba8(
        format!("{} alpha x Constant1", map.label),
        map.width,
        map.height,
        out,
        None,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material(name: &str) -> Material {
        Material {
            name: name.to_string(),
            technique: None,
            textures: vec!["a/ds_pipe_nc.gxt".to_string()],
            lightmap: None,
            params: Vec::new(),
            samplers: vec![(name_hash("Texture1"), "a/ds_pipe_nc.gxt".to_string())],
            state: Some(124),
        }
    }

    #[test]
    fn the_family_is_named_on_both_titles_and_nothing_else() {
        for name in [
            "data/art/published/DLC1/environments/Vineta_K/materials/m_diffuse_normal_constantspecular.rcsmaterial",
            "data/art/published/DLC1/environments/Chenghou_Project/materials/m_diffuse_normal_constantspecularNEW.rcsmaterial",
            "Data/environments/01_vineta_k/Materials/M_Diffuse_Normal_ConstantSpecular.rcsmaterial",
            "Data/environments/01_vineta_k/Materials/M_Diffuse_Normal_ConstantSpecular_2.rcsmaterial",
        ] {
            assert!(is_family(&material(name)), "{name}");
        }
        for name in [
            "x/materials/diffuse_normal_specular_emmissivenew.rcsmaterial",
            "x/materials/constantdiffuse_specular_normal.rcsmaterial",
            "x/materials/m_diffuse_normal_constantspecular_glow.rcsmaterial",
        ] {
            assert!(!is_family(&material(name)), "{name}");
        }
    }

    #[test]
    fn the_picture_is_alpha_times_the_constant_and_never_the_normal() {
        let map = ModelTexture::rgba8(
            "ds_pipe_nc".to_string(),
            2,
            1,
            vec![128, 128, 255, 255, 128, 128, 255, 51],
            None,
        );
        let out = picture(&map, [1.0, 0.5, 0.0]).expect("decodes");
        assert_eq!(
            &*out.to_rgba().expect("rgba"),
            &[255u8, 128, 0, 255, 51, 26, 0, 255][..]
        );
    }
}
