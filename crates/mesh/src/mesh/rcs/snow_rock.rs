//! HD's Sebenco snow-capped rock: `sebenco_snow`, snow and rock mixed by a painted mask.
//!
//! # What the program does
//!
//! Resolved fragment block #21 `@0xe0d0` of `10_sebenco_climb/materials/sebenco_snow.rcsmaterial`
//! (`scripts/ps3-microcode.py fp-file`), its vertex block #15 `@0xdef0` writing `TC4` (both
//! coordinate pairs), `TC5` (the atlas pair), `TC1 = eye - position` and the tangent frame
//! (`TC3` the tangent, `TC0 = (N0 x T) w`, `TC2 = N0`). Six units:
//!
//! ```text
//! m   = tex(unit 1, TC4.zw).x                      the mask, second pair       @0x0b
//! n   = lerp(2 tex(unit 2, uv) - 1,                the snow normal             @0x01..@0x0c
//!            2 tex(unit 5, uv k) - 1, m)           the rock normal, k 0xe7b6ae63
//! N   = normalize(n.x T + n.y B + n.z N0)                                      @0x0d..@0x1e
//! s   = lerp(pale, tex(unit 0, uv).rgb, N.V)       the snow, pale 0x9dbd80db   @0x28..@0x2c
//! d   = lerp(s, tex(unit 4, uv k).rgb, m)          the rock                    @0x36..@0x3f
//! e   = lerp(0xbd4582d3, 0x1017e83c, m)            the specular exponent       @0x3d
//! c   = lerp(snow.a 0x06f7d6c7, rock.a 0x74a4fe88, m)  the specular colour     @0x41..@0x43
//! out = fog((atlas light) d + (N.H)^e sat(8 N.L) atlas.a sun c) / 2            @0x5b..@0x5d
//! ```
//!
//! **The rock is mixed in by the mask, not by a slope.** The factor at `@0x3f` is `H3.x`, the
//! mask texel `@0x0b` wrote, which nothing overwrites before it (the RSX's `H3` is the upper half
//! of `R1`, and no instruction between writes `R1.z`); `R3.x` at `@0x24` is the specular `N.H`.
//! The bumped normal shades the snow toward `pale` by `N.V` and lights both layers.
//!
//! Bound: the snow as the albedo, the atlas where the generic reading put it, the mask third
//! (at the second pair, `texcoord2`), the snow normal fourth, the rock fifth, the rock normal
//! sixth. `shade.wesl` reads them under a glow-table entry of mode [`PROGRAM_SNOW`] and the two
//! after it. The five sampled colour units are decoded like every colour unit read live so
//! far: **chosen, not measured** for these units, whose address words were not read; the
//! normal units are read raw, as the hulls' are.

use std::sync::Arc;

use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{Emissive, ModelTexture, PROGRAM_SNOW, TextureSlots, slots};

use super::emissive::EMISSIVE_LIMIT;
use super::skin::decode_texture;
use super::{Report, Textures};

/// The snow (`and_snow3alpha`), unit 0.
const SNOW: u32 = 0xc97c_5a4c;
/// The mask, unit 1, read at the second coordinate pair.
const MASK: u32 = 0x40db_67c1;
/// The snow's normal map, unit 2.
const SNOW_NORMAL: u32 = 0x67a3_0ab1;
/// The rock, unit 4.
const ROCK: u32 = 0xc8b6_445d;
/// The rock's normal map, unit 5.
const ROCK_NORMAL: u32 = 0x85ba_9c63;
/// What the rock's coordinate is the diffuse one scaled by.
const ROCK_SCALE: u32 = 0xe7b6_ae63;
/// The colour the snow is shaded toward where `N.V` falls.
const PALE: u32 = 0x9dbd_80db;
/// The snow's and the rock's specular exponents.
const SNOW_EXPONENT: u32 = 0xbd45_82d3;
const ROCK_EXPONENT: u32 = 0x1017_e83c;
/// The snow's and the rock's specular colours, each times its picture's alpha.
const SNOW_SPECULAR: u32 = 0x06f7_d6c7;
const ROCK_SPECULAR: u32 = 0x74a4_fe88;

/// Whether a resolved program is this one, by what it declares.
fn is_snow_rock(declared: &rcsmaterial::Declared) -> bool {
    let sampler = |h: u32| declared.samplers.iter().any(|&(s, _)| s == h);
    [SNOW, MASK, SNOW_NORMAL, ROCK, ROCK_NORMAL]
        .into_iter()
        .all(sampler)
        && [
            ROCK_SCALE,
            PALE,
            SNOW_EXPONENT,
            ROCK_EXPONENT,
            SNOW_SPECULAR,
            ROCK_SPECULAR,
        ]
        .iter()
        .all(|h| declared.parameters.contains(h))
}

/// What [`snow_rock`] binds, per material slot: the albedo it replaces, then the third to sixth
/// bindings, and whether the slot's chunks carry their tangent.
pub(super) struct Bound {
    pub(super) mask: TextureSlots,
    pub(super) snow_normal: TextureSlots,
    pub(super) rock: TextureSlots,
    pub(super) rock_normal: TextureSlots,
    pub(super) tangent: Vec<bool>,
}

/// Binds each snow-capped rock material's six pictures, replaces its albedo with the snow, and
/// writes its three glow-table entries: `[pale, snow exponent, rock scale, mode, rock
/// exponent]`, the snow's specular colour, the rock's.
pub(super) fn snow_rock(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    skins: &mut TextureSlots,
    table: &mut Vec<Emissive>,
    report: &mut Report,
) -> Bound {
    let count = model.materials.len();
    let mut bound = Bound {
        mask: vec![None; count],
        snow_normal: vec![None; count],
        rock: vec![None; count],
        rock_normal: vec![None; count],
        tangent: vec![false; count],
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
        let Some(declared) = rcsmaterial::Declared::parse(&blob, variant.fragment.offset) else {
            continue;
        };
        if !is_snow_rock(&declared) {
            continue;
        }
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        // The vertex-lit variant (block #5 `@0x38f0`, the same law with no atlas) reaches here
        // with an additive glow the generic reading gave its unit 1, which is the mask: that
        // reading is replaced. Any other family's index is kept.
        if slots::material_index(*word) != 0 && *word & slots::ADD_SECOND == 0 {
            report.snow_rock_unread += 1;
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
        let pictures = (
            picture(SNOW),
            picture(MASK),
            picture(SNOW_NORMAL),
            picture(ROCK),
            picture(ROCK_NORMAL),
        );
        let value = |hash: u32| {
            material
                .parameters
                .iter()
                .find(|p| p.hash == hash)
                .map(|p| p.value)
        };
        let (
            (Some(snow), Some(mask), Some(snow_normal), Some(rock), Some(rock_normal)),
            Some(scale),
            Some(pale),
            Some(snow_exponent),
            Some(rock_exponent),
            Some(snow_specular),
            Some(rock_specular),
        ) = (
            pictures,
            value(ROCK_SCALE),
            value(PALE),
            value(SNOW_EXPONENT),
            value(ROCK_EXPONENT),
            value(SNOW_SPECULAR),
            value(ROCK_SPECULAR),
        )
        else {
            report.snow_rock_unread += 1;
            continue;
        };
        let colour = |tint: [f32; 4]| Emissive {
            tint: [tint[0], tint[1], tint[2]],
            offset: 0.0,
            scale: 0.0,
            rate: 0.0,
            program: [0.0; 2],
        };
        let entries = [
            Emissive {
                tint: [pale[0], pale[1], pale[2]],
                offset: snow_exponent[0],
                scale: scale[0],
                rate: 0.0,
                program: [PROGRAM_SNOW, rock_exponent[0]],
            },
            colour(snow_specular),
            colour(rock_specular),
        ];
        let first = table
            .windows(3)
            .position(|w| w == entries.as_slice())
            .or_else(|| {
                (table.len() + 3 < EMISSIVE_LIMIT).then(|| {
                    table.extend(entries);
                    table.len() - 3
                })
            });
        let Some(first) = first else {
            report.snow_rock_unread += 1;
            continue;
        };
        skins[slot] = Some(snow);
        bound.mask[slot] = Some(mask);
        bound.snow_normal[slot] = Some(snow_normal);
        bound.rock[slot] = Some(rock);
        bound.rock_normal[slot] = Some(rock_normal);
        bound.tangent[slot] = true;
        *word &= slots::ROLE_MASK & !slots::ADD_SECOND;
        *word |= u32::try_from(first + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        report.snow_rock_bound += 1;
    }
    bound
}
