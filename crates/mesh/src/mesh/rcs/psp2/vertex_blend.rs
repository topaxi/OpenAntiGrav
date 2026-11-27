//! 2048's `fc07_blend_vertred_lambert_2diff`: the lit albedo is two pictures mixed per vertex.
//!
//! The family's programs square the `DiffuseMap` and the `DiffuseMap01` texel, mix them by
//! `VertexColour1`'s red and light the mix (`oag_rcs::rcsmodel::psp2::vertex_blend`,
//! `docs/formats/gxp.md`, "`fc07_blend_vertred_lambert_2diff` mixes two pictures by the vertex
//! red"). The generic reading draws `DiffuseMap` alone: Altima's gravel shore and rock faces
//! without their earth and grass.
//!
//! The second picture rides the third binding ([`crate::mesh::Model::pad_masks`]), the mode is
//! one glow-table entry of [`PROGRAM_VERTEX_BLEND`], and the weight rides the second coordinate
//! pair's `x`, as the blended water edge's two `VertexColour1` lanes do (`super::water`); both
//! taps are at the diffuse coordinate, as the programs fetch them. The light is whatever the
//! submesh already takes (vertex-lit, lightmapped or ambient): the mix replaces the albedo only.
//!
//! Vita only. Omega's copy of the plain file runs the same mix on its own programs and is not
//! wired; its circuits mostly use the `_new` and `2diffnormal` siblings, which are unread.

use std::sync::Arc;

use oag_rcs::rcsmodel::psp2::{self, vertex_blend};

use super::{Pictures, Report, decode_picture};
use crate::mesh::rcs::Textures;
use crate::mesh::{Emissive, ModelTexture, PROGRAM_VERTEX_BLEND, slots};

/// One admitted material: its glow-table slot (entry index plus one) and its second picture.
pub(super) struct Blend {
    pub slot: u32,
    pub second: Arc<ModelTexture>,
}

/// Per material, the blend it draws.
#[derive(Default)]
pub(super) struct Plan {
    blends: Vec<Option<Blend>>,
}

impl Plan {
    pub fn of(&self, material: Option<usize>) -> Option<&Blend> {
        self.blends.get(material?)?.as_ref()
    }

    /// The vertex bits that select this material's glow-table entry; `0` for any other.
    pub fn role(&self, material: Option<usize>) -> u32 {
        self.of(material)
            .map_or(0, |b| b.slot << slots::MATERIAL_SHIFT)
    }

    /// Whether any material rides the third binding.
    pub fn any(&self) -> bool {
        self.blends.iter().any(Option::is_some)
    }

    /// The second picture this material binds third.
    pub fn second(&self, material: Option<usize>) -> Option<&Arc<ModelTexture>> {
        self.of(material).map(|b| &b.second)
    }

    /// The second coordinate pair of a blended vertex: the weight in `x`.
    pub fn texcoord2(
        &self,
        material: Option<usize>,
        vertex_colour1: Option<&[u8; 4]>,
    ) -> Option<[f32; 2]> {
        self.of(material)?;
        Some([vertex_blend::weight(*vertex_colour1?), 0.0])
    }
}

/// The family's one glow-table entry: the mode alone, nothing authored rides it.
const ENTRY: Emissive = Emissive {
    tint: [0.0; 3],
    offset: 0.0,
    scale: 0.0,
    rate: 0.0,
    program: [PROGRAM_VERTEX_BLEND, 0.0],
};

/// Admits each family material whose every submesh carries `VertexColour1` and whose second
/// picture decodes, writing the family's glow-table entry into `emissive`. `taken` says, per
/// material, that another reading already rides its third binding or a glow-table entry; such
/// a material, like one that fails the above, is counted and left to the generic reading. The
/// second picture is decoded into `pictures`, the cache the first pictures are bound from, so
/// a file both a first and a second picture is read once. Empty on a PS4 model.
pub(super) fn plan(
    decoded: &psp2::Model,
    ps4: bool,
    taken: &[bool],
    emissive: &mut Vec<Emissive>,
    pictures: &mut Pictures,
    textures: Textures<'_>,
    report: &mut Report,
) -> Plan {
    let mut out = Plan::default();
    for (index, material) in decoded.materials.iter().enumerate() {
        out.blends.push(None);
        let Some(named) = vertex_blend::of(material).filter(|_| !ps4) else {
            continue;
        };
        let weighted = decoded
            .submeshes
            .iter()
            .filter(|s| s.material == Some(index))
            .all(|s| !s.positions.is_empty() && s.vita_vertex_colour1.len() == s.positions.len());
        if !weighted || taken.get(index).copied().unwrap_or(true) {
            crate::mesh::rcs::census::claim(
                index,
                "vertex_blend",
                crate::mesh::rcs::census::Outcome::NotDrawn,
            );
            report.vertex_blend_unread += 1;
            continue;
        }
        let second = pictures
            .entry(named.second.to_string())
            .or_insert_with(|| decode_picture(named.second, &mut *textures));
        let Ok((second, _)) = second.clone() else {
            crate::mesh::rcs::census::claim(
                index,
                "vertex_blend",
                crate::mesh::rcs::census::Outcome::NotDrawn,
            );
            report.vertex_blend_unread += 1;
            continue;
        };
        let at = emissive.iter().position(|e| *e == ENTRY).or_else(|| {
            (emissive.len() + 1 < crate::mesh::rcs::EMISSIVE_LIMIT).then(|| {
                emissive.push(ENTRY);
                emissive.len() - 1
            })
        });
        let Some(slot) = at.and_then(|at| u32::try_from(at + 1).ok()) else {
            crate::mesh::rcs::census::claim(
                index,
                "vertex_blend",
                crate::mesh::rcs::census::Outcome::NotDrawn,
            );
            report.vertex_blend_unread += 1;
            continue;
        };
        out.blends[index] = Some(Blend { slot, second });
        crate::mesh::rcs::census::claim(
            index,
            "vertex_blend",
            crate::mesh::rcs::census::Outcome::Drawn,
        );
        report.vertex_blend_materials += 1;
    }
    out
}
