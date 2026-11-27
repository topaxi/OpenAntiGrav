//! The 2048 lineage's shield shells: `2048_shipshield_old` (HD-content shells) and
//! `hd_shipshield_old` (the cockpit sphere) carry a scrolling "Lightning" layer, and
//! `2048_magnetic_shield` (2048's own craft) scrolls its texture.
//!
//! The pixel program of the first two (`docs/ghidra/functions/ps4-omega-eu/shield.md`, "The two
//! shell programs"; the same on the Vita, `docs/ghidra/functions/vita-2048-eu-v104/shield.md`,
//! "The lattice scrolls") draws `ShieldColour * diffuse(u + A * time, v)` plus
//! `Lightning_RGB_Multiplier * lightning(uv2 + time + noise)`, where `noise` is the lightning
//! picture's alpha at `(u2, B * v2 + C * time)`. `A`, `B` and `C` are literals of the program,
//! read by [`oag_rcs::gcn::lightning_rates`] (PS4) and [`oag_rcs::gxp::lightning_rates`] (Vita)
//! and never typed here; the multiplier is a material parameter.
//!
//! The picture rides the third binding (the same carrier as [`super::emissive_add`], whose
//! [`Plan`] this fills in), the first pair of the packed `Uv1_Uv2` is the diffuse and the second
//! rides `texcoord2`, and the four numbers are one glow-table entry of
//! [`PROGRAM_SHIELD_LIGHTNING`]: tint `(multiplier, A, C)`, offset `B`. `shade.wesl` adds the
//! term after the shell's colour multiply, so the lightning is not tinted by `ShieldColour`.
//!
//! The magnetic shell's **vertex** program adds `time * Speed_multiplier` to `u`, so it rides
//! the plain scroll track ([`magnetic_scroll`]).

use std::sync::Arc;

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

/// `Speed_multiplier`, the magnetic shell's scroll rate.
const SPEED_MULTIPLIER: &str = "Speed_multiplier";

fn ends_with(material: &Material, file: &str) -> bool {
    material.name.to_ascii_lowercase().ends_with(file)
}

fn is_shell(material: &Material) -> bool {
    ends_with(material, "2048_shipshield_old.rcsmaterial")
        || ends_with(material, "hd_shipshield_old.rcsmaterial")
}

/// `2048_magnetic_shield`'s `(u, v)` scroll in texture units per second: its authored
/// `Speed_multiplier` along `u`; `None` for any other material.
pub(super) fn magnetic_scroll(material: &Material) -> Option<[f32; 2]> {
    if !ends_with(material, "2048_magnetic_shield.rcsmaterial") {
        return None;
    }
    let rate = *material.param(name_hash(SPEED_MULTIPLIER))?.first()?;
    (rate.is_finite() && rate != 0.0).then_some([rate, 0.0])
}

/// Admits each shell material of the first two kinds whose picture, weight and program rates all read,
/// writing its glow-table entry into `emissive` and its picture into `plan`.
pub(super) fn plan(
    decoded: &psp2::Model,
    ps4: bool,
    plan: &mut Plan,
    emissive: &mut Vec<Emissive>,
    textures: Textures<'_>,
    report: &mut Report,
) {
    for (index, material) in decoded.materials.iter().enumerate() {
        if !is_shell(material) {
            continue;
        }
        let read = (|| {
            let path = material
                .sampler(name_hash(SAMPLER))
                .filter(|p| !p.is_empty())?;
            let weight = *material.param(name_hash(MULTIPLIER))?.first()?;
            let program = textures(&material.name)?;
            let rates = if ps4 {
                oag_rcs::gcn::lightning_rates(&program)?
            } else {
                oag_rcs::gxp::lightning_rates(&program)?
            };
            let picture = decode_material_texture(path, &textures(path)?, report)?;
            Some((weight, rates, Arc::new(picture)))
        })();
        let Some((weight, rates, texture)) = read else {
            crate::mesh::rcs::census::claim(
                index,
                "shield_lightning",
                crate::mesh::rcs::census::Outcome::NotDrawn,
            );
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
            crate::mesh::rcs::census::claim(
                index,
                "shield_lightning",
                crate::mesh::rcs::census::Outcome::NotDrawn,
            );
            report.shield_lightning_unread += 1;
            continue;
        };
        plan.adds[index] = Some(Add {
            slot,
            texture: Some(texture),
            colour: None,
            alpha_gated: false,
        });
        crate::mesh::rcs::census::claim(
            index,
            "shield_lightning",
            crate::mesh::rcs::census::Outcome::Drawn,
        );
        report.shield_lightning_materials += 1;
    }
}
