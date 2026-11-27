//! HD's Detonator bomb body and equalisers: two named constants, a colour and a brightness,
//! that the engine writes per bomb.
//!
//! `detonator_bomb_normal` patches `BombColour` and `BombBrightness`
//! (`0x1eb1279c`, `0x6cf44555`), `detonator_bomb_normal_equalisers` `BombEQColour` and
//! `BombEQBrightness` (`0x659bbb32`, `0x9df50e6b`); both end in a multiply by the brightness
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-material.md`, "The bomb's programs").
//! How the colour enters beyond that (a normal-mapped facing term, an environment picture's
//! overlay) is unread, so a material that patches one of the pairs draws as its colour times its
//! brightness, unlit ([`PROGRAM_BOMB_TINT`], **chosen, not measured**), on a glow-table entry
//! holding the authored values that the caller rewrites per instance.
//!
//! Matched by what the program patches, never by the model's name. The generic `Colour`,
//! `Brightness` and `RainbowColour` the rays and the glow edges patch are other families' names
//! and are left as they draw.

use oag_rcs::rcsmaterial::{self, fragment::Program};
use oag_rcs::rcsmodel;

use crate::mesh::{Emissive, PROGRAM_BOMB_TINT, slots};

use super::Report;
use super::emissive::EMISSIVE_LIMIT;

/// `(~crc32("BombColour"), ~crc32("BombBrightness"))` and the equalisers' pair.
const PAIRS: [(u32, u32); 2] = [(0x1eb1_279c, 0x6cf4_4555), (0x659b_bb32, 0x9df5_0e6b)];

/// Gives each bomb material its own entry of [`PROGRAM_BOMB_TINT`], holding the authored colour and
/// brightness, and its index in `packed`.
pub(super) fn bomb_tint(
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
        let Some(program) = Program::parse(&blob, variant.fragment.offset) else {
            continue;
        };
        let authored = |hash: u32| material.parameters.iter().find(|p| p.hash == hash);
        let Some((colour, brightness)) = PAIRS.iter().find_map(|&(colour, brightness)| {
            program.patches(colour).next()?;
            program.patches(brightness).next()?;
            Some((authored(colour)?, authored(brightness)?))
        }) else {
            continue;
        };
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        if table.len() + 1 >= EMISSIVE_LIMIT {
            continue;
        }
        table.push(Emissive {
            tint: [colour.value[0], colour.value[1], colour.value[2]],
            offset: brightness.value[0],
            scale: 1.0,
            rate: 0.0,
            program: [PROGRAM_BOMB_TINT, 0.0],
        });
        let Ok(index) = u32::try_from(table.len()) else {
            continue;
        };
        *word &= slots::ROLE_MASK & !slots::ADD_SECOND;
        *word |= index << slots::MATERIAL_SHIFT;
        super::census::claim(slot, "bomb_tint", super::census::Outcome::Drawn);
        report.bomb_tint_bound += 1;
    }
}
