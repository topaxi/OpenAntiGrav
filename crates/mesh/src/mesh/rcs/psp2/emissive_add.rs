//! 2048's `diffuse_normal_specular_emmissive_alpha`: an emissive add **outside the light**.
//!
//! The family's fragment program (`f7b139cf...`, read from Vita3K's cached SPIR-V,
//! `docs/formats/2048-material-params.md`, "The emissive add of the `*_emmissive_alpha`
//! family") ends on `mix(fog, spec + Texture2Alpha.rgb^2 * Constant1 + lit * D^2, vis)`. The
//! emissive term is neither lit nor shadowed, so a dark baked panel still glows: Vineta K's
//! blue floor panels (`Constant1` = `(0, 0.769, 0.992)`).
//!
//! **The plain `diffuse_normal_specular_emmissive` is the same add with a different mask**
//! (programs `a17f5c52...` vertex-lit and `419db654...` lightmapped, same walk, conf 70):
//! `Texture2.a * Constant1`, `Texture2` being the normal map, whose alpha is the glow mask and is
//! not squared. It is the family of Vineta K's floor panels. [`PROGRAM_EMISSIVE_ALPHA_MASK`]
//! selects it.
//!
//! The picture rides the third binding ([`crate::mesh::Model::pad_masks`]), the tint is one
//! glow-table entry of [`PROGRAM_EMISSIVE_ADD`], and `shade.wesl` joins the term to the pad glow
//! it already adds after the light.
//!
//! **Chenghou's `diffuse_normal_specular_emmissivenew` is the plain family with the samplers
//! renamed**: `Norm` is the `_ne` map whose alpha is the mask, `Texture1` the `_cs` colour that is
//! listed second, so the colour is named rather than the first texture listed (the first is the
//! normal map; `docs/formats/gxp.md`, "`diffuse_normal_specular_emmissivenew`"). Omega's plain
//! family has the same shape on its own programs and takes the plain member; its `_emmissive_alpha`
//! programs are unread, so that member stays Vita only.

use std::sync::Arc;

use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmodel::psp2::{self, material::Material};

use super::{Report, decode_material_texture};
use crate::mesh::rcs::Textures;
use crate::mesh::{
    Emissive, ModelTexture, PROGRAM_EMISSIVE_ADD, PROGRAM_EMISSIVE_ALPHA_MASK, slots,
};

/// One admitted material: its glow-table slot (entry index plus one) and its mask map, which a
/// term read off the picture's own alpha ([`super::alpha_emissive`]) does not have.
pub(super) struct Add {
    pub slot: u32,
    pub texture: Option<Arc<ModelTexture>>,
    /// The `.gxt` the colour is drawn from when the material's first listed texture is the mask
    /// itself ([`Kind::colour`]).
    pub colour: Option<String>,
}

/// Per material, the add it draws. [`super::shield_lightning`] fills in the same table: both ride
/// the third binding and one glow-table entry.
#[derive(Default)]
pub(super) struct Plan {
    pub adds: Vec<Option<Add>>,
}

impl Plan {
    pub fn of(&self, material: Option<usize>) -> Option<&Add> {
        self.adds.get(material?)?.as_ref()
    }

    /// The vertex bits that select this material's glow-table entry; `0` for any other.
    pub fn role(&self, material: Option<usize>) -> u32 {
        self.of(material)
            .map_or(0, |a| a.slot << slots::MATERIAL_SHIFT)
    }

    /// Whether any material rides the third binding.
    pub fn any(&self) -> bool {
        self.adds.iter().flatten().any(|a| a.texture.is_some())
    }

    /// The path this material's colour is drawn from, when its own program says it is not the
    /// first texture listed.
    pub fn colour(&self, material: Option<usize>) -> Option<&str> {
        self.of(material)?.colour.as_deref()
    }

    /// The mask map this material binds third, when it has one.
    pub fn mask(&self, material: Option<usize>) -> Option<&Arc<ModelTexture>> {
        self.of(material)?.texture.as_ref()
    }
}

/// One member of the family: the sampler whose alpha is the mask, the glow-table mode that says
/// how, and the sampler that carries the colour when it is not the first texture listed.
struct Kind {
    mask: &'static str,
    program: f32,
    colour: Option<&'static str>,
    /// Whether Omega's programs of the same file were read and agree.
    ps4: bool,
}

fn kind_of(material: &Material) -> Option<Kind> {
    let stem = material
        .name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&material.name);
    let stem = stem.trim_suffix(".rcsmaterial");
    if stem.eq_ignore_ascii_case("diffuse_normal_specular_emmissive_alpha") {
        Some(Kind {
            mask: "Texture2Alpha",
            program: PROGRAM_EMISSIVE_ADD,
            colour: None,
            ps4: false,
        })
    } else if stem.eq_ignore_ascii_case("diffuse_normal_specular_emmissive") {
        Some(Kind {
            mask: "Texture2",
            program: PROGRAM_EMISSIVE_ALPHA_MASK,
            colour: None,
            ps4: true,
        })
    } else if stem.eq_ignore_ascii_case("diffuse_normal_specular_emmissivenew") {
        // Chenghou's: the mask is `Norm`'s alpha and the colour is `Texture1`, listed second.
        Some(Kind {
            mask: "Norm",
            program: PROGRAM_EMISSIVE_ALPHA_MASK,
            colour: Some("Texture1"),
            ps4: false,
        })
    } else {
        None
    }
}

/// Admits each family material that names `Texture2Alpha` and carries `Constant1` and whose
/// picture decodes, writing its glow-table entry into `emissive`. Empty on a PS4 model.
pub(super) fn plan(
    decoded: &psp2::Model,
    ps4: bool,
    emissive: &mut Vec<Emissive>,
    textures: Textures<'_>,
    report: &mut Report,
) -> Plan {
    let mut out = Plan::default();
    let mut cache: std::collections::HashMap<String, Option<Arc<ModelTexture>>> =
        std::collections::HashMap::new();
    for material in &decoded.materials {
        out.adds.push(None);
        let Some(Kind {
            mask: sampler,
            program,
            colour,
            ..
        }) = kind_of(material).filter(|k| !ps4 || k.ps4)
        else {
            continue;
        };
        let index = out.adds.len() - 1;
        let (Some(path), Some(tint)) = (
            material
                .sampler(name_hash(sampler))
                .filter(|p| !p.is_empty()),
            material.param(name_hash("Constant1")),
        ) else {
            report.emissive_add_unread += 1;
            continue;
        };
        let channel = |i: usize| tint.get(i).copied().unwrap_or(0.0);
        let Some(texture) = cache
            .entry(path.to_string())
            .or_insert_with(|| {
                textures(path)
                    .and_then(|blob| decode_material_texture(path, &blob, report))
                    .map(Arc::new)
            })
            .clone()
        else {
            report.emissive_add_unread += 1;
            continue;
        };
        let entry = Emissive {
            tint: [channel(0), channel(1), channel(2)],
            offset: 0.0,
            scale: 0.0,
            rate: 0.0,
            program: [program, 0.0],
        };
        let at = emissive.iter().position(|e| *e == entry).or_else(|| {
            (emissive.len() + 1 < crate::mesh::rcs::EMISSIVE_LIMIT).then(|| {
                emissive.push(entry);
                emissive.len() - 1
            })
        });
        let Some(slot) = at.and_then(|at| u32::try_from(at + 1).ok()) else {
            report.emissive_add_unread += 1;
            continue;
        };
        out.adds[index] = Some(Add {
            slot,
            texture: Some(texture),
            colour: colour
                .and_then(|name| material.sampler(name_hash(name)))
                .filter(|p| !p.is_empty())
                .map(str::to_string),
        });
        report.emissive_add_materials += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str) -> Material {
        Material {
            name: name.to_string(),
            technique: None,
            textures: Vec::new(),
            lightmap: None,
            params: Vec::new(),
            samplers: Vec::new(),
            state: None,
        }
    }

    #[test]
    fn chenghous_new_family_masks_by_norm_and_colours_by_texture1() {
        let new = kind_of(&named(
            "data/art/published/dlc1/environments/chenghou_project/materials/diffuse_normal_specular_emmissivenew.rcsmaterial",
        ))
        .expect("admitted");
        assert_eq!(new.mask, "Norm");
        assert_eq!(new.colour, Some("Texture1"));
        assert!(!new.ps4, "Omega ships no `new` file");
    }

    #[test]
    fn omega_takes_the_plain_member_only() {
        let plain = |n: &str| kind_of(&named(n)).map(|k| (k.mask, k.ps4));
        assert_eq!(
            plain("x/materials/diffuse_normal_specular_emmissive.rcsmaterial"),
            Some(("Texture2", true))
        );
        assert_eq!(
            plain("x/materials/diffuse_normal_specular_emmissive_alpha.rcsmaterial"),
            Some(("Texture2Alpha", false))
        );
        assert_eq!(
            plain("x/materials/diffuse_normal_specular.rcsmaterial"),
            None
        );
    }
}
