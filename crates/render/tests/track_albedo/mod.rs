//! What a circuit's draws bind as their picture, read off the built model.
//!
//! The draw-level half of "no purple line": a draw's colour texture is a
//! decoded picture, not a tangent-space normal map and not the circuit's
//! lightmap atlas. Every per-circuit walk that builds a [`Model`] calls
//! [`faults`] on it, so a title's whole track side is covered by walks that
//! already exist rather than a pass of its own. On 2026-10-19 HD's start line
//! drew `ds_sfblend_ns` (a normal-and-specular map) as its colour, a purple
//! ridged strip across the road on seven circuits, and no test saw it.
//!
//! **The normal-map oracle is the texels, not the file name.** A tangent-space
//! map decoded as `2c - 1` is a unit vector with a positive z on nearly every
//! texel, and a picture is not: `bluehorizontal` is as blue as a normal map
//! and fails it. Two-channel DXT5 packings (`*_n` files that carry x and y
//! only) decode to a flat `(128, 128, 128)` and are not caught; that is a
//! named gap, not a measured absence.

#![allow(dead_code)]

use std::sync::Arc;

use oag_mesh::mesh::{DrawCall, Model, ModelTexture, slots};

/// A texel is normal-like when `2c - 1` has unit length within this and points
/// out of the surface, z-dominant.
const UNIT_TOLERANCE: f32 = 0.12;
/// The fraction of sampled texels that must be normal-like to call the whole
/// texture a normal map. Measured: real normal maps clear 0.9, pictures sit
/// near 0.
const NORMAL_FRACTION: f32 = 0.9;
const SAMPLES: usize = 4096;

/// Why a draw's colour texture is not a picture.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fault {
    /// The bound texture reads as a tangent-space normal map.
    NormalMap(String),
    /// A draw names a texture slot that holds no texture: it binds the white
    /// texel, which reads as a plain surface and is exactly how a missing
    /// picture hides.
    Undecoded,
    /// The bound texture is the same atlas the slot's own lightmap binding holds.
    Lightmap(String),
}

/// Whether `texture`'s texels are a tangent-space normal map. `None` when they
/// cannot be read on the CPU (an uploaded or undecodable texture).
pub fn is_normal_map(texture: &ModelTexture) -> Option<bool> {
    let rgba = texture.to_rgba()?;
    let texels = rgba.len() / 4;
    if texels == 0 {
        return None;
    }
    let step = (texels / SAMPLES).max(1);
    let (mut normal, mut seen) = (0usize, 0usize);
    for t in (0..texels).step_by(step) {
        let p = &rgba[t * 4..t * 4 + 4];
        let v = [
            f32::from(p[0]) / 127.5 - 1.0,
            f32::from(p[1]) / 127.5 - 1.0,
            f32::from(p[2]) / 127.5 - 1.0,
        ];
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        seen += 1;
        // Out of the surface and leaning that way: a light grey picture is a
        // unit vector too (`jay_basecreamplain`, `(0.6, 0.6, 0.6)`), and what
        // it is not is z-dominant.
        if (len - 1.0).abs() < UNIT_TOLERANCE
            && v[2] > 0.4
            && v[2] > 1.1 * v[0].abs().max(v[1].abs())
        {
            normal += 1;
        }
    }
    Some(normal as f32 / seen as f32 >= NORMAL_FRACTION)
}

fn lists(model: &Model) -> [&Vec<DrawCall>; 3] {
    [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ]
}

/// Every distinct fault among the model's drawn texture slots, with the slot
/// it came from. Each texture is judged once, however many draws share it.
///
/// `normal_maps` is whether the title ships any: the PSP and PS2 Wipeout
/// games do not, and a blue glow sprite reads as a flat normal there
/// (`New_Blue_Glow-1.tga`), so the oracle runs only where a normal map can
/// exist (HD, 2048, Omega).
pub fn faults(model: &Model, normal_maps: bool) -> Vec<(usize, Fault)> {
    let mut judged: Vec<(*const ModelTexture, bool)> = Vec::new();
    let mut out = Vec::new();
    let mut slots: Vec<usize> = lists(model)
        .into_iter()
        .flatten()
        .filter_map(|d| d.texture)
        .collect();
    slots.sort_unstable();
    slots.dedup();
    for slot in slots {
        let Some(Some(texture)) = model.textures.get(slot) else {
            out.push((slot, Fault::Undecoded));
            continue;
        };
        let own_atlas = model
            .material_slots
            .get(slot)
            .is_some_and(|&packed| packed & slots::SECOND_IS_LIGHTMAP != 0)
            .then(|| model.lightmaps.get(slot))
            .flatten()
            .and_then(Option::as_ref);
        if own_atlas.is_some_and(|atlas| Arc::ptr_eq(atlas, texture)) {
            out.push((slot, Fault::Lightmap(texture.label.clone())));
            continue;
        }
        let key = Arc::as_ptr(texture);
        let bad = match judged.iter().find(|(k, _)| *k == key) {
            Some(&(_, bad)) => bad,
            None => {
                let bad = normal_maps && is_normal_map(texture).unwrap_or(false);
                judged.push((key, bad));
                bad
            }
        };
        if bad {
            out.push((slot, Fault::NormalMap(texture.label.clone())));
        }
    }
    out
}

/// [`faults`] as a sorted, de-duplicated list of what was wrong, for a message.
pub fn describe(faults: &[(usize, Fault)]) -> String {
    let mut names: Vec<String> = faults
        .iter()
        .map(|(s, f)| format!("slot {s}: {f:?}"))
        .collect();
    names.dedup();
    names.join("; ")
}
