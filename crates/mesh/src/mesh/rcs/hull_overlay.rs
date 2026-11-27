//! The iridescent overlay a Detonator hull panel adds to its paint.
//!
//! `detonator_ship_dg_iridescent`'s `IBL` block (`0x46d0`, `scripts/ps3-microcode.py
//! fp-file`) ends in
//!
//! ```text
//! @0x22  TEX H6.xyz, R1.wwww unit3        ramp = detonator_stripe(N.V, N.V)
//! @0x23  MUL H6.xyz, R2, H6               H6 = picture * ramp
//! @0x3a  MUL x2 H8.xyz, H6, Colour [NE]   H8 = 2 * Colour * H6        (Colour < 0.5)
//! @0x4a  MAD H1.xyz, H9, R2, H8           light * picture + H8
//! ```
//!
//! then the fog and the `/2`: an untinted term added after the light multiply, so neither
//! the probe tint nor the stage colours it. `R1.w` is `dot(N, R3)` with `R3` the normalised
//! `TC1`, and the hull's vertex block (`0x3a20`, `@9`) writes `o[TC1].xyz = eye - position`: the
//! view vector, so the ramp is addressed at the facing term, positive on a front face
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-look.md`, "The hull's paint").
//!
//! The material's glow-table entry carries `Colour` in `tint.x` and the rate
//! [`HULL_OVERLAY_RATE`]; the ramp is the slot's second binding. Only the branch the authored
//! `Colour` takes is wired (`Colour < 0.5`, which is `0.10948` on this material); the overlay
//! blend above it is not read from a material that authors it.

use oag_rcs::rcsmodel;

use super::skin::Pick;
use super::{Report, slots};
use crate::mesh::{Emissive, HULL_OVERLAY_RATE};

/// `Colour`, the overlay's weight.
const COLOUR: u32 = 0x02ab_9f07;

pub(super) fn hull_overlay(
    model: &rcsmodel::Model,
    picks: &[Pick],
    seconds: &[Option<std::sync::Arc<crate::mesh::ModelTexture>>],
    packed: &mut [u32],
    table: &mut Vec<Emissive>,
    report: &mut Report,
) {
    for (slot, material) in model.materials.iter().enumerate() {
        let ramp = picks
            .get(slot)
            .and_then(|pick| pick.aux)
            .and_then(|aux| material.samplers.get(aux))
            .is_some_and(|(hash, _)| super::emissive::SHIP_FACING_RAMP_SAMPLERS.contains(hash));
        if !ramp || !seconds.get(slot).is_some_and(Option::is_some) {
            continue;
        }
        let Some(colour) = material
            .parameters
            .iter()
            .find(|p| p.hash == COLOUR)
            .map(|p| p.value[0])
            .filter(|c| *c < 0.5)
        else {
            continue;
        };
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        if slots::material_index(*word) != 0 {
            continue;
        }
        let layer = Emissive {
            tint: [colour, 0.0, 0.0],
            offset: 0.0,
            scale: 1.0,
            rate: HULL_OVERLAY_RATE,
            program: [0.0; 2],
        };
        let index = table.iter().position(|seen| *seen == layer).or_else(|| {
            (table.len() + 1 < super::EMISSIVE_LIMIT).then(|| {
                table.push(layer);
                table.len() - 1
            })
        });
        let Some(index) = index else { continue };
        *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        super::census::claim(slot, "hull_overlay", super::census::Outcome::Drawn);
        report.hull_overlay_bound += 1;
    }
}
