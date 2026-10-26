//! HD's `nr_twinblend`: two pictures cross-faded by a scrolling ball wipe, striped by a bar
//! texture.
//!
//! # What the program does
//!
//! Resolved fragment block #3 `@0x1430` of `tech_de_ra/materials/nr_twinblend.rcsmaterial`
//! (`scripts/ps3-microcode.py fp-file`) and its vertex block #3 `@0x1240` (`vp-file`). Four
//! units, every tap at a pair the vertex program lays from the chunk's own attributes:
//!
//! ```text
//! TC4.x = rate * time + v[8].z    MOV R0.w, c[207].y ; MAD o[TC4].x, R0.w, c[208].x, v[8].z
//! TC4.y = v[8].w                  the wipe's coordinate (`rate` the block's literal, 1.875)
//! TC4.zw = v[2].xy                both pictures' coordinate
//! TC5.xy = v[8].xy                the bars' coordinate
//!
//! w   = (1 - BlendTexture(TC4.xy).x)^4         TEX unit2 ; ADD ; LG2 x4 ; EX2
//! c   = lerp(diffuseTexture1(TC4.zw),          TEX unit0, unit1 ; ADD ; MAD
//!            diffuseTexture2(TC4.zw), w)
//! out = fog(bars(TC5) * c) / 2                 TEX unit3 ; MAD ; EX2_SAT ; MAD ; ADD /2
//! ```
//!
//! The closing `ADD /2` adds `f[TC1]`, which this vertex block writes as `c[207].x = 0`, and
//! halves; the `/2` is the one the emissive path's output scale already applies. The bar tap
//! is predicated `[NE(xyzx)]` on the condition the lerp's `MAD` writes: where the lerp is zero
//! the product is zero either way, so it is drawn unpredicated. The alpha lane is the literal
//! `MOV H0.w, {0}`, so the surface stamps no glow mask (`super::glow`, unchanged).
//!
//! On Tech de Ra the pictures are `ag_square01cl_glow` and `ag_square02cl_glow` (the red and
//! the orange AG square), the wipe `animscaledballs` (a `512x16` strip of growing discs), the
//! bars `glowbars_noisy`. The generic reading drew the wipe as the picture and added the bars,
//! a near-white banner where the original's is red.
//!
//! **Read, not measured live:** the units' gamma nibbles. The three colour taps are decoded
//! and the wipe read raw, as the ice pool and the snow-capped rock read their colour units
//! and masks: **chosen, not measured**. The rate is the vertex block's own literal, read by
//! [`oag_rcs::rcsmaterial::vertex::Program::clock_rate`], never typed.
//!
//! Bound: the first picture as the albedo, the second where the generic reading put the
//! second texture, the wipe third (at `lightmap_texcoord`), the bars fourth (at
//! `texcoord2`); the rate in a glow-table entry of mode [`PROGRAM_TWIN_BLEND`].

use std::sync::Arc;

use oag_rcs::rcsmaterial::fragment::Program;
use oag_rcs::rcsmaterial::vertex;
use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{Emissive, ModelTexture, PROGRAM_TWIN_BLEND, TextureSlots, slots};

use super::emissive::EMISSIVE_LIMIT;
use super::skin::decode_texture;
use super::{Report, Textures};

/// `~crc32("BlendTexture")`, the wipe, unit 2.
pub(in crate::mesh::rcs) const BLEND: u32 = 0x25c5_c4d3;
/// `~crc32("diffuseTexture1")`, unit 0.
pub(in crate::mesh::rcs) const FIRST: u32 = 0x8e62_40ce;
/// `~crc32("diffuseTexture2")`, unit 1.
pub(in crate::mesh::rcs) const SECOND: u32 = 0x176b_1174;
/// The bars, unit 3 (no preimage).
pub(in crate::mesh::rcs) const BARS: u32 = 0xd284_858e;
/// The fog colour, which only the fogged blocks declare.
const FOG: u32 = 0x3dc3_1258;
/// `~crc32("time")`, the engine clock the vertex block multiplies by its literal.
const TIME: u32 = 0x906b_67ba;
/// The four-lane attribute the vertex block reads as `v[8]`: the bars' pair in `.xy`, the
/// wipe's in `.zw`.
const PAIRS: u32 = 0x0e7e_a5a8;
/// The chunk declaration's name for `v[8].xy`, the bars' pair (offset 14 of the 22-byte vertex).
const BARS_UV: u32 = 0x7d16_4d3e;
/// The chunk declaration's name for `v[8].zw`, the wipe's pair (offset 18).
const WIPE_UV: u32 = 0x81bd_f44d;
/// `v[2]`, both pictures' pair (offset 10).
const PICTURE_UV: u32 = 0xa276_2127;
/// The fragment `scale` code of `x4` and of `/2`.
const SCALE_X4: u8 = 2;
const SCALE_HALF: u8 = 5;

/// Whether a resolved fragment block is the fogged unlit combine: exactly the four samplers,
/// nothing declared but the fog colour, the `LG2 x4` of the wipe's fourth power and a `/2`
/// (the closing `ADD /2` on Tech de Ra, the fog's `MAD /2` before `MOV H0.w` in the `DATA02`
/// circuits' copy, block #3 `@0x13e0`, the same law with its registers renumbered). The unfogged block #2 has no `/2` and the lit blocks declare the light; both
/// are counted, not bound.
fn is_twin_blend(declared: &rcsmaterial::Declared, program: &Program) -> bool {
    let mut hashes: Vec<u32> = declared.samplers.iter().map(|s| s.0).collect();
    hashes.sort_unstable();
    let mut want = [BLEND, FIRST, SECOND, BARS];
    want.sort_unstable();
    hashes == want
        && declared.parameters.iter().all(|&h| h == FOG)
        && program
            .instructions
            .iter()
            .any(|i| i.name() == Some("LG2") && i.scale == SCALE_X4)
        && program.instructions.iter().any(|i| i.scale == SCALE_HALF)
}

/// Whether a block declares the family's four samplers, whatever program it runs.
fn declares_the_four(declared: &rcsmaterial::Declared) -> bool {
    [BLEND, FIRST, SECOND, BARS]
        .iter()
        .all(|h| declared.samplers.iter().any(|&(s, _)| s == *h))
}

/// The wipe's scroll rate: the vertex block's literal times `time`, added to `v[8].z`. Which
/// interpolator lane carries it differs by copy (`o[TC4].x` on Tech de Ra, `o[TC0].w` in the
/// `DATA02` circuits'), and the fragment block reads it back from there either way.
fn rate(blob: &[u8], variant: rcsmaterial::Variant) -> Option<f32> {
    let declared = rcsmaterial::Declared::parse(blob, variant.vertex.offset)?;
    let (_, register, _) = declared
        .parameter_patches
        .iter()
        .find(|(h, ..)| *h == TIME)?;
    let program = vertex::Program::of(blob, variant.vertex)?;
    let read = program.clock_rate(u32::from(*register).checked_sub(256)?, PAIRS)?;
    (read.from == 2).then_some(read.rate)
}

/// What [`twin_blend`] binds third and fourth, per material slot.
pub(super) struct Bound {
    pub(super) wipe: TextureSlots,
    pub(super) bars: TextureSlots,
}

/// Binds each `nr_twinblend` slot's four pictures, clears the generic additive second
/// texture, and writes its glow-table entry `[rate, 0, 0]` of mode [`PROGRAM_TWIN_BLEND`].
pub(super) fn twin_blend(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    (skins, seconds): (&mut TextureSlots, &mut TextureSlots),
    table: &mut Vec<Emissive>,
    report: &mut Report,
) -> Bound {
    let count = model.materials.len();
    let mut bound = Bound {
        wipe: vec![None; count],
        bars: vec![None; count],
    };
    let mut decoded: std::collections::HashMap<String, Option<Arc<ModelTexture>>> =
        Default::default();
    for (slot, material) in model.materials.iter().enumerate() {
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(blob) = textures(&format!("/{}", material.name)) else {
            continue;
        };
        let (Some(declared), Some(program)) = (
            rcsmaterial::Declared::parse(&blob, variant.fragment.offset),
            Program::parse(&blob, variant.fragment.offset),
        ) else {
            continue;
        };
        if !declares_the_four(&declared) {
            continue;
        }
        if !is_twin_blend(&declared, &program) {
            report.twin_blend_other_shape += 1;
            continue;
        }
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        // The generic reading gives this slot an additive glow off its second texture; any
        // other family's index is kept.
        if slots::material_index(*word) != 0 && *word & slots::ADD_SECOND == 0 {
            report.twin_blend_unread += 1;
            continue;
        }
        let mut picture = |hash: u32| {
            let path = material
                .samplers
                .iter()
                .find(|(h, p)| *h == hash && p.is_some())
                .and_then(|(_, p)| p.clone())?;
            decoded
                .entry(path.clone())
                .or_insert_with(|| {
                    textures(&format!("/{path}"))
                        .and_then(|bytes| decode_texture(&path, &bytes))
                        .map(Arc::new)
                })
                .clone()
        };
        let (Some(first), Some(second), Some(wipe), Some(bars), Some(rate)) = (
            picture(FIRST),
            picture(SECOND),
            picture(BLEND),
            picture(BARS),
            rate(&blob, variant),
        ) else {
            report.twin_blend_unread += 1;
            continue;
        };
        let entry = Emissive {
            tint: [rate, 0.0, 0.0],
            offset: 0.0,
            scale: 0.0,
            rate: 0.0,
            program: [PROGRAM_TWIN_BLEND, 0.0],
        };
        let index = table.iter().position(|e| *e == entry).or_else(|| {
            (table.len() + 1 < EMISSIVE_LIMIT).then(|| {
                table.push(entry);
                table.len() - 1
            })
        });
        let Some(index) = index else {
            report.twin_blend_unread += 1;
            continue;
        };
        skins[slot] = Some(first);
        seconds[slot] = Some(second);
        bound.wipe[slot] = Some(wipe);
        bound.bars[slot] = Some(bars);
        // The picture is the first texture, its colour and coverage: the combine replaces it.
        *word &= slots::ROLE_MASK
            & !(slots::ADD_SECOND
                | slots::SECOND_IS_LIGHTMAP
                | slots::ALBEDO_FROM_SECOND
                | slots::ALPHA_FROM_SECOND);
        *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        report.twin_blend_bound += 1;
    }
    bound
}

type Pairs = Option<Vec<[f32; 2]>>;

/// A twin-blend chunk's three coordinate pairs, `(picture, wipe, bars)`, read by the names
/// its declaration gives them; every other surface, or a chunk missing one, passes through.
pub(super) fn route(
    roles: u32,
    table: &[Emissive],
    (mesh, model_blob, submesh, stride): (&rcsmodel::Mesh, &[u8], &rcsmodel::SubMesh, usize),
    (texcoords, lightmap, second): (Pairs, Pairs, Pairs),
) -> (Pairs, Pairs, Pairs) {
    let index = slots::material_index(roles);
    let is_twin = index != 0
        && table
            .get(index as usize - 1)
            .is_some_and(|e| e.program[0] == PROGRAM_TWIN_BLEND);
    if !is_twin {
        return (texcoords, lightmap, second);
    }
    let pair = |hash| {
        mesh.texcoords_by_hash(model_blob, submesh, stride, hash)
            .ok()
    };
    match (pair(PICTURE_UV), pair(WIPE_UV), pair(BARS_UV)) {
        (Some(picture), Some(wipe), Some(bars)) => (Some(picture), Some(wipe), Some(bars)),
        _ => (texcoords, lightmap, second),
    }
}
