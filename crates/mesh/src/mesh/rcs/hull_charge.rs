//! HD's Detonator craft's two plasma shells: `detonator_shield_plasma` and `detonator_shield_plasma1`.
//!
//! Each fragment program ends its alpha with `MOV H0.w, <param>` and no other writer: the alpha is
//! `Charge1` on the first material and `Charge2` on the second (`scripts/ps3-microcode.py fp-file`,
//! every block). The engine binds both by pointer to one static block
//! (`Ship_BindDetonatorHullParams`, `0x00107b58`) that `Ship_UpdateDetonatorHullParams` writes each
//! frame, so the materials' own authored values never reach the draw
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-ammo-gauge.md`, "`Charge1`, `Charge2`"). The
//! entry made here holds the bind-time value, `0`, as its offset; the caller rewrites it per frame.
//! [`PROGRAM_HULL_CHARGE_1`] and [`PROGRAM_HULL_CHARGE_2`] scale the draw's alpha by it.
//!
//! Matched by the parameter the fragment program declares, never by the material's name.

use oag_rcs::rcsmaterial;
use oag_rcs::rcsmodel;

use crate::mesh::{Emissive, PROGRAM_HULL_CHARGE_1, PROGRAM_HULL_CHARGE_2, slots};

use super::Report;
use super::emissive::EMISSIVE_LIMIT;

/// `~crc32("Charge1")` and `~crc32("Charge2")`.
const CHARGE: [(u32, f32); 2] = [
    (0x927f_b6c5, PROGRAM_HULL_CHARGE_1),
    (0x0b76_e77f, PROGRAM_HULL_CHARGE_2),
];

/// Gives each plasma-shell material an entry holding the bind-time charge, `0`.
pub(super) fn hull_charge(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: super::Textures<'_>,
    packed: &mut [u32],
    table: &mut Vec<Emissive>,
    report: &mut Report,
) {
    for (slot, material) in model.materials.iter().enumerate() {
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(blob) = textures(&format!("/{}", material.name)) else {
            continue;
        };
        let Some(declared) = rcsmaterial::Declared::parse(&blob, variant.fragment.offset) else {
            continue;
        };
        let Some(&(_, program)) = CHARGE
            .iter()
            .find(|(hash, _)| declared.parameters.contains(hash))
        else {
            continue;
        };
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        if slots::material_index(*word) != 0 || table.len() + 1 >= EMISSIVE_LIMIT {
            continue;
        }
        table.push(Emissive {
            tint: [0.0; 3],
            offset: 0.0,
            scale: 1.0,
            rate: 0.0,
            program: [program, 0.0],
        });
        let Ok(index) = u32::try_from(table.len()) else {
            continue;
        };
        *word |= index << slots::MATERIAL_SHIFT;
        super::census::claim(slot, "hull_charge", super::census::Outcome::Drawn);
        report.hull_charge_bound += 1;
    }
}
