//! `dn_diffuse_specular_emiss` and `dn_diffuse_specular_emiss_glow`: which of the two bound
//! samplers is the picture.
//!
//! Both bind `alpha_emissive` and `SpecularTexture`, and the instance lists `SpecularTexture`
//! **first**, so "the first sampler listed" ([`Material::diffuse_texture`]) draws the specular
//! map as the picture. The programs say otherwise. Omega's lit pixel programs (`gcn_dump` on
//! `02_metropia/Materials/dn_diffuse_specular_emiss`, `@0x3fc4` and its 8 siblings; the `_glow`
//! file's, `@0x4504`) declare `alpha_emissive` as texture slot 0 and `SpecularTexture` as slot 1,
//! and combine
//!
//! ```text
//! rgb = fog(light * T.rgb + T.a * glowtinrt + specular * liveLighting0specular * S.rgb)
//! ```
//!
//! with `T` the `alpha_emissive` sample and `S` the `SpecularTexture` sample: the second is only
//! the weight of the specular term, and no alpha of either gates it. 2048's GXP copies declare
//! the same pair at resources 0 and 1, and their lit permutations end
//! `mul r0.xyz, spec, S.rgb; mad pa2, r0, S, pa2` (`S` squared, 2048's texel domain)
//! (`scripts/vita-usse.py disasm` on `dlc1.psarc`'s Metropia and Sol 2 files). See
//! `docs/formats/rcsmaterial.md`, "The specular map of `dn_diffuse_specular_emiss`".
//!
//! Keyed by the file's stem and the instance's two samplers, not by sampler order: a family not
//! read here keeps drawing what it drew.

use super::lighting_only;
use super::material::Material;
use crate::rcsmaterial::name_hash;

/// The family files whose programs were read.
const FAMILIES: [&str; 2] = [
    "dn_diffuse_specular_emiss",
    "dn_diffuse_specular_emiss_glow",
];

/// What a family instance binds, by role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecularPicture<'m> {
    /// The `alpha_emissive` texture: the picture, whose alpha is the glow mask.
    pub picture: &'m str,
    /// The `SpecularTexture` texture: the specular term's weight.
    pub specular: &'m str,
}

/// The instance's picture and specular map, when it is a family member that binds both.
#[must_use]
pub fn of(material: &Material) -> Option<SpecularPicture<'_>> {
    let family = lighting_only::family(material);
    if !FAMILIES.contains(&family.as_str()) {
        return None;
    }
    let picture = material
        .sampler(name_hash("alpha_emissive"))
        .filter(|p| !p.is_empty())?;
    let specular = material
        .sampler(name_hash("SpecularTexture"))
        .filter(|p| !p.is_empty())?;
    Some(SpecularPicture { picture, specular })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(name: &str, samplers: &[(&str, &str)]) -> Material {
        Material {
            name: name.to_string(),
            technique: None,
            textures: samplers.iter().map(|(_, p)| (*p).to_string()).collect(),
            lightmap: None,
            params: Vec::new(),
            samplers: samplers
                .iter()
                .map(|(n, p)| (name_hash(n), (*p).to_string()))
                .collect(),
            state: None,
        }
    }

    #[test]
    fn the_picture_is_alpha_emissive_though_the_specular_map_is_listed_first() {
        let m = instance(
            "x/materials/dn_diffuse_specular_emiss.rcsmaterial",
            &[
                ("SpecularTexture", "a_s.gnf"),
                ("alpha_emissive", "a_d.gnf"),
            ],
        );
        assert_eq!(m.diffuse_texture(), Some("a_s.gnf"), "the order alone");
        assert_eq!(
            of(&m),
            Some(SpecularPicture {
                picture: "a_d.gnf",
                specular: "a_s.gnf"
            })
        );
    }

    #[test]
    fn a_family_not_read_keeps_the_order() {
        let m = instance(
            "x/materials/mr_diffuse_emissive_alpha_glow_specular.rcsmaterial",
            &[
                ("SpecularTexture", "a_s.gnf"),
                ("alpha_emissive", "a_d.gnf"),
            ],
        );
        assert_eq!(of(&m), None);
    }

    #[test]
    fn an_instance_without_the_pair_is_not_a_member() {
        let m = instance(
            "x/materials/dn_diffuse_specular_emiss.rcsmaterial",
            &[("alpha_emissive", "a_d.gnf")],
        );
        assert_eq!(of(&m), None);
    }
}
