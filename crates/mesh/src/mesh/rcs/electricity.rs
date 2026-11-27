//! HD's `electricity.rcsmaterial`: the Detonator mine's `Electric_Visualiser`, a camera-facing
//! ring of music-driven lightning.
//!
//! # What the program does
//!
//! Fragment block #1 (lit, unfogged; the fogged block #2 ends in the fog lerp and the final
//! `/2`) of `data/weapons/materials/electricity.rcsmaterial`, and its vertex block #1
//! (`scripts/ps3-microcode.py fp-file`, `vp-file`). The vertex program hands over
//! `TC0.xyz = eye - position`, `TC1.xyz = normal`, `TC2.w = VertexColour1.w`, `TC3.xy = v[3]`
//! (`pair`), `TC3.zw = v[8].xy` (`uv1`), `TC0.w = v[8].z`, `TC1.w = v[8].w - BassHit` (`uv2`)
//! and `TC3.y -= TrebleHit`. With `t` the engine clock (`0x906b67ba`), `bt` unit 0
//! (`electricity_bass_treble_hits`), `pit` unit 1 (`electricity_pitch`) and `uvd` unit 2
//! (`electricity_uv_dist`):
//!
//! ```text
//! n0   = bt(uv2.x, uv2.y - BassHit).r                  @0x0a  TEX R0.x, R3 unit0
//! n1   = bt(pair.x, pair.y - TrebleHit).a              @0x0c  TEX R0.w, f[TC3] unit0
//! s    = (uv1.y + t) + n0 + n1                         @0x03, @0x0d, @0x0e
//! d    = uvd(uv1.x, Tempo s).a - n0 - n1               @0x0f..@0x16
//! m    = (pit(uv1.x, PitchOffset).r PitchVolume + d) MasterVolume   @0x15, @0x19, @0x1d
//! c    = uvd(uv1.x, uv1.y + 0.3 m).rgb VertexColour1.w             @0x1f, @0x23, @0x24
//! over = c < 0.5 ? 2 c E : 1 - 2 (1 - c)(1 - E)        E = ElectricityColour   @0x26..@0x32
//! q    = |N.V|^P                                       @0x25, @0x2a, @0x2b, @0x31
//! out  = (over q, VertexColour1.w q)
//! ```
//!
//! The material authors `P` (`0x5cfb25e3`) as 1.0. The mine writes `ElectricityColour`,
//! `MasterVolume`, `PitchOffset`, `PitchVolume`, `BassHit`, `TrebleHit` and `Tempo` per
//! instance each tick (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-material.md`); here
//! they ride two glow-table entries, the values the caller rewrites each frame
//! ([`PROGRAM_ELECTRICITY`] holds `E` and `MasterVolume`, [`PROGRAM_ELECTRICITY_AUDIO`] the
//! rest), and `shade.wesl` reads them.
//!
//! Matched by what the program reads, never by the model's name. **Read, not measured live:**
//! the units' gamma nibbles; the colour tap is decoded as every colour tap on this path is
//! (chosen, not measured).

use std::sync::Arc;

use oag_rcs::rcsmaterial::{self, fragment::Program};
use oag_rcs::rcsmodel;

use crate::mesh::{
    Emissive, ModelTexture, PROGRAM_ELECTRICITY, PROGRAM_ELECTRICITY_AUDIO, TextureSlots, slots,
};

use super::emissive::EMISSIVE_LIMIT;
use super::skin::decode_texture;
use super::{Report, Textures};

/// `~crc32("ElectricityColour")`.
pub(super) const COLOUR: u32 = 0xce57_6a6e;
/// `~crc32("MasterVolume")`.
pub(super) const MASTER_VOLUME: u32 = 0xe785_6ca2;
/// `~crc32("PitchOffset")`.
pub(super) const PITCH_OFFSET: u32 = 0x6522_e5cb;
/// `~crc32("PitchVolume")`.
pub(super) const PITCH_VOLUME: u32 = 0x85b2_e2c5;
/// `~crc32("Tempo")`.
pub(super) const TEMPO: u32 = 0x4a8b_3b94;
/// `~crc32("BassHit")`.
pub(super) const BASS_HIT: u32 = 0x2889_b4f0;
/// `~crc32("TrebleHit")`.
pub(super) const TREBLE_HIT: u32 = 0x896b_841b;
/// The exponent the facing term `|N.V|` is raised to (`0x5cfb25e3`); the material authors 1.0.
const POWER: u32 = 0x5cfb_25e3;
/// Unit 0, `electricity_bass_treble_hits`.
const BASS_TREBLE: u32 = 0x15f4_ded4;
/// Unit 1, `electricity_pitch`.
const PITCH: u32 = 0x19a1_6e22;
/// Unit 2, `electricity_uv_dist`.
const UV_DIST: u32 = 0x76dc_ec15;

/// The `Tempo` the mine binds on both of its branches (`m+0x118 = 1.0`).
const MINE_TEMPO: f32 = 1.0;

/// Whether the program is this one: its three samplers and the parameters only it patches.
fn is_electricity(declared: &rcsmaterial::Declared) -> bool {
    let mut samplers: Vec<u32> = declared.samplers.iter().map(|s| s.0).collect();
    samplers.sort_unstable();
    let mut want = [BASS_TREBLE, PITCH, UV_DIST];
    want.sort_unstable();
    samplers == want
        && [
            COLOUR,
            MASTER_VOLUME,
            PITCH_OFFSET,
            PITCH_VOLUME,
            TEMPO,
            POWER,
        ]
        .iter()
        .all(|h| declared.parameters.contains(h))
}

/// The glow-table entries of a material with this program, from its authored values.
fn entries(parameters: &[rcsmodel::material::Parameter]) -> Option<[Emissive; 2]> {
    let value = |hash: u32| {
        parameters
            .iter()
            .find(|p| p.hash == hash)
            .map(|p| p.value[0])
    };
    let colour = parameters.iter().find(|p| p.hash == COLOUR)?.value;
    Some([
        Emissive {
            tint: [colour[0], colour[1], colour[2]],
            offset: value(MASTER_VOLUME)?,
            scale: MINE_TEMPO,
            rate: 0.0,
            program: [PROGRAM_ELECTRICITY, value(POWER)?],
        },
        Emissive {
            tint: [
                value(PITCH_VOLUME)?,
                value(BASS_HIT).unwrap_or(0.0),
                value(TREBLE_HIT).unwrap_or(0.0),
            ],
            offset: value(PITCH_OFFSET)?,
            scale: 0.0,
            rate: 0.0,
            program: [PROGRAM_ELECTRICITY_AUDIO, 0.0],
        },
    ])
}

/// Whether `roles` names an entry of [`PROGRAM_ELECTRICITY`] in `table`: the vertices of such
/// a chunk are read through [`route`].
pub(super) fn is_bound(roles: u32, table: &[Emissive]) -> bool {
    slots::material_index(roles)
        .checked_sub(1)
        .and_then(|i| table.get(i as usize))
        .is_some_and(|entry| entry.program[0] == PROGRAM_ELECTRICITY)
}

/// The vertex inputs of an electricity chunk in the pairs the shader reads: the diffuse pair is
/// `v[3]`, the second set (`lightmap_texcoord`) `Uv1`, the third (`texcoords2`) `Uv2`, and the
/// vertex alpha rides the baked light's mask.
pub(super) type Routed = (Vec<[f32; 2]>, Vec<[f32; 2]>, Vec<[f32; 2]>, Vec<[f32; 4]>);

/// [`Routed`] for a chunk that is bound to this program, else `None`.
pub(super) fn route(
    roles: u32,
    table: &[Emissive],
    (mesh, model_blob, submesh, stride): (&rcsmodel::Mesh, &[u8], &rcsmodel::SubMesh, usize),
) -> Option<Routed> {
    if !is_bound(roles, table) {
        return None;
    }
    let vertices = mesh.inline_electricity(model_blob, submesh, stride).ok()?;
    Some((
        vertices.iter().map(|v| v.pair).collect(),
        vertices.iter().map(|v| v.uv1).collect(),
        vertices.iter().map(|v| v.uv2).collect(),
        vertices.iter().map(|v| [0.0, 0.0, 0.0, v.alpha]).collect(),
    ))
}

/// Binds each electricity material's three pictures (unit 0 as the albedo, unit 1 second,
/// unit 2 third, which is returned) and gives it two consecutive glow-table entries.
pub(super) fn electricity(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    (skins, seconds): (&mut TextureSlots, &mut TextureSlots),
    table: &mut Vec<Emissive>,
    report: &mut Report,
) -> TextureSlots {
    let mut thirds: TextureSlots = vec![None; model.materials.len()];
    for (slot, material) in model.materials.iter().enumerate() {
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(blob) = textures(&format!("/{}", material.name)) else {
            continue;
        };
        let (Some(declared), Some(_)) = (
            rcsmaterial::Declared::parse(&blob, variant.fragment.offset),
            Program::parse(&blob, variant.fragment.offset),
        ) else {
            continue;
        };
        if !is_electricity(&declared) {
            continue;
        }
        let mut picture = |hash: u32| -> Option<Arc<ModelTexture>> {
            let path = material
                .samplers
                .iter()
                .find(|(h, p)| *h == hash && p.is_some())
                .and_then(|(_, p)| p.clone())?;
            textures(&format!("/{path}"))
                .and_then(|bytes| decode_texture(&path, &bytes))
                .map(Arc::new)
        };
        let (Some(unit0), Some(unit1), Some(unit2), Some(pair), Some(word)) = (
            picture(BASS_TREBLE),
            picture(PITCH),
            picture(UV_DIST),
            entries(&material.parameters),
            packed.get_mut(slot),
        ) else {
            super::census::claim(slot, "electricity", super::census::Outcome::NotDrawn);
            report.electricity_unread += 1;
            continue;
        };
        let index = table.windows(2).position(|w| w == pair).or_else(|| {
            (table.len() + 2 < EMISSIVE_LIMIT).then(|| {
                table.extend(pair);
                table.len() - 2
            })
        });
        let Some(index) = index else {
            super::census::claim(slot, "electricity", super::census::Outcome::NotDrawn);
            report.electricity_unread += 1;
            continue;
        };
        skins[slot] = Some(unit0);
        seconds[slot] = Some(unit1);
        thirds[slot] = Some(unit2);
        *word &= slots::ROLE_MASK
            & !(slots::ADD_SECOND
                | slots::SECOND_IS_LIGHTMAP
                | slots::ALBEDO_FROM_SECOND
                | slots::ALPHA_FROM_SECOND);
        *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        super::census::claim(slot, "electricity", super::census::Outcome::Drawn);
        report.electricity_bound += 1;
    }
    thirds
}
