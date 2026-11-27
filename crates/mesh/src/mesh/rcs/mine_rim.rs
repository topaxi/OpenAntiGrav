//! HD's Detonator mine: the football and the spikes share `detonator_mine_normal_alpha`, whose
//! fragment program is an unlit Fresnel rim over two named constants.
//!
//! Read off the material's own microcode (`ps3-microcode.py fp-file`, the unfogged variant's
//! block; `docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-material.md`, "The mine's fragment program"):
//!
//! ```text
//! rim = sat(1 - N.V)
//! p   = 4 sat(1 - MineBrightness) + 1        ; MOV x4, then ADD 1
//! rgb = rim^p / (1 - 0.9 MineColour)         ; RCP of three MAD 1 - 0.9 c, per channel
//! a   = the normal map's alpha
//! ```
//!
//! so a mine in silence (`MineBrightness 0`) is a tight rim, `rim^5`, and a loud one (`>= 1`) a
//! wide one, `rim^1`; `MineColour` is a dodge, `c / (1 - 0.9 c)`-like, that a literal `2` (a dark
//! mine) turns negative. Both constants are `float`s the engine writes per mine, which the glow
//! table's entry carries here: the tint is `MineColour`, the offset `MineBrightness`, both
//! rewritten per instance by the caller each frame
//! ([`PROGRAM_MINE_RIM`](crate::mesh::PROGRAM_MINE_RIM), `mesh.wesl`).
//!
//! Matched by what the program reads, never by the model's name: a program that patches both
//! constants and declares one sampler. The sampler's picture (`mine_rita_football_n`, the
//! spikes' `_n`) is the tangent-space normal the program dots against the eye vector and whose
//! alpha it outputs: bound third, the chunk carrying its authored tangent, as a hull's normal
//! map is (`super::hull_normal`).

use oag_rcs::rcsmaterial::{self, fragment::Program};
use oag_rcs::rcsmodel;

use std::sync::Arc;

use crate::mesh::{Emissive, ModelTexture, PROGRAM_MINE_RIM, TangentFrame, TextureSlots, slots};

use super::Report;
use super::emissive::EMISSIVE_LIMIT;
use super::skin::decode_texture;

/// `~crc32("MineBrightness")`.
pub(super) const BRIGHTNESS: u32 = 0xc420_e90d;
/// `~crc32("MineColour")`.
pub(super) const COLOUR: u32 = 0xd788_3cf6;

/// Whether `program` is the mine's: it patches both constants and samples one texture.
fn is_mine(declared: &rcsmaterial::Declared, program: &Program) -> bool {
    declared.samplers.len() == 1
        && program.patches(BRIGHTNESS).next().is_some()
        && program.patches(COLOUR).next().is_some()
}

/// Gives each mine material its own entry of [`PROGRAM_MINE_RIM`], holding the authored
/// `MineColour` and `MineBrightness` as the starting values, and its index in `packed`; returns
/// the normal maps to bind third, and marks the materials' tangent frames in `tangent`.
pub(super) fn mine_rim(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: super::Textures<'_>,
    (packed, skins): (&mut [u32], &mut TextureSlots),
    (table, tangent): (&mut Vec<Emissive>, &mut [TangentFrame]),
    report: &mut Report,
) -> TextureSlots {
    let mut maps: TextureSlots = vec![None; model.materials.len()];
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
        if !is_mine(&program.declared, &program) {
            continue;
        }
        let authored = |hash: u32| material.parameters.iter().find(|p| p.hash == hash);
        let (Some(colour), Some(brightness)) = (authored(COLOUR), authored(BRIGHTNESS)) else {
            continue;
        };
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        if table.len() + 1 >= EMISSIVE_LIMIT {
            continue;
        }
        let map = program
            .declared
            .samplers
            .first()
            .and_then(|&(hash, _)| {
                material
                    .samplers
                    .iter()
                    .find(|(h, path)| *h == hash && path.is_some())
            })
            .and_then(|(_, path)| path.as_deref())
            .and_then(|path| textures(&format!("/{path}")).and_then(|b| decode_texture(path, &b)));
        if let Some(map) = map {
            maps[slot] = Some(Arc::new(map));
            if let Some(frame) = tangent.get_mut(slot) {
                *frame = if rcsmaterial::vertex::Program::of(&blob, variant.vertex)
                    .is_some_and(|p| p.bitangent_is_signed(rcsmodel::vertex_decl::TANGENT_HASH))
                {
                    TangentFrame::Signed
                } else {
                    TangentFrame::Raw
                };
            }
        }
        table.push(Emissive {
            tint: [colour.value[0], colour.value[1], colour.value[2]],
            offset: brightness.value[0],
            scale: 1.0,
            rate: 0.0,
            program: [PROGRAM_MINE_RIM, 0.0],
        });
        let Ok(index) = u32::try_from(table.len()) else {
            continue;
        };
        // A draw names its material's row of pictures through its albedo: a chunk with none
        // reads the shared row, which holds no normal map. The program never samples it.
        if let Some(skin) = skins.get_mut(slot).filter(|skin| skin.is_none()) {
            *skin = Some(Arc::new(ModelTexture::rgba8(
                "mine rim row".into(),
                1,
                1,
                vec![255; 4],
                None,
            )));
        }
        *word &= slots::ROLE_MASK & !slots::ADD_SECOND;
        *word |= index << slots::MATERIAL_SHIFT;
        super::census::claim(slot, "mine_rim", super::census::Outcome::Drawn);
        report.mine_rim_bound += 1;
    }
    maps
}

/// Whether a surface's packed word names a [`PROGRAM_MINE_RIM`] entry of `table`.
pub(super) fn draws_mine_rim(roles: u32, table: &[Emissive]) -> bool {
    let index = slots::material_index(roles) as usize;
    index > 0
        && table
            .get(index - 1)
            .is_some_and(|entry| entry.program[0] == PROGRAM_MINE_RIM)
}

/// A chunk's authored tangent: the declared one, or on a mine surface the inline one at `+10`,
/// which the mine's vertex program reads (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-material.md`,
/// "The lit draw, read live"). The mine's chunks declare no layout.
pub(super) fn tangents(
    mine: bool,
    mesh: &rcsmodel::Mesh,
    data: &[u8],
    submesh: &rcsmodel::SubMesh,
    stride: usize,
) -> Option<Vec<[u8; 4]>> {
    let declared = mesh.tangents(data, submesh, stride).ok().flatten();
    if declared.is_some() || !mine {
        return declared;
    }
    mesh.inline_tangents(data, submesh, stride).ok().flatten()
}
