//! HD's `dc_hologramwithstatic`: the start gantry's hologram board on Amphiseum, an
//! advert and a gradient of "static" scrolling past each other, blended by a scanline mask.
//!
//! # What the program does
//!
//! Resolved fragment block #3 `@0x1530` of
//! `amphiseum/materials/dc_hologramwithstatic.rcsmaterial` (`scripts/ps3-microcode.py
//! fp-file`; block #4 is the same law with its interpolators and units renumbered, block #2
//! the unfogged one) and its vertex block #3 `@0x13a0` (`vp-file`), which writes the one
//! coordinate pair `TC4.xy = v[2].xy` (the diffuse pair) and `TC1.xyz = c[208].x = 0`. With
//! `t` the engine clock (`0x906b67ba`), `(u, v)` that pair, `S`, `V` and `K` the material's
//! `0x31182e0d`, `0xb6573513` and `0xeedee991`, and `scan`, `advert`, `static` the units 0, 1
//! and 2:
//!
//! ```text
//! c     = advert(u + S t, v) + static(u, v + V t)       TEX unit1, unit2 ; ADD
//! out   = fog(c) / 2                                    EX2_SAT ; MAD ; ADD /2 (TC1 = 0)
//! alpha = scan(u, v).x * K                              TEX H2.x, f[TC4] unit0 ; MUL
//! ```
//!
//! The material blends `SRC_ALPHA`/`ONE` (its authored pair), so the board adds its colour
//! weighted by the scanline mask: where the mask is dark the board is not there. The generic
//! reading added the advert to the scanlines' own colour and drew it at full strength, a
//! band 70 grey too bright over the gantry (`docs/rendering/hd-track-tone.md`).
//!
//! The three rates are the record's own parameters, read here and never typed. The scanline
//! mask is read raw (`.x` of the first unit). **Read, not measured live:** the two colour
//! taps' gamma nibbles. They are decoded as `nr_twinblend`'s colour taps are: **chosen, not
//! measured**.
//!
//! Bound: the scanlines as the albedo, the advert second, the static third; the three rates
//! in a glow-table entry of mode [`PROGRAM_STATIC_HOLOGRAM`].

use std::sync::Arc;

use oag_rcs::rcsmaterial::fragment::Program;
use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{Emissive, ModelTexture, PROGRAM_STATIC_HOLOGRAM, TextureSlots, slots};

use super::emissive::EMISSIVE_LIMIT;
use super::skin::decode_texture;
use super::{Report, Textures};

/// The scanline mask, unit 0 (no preimage).
const SCAN: u32 = 0x05df_f912;
/// The advert, unit 1 (no preimage).
const ADVERT: u32 = 0x3bdc_0403;
/// The static, unit 2 (no preimage).
const STATIC: u32 = 0xdba0_a35a;
/// `u`'s scroll rate, added to the advert's coordinate.
const ADVERT_RATE: u32 = 0x3118_2e0d;
/// `v`'s scroll rate, added to the static's coordinate.
const STATIC_RATE: u32 = 0xb657_3513;
/// The scale on the scanline mask that becomes the alpha.
const ALPHA_SCALE: u32 = 0xeede_e991;
/// The engine clock.
const TIME: u32 = 0x906b_67ba;
/// The fog colour, which only the fogged blocks declare.
const FOG: u32 = 0x3dc3_1258;

/// Whether a resolved fragment block is the unlit combine: exactly the three samplers and
/// no parameter beyond the four it reads and the fog colour. The lit blocks declare the
/// light and are counted, not bound.
fn is_static_hologram(declared: &rcsmaterial::Declared) -> bool {
    let mut hashes: Vec<u32> = declared.samplers.iter().map(|s| s.0).collect();
    hashes.sort_unstable();
    let mut want = [SCAN, ADVERT, STATIC];
    want.sort_unstable();
    hashes == want
        && declared
            .parameters
            .iter()
            .all(|h| [TIME, ADVERT_RATE, STATIC_RATE, ALPHA_SCALE, FOG].contains(h))
}

/// Binds each `dc_hologramwithstatic` slot's three pictures and writes its glow-table entry
/// `[advert rate, static rate, alpha scale]` of mode [`PROGRAM_STATIC_HOLOGRAM`]. Returns the
/// static, which rides the third binding.
pub(super) fn static_hologram(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    (skins, seconds): (&mut TextureSlots, &mut TextureSlots),
    table: &mut Vec<Emissive>,
    report: &mut Report,
) -> TextureSlots {
    let mut statics: TextureSlots = vec![None; model.materials.len()];
    let mut decoded: std::collections::HashMap<String, Option<Arc<ModelTexture>>> =
        Default::default();
    for (slot, material) in model.materials.iter().enumerate() {
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        if super::emissive::family(material) != "dc_hologramwithstatic" {
            continue;
        }
        let Some(blob) = textures(&format!("/{}", material.name)) else {
            continue;
        };
        let (Some(declared), Some(_)) = (
            rcsmaterial::Declared::parse(&blob, variant.fragment.offset),
            Program::parse(&blob, variant.fragment.offset),
        ) else {
            continue;
        };
        if !is_static_hologram(&declared) {
            report.static_hologram_other_shape += 1;
            continue;
        }
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        // The generic reading gives this slot an additive glow off its second texture; any
        // other family's index is kept.
        if slots::material_index(*word) != 0 && *word & slots::ADD_SECOND == 0 {
            report.static_hologram_unread += 1;
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
        let parameter = |hash: u32| {
            material
                .parameters
                .iter()
                .find(|p| p.hash == hash)
                .map(|p| p.value[0])
        };
        let (
            Some(scan),
            Some(advert),
            Some(noise),
            Some(advert_rate),
            Some(static_rate),
            Some(alpha),
        ) = (
            picture(SCAN),
            picture(ADVERT),
            picture(STATIC),
            parameter(ADVERT_RATE),
            parameter(STATIC_RATE),
            parameter(ALPHA_SCALE),
        )
        else {
            report.static_hologram_unread += 1;
            continue;
        };
        let entry = Emissive {
            tint: [advert_rate, static_rate, alpha],
            offset: 0.0,
            scale: 0.0,
            rate: 0.0,
            program: [PROGRAM_STATIC_HOLOGRAM, 0.0],
        };
        let index = table.iter().position(|e| *e == entry).or_else(|| {
            (table.len() + 1 < EMISSIVE_LIMIT).then(|| {
                table.push(entry);
                table.len() - 1
            })
        });
        let Some(index) = index else {
            report.static_hologram_unread += 1;
            continue;
        };
        skins[slot] = Some(scan);
        seconds[slot] = Some(advert);
        statics[slot] = Some(noise);
        // The combine replaces the generic additive second layer.
        *word &= slots::ROLE_MASK
            & !(slots::ADD_SECOND
                | slots::SECOND_IS_LIGHTMAP
                | slots::ALBEDO_FROM_SECOND
                | slots::ALPHA_FROM_SECOND);
        *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        report.static_hologram_bound += 1;
    }
    statics
}
