//! HD's `hexagonalshield_rich`, the shield shell and cockpit sphere: its colour tap scrolls.
//!
//! The program samples the one texture twice: the alpha at `(u, v)` and the colour at
//! `(u + A * time, v)`, then multiplies the colour by `ShieldColour`
//! (`docs/ghidra/functions/ps3-hdfury-eu/shield.md`, "The lattice scrolls"). The texture holds
//! the lattice in its alpha and brightness bands in its colour, so the lattice stands still
//! and a highlight sweeps through it. `A` is a literal of the program, read off the resolved
//! block and never typed here.
//!
//! The scroll rides the glow table as [`PROGRAM_SHIELD_LIGHTNING`] with a zero lightning
//! weight: that mode is Omega's recompile of the same tap, plus a lightning layer HD lacks.

use oag_rcs::rcsmaterial::fragment::Program;
use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{Emissive, PROGRAM_SHIELD_LIGHTNING, slots};

use super::emissive::EMISSIVE_LIMIT;
use super::{Report, Textures};

/// The engine clock.
const TIME: u32 = 0x906b_67ba;

/// Writes each `hexagonalshield_rich` slot's glow-table entry `[0, A, 0]`.
pub(super) fn shield_scroll(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    table: &mut Vec<Emissive>,
    report: &mut Report,
) {
    for (slot, material) in model.materials.iter().enumerate() {
        if super::emissive::family(material) != "hexagonalshield_rich" {
            continue;
        }
        let rate = variants.get(slot).copied().flatten().and_then(|variant| {
            let blob = textures(&format!("/{}", material.name))?;
            Program::parse(&blob, variant.fragment.offset)?.clock_scroll_rate(TIME)
        });
        let (Some(rate), Some(word)) = (rate, packed.get_mut(slot)) else {
            super::census::claim(slot, "shield_scroll", super::census::Outcome::NotDrawn);
            report.shield_scroll_unread += 1;
            continue;
        };
        // Any other family's glow index is kept.
        if slots::material_index(*word) != 0 && *word & slots::ADD_SECOND == 0 {
            super::census::claim(slot, "shield_scroll", super::census::Outcome::NotDrawn);
            report.shield_scroll_unread += 1;
            continue;
        }
        let entry = Emissive {
            tint: [0.0, rate, 0.0],
            offset: 0.0,
            scale: 1.0,
            rate: 0.0,
            program: [PROGRAM_SHIELD_LIGHTNING, 0.0],
        };
        let index = table.iter().position(|e| *e == entry).or_else(|| {
            (table.len() + 1 < EMISSIVE_LIMIT).then(|| {
                table.push(entry);
                table.len() - 1
            })
        });
        let Some(index) = index else {
            super::census::claim(slot, "shield_scroll", super::census::Outcome::NotDrawn);
            report.shield_scroll_unread += 1;
            continue;
        };
        *word &= slots::ROLE_MASK & !slots::ADD_SECOND;
        *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        super::census::claim(slot, "shield_scroll", super::census::Outcome::Drawn);
        report.shield_scroll_bound += 1;
    }
}
