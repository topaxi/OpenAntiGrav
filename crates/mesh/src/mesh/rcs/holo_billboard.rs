//! HD's holographic billboard, `nr_billboardholographicscanlines.rcsmaterial`
//! in its inline-literal shape: the advert, multiplied by a second layer and
//! put through an overlay combine with a scrolling scanline texture.
//!
//! # What the program does
//!
//! Read off the resolved fragment block (Amphiseum `@0x14e0`, block #3 of
//! `amphiseum/materials/nr_billboardholographicscanlines.rcsmaterial`; the
//! unfogged block #2 of the same file is the same combine with the interpolators
//! renumbered; `scripts/ps3-microcode.py fp-file`). With `t` the engine's
//! `time` (parameter `0x906b67ba`), `adv` the unit-0 tap, `lay` the unit-2 tap
//! and `scan` the unit-1 texture:
//!
//! ```text
//! A     = scan(P)                       TEX R0.yz, f[TC4] unit1   (P = v[2].xy)
//! lay   = unit2(Q.x, Q.y - r2 t)        TEX H0.xyz, R1.zwzz       (Q = v[8].xy)
//! base  = adv(S) * lay                  TEX H4.xyz, f[TC5] unit0  (S = v[8].zw)
//! B     = scan(P.x + r1 t, P.y + r1 t + A.y)                      dependent read
//! out   = 2 base B               where base < 0.5   (SLT, MUL x2 [NE])
//!       = 1 - 2 (1 - base)(1 - B) elsewhere
//! alpha = A.z
//! ```
//!
//! `r1` and `r2` are literals of the program (`0.25` and `5.625` on Amphiseum),
//! read from it here and never typed: the unpatched `MUL` and the
//! negated-register `MAD` whose constant is the second source. The material
//! blends `ONE`/`ONE` (its own authored pair), so `alpha` reaches nothing.
//!
//! # Where the three coordinate pairs come from
//!
//! The paired vertex programs write `TC4.xy = v[2].xy`, `TC4.zw = v[8].xy` and
//! `TC5.xy = v[8].zw`. Slot 8 is the four-component attribute `0x1589348f`,
//! which the chunk's declaration splits into `0xb0528e1e` (`.xy`) and
//! `0x6ca4e3cc` (`.zw`, what `diffuse_texcoord` already returns, so the advert
//! was sampled at the right coordinate before this module); slot 2 is
//! `0x0641512d`. [`uvs`] carries the other two pairs in the vertex's
//! `lightmap_texcoord` and `texcoord2`, as the ice pool already does.
//!
//! # The three shapes of the one program
//!
//! The same combine ships three ways, and all three bind here, each reading
//! its own three numbers (scanline drift rate, second-layer scroll per second,
//! second-layer coordinate scale) and handing them to the shader in the
//! glow-table entry's tint:
//!
//! - **inline literals** (Amphiseum's `nr_` file): the rates are the program's
//!   own `0.25` and `5.625`; the scroll is the negated `MAD`, so it is stored
//!   as `-5.625`, and the scale is the absent multiply, `1`;
//! - **parameters** (`wes_` on every circuit, `nr_` on Modesto Heights): the
//!   rates are material parameters [`DRIFT`], [`SCROLL`] and [`SCALE`], read
//!   from the material record, never typed. The program adds `t * SCROLL` to
//!   the layer's `v` (`MAD`, no negate) and multiplies *both* layer
//!   coordinates by `SCALE` afterwards;
//! - **the DLC copy** (`02_track`, inline `0.25`/`5.625`): the advert tap is
//!   predicated `[NE(xyzx)]`. The condition register it tests is written by
//!   the preceding unit-2 tap (`TEX H2.xyz, R3.zwzz unit2` carries the
//!   condition-write bit, dword 0 bit 8, the one the `SLT` select also
//!   carries), not by the `FENCT`, so the predicate says "skip the advert
//!   where the layer sample is zero in a lane" and the product that follows is
//!   `advert * 0 = 0` there either way. Drawn as the unpredicated combine.
//!   **That reading does not rest on `FENCT` setting the condition register**
//!   (the confidence-60 reading `dc_lightcone` uses); it rests on bit 8 of a
//!   `TEX` meaning what it means on the `SLT`.
//!
//! Every shape reads the same three attributes (see above): the vertex
//! programs of all three write `v[2].xy`, `v[8].xy` and `v[8].zw`, only into
//! different interpolators.
//!
//! # `scanlinebillboard`, the fourth shape
//!
//! `scanlinebillboard` (Talon's Junction, Tech de Ra) and its `_alphaboost`
//! and `_desaturate` siblings run the same overlay on the same three samplers,
//! but every tap reads **one** coordinate pair: the vertex programs write only
//! `v[2]`, attribute `0x6ca4e3cc` (the advert's own pair), into the fragment's
//! coordinate (Talon's `scanlinebillboard.rcsmaterial` vertex blocks `@0x2010`,
//! `@0x53b0`, `@0x6ea0`; fragment blocks `@0x21a0`, `@0x5560`, `@0x70f0`).
//! With `S` that pair:
//!
//! ```text
//! A     = scan(k S)                              MUL R1.zw, R1, [SCAN_SCALE]
//! lay   = unit2(S.x, S.y w - t s)                MUL [LAYER_V] ; MAD -t [LAYER_SCROLL]
//! base  = adv(S) * lay
//! B     = scan(k S.x + d t, k S.y + d t + A.y)   MUL t [SCAN_DRIFT]
//! out   = overlay(base, B) * tint                MAD R2.xyz, H0, [TINT], -fog
//! alpha = A.z (+ ALPHA_BOOST on `_alphaboost`)
//! ```
//!
//! then the fog lerp, `+ f[TC1]` (`f[TC2]` in the `TC5`-packed blocks) and the
//! closing `/2`. The added term is the vertex program's literal `c[208].x` (or
//! `c[206].x`), `0` in every block of the three files, so it adds nothing.
//! `_desaturate` replaces the advert with `mix(adv.r, adv, DESATURATE)`
//! (`ADD H4, H5, -H5.xxxx ; MAD H4, H4, [DESATURATE], H5.xxxx`). [`classify`]
//! knows the shape by its parameter list, and [`route`] lays `(S.x, w S.y)`
//! and `k S` into the two coordinate pairs the shader reads, so `holo.wesl`
//! runs the one combine for every shape.
//!
//! # The two table entries
//!
//! Each bound slot takes two consecutive glow-table entries, the way the ice
//! takes three. The first, which the vertex's material index names, carries
//! `[drift rate, signed layer scroll, layer scale]` in its tint and, in its
//! offset, `1` where the coordinates come from the single pair. The second
//! carries the output tint, the scanline coordinate scale `k` (offset), the
//! layer's `v` scale `w` (scale) and `[alpha boost, desaturate]` (program);
//! the three holographic shapes write `(1, 1, 1)`, `1`, `1` and `[0, 1]`,
//! which change nothing. Neither entry sets `rate`, whose bands other paths
//! read off the same index.

use std::sync::Arc;

use oag_rcs::rcsmaterial::fragment::{Instruction, Program, Source};
use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{Emissive, ModelTexture, TextureSlots, slots};

use super::emissive::EMISSIVE_LIMIT;
use super::skin::decode_texture;
use super::{Report, Textures};

/// `~crc32` of the advert's sampler, unit 0.
const ADVERT: u32 = 0x6f46_9b89;
/// The scanline texture, unit 1.
const SCANLINES: u32 = 0xfad8_b460;
/// The second layer the advert is multiplied by, unit 2.
const SECOND: u32 = 0xdd7e_c609;
/// The engine clock, `time`.
const TIME: u32 = 0x906b_67ba;
/// The declared coordinate pair feeding the second layer: `v[8].xy`.
const SECOND_UV: u32 = 0xb052_8e1e;
/// The declared coordinate pair feeding the scanline texture: `v[2].xy`.
const SCAN_UV: u32 = 0x0641_512d;
/// The second layer's scroll, per second, added to its `v` (`MAD` with `time`).
const SCROLL: u32 = 0x088b_2c6a;
/// The scanline texture's drift rate (`MUL` by `time`).
const DRIFT: u32 = 0x4f2a_ef81;
/// The scale applied to both of the second layer's coordinates.
const SCALE: u32 = 0xef6e_4697;
/// The fog colour, which only the fogged blocks declare.
const FOG: u32 = 0x3dc3_1258;
/// `scanlinebillboard`'s scale of the single pair for the scanline taps.
const SCAN_SCALE: u32 = 0xd09c_054d;
/// `scanlinebillboard`'s scale of the layer's `v`, before the scroll.
const LAYER_V: u32 = 0x5895_bced;
/// `scanlinebillboard`'s layer scroll per second, subtracted from `v`.
const LAYER_SCROLL: u32 = 0x08a1_11e3;
/// `scanlinebillboard`'s scanline drift rate, `MUL` by `time`.
const SCAN_DRIFT: u32 = 0x0942_573b;
/// `scanlinebillboard`'s output tint, the float3 the overlay is multiplied by.
const TINT: u32 = 0x464a_c094;
/// `scanlinebillboard_alphaboost`'s addend to the output alpha.
const ALPHA_BOOST: u32 = 0x90bd_163f;
/// `scanlinebillboard_desaturate`'s weight of the advert against its red channel.
const DESATURATE: u32 = 0xcde7_e5d7;
/// The advert's own coordinate pair, the one `scanlinebillboard` reads for all three taps.
const ADVERT_UV: u32 = 0x6ca4_e3cc;
/// The condition field of an unconditional instruction.
const UNCONDITIONAL: u16 = 0x727;

/// Whether `instruction` carries a literal the file authored rather than one the
/// engine patches over.
fn inline_literal(
    program: &Program,
    declared: &rcsmaterial::Declared,
    i: &Instruction,
) -> Option<f32> {
    let slot = i.const_slot?;
    let patched = declared
        .parameters
        .iter()
        .any(|&h| program.patches(h).any(|s| s == slot));
    (!patched).then(|| i.constant.map(|c| c[0])).flatten()
}

/// The two clock rates of the inline-literal shape, `(scanline, second layer)`,
/// or `None` for a program that is not it.
///
/// The scanline rate is the unpatched `MUL` of a register by a `{c, 0, 0, 0}.xxxx`
/// literal; the second layer's is the `MAD` that subtracts a register times
/// such a literal from the coordinate. Fog and the overlay's own `1` are other
/// literals and neither has that shape.
fn rates(declared: &rcsmaterial::Declared, program: &Program) -> Option<(f32, f32)> {
    let named = |i: &&Instruction, name: &str| i.name() == Some(name) && !i.dst_half;
    let literal_x = |i: &Instruction, at: usize| {
        let lanes = i.constant?;
        (i.swizzles[at] == [0; 4] && lanes[1..].iter().all(|&l| l == 0.0)).then_some(())?;
        inline_literal(program, declared, i)
    };
    let scan = program.instructions.iter().find_map(|i| {
        (named(&i, "MUL") && i.sources[1] == Source::Constant && i.cond == UNCONDITIONAL)
            .then(|| literal_x(i, 1))
            .flatten()
    })?;
    let scroll = program.instructions.iter().find_map(|i| {
        (named(&i, "MAD") && i.negate[0] && i.sources[1] == Source::Constant)
            .then(|| literal_x(i, 1))
            .flatten()
    })?;
    Some((scan, scroll))
}

/// Whether a block declares exactly the advert, scanline and second-layer
/// samplers - the family, whichever program shape it runs.
fn declares_the_three(declared: &rcsmaterial::Declared) -> bool {
    let mut hashes: Vec<u32> = declared.samplers.iter().map(|s| s.0).collect();
    hashes.sort_unstable();
    let mut want = [ADVERT, SCANLINES, SECOND];
    want.sort_unstable();
    hashes == want
}

/// What the shader takes for one slot: the two glow-table entries described
/// in the module doc, and whether the coordinates come from the single pair.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Shape {
    /// Scanline drift rate, signed layer scroll per second, layer scale.
    rates: [f32; 3],
    /// Whether every tap reads the advert's pair (`scanlinebillboard`).
    single_pair: bool,
    /// The colour the overlay is multiplied by.
    tint: [f32; 3],
    /// The scale of the single pair for the scanline taps (`k`).
    scan_scale: f32,
    /// The scale of the layer's `v` before the scroll (`w`).
    layer_v: f32,
    /// Added to the output alpha.
    alpha_boost: f32,
    /// The advert's weight against its own red channel (`1` = the advert).
    desaturate: f32,
}

impl Shape {
    /// A holographic shape: the three rates and nothing else.
    fn holo(rates: [f32; 3]) -> Self {
        Shape {
            rates,
            single_pair: false,
            tint: [1.0; 3],
            scan_scale: 1.0,
            layer_v: 1.0,
            alpha_boost: 0.0,
            desaturate: 1.0,
        }
    }

    /// The two table entries, in order.
    fn entries(&self) -> [Emissive; 2] {
        [
            Emissive {
                tint: self.rates,
                offset: if self.single_pair { 1.0 } else { 0.0 },
                scale: 1.0,
                rate: 0.0,
                program: [0.0; 2],
            },
            Emissive {
                tint: self.tint,
                offset: self.scan_scale,
                scale: self.layer_v,
                rate: 0.0,
                program: [self.alpha_boost, self.desaturate],
            },
        ]
    }
}

/// The `scanlinebillboard` shape, or `None` for a program that does not
/// declare its parameters. The five are required; the alpha boost and the
/// desaturation only on the siblings that declare them.
fn scanline(
    declared: &rcsmaterial::Declared,
    parameters: &[oag_rcs::rcsmodel::material::Parameter],
) -> Option<Shape> {
    const REQUIRED: [u32; 5] = [SCAN_SCALE, LAYER_V, LAYER_SCROLL, SCAN_DRIFT, TINT];
    let allowed =
        |h: &u32| REQUIRED.contains(h) || [TIME, FOG, ALPHA_BOOST, DESATURATE].contains(h);
    if !REQUIRED.iter().all(|h| declared.parameters.contains(h))
        || !declared.parameters.iter().all(allowed)
    {
        return None;
    }
    let value = |hash| parameters.iter().find(|p| p.hash == hash).map(|p| p.value);
    let optional = |hash, absent| {
        if declared.parameters.contains(&hash) {
            value(hash).map(|v| v[0])
        } else {
            Some(absent)
        }
    };
    let tint = value(TINT)?;
    Some(Shape {
        rates: [value(SCAN_DRIFT)?[0], -value(LAYER_SCROLL)?[0], 1.0],
        single_pair: true,
        tint: [tint[0], tint[1], tint[2]],
        scan_scale: value(SCAN_SCALE)?[0],
        layer_v: value(LAYER_V)?[0],
        alpha_boost: optional(ALPHA_BOOST, 0.0)?,
        desaturate: optional(DESATURATE, 1.0)?,
    })
}

/// The shape a block runs, or `None` for a program that is none of the four.
///
/// The select (`SLT` and the predicated `MUL x2`) and `time` are common to all
/// of them; what tells the shapes apart is the parameter list. The three
/// holographic shapes may declare nothing beyond `time`, the fog colour and
/// their three rates; `scanlinebillboard` declares its own five.
fn classify(
    declared: &rcsmaterial::Declared,
    program: &Program,
    parameters: &[oag_rcs::rcsmodel::material::Parameter],
) -> Option<Shape> {
    let has = |name: &str, half: bool| {
        program
            .instructions
            .iter()
            .any(|i| i.name() == Some(name) && i.dst_half == half)
    };
    let select = declared.parameters.contains(&TIME)
        && has("SLT", false)
        && program
            .instructions
            .iter()
            .any(|i| i.name() == Some("MUL") && i.dst_half && i.cond != UNCONDITIONAL);
    if !select {
        return None;
    }
    if let Some(shape) = scanline(declared, parameters) {
        return Some(shape);
    }
    let allowed = |h: &u32| [TIME, FOG, SCROLL, DRIFT, SCALE].contains(h);
    if !declared.parameters.iter().all(allowed) {
        return None;
    }
    if [SCROLL, DRIFT, SCALE]
        .iter()
        .all(|h| declared.parameters.contains(h))
    {
        let value = |hash| {
            parameters
                .iter()
                .find(|p| p.hash == hash)
                .map(|p| p.value[0])
        };
        return Some(Shape::holo([value(DRIFT)?, value(SCROLL)?, value(SCALE)?]));
    }
    let (drift, scroll) = rates(declared, program)?;
    Some(Shape::holo([drift, -scroll, 1.0]))
}

type Pairs = Option<Vec<[f32; 2]>>;

/// The `lightmap_texcoord` and `texcoord2` pairs of a submesh, with a
/// billboard's (`roles` carries [`slots::HOLO_BILLBOARD`]) laid over whatever
/// the caller read: the second layer's in the first, the scanlines' in the
/// second. A `scanlinebillboard` slot (its first entry's offset is `1`) takes
/// both from the advert's pair, `(S.x, w S.y)` and `k S` with `w` and `k` from
/// its second entry. Every other surface, and a chunk that does not declare
/// the pairs, passes through unchanged.
pub(super) fn route(
    roles: u32,
    table: &[Emissive],
    (mesh, model_blob, submesh, stride): (&rcsmodel::Mesh, &[u8], &rcsmodel::SubMesh, usize),
    (lightmap, second): (Pairs, Pairs),
) -> (Pairs, Pairs) {
    if roles & slots::HOLO_BILLBOARD == 0 {
        return (lightmap, second);
    }
    let pair = |hash| {
        mesh.texcoords_by_hash(model_blob, submesh, stride, hash)
            .ok()
    };
    let entries = slots::material_index(roles)
        .checked_sub(1)
        .and_then(|i| table.get(i as usize..i as usize + 2));
    if let Some([first, finish]) = entries
        && first.offset == 1.0
    {
        return match pair(ADVERT_UV) {
            Some(uv) => {
                let (k, w) = (finish.offset, finish.scale);
                let layer = uv.iter().map(|&[u, v]| [u, w * v]).collect();
                let scan = uv.iter().map(|&[u, v]| [k * u, k * v]).collect();
                (Some(layer), Some(scan))
            }
            None => (lightmap, second),
        };
    }
    match (pair(SECOND_UV), pair(SCAN_UV)) {
        (Some(layer), Some(scanlines)) => (Some(layer), Some(scanlines)),
        _ => (lightmap, second),
    }
}

/// Binds every holographic billboard's three textures (advert as the albedo,
/// scanlines as the second, the layer as the third, which is returned), writes
/// [`slots::HOLO_BILLBOARD`] and its glow-table entry, and counts the
/// materials it refuses.
pub(super) fn holo_billboard(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    (skins, seconds): (&mut TextureSlots, &mut TextureSlots),
    table: &mut Vec<Emissive>,
    report: &mut Report,
) -> TextureSlots {
    let mut thirds: TextureSlots = vec![None; model.materials.len()];
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
        if !declares_the_three(&declared) {
            continue;
        }
        let Some(shape) = classify(&declared, &program, &material.parameters) else {
            report.holo_billboard_other_shape += 1;
            continue;
        };
        let path_of = |hash: u32| {
            material
                .samplers
                .iter()
                .find(|(h, p)| *h == hash && p.is_some())
                .and_then(|(_, p)| p.clone())
        };
        let mut load = |hash: u32| {
            let path = path_of(hash)?;
            decoded
                .entry(path.clone())
                .or_insert_with(|| {
                    textures(&format!("/{path}"))
                        .and_then(|bytes| decode_texture(&path, &bytes))
                        .map(Arc::new)
                })
                .clone()
        };
        let (Some(advert), Some(scanlines), Some(second)) =
            (load(ADVERT), load(SCANLINES), load(SECOND))
        else {
            report.holo_billboard_unread += 1;
            continue;
        };
        let pair = shape.entries();
        let index = table.windows(2).position(|seen| seen == pair).or_else(|| {
            (table.len() + 2 < EMISSIVE_LIMIT).then(|| {
                table.extend(pair);
                table.len() - 2
            })
        });
        let Some(index) = index else {
            report.holo_billboard_unread += 1;
            continue;
        };
        skins[slot] = Some(advert);
        seconds[slot] = Some(scanlines);
        thirds[slot] = Some(second);
        if let Some(word) = packed.get_mut(slot) {
            *word &= slots::ROLE_MASK & !slots::ADD_SECOND;
            *word |= slots::HOLO_BILLBOARD;
            *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        }
        report.holo_billboard_bound += 1;
    }
    thirds
}
