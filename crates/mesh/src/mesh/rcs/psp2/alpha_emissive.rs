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
//! A material is admitted when it binds the sampler, authors the colour and draws that sampler's
//! picture: the mask is the alpha of the texel the draw samples. One that binds the sampler and
//! authors no colour (Sol 2's `dn_diffuse_specular_emiss_glow`) has no entry.

use oag_rcs::rcsmodel::psp2;

use super::emissive_add::{Add, Plan};
use super::{Report, glow};
use crate::mesh::rcs::alpha_emissive::{COLOUR, PICTURE};
use crate::mesh::{Emissive, PROGRAM_PICTURE_ALPHA_EMISSIVE};

/// Admits each material that binds `alpha_emissive` and authors `glowtinrt`, writing its
/// glow-table entry into the glow plan's table and its slot into `plan`. A slot another reading
/// holds keeps it.
pub(super) fn plan(
    decoded: &psp2::Model,
    glow: &mut glow::Plan,
    plan: &mut Plan,
    report: &mut Report,
) {
    if crate::mesh::rcs::isolate::alpha_emissive_off() {
        return;
    }
    for (index, material) in decoded.materials.iter().enumerate() {
        let Some(picture) = material.sampler(PICTURE).filter(|p| !p.is_empty()) else {
            continue;
        };
        let Some(tint) = material.param(COLOUR) else {
            continue;
        };
        let channel = |i: usize| tint.get(i).copied().unwrap_or(0.0);
        let held = plan.adds.get(index).is_some_and(Option::is_some) || glow.role(Some(index)) != 0;
        if held || material.diffuse_texture() != Some(picture) {
            report.alpha_emissive_unread += 1;
            continue;
        }
        let entry = Emissive {
            tint: [channel(0), channel(1), channel(2)],
            offset: 0.0,
            scale: 1.0,
            rate: 0.0,
            program: [PROGRAM_PICTURE_ALPHA_EMISSIVE, 0.0],
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
            texture: None,
        });
        report.alpha_emissive_materials += 1;
    }
}
