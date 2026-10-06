//! HD's vertex-side texture scroll: `uv + time * rate`, off the material's own
//! authored rate parameters.
//!
//! Wipeout HD authors no keyframe block (`docs/formats/rcsmaterial.md`, "A
//! surface scrolls off an engine `time`"). Besides the fragment-side scroll of
//! an additive glow layer, a second family does it in the **vertex** program:
//! the lit block copies the texture coordinate attribute to the interpolator
//! the fragment samples with a multiply-add in between,
//!
//! ```text
//! MOV R0.w, c[207]                          ; one of the two factors
//! MAD o[TC3].x, R0.w, c[208], v[2].x        ; the other factor, then + uv
//! MAD o[TC3].y, R0.w, c[206], v[2].y
//! ```
//!
//! where, in the blocks read, one factor is the engine `time` and the other is
//! a parameter the material authors (`scripts/ps3-microcode.py vp-file`, every
//! block that declares `time`; swept in
//! `crates/render/examples/hd_anim_family_census.rs`). The fragment program
//! samples its one material texture at that interpolator and its lightmap at
//! another, so the surface scrolls and its bake does not. That is exactly
//! [`AnimTrack::Scroll`]: a rate per axis, in texture units per second.
//!
//! **Only the shapes read are admitted**, by the rate hash a block was seen to
//! use for each axis, and only on a material that names exactly one texture
//! besides the lightmap. A second material texture means a layer the vertex
//! scroll may or may not reach, which this reading did not measure, so such a
//! material draws still (`dc_hologramwithstatic2`, `cf_waterfall`).
//! `animlights` and `animhexlights` scroll the *second* UV set (`v[8]`) and
//! are not admitted.

use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{ANIM_TRACK_LIMIT, AnimTrack};

use super::{Report, Textures};

/// `~crc32("time")`.
const TIME: u32 = 0x906b_67ba;

/// One measured shape: the `u` and `v` rate parameter a vertex block adds
/// `time` times to the primary coordinate, either half absent.
struct Shape {
    u: Option<u32>,
    v: Option<u32>,
}

/// The shapes read, by the shader they were read on. The names are for the
/// reader; the match is on the hashes the vertex block declares.
const SHAPES: &[(&str, Shape)] = &[
    // `basic_uv_scroll`: `USpeed` and `VSpeed`.
    (
        "basic_uv_scroll",
        Shape {
            u: Some(0x1abb_e1f7),
            v: Some(0x9c2f_9359),
        },
    ),
    // `cf_uvanim_emssive*`: two unnamed floats, `x` and `y` in that order.
    (
        "cf_uvanim_emssive",
        Shape {
            u: Some(0x87d7_69dc),
            v: Some(0x2481_ef75),
        },
    ),
    // `hologram`: `Speed`, on `u` alone.
    (
        "hologram",
        Shape {
            u: Some(0x3118_2e0d),
            v: None,
        },
    ),
    // `emissive_bloom` and `emissive_lights`: one unnamed float, on `v`.
    (
        "emissive_bloom",
        Shape {
            u: None,
            v: Some(0x6829_2521),
        },
    ),
    // `uv_anim_diffuse_alpha`: one unnamed float, on `u`.
    (
        "uv_anim_diffuse_alpha",
        Shape {
            u: Some(0x33d5_1367),
            v: None,
        },
    ),
];

/// Replaces nothing a curve already drives: a slot with a nonzero
/// `material_anim` keeps it. Appends [`AnimTrack::Scroll`] tracks, deduplicated
/// by rate, and points each admitted slot at one.
pub(super) fn vertex_scroll(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    material_anim: &mut [u32],
    tracks: &mut Vec<AnimTrack>,
    report: &mut Report,
) {
    let mut cache: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();
    for (slot, material) in model.materials.iter().enumerate() {
        if material_anim.get(slot).is_none_or(|&a| a != 0) {
            continue;
        }
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let own_textures = material
            .samplers
            .iter()
            .filter(|&&(hash, _)| hash != rcsmaterial::LIGHTMAP_SAMPLER)
            .count();
        if own_textures != 1 {
            continue;
        }
        let blob = cache
            .entry(material.name.clone())
            .or_insert_with(|| textures(&format!("/{}", material.name)))
            .clone();
        let Some(blob) = blob else { continue };
        let Some(declared) = rcsmaterial::Declared::parse(&blob, variant.vertex.offset) else {
            continue;
        };
        if !declared.parameters.contains(&TIME) {
            continue;
        }
        let Some(shape) = SHAPES.iter().map(|(_, s)| s).find(|s| {
            [s.u, s.v]
                .into_iter()
                .flatten()
                .all(|h| declared.parameters.contains(&h))
                && (s.u.is_some() || s.v.is_some())
        }) else {
            continue;
        };
        let rate = |hash: Option<u32>| {
            hash.and_then(|h| material.parameters.iter().find(|p| p.hash == h))
                .map_or(0.0, |p| p.value[0])
        };
        let rate = [rate(shape.u), rate(shape.v)];
        if rate.iter().any(|r| !r.is_finite()) || rate == [0.0, 0.0] {
            continue;
        }
        let at = tracks
            .iter()
            .position(|seen| matches!(seen, AnimTrack::Scroll(r) if *r == rate))
            .or_else(|| {
                (tracks.len() + 1 < ANIM_TRACK_LIMIT).then(|| {
                    tracks.push(AnimTrack::Scroll(rate));
                    tracks.len() - 1
                })
            });
        let Some(at) = at else { continue };
        material_anim[slot] = u32::try_from(at + 1).unwrap_or(0);
        report.vertex_scrolls += 1;
    }
}
