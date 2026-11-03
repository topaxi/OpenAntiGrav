//! Omega's `2048_shipshield_old`: the second, scrolling "Lightning" layer of the shield shell.
//!
//! The pixel program (`docs/ghidra/functions/ps4-omega-eu/shield.md`, "The Lightning layer")
//! draws `ShieldColour * diffuse(u + A * time, v)` plus `Lightning_RGB_Multiplier *
//! lightning(uv2 + time + noise)`, where `noise` is the lightning picture's alpha at
//! `(u2, B * v2 + C * time)`. `A`, `B` and `C` are literals of the program, read by
//! [`oag_rcs::gcn::lightning_rates`] and never typed here; the multiplier is a material
//! parameter.
//!
//! The picture rides the third binding (the same carrier as [`super::emissive_add`], whose
//! [`Plan`] this fills in), the second coordinate set rides `texcoord2`, and the four numbers
//! are one glow-table entry of [`PROGRAM_SHIELD_LIGHTNING`]: tint `(multiplier, A, C)`, offset
//! `B`. `shade.wesl` adds the term after the shell's colour multiply, so the lightning is not
//! tinted by `ShieldColour`.

use std::sync::Arc;

use oag_rcs::gcn::lightning_rates;
use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmodel::psp2::{self, material::Material};

use super::emissive_add::{Add, Plan};
use super::{Report, decode_material_texture};
use crate::mesh::rcs::Textures;
use crate::mesh::{Emissive, PROGRAM_SHIELD_LIGHTNING};

/// `LightningTexture`, the second picture.
const SAMPLER: &str = "LightningTexture";
/// `Lightning_RGB_Multiplier`, its weight.
const MULTIPLIER: &str = "Lightning_RGB_Multiplier";

fn is_shell(material: &Material) -> bool {
    material
        .name
        .to_ascii_lowercase()
        .ends_with("2048_shipshield_old.rcsmaterial")
}

/// Admits each `2048_shipshield_old` material whose picture, weight and program rates all read,
/// writing its glow-table entry into `emissive` and its picture into `plan`.
pub(super) fn plan(
    decoded: &psp2::Model,
    ps4: bool,
    plan: &mut Plan,
    emissive: &mut Vec<Emissive>,
    textures: Textures<'_>,
    report: &mut Report,
) {
    if !ps4 {
        return;
    }
    for (index, material) in decoded.materials.iter().enumerate() {
        if !is_shell(material) {
            continue;
        }
        let read = (|| {
            let path = material
                .sampler(name_hash(SAMPLER))
                .filter(|p| !p.is_empty())?;
            let weight = *material.param(name_hash(MULTIPLIER))?.first()?;
            let rates = lightning_rates(&textures(&material.name)?)?;
            let picture = decode_material_texture(path, &textures(path)?, report)?;
            Some((weight, rates, Arc::new(picture)))
        })();
        let Some((weight, rates, texture)) = read else {
            report.shield_lightning_unread += 1;
            continue;
        };
        let entry = Emissive {
            tint: [weight, rates.diffuse_scroll, rates.noise_rate],
            offset: rates.noise_scale,
            scale: 1.0,
            rate: 0.0,
            program: [PROGRAM_SHIELD_LIGHTNING, 0.0],
        };
        let at = emissive.iter().position(|e| *e == entry).or_else(|| {
            (emissive.len() + 1 < crate::mesh::rcs::EMISSIVE_LIMIT).then(|| {
                emissive.push(entry);
                emissive.len() - 1
            })
        });
        let Some(slot) = at.and_then(|at| u32::try_from(at + 1).ok()) else {
            report.shield_lightning_unread += 1;
            continue;
        };
        plan.adds[index] = Some(Add { slot, texture });
        report.shield_lightning_materials += 1;
    }
}
