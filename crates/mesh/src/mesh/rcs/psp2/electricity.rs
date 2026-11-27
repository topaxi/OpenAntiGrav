//! `electricity.rcsmaterial` on Omega and 2048: the Detonator mine's `Electric_Visualiser`, HD's
//! program.
//!
//! The pixel program is HD's arithmetic, read off Omega's GCN code (`oag_rcs::gcn::electricity`)
//! or 2048's USSE code (`oag_rcs::gxp::electricity`): two taps of the bass and treble picture,
//! one of the pitch picture, two of the electricity picture, the colour tap's `0.3` shift read
//! off the code, with the pictures named by sampler. 2048's program squares the bass tap's and
//! the pitch tap's red itself; the second glow-table entry's scale says so (`1.0`), and the colour
//! tap's square is the scene's own decode on 2048. The draw is
//! `super::super::electricity`'s: the material's three pictures ride the bindings
//! [`super::twin_blend`] gives a material (the bass and treble picture the diffuse, the pitch
//! picture the second, the electricity picture the third), and two consecutive glow-table entries
//! carry the authored values (`docs/ghidra/functions/ps4-omega-eu/detonator-hull.md`, "The
//! visualiser").
//!
//! The vertex program hands the fragment stage `Uv3` as the bass and treble pair, `Uv1_Uv2`'s first
//! pair as `uv1` and its second as `uv2` (minus `BassHit`), and `VertexColour1.w` as the ring's
//! alpha; [`vertex`] routes them as HD's chunk is routed. **Read, not measured live:** no PS4
//! emulator has drawn this ring; the units' gamma nibbles are HD's, and the fog lerp the
//! program ends in is the scene's own fog.

use std::sync::Arc;

use oag_rcs::gcn::{BASS_TREBLE_TEXTURE, ELECTRICITY_TEXTURE, PITCH_TEXTURE};
use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmodel::psp2::{self, SubMesh, material::Material};

use super::twin_blend::{Plan, Twin};
use super::{Report, picture_memo};
use crate::mesh::rcs::Textures;
use crate::mesh::{Emissive, ModelTexture, PROGRAM_ELECTRICITY, PROGRAM_ELECTRICITY_AUDIO};

/// The material parameters the program's constant buffer names.
const COLOUR: &str = "ElectricityColour";
const MASTER_VOLUME: &str = "MasterVolume";
const PITCH_OFFSET: &str = "PitchOffset";
const PITCH_VOLUME: &str = "PitchVolume";
const FACING_POWER: &str = "FacingPower";
const BASS_HIT: &str = "BassHit";
const TREBLE_HIT: &str = "TrebleHit";
/// The `Tempo` the mine binds on both of its branches (`m+0x118 = 1.0`).
const MINE_TEMPO: f32 = 1.0;

/// Whether a material is `electricity.rcsmaterial` by file name.
fn is_electricity(material: &Material) -> bool {
    let stem = material.name.rsplit(['/', '\\']).next().unwrap_or("");
    stem.trim_suffix(".rcsmaterial")
        .eq_ignore_ascii_case("electricity")
}

/// The two glow-table entries from the material's authored values.
fn entries(material: &Material, squared_taps: bool) -> Option<[Emissive; 2]> {
    let value = |name: &str| {
        material
            .param(name_hash(name))
            .and_then(|v| v.first().copied())
    };
    let colour = material.param(name_hash(COLOUR))?;
    let &[r, g, b, ..] = colour.as_slice() else {
        return None;
    };
    Some([
        Emissive {
            tint: [r, g, b],
            offset: value(MASTER_VOLUME)?,
            scale: MINE_TEMPO,
            rate: 0.0,
            program: [PROGRAM_ELECTRICITY, value(FACING_POWER)?],
        },
        Emissive {
            tint: [
                value(PITCH_VOLUME)?,
                value(BASS_HIT).unwrap_or(0.0),
                value(TREBLE_HIT).unwrap_or(0.0),
            ],
            offset: value(PITCH_OFFSET)?,
            scale: if squared_taps { 1.0 } else { 0.0 },
            rate: 0.0,
            program: [PROGRAM_ELECTRICITY_AUDIO, 0.0],
        },
    ])
}

/// Admits each `electricity.rcsmaterial` material whose program reads as the visualiser's, whose
/// three pictures and authored values read and whose every submesh carries the vertex pairs,
/// filing it in `twin` and writing its two glow-table entries. Counted, never guessed past.
pub(super) fn plan(
    decoded: &psp2::Model,
    (twin, emissive): (&mut Plan, &mut Vec<Emissive>),
    textures: Textures<'_>,
    report: &mut Report,
) {
    for (index, material) in decoded.materials.iter().enumerate() {
        if !is_electricity(material) || index >= twin.twins.len() {
            continue;
        }
        let read = (|| {
            let file = textures(&material.name)?;
            let squared_taps = match oag_rcs::gcn::electricity(&file) {
                Some(_) => false,
                None => oag_rcs::gxp::electricity::electricity(&file)?.squared_taps,
            };
            let readable = decoded
                .submeshes
                .iter()
                .filter(|s| s.material == Some(index))
                .all(routable);
            readable.then_some(())?;
            let sampler = |name: &str| {
                material
                    .sampler(name_hash(name))
                    .filter(|path| !path.is_empty())
            };
            let bass_treble = sampler(BASS_TREBLE_TEXTURE.trim_suffix("Texture"))?;
            let pitch = sampler(PITCH_TEXTURE.trim_suffix("Texture"))?;
            let uv_dist = sampler(ELECTRICITY_TEXTURE.trim_suffix("Texture"))?;
            let mut picture = |path: &str| -> Option<Arc<ModelTexture>> {
                picture_memo::picture(path, textures, report)
            };
            let pictures = (picture(pitch)?, picture(uv_dist)?, picture(bass_treble)?);
            Some((
                bass_treble.to_string(),
                pictures,
                entries(material, squared_taps)?,
            ))
        })();
        let Some((first, (second, wipe, _), pair)) = read else {
            crate::mesh::rcs::census::claim(
                index,
                "electricity",
                crate::mesh::rcs::census::Outcome::NotDrawn,
            );
            report.electricity_unread += 1;
            continue;
        };
        let at = emissive.windows(2).position(|w| w == pair).or_else(|| {
            (emissive.len() + 2 < crate::mesh::rcs::EMISSIVE_LIMIT).then(|| {
                emissive.extend(pair);
                emissive.len() - 2
            })
        });
        let Some(slot) = at.and_then(|at| u32::try_from(at + 1).ok()) else {
            crate::mesh::rcs::census::claim(
                index,
                "electricity",
                crate::mesh::rcs::census::Outcome::NotDrawn,
            );
            report.electricity_unread += 1;
            continue;
        };
        twin.admit(
            index,
            Twin {
                slot,
                first,
                second,
                wipe,
                bars: None,
            },
        );
        crate::mesh::rcs::census::claim(
            index,
            "electricity",
            crate::mesh::rcs::census::Outcome::Drawn,
        );
        report.electricity_materials += 1;
    }
}

/// `VertexColour1`, whose alpha is the ring's: the Vita's own array where the declaration names it
/// there, else the PS4's.
fn vertex_colour1(submesh: &SubMesh) -> &[[u8; 4]] {
    if submesh.vita_vertex_colour1.is_empty() {
        &submesh.colours
    } else {
        &submesh.vita_vertex_colour1
    }
}

/// `Uv3`, the bass and treble pair: its own array where the declaration names it (2048's also
/// names a `Uv1`, which is `Uv1_Uv2`'s second pair), else the diffuse pick.
fn pair(submesh: &SubMesh) -> &[[f32; 2]] {
    if submesh.uv3_texcoords.is_empty() {
        &submesh.texcoords
    } else {
        &submesh.uv3_texcoords
    }
}

/// Whether a submesh carries every attribute the ring's vertex program reads.
fn routable(submesh: &SubMesh) -> bool {
    let n = submesh.positions.len();
    n > 0
        && pair(submesh).len() == n
        && submesh.packed_texcoords.len() == n
        && vertex_colour1(submesh).len() == n
}

/// `(texcoord, lightmap_texcoord, texcoord2, sun_mask)`.
pub(super) type RingVertex = ([f32; 2], [f32; 2], [f32; 2], f32);

/// A vertex of the ring: `(texcoord, lightmap_texcoord, texcoord2, sun_mask)` as the shader reads
/// them (`Uv3`, `uv1`, `uv2`, `VertexColour1.w`), for a vertex of a material this module
/// admitted (its first entry is in `emissive` at `slot`).
pub(super) fn vertex(
    twin: Option<&Twin>,
    emissive: &[Emissive],
    submesh: &SubMesh,
    i: usize,
) -> Option<RingVertex> {
    let slot = usize::try_from(twin?.slot).ok()?.checked_sub(1)?;
    (emissive.get(slot)?.program[0] == PROGRAM_ELECTRICITY).then_some(())?;
    let &[a, b, c, d] = submesh.packed_texcoords.get(i)?;
    let &[.., alpha] = vertex_colour1(submesh).get(i)?;
    Some((
        *pair(submesh).get(i)?,
        [a, b],
        [c, d],
        f32::from(alpha) / 255.0,
    ))
}
