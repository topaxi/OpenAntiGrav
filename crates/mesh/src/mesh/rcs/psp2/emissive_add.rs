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
//! **`fc10_Lambert_2UV_Emissive` is a window with a reflection map** (`docs/formats/2048-material-params.md`,
//! "The reflective window and the awning"): every lit and ambient-only program of the file ends
//! `fog(light * D^2 + D.a * R^2)`, `R` the `ReflectMap` read at `0.5 + 0.5 * reflect(-V, N).xy`
//! with the view vector and the normal in world space ([`PROGRAM_REFLECT_MAP`]).
//!
//! **`dc_diffusetexturealphaemissive` adds its own picture's square** (Moa Therma's Feisar
//! awnings, `docs/formats/2048-material-params.md`, "The reflective window and the awning"): its lit programs end
//! `mad light, D^2, D^2` before the fog and write the texel's alpha out, so `rgb = D^2 * (light + 1)`
//! and the awning's alpha is the picture's ([`PROGRAM_PICTURE_ADD`]).
//!
//! The picture rides the third binding ([`crate::mesh::Model::pad_masks`]), the tint is one
//! glow-table entry of [`PROGRAM_EMISSIVE_ADD`], and `shade.wesl` joins the term to the pad glow
//! it already adds after the light.
//!
//! **The track's `fc16_phong_alphaspec_normal_emissive` adds its `EmissiveMap` squared, untinted**
//! (`docs/formats/gxp.md`, "`fc16_phong_alphaspec_normal_emissive` adds its emissive map"): every
//! lit program of the file ends `mad out, E, E, light * D^2` before the specular and the fog, and
//! the material declares no tint uniform, so the entry's tint is one. It is the road and wall of
//! the 2048 circuits' track surface (Altima's teal panel outlines).
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

use super::Report;
use crate::mesh::rcs::Textures;
use crate::mesh::{
    Emissive, ModelTexture, PROGRAM_EMISSIVE_ADD, PROGRAM_EMISSIVE_ALPHA_MASK, PROGRAM_PICTURE_ADD,
    PROGRAM_REFLECT_MAP, slots,
};

/// One admitted material: its glow-table slot (entry index plus one) and its mask map, which a
/// term read off the picture's own alpha ([`super::alpha_emissive`]) does not have.
pub(super) struct Add {
    pub slot: u32,
    pub texture: Option<Arc<ModelTexture>>,
    /// The `.gxt` the colour is drawn from when the material's first listed texture is the mask
    /// itself ([`Kind::colour`]).
    pub colour: Option<String>,
    /// Whether the family's program reads the picture's alpha as the surface's own
    /// ([`PROGRAM_REFLECT_MAP`] gates by it, [`PROGRAM_PICTURE_ADD`] writes it out), which takes
    /// the alpha lane of a Vita slot word.
    pub alpha_gated: bool,
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

    /// Whether this material's term is gated by the diffuse alpha, so its slot word reads lane 3.
    pub fn alpha_gated(&self, material: Option<usize>) -> bool {
        self.of(material).is_some_and(|a| a.alpha_gated)
    }

    /// The mask map this material binds third, when it has one.
    pub fn mask(&self, material: Option<usize>) -> Option<&Arc<ModelTexture>> {
        self.of(material)?.texture.as_ref()
    }
}

/// One member of the family: the sampler whose alpha is the mask, the glow-table mode that says
/// how, and the sampler that carries the colour when it is not the first texture listed.
struct Kind {
    /// The sampler whose picture rides the third binding; `None` for a term of the first
    /// binding's own.
    mask: Option<&'static str>,
    program: f32,
    colour: Option<&'static str>,
    /// The uniform the tint is read from; `None` where the program multiplies by nothing (one).
    tint: Option<&'static str>,
    /// Whether Omega's programs of the same file were read and agree.
    ps4: bool,
}

fn kind_of(material: &Material, ps4: bool) -> Option<Kind> {
    let stem = material
        .name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&material.name);
    let stem = stem.trim_suffix(".rcsmaterial");
    let names_alpha_sampler = material.sampler(name_hash("Texture2Alpha")).is_some();
    if (ps4 || !names_alpha_sampler)
        && stem.eq_ignore_ascii_case("diffuse_normal_specular_emmissive_alpha")
    {
        // Omega's Detonator pad (`gcn_dump` of the lit-by-ambient block, 60 instructions):
        // `Constant1 * Texture2.a + constantAmbientColour * Texture1.rgb`, then fog. The mask
        // is the `_ne` alpha **unsquared**, which is the plain family's reading, not the Vita
        // program's `Texture2Alpha.rgb^2`. Only the block that takes no sun was read. A Vita file
        // that names `Texture2` and no `Texture2Alpha` (2048's Detonator pad) takes the same
        // reading, **inherited from Omega**: its own program is unread.
        Some(Kind {
            mask: Some("Texture2"),
            program: PROGRAM_EMISSIVE_ALPHA_MASK,
            colour: None,
            tint: Some("Constant1"),
            ps4: true,
        })
    } else if stem.eq_ignore_ascii_case("diffuse_normal_specular_emmissive_alpha") {
        Some(Kind {
            mask: Some("Texture2Alpha"),
            program: PROGRAM_EMISSIVE_ADD,
            colour: None,
            tint: Some("Constant1"),
            ps4: false,
        })
    } else if stem.eq_ignore_ascii_case("diffuse_normal_specular_emmissive") {
        Some(Kind {
            mask: Some("Texture2"),
            program: PROGRAM_EMISSIVE_ALPHA_MASK,
            colour: None,
            tint: Some("Constant1"),
            ps4: true,
        })
    } else if stem.eq_ignore_ascii_case("diffuse_normal_specular_emmissivenew") {
        // Chenghou's: the mask is `Norm`'s alpha and the colour is `Texture1`, listed second.
        Some(Kind {
            mask: Some("Norm"),
            program: PROGRAM_EMISSIVE_ALPHA_MASK,
            colour: Some("Texture1"),
            tint: Some("Constant1"),
            ps4: false,
        })
    } else if super::circuit_pads::material_stem(&material.name) == "weapon_pads" {
        // The pads' light bars: `W_Cycle * Texture2.a + light`, read on Omega's own pixel
        // programs and on all 243 of 2048's that declare `W_Cycle`
        // (`docs/rendering/pads.md`, "2048 and Omega").
        Some(Kind {
            mask: Some("Texture2"),
            program: PROGRAM_EMISSIVE_ALPHA_MASK,
            colour: None,
            tint: Some("W_Cycle"),
            ps4: true,
        })
    } else if stem.eq_ignore_ascii_case("fc16_phong_alphaspec_normal_emissive") {
        Some(Kind {
            mask: Some("EmissiveMap"),
            program: PROGRAM_EMISSIVE_ADD,
            colour: None,
            tint: None,
            ps4: false,
        })
    } else if stem.eq_ignore_ascii_case("fc10_lambert_2uv_emissive") {
        // Moa Therma's glass facades; the file ships in the base circuits' packages too.
        Some(Kind {
            mask: Some("ReflectMap"),
            program: PROGRAM_REFLECT_MAP,
            colour: None,
            tint: None,
            ps4: false,
        })
    } else if stem.eq_ignore_ascii_case("dc_diffusetexturealphaemissive") {
        // Moa Therma's Feisar awnings.
        Some(Kind {
            mask: None,
            program: PROGRAM_PICTURE_ADD,
            colour: None,
            tint: None,
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
            tint,
            ..
        }) = kind_of(material, ps4).filter(|k| !ps4 || k.ps4)
        else {
            continue;
        };
        let index = out.adds.len() - 1;
        const UNIT: [f32; 3] = [1.0; 3];
        let path = match sampler {
            None => Some(None),
            Some(name) => material
                .sampler(name_hash(name))
                .filter(|p| !p.is_empty())
                .map(Some),
        };
        let (Some(path), Some(tint)) = (
            path,
            tint.map_or(Some(UNIT.to_vec()), |name| material.param(name_hash(name))),
        ) else {
            crate::mesh::rcs::census::claim(
                index,
                "emissive_add",
                crate::mesh::rcs::census::Outcome::NotDrawn,
            );
            report.emissive_add_unread += 1;
            continue;
        };
        let channel = |i: usize| tint.get(i).copied().unwrap_or(0.0);
        let texture = match path {
            None => None,
            Some(path) => {
                let loaded = cache
                    .entry(path.to_string())
                    .or_insert_with(|| super::picture_memo::picture(path, textures, report))
                    .clone();
                if loaded.is_none() {
                    crate::mesh::rcs::census::claim(
                        index,
                        "emissive_add",
                        crate::mesh::rcs::census::Outcome::NotDrawn,
                    );
                    report.emissive_add_unread += 1;
                    continue;
                }
                loaded
            }
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
            crate::mesh::rcs::census::claim(
                index,
                "emissive_add",
                crate::mesh::rcs::census::Outcome::NotDrawn,
            );
            report.emissive_add_unread += 1;
            continue;
        };
        out.adds[index] = Some(Add {
            slot,
            texture,
            colour: colour
                .and_then(|name| material.sampler(name_hash(name)))
                .filter(|p| !p.is_empty())
                .map(str::to_string),
            alpha_gated: program == PROGRAM_REFLECT_MAP || program == PROGRAM_PICTURE_ADD,
        });
        crate::mesh::rcs::census::claim(
            index,
            "emissive_add",
            crate::mesh::rcs::census::Outcome::Drawn,
        );
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
    fn omegas_second_sampler_of_the_window_file_is_the_reflect_map() {
        // The sampler hash Omega's `fc10_lambert_2uv_emissive` records beside its window atlas.
        assert_eq!(name_hash("ReflectMap"), 0xdba5_1b4c);
        let family = kind_of(&named("data/art/published/dlc1/environments/moa_therma/materials/fc10_lambert_2uv_emissive.rcsmaterial"), false)
        .expect("admitted");
        assert_eq!(family.mask, Some("ReflectMap"));
        assert!(!family.ps4, "Omega's programs are unread");
    }

    #[test]
    fn chenghous_new_family_masks_by_norm_and_colours_by_texture1() {
        let new = kind_of(&named("data/art/published/dlc1/environments/chenghou_project/materials/diffuse_normal_specular_emmissivenew.rcsmaterial"), false)
        .expect("admitted");
        assert_eq!(new.mask, Some("Norm"));
        assert_eq!(new.colour, Some("Texture1"));
        assert!(!new.ps4, "Omega ships no `new` file");
    }

    #[test]
    fn the_track_surface_adds_its_emissive_map_untinted_on_2048_only() {
        let road = kind_of(&named("data/art/published/environments/shared/objectlibrary/materials/fc16_phong_AlphaSpec_Normal_Emissive.rcsmaterial"), false)
        .expect("admitted");
        assert_eq!(road.mask, Some("EmissiveMap"));
        assert_eq!(road.program, PROGRAM_EMISSIVE_ADD);
        assert_eq!(
            road.tint, None,
            "the programs multiply the square by nothing"
        );
        assert!(!road.ps4);
        assert!(
            kind_of(
                &named("x/materials/fc16_phong_alpha_normal_specpow.rcsmaterial"),
                false
            )
            .is_none(),
            "the sibling without an emissive map adds nothing"
        );
    }

    #[test]
    fn omega_takes_the_plain_member_only() {
        let plain = |n: &str| kind_of(&named(n), false).map(|k| (k.mask, k.ps4));
        let omega = |n: &str| kind_of(&named(n), true).map(|k| (k.mask, k.program, k.ps4));
        assert_eq!(
            plain("x/materials/diffuse_normal_specular_emmissive.rcsmaterial"),
            Some((Some("Texture2"), true))
        );
        assert_eq!(
            plain("x/materials/diffuse_normal_specular_emmissive_alpha.rcsmaterial"),
            Some((Some("Texture2"), true)),
            "a Vita `_alpha` file that names no `Texture2Alpha` (the Detonator pad) is read like Omega's"
        );
        assert_eq!(
            omega("x/materials/diffuse_normal_specular_emmissive_alpha.rcsmaterial"),
            Some((Some("Texture2"), PROGRAM_EMISSIVE_ALPHA_MASK, true)),
            "Omega's pad program masks by the unsquared `_ne` alpha"
        );
        assert_eq!(
            plain("x/materials/diffuse_normal_specular.rcsmaterial"),
            None
        );
    }
}
