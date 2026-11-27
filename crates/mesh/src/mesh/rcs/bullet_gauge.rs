//! HD's Detonator craft's cannon gauge: `detonator_bulletgaugesegments`, the thirty segments on the
//! hull's back.
//!
//! The programs (`scripts/ps3-microcode.py fp-file`, every lit block, SVC0 and SVC1 alike) take the
//! segment picture twice at the coordinate set, each shifted along `v`:
//!
//! ```text
//! B   = tex(unit 0, (u, v + 0.5 * StandardBullets))    the vertex program adds the shift
//! A   = tex(unit 0, (u, v + 0.5 * PowerBullets)) * PowerColour
//! out = B * StandardColour * B.r + A * (1 - B.r)      no light term; fogged, then `/2`
//! ```
//!
//! The engine binds all four by pointer to one static block, so the material's own authored values
//! never reach the draw: `PowerBullets` is `0` and `PowerColour` white for the whole race, and
//! `Detonator_UpdateHud` writes `StandardBullets` (the clip over 15, or the reload's refill) and
//! `StandardColour` (blue, red while locked) each frame
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-ammo-gauge.md`). The entry made here holds the
//! bind-time state (full, red) as its tint and offset, which the caller rewrites per frame;
//! [`PROGRAM_BULLET_GAUGE`] draws it.
//!
//! Matched by what the programs declare, never by the material's name.

use oag_rcs::rcsmaterial;
use oag_rcs::rcsmodel;

use crate::mesh::{Emissive, PROGRAM_BULLET_GAUGE, slots};

use super::Report;
use super::emissive::EMISSIVE_LIMIT;

/// `~crc32("StandardBullets")`, a vertex-program constant.
const STANDARD_BULLETS: u32 = 0x6bdb_824c;
/// `~crc32("StandardColour")`, `~crc32("PowerBullets")`, `~crc32("PowerColour")`: fragment constants.
const FRAGMENT: [u32; 3] = [0xf038_ed98, 0xfe63_72ef, 0xf782_cc50];

/// The block's state when the hull binds it (`0x0029d458`): a full gauge, red.
pub const BIND_TIME_COLOUR: [f32; 3] = [1.0, 0.0, 0.0];
/// The bind-time fill, `StandardBullets = 1.0`.
pub const BIND_TIME_FILL: f32 = 1.0;

/// Gives each gauge material an entry of [`PROGRAM_BULLET_GAUGE`] holding the bind-time state, and
/// its index in `packed`.
pub(super) fn bullet_gauge(
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
        let declares = |offset: usize, hashes: &[u32]| {
            rcsmaterial::Declared::parse(&blob, offset)
                .is_some_and(|d| hashes.iter().all(|h| d.parameters.contains(h)))
        };
        if !declares(variant.vertex.offset, &[STANDARD_BULLETS])
            || !declares(variant.fragment.offset, &FRAGMENT)
        {
            continue;
        }
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        if table.len() + 1 >= EMISSIVE_LIMIT {
            continue;
        }
        table.push(Emissive {
            tint: BIND_TIME_COLOUR,
            offset: BIND_TIME_FILL,
            scale: 1.0,
            rate: 0.0,
            program: [PROGRAM_BULLET_GAUGE, 0.0],
        });
        let Ok(index) = u32::try_from(table.len()) else {
            continue;
        };
        *word &= slots::ROLE_MASK & !slots::ADD_SECOND;
        *word |= index << slots::MATERIAL_SHIFT;
        super::census::claim(slot, "bullet_gauge", super::census::Outcome::Drawn);
        report.bullet_gauge_bound += 1;
    }
}
