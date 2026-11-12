//! HD's `uvdistortion_diff_vertexcolplusoverlay` (and its `bloom` variant): Modesto Heights'
//! banner band, a dark grille lit like any surface with a teal cloud screen overlaid by the
//! Ossego logo.
//!
//! # What the program does
//!
//! Resolved fragment block #5 `@0x2ad0` of
//! `modesto_heights/materials_reversed/uvdistortion_diff_vertexcolplusoverlay.rcsmaterial`
//! (`scripts/ps3-microcode.py fp-file`; blocks #2 and #3 are the same law with other light
//! terms) and its vertex block `@0x2880` (`vp-file`), which writes `TC0 = VertexColour1`
//! (`v[3]`) and the diffuse pair `(TC3.w, TC4.w) = v[2].xy`. With `t` the engine clock
//! (`0x906b67ba`), `uv` that pair, `screens` the unit 1 texel, `clouds` and `logo` units 0 and
//! 2, and the material's own values `D = d0989794`, `A = c0594d0e`, `S = 2bdce348`,
//! `B = fe619466`, `R = 96e30da5` and `teal = 961662ae`:
//!
//! ```text
//! warp  = clouds(uv).a                                          @0x12 TEX R0.w, unit0
//! c     = clouds((uv + A t + D warp) S)                         @0x13..@0x19
//! a     = VertexColour1 teal c screens.a                        @0x1a, @0x1d, @0x1f
//! b     = logo((uv + B t) R)                                    @0x0b, @0x24
//! over  = 2ab                      where a < 0.5                @0x20 SLT, @0x29 [NE]
//!       = 1 - 2 (1 - a)(1 - b)     elsewhere                    @0x22..@0x27
//! out   = fog(VertexColour1 screens.rgb light + over) / 2       @0x33, @0x35
//! ```
//!
//! `light` is the sum every lit HD program shares, so the generic lit path draws the first
//! term once the grille is the albedo. The generic reading bound the Ossego logo as the
//! albedo and added the second picture on top, which is white where the logo's ground is: the
//! band drew white where the original draws it dark navy
//! (`docs/formats/rcsmaterial.md`, "Modesto Heights: the banner band is drawn white").
//!
//! All six values are the record's own parameters, read here and never typed. **Read, not
//! measured live:** the units' gamma nibbles. The colour taps are decoded as every colour tap
//! on this path is: **chosen, not measured**.
//!
//! Bound: the grille as the albedo, the clouds second, the logo third; the teal and `D` in a
//! glow-table entry of mode [`PROGRAM_UV_OVERLAY`], and `A`, `S`, `B`, `R` in the entry after
//! it.

use std::sync::Arc;

use oag_rcs::rcsmaterial::fragment::Program;
use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{Emissive, ModelTexture, PROGRAM_UV_OVERLAY, TextureSlots, slots};

use super::emissive::EMISSIVE_LIMIT;
use super::skin::decode_texture;
use super::{Report, Textures};

/// The cloud map, unit 0: its alpha displaces its own coordinate.
pub(super) const CLOUDS: u32 = 0x030f_d39b;
/// The grille, unit 1: the lit diffuse, its alpha weighting the overlay's cloud term.
pub(super) const SCREENS: u32 = 0x515e_298e;
/// The logo, unit 2: the overlay's second operand.
pub(super) const LOGO: u32 = 0xfb3f_2dac;
/// `D`: how far the cloud map's alpha moves its coordinate.
pub(super) const DISTORTION: u32 = 0xd098_9794;
/// `A`: the cloud map's scroll rate, both axes.
pub(super) const CLOUD_SCROLL: u32 = 0xc059_4d0e;
/// `S`: the cloud map's coordinate scale.
pub(super) const CLOUD_SCALE: u32 = 0x2bdc_e348;
/// `B`: the logo's scroll rate, both axes.
pub(super) const LOGO_SCROLL: u32 = 0xfe61_9466;
/// `R`: the logo's coordinate scale.
pub(super) const LOGO_SCALE: u32 = 0x96e3_0da5;
/// The overlay's colour, a float3.
pub(super) const TEAL: u32 = 0x9616_62ae;

/// Whether a resolved fragment block declares exactly the three pictures and is the overlay
/// combine (its `SLT` against one half). The vertex-colour-only family has no logo unit, and the
/// shadow-mapped variants add two more.
fn is_overlay(declared: &rcsmaterial::Declared, program: &Program) -> bool {
    let mut hashes: Vec<u32> = declared.samplers.iter().map(|s| s.0).collect();
    hashes.sort_unstable();
    let mut want = [CLOUDS, SCREENS, LOGO];
    want.sort_unstable();
    hashes == want && program.instructions.iter().any(|i| i.name() == Some("SLT"))
}

/// Binds each banner slot's three pictures and writes its two glow-table entries
/// `[teal, D]` and `[A, S, B, R]`, both of mode [`PROGRAM_UV_OVERLAY`]. Returns the logo, which
/// rides the third binding.
pub(super) fn uv_overlay(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    (skins, seconds): (&mut TextureSlots, &mut TextureSlots),
    table: &mut Vec<Emissive>,
    report: &mut Report,
) -> TextureSlots {
    let mut logos: TextureSlots = vec![None; model.materials.len()];
    let mut decoded: std::collections::HashMap<String, Option<Arc<ModelTexture>>> =
        Default::default();
    for (slot, material) in model.materials.iter().enumerate() {
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        if !super::emissive::family(material).starts_with("uvdistortion_diff_vertexcolplusoverlay")
        {
            continue;
        }
        let Some(blob) = textures(&format!("/{}", material.name)) else {
            continue;
        };
        let (Some(declared), Some(program)) = (
            rcsmaterial::Declared::parse(&blob, variant.fragment.offset),
            Program::parse(&blob, variant.fragment.offset),
        ) else {
            continue;
        };
        if !is_overlay(&declared, &program) {
            report.uv_overlay_other_shape += 1;
            continue;
        }
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        // The generic reading gives this slot an additive glow off its second texture; any
        // other family's index is kept.
        if slots::material_index(*word) != 0 && *word & slots::ADD_SECOND == 0 {
            report.uv_overlay_unread += 1;
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
        let value = |hash: u32| {
            material
                .parameters
                .iter()
                .find(|p| p.hash == hash)
                .map(|p| p.value)
        };
        let (
            Some(screens),
            Some(clouds),
            Some(logo),
            Some(teal),
            Some(distortion),
            Some(cloud_scroll),
            Some(cloud_scale),
            Some(logo_scroll),
            Some(logo_scale),
        ) = (
            picture(SCREENS),
            picture(CLOUDS),
            picture(LOGO),
            value(TEAL),
            value(DISTORTION),
            value(CLOUD_SCROLL),
            value(CLOUD_SCALE),
            value(LOGO_SCROLL),
            value(LOGO_SCALE),
        )
        else {
            report.uv_overlay_unread += 1;
            continue;
        };
        let entries = [
            Emissive {
                tint: [teal[0], teal[1], teal[2]],
                offset: distortion[0],
                scale: 0.0,
                rate: 0.0,
                program: [PROGRAM_UV_OVERLAY, 0.0],
            },
            Emissive {
                tint: [cloud_scroll[0], cloud_scale[0], logo_scroll[0]],
                offset: logo_scale[0],
                scale: 0.0,
                rate: 0.0,
                program: [PROGRAM_UV_OVERLAY, 0.0],
            },
        ];
        let index = table
            .windows(2)
            .position(|pair| pair == entries)
            .or_else(|| {
                (table.len() + 2 < EMISSIVE_LIMIT).then(|| {
                    table.extend(entries);
                    table.len() - 2
                })
            });
        let Some(index) = index else {
            report.uv_overlay_unread += 1;
            continue;
        };
        skins[slot] = Some(screens);
        seconds[slot] = Some(clouds);
        logos[slot] = Some(logo);
        // The combine replaces the generic additive second layer.
        *word &= slots::ROLE_MASK
            & !(slots::ADD_SECOND
                | slots::SECOND_IS_LIGHTMAP
                | slots::ALBEDO_FROM_SECOND
                | slots::ALPHA_FROM_SECOND);
        *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        report.uv_overlay_bound += 1;
    }
    logos
}
