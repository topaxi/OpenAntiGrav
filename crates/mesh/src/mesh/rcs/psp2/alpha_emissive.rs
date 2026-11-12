//! 2048's and Omega's emissive-in-alpha families: the picture's own alpha is a glow mask.
//!
//! `diffuse_emissive_alpha`, `diffuse_emissive_alpha_glow`, `mr_diffuse_emissive_alpha_glow_specular`
//! and `dn_diffuse_specular_emiss` bind their picture to `alpha_emissive` and author a colour
//! `glowtinrt`. Every lit fragment program that declares the colour ends
//!
//! ```text
//! mad.f16 dst, texel.wwww, glowtinrt.xyzw, light * texel * texel
//! ```
//!
//! before the fog mix: `rgb = fog(light * D^2 + D.a * glowtinrt)`, the colour neither lit nor
//! shadowed and the alpha unsquared (`scripts/vita-alpha-emissive-sweep.py`: 171 of 171 programs
//! of `data.psarc`, `dlc1.psarc` and `dlc2.psarc`; `docs/formats/rcsmaterial.md`, "The picture's
//! alpha times a colour"). It is HD's combine, so the term is HD's glow-table mode
//! ([`PROGRAM_PICTURE_ALPHA_EMISSIVE`], `shade.wesl`) and the sampler and colour hashes are
//! HD's ([`crate::mesh::rcs::alpha_emissive`]). The Zone permutations do not declare the colour.
//!
//! **`dn_diffuse_specular_emiss` and its `_glow` twin** list `SpecularTexture` first, but their
//! programs sample `alpha_emissive` as the picture and use the other only as the weight of the
//! specular term (`oag_rcs::rcsmodel::psp2::specular_picture`). The picture is `alpha_emissive`
//! (`psp2::bind_textures`), and the specular map rides the third binding with the entry's second
//! lane set to [`ALPHA_EMISSIVE_SPECULAR_MAP`], which `shade.wesl` reads as `specular * S.rgb`.
//! The `_glow` twin authors no colour, so its entry carries a zero tint.
//!
//! A material is admitted when it binds the sampler, draws that sampler's picture (the mask is
//! the alpha of the texel the draw samples) and either authors the colour or weights its
//! specular by a map. One that binds the sampler and does neither has no entry.

use std::collections::HashMap;
use std::sync::Arc;

use oag_rcs::rcsmodel::psp2::{self, specular_picture};

use super::emissive_add::{Add, Plan};
use super::{Report, decode_material_texture, glow};
use crate::mesh::rcs::Textures;
use crate::mesh::rcs::alpha_emissive::{COLOUR, PICTURE};
use crate::mesh::{
    ALPHA_EMISSIVE_SPECULAR_MAP, Emissive, ModelTexture, PROGRAM_PICTURE_ALPHA_EMISSIVE,
};

/// The family's picture and specular map, unless the diagnostic switch leaves them to the
/// sampler order.
pub(super) fn specular_picture(
    material: &psp2::material::Material,
) -> Option<specular_picture::SpecularPicture<'_>> {
    if crate::mesh::rcs::isolate::specular_picture_off() {
        return None;
    }
    specular_picture::of(material)
}

/// Admits each material that binds `alpha_emissive` and authors `glowtinrt` (or weights its
/// specular by a map), writing its glow-table entry into the glow plan's table and its slot
/// (and specular map) into `plan`. A slot another reading holds keeps it.
pub(super) fn plan(
    decoded: &psp2::Model,
    glow: &mut glow::Plan,
    plan: &mut Plan,
    textures: Textures<'_>,
    report: &mut Report,
) {
    if crate::mesh::rcs::isolate::alpha_emissive_off() {
        return;
    }
    let mut maps: HashMap<&str, Option<Arc<ModelTexture>>> = HashMap::new();
    for (index, material) in decoded.materials.iter().enumerate() {
        let Some(picture) = material.sampler(PICTURE).filter(|p| !p.is_empty()) else {
            continue;
        };
        let tint = material.param(COLOUR);
        let weighted = specular_picture(material);
        if tint.is_none() && weighted.is_none() {
            continue;
        }
        let drawn = weighted.map_or_else(|| material.diffuse_texture(), |w| Some(w.picture));
        let held = plan.adds.get(index).is_some_and(Option::is_some) || glow.role(Some(index)) != 0;
        if held || drawn != Some(picture) {
            report.alpha_emissive_unread += 1;
            continue;
        }
        let specular = match weighted {
            Some(w) => {
                let map = maps
                    .entry(w.specular)
                    .or_insert_with(|| {
                        textures(w.specular)
                            .and_then(|blob| decode_material_texture(w.specular, &blob, report))
                            .map(Arc::new)
                    })
                    .clone();
                if map.is_none() {
                    report.alpha_emissive_unread += 1;
                    continue;
                }
                map
            }
            None => None,
        };
        let channel = |i: usize| tint.as_ref().and_then(|t| t.get(i)).copied().unwrap_or(0.0);
        let entry = Emissive {
            tint: [channel(0), channel(1), channel(2)],
            offset: 0.0,
            scale: 1.0,
            rate: 0.0,
            program: [
                PROGRAM_PICTURE_ALPHA_EMISSIVE,
                if specular.is_some() {
                    ALPHA_EMISSIVE_SPECULAR_MAP
                } else {
                    0.0
                },
            ],
        };
        let emissive = &mut glow.emissive;
        let at = emissive.iter().position(|e| *e == entry).or_else(|| {
            (emissive.len() + 1 < crate::mesh::rcs::EMISSIVE_LIMIT).then(|| {
                emissive.push(entry);
                emissive.len() - 1
            })
        });
        let Some(slot) = at.and_then(|at| u32::try_from(at + 1).ok()) else {
            report.alpha_emissive_unread += 1;
            continue;
        };
        plan.adds[index] = Some(Add {
            slot,
            texture: specular,
            colour: None,
        });
        if tint.is_some() {
            report.alpha_emissive_materials += 1;
        }
        report.alpha_emissive_specular_maps += usize::from(weighted.is_some());
    }
}
