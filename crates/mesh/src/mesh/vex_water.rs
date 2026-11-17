//! `enhanced` water on a `.vex` circuit: this project's own sea for a title whose original
//! draws none.
//!
//! Pulse and Pure draw their sea as a plain textured mesh. Under `enhanced` the batches that
//! bind a listed water texture (`oag_title::water`) are drawn with HD's sea law
//! (`super::rcs::water`, `docs/rendering/water.md`): two taps of a wave normal map scrolled in
//! opposite directions, light times a colour, a sun glint, and the sky's reflection and nothing
//! else. The shader's own branch is `shade.wesl`'s kind [`ENHANCED_KIND`].
//!
//! **Everything but the law's shape is this project's own, chosen, not measured**: the normal
//! map is generated ([`super::water_normal`]); the colour is the mean of the surface's own
//! texture; the taps run on the world's `x` and `z`, not the surface's texture coordinates
//! (a Pulse sea repeats its texture anywhere from sixteen times to once across a kilometre,
//! so the surface's own coordinates give no wave size), at [`WAVE_SIZE`] units a tile; the
//! reflectivity and the glint are tuned for the picture. No other disc is read.

use std::sync::Arc;

use super::{Model, ModelTexture, PROGRAM_WATER, Texels, slots, water_normal};
use crate::mesh::rcs::{Tap, lay, sea_block};

/// `program[1]` of an enhanced water entry: the family `shade.wesl` and `water.wesl` branch on.
/// HD's four families are 1 to 4.
pub const ENHANCED_KIND: f32 = 5.0;

/// The seed of the generated normal map.
const SEED: u32 = 0x5ea0_0001;

/// How many world units one tile of the normal map spans.
pub const WAVE_SIZE: f32 = 240.0;

/// The reflection's weight.
const REFLECTIVITY: f32 = 0.22;

/// The glint's power and colour.
const SPECULAR_POWER: f32 = 90.0;
const SPECULAR_COLOUR: [f32; 3] = [0.9, 0.95, 1.0];

/// What [`enhanced`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// The texture slots tagged as water.
    pub slots: usize,
    /// The draws laid over them, and their vertices.
    pub draws: usize,
    pub vertices: usize,
    /// Listed names no slot bound, and slots whose texels this cannot average.
    pub missing: Vec<String>,
    pub unread: usize,
}

/// A texture label against a listed name: the file name without its extension, any case.
fn names_match(label: &str, name: &str) -> bool {
    let file = label.rsplit(['\\', '/']).next().unwrap_or(label);
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    let wanted = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    stem.eq_ignore_ascii_case(wanted)
}

/// The mean of a texture's base level as `0..1` RGB, or `None` for texels this cannot read.
fn mean_colour(texture: &ModelTexture) -> Option<[f32; 3]> {
    let base: &[u8] = match &texture.texels {
        Texels::Rgba8(bytes) => bytes,
        Texels::Chain(levels) => levels.first()?,
        _ => return None,
    };
    let count = base.len() / 4;
    if count == 0 {
        return None;
    }
    let mut sum = [0u64; 3];
    for texel in base.chunks_exact(4) {
        for (total, &byte) in sum.iter_mut().zip(texel) {
            *total += u64::from(byte);
        }
    }
    Some(sum.map(|total| total as f32 / (count as f32 * 255.0)))
}

/// The two taps: HD's sea (`Tap::sea`) with the scale in tiles per world unit.
fn taps() -> [Tap; 2] {
    let [a, b] = Tap::sea();
    let k = 1.0 / WAVE_SIZE;
    let scaled = |tap: Tap| Tap {
        scale: [tap.scale[0] * k, tap.scale[1] * k],
        ..tap
    };
    [scaled(a), scaled(b)]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalised(v: [f32; 3]) -> Option<[f32; 3]> {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    (length > 1.0e-6).then(|| [v[0] / length, v[1] / length, v[2] / length])
}

/// The packed `Unorm8x4` tangent of a vertex with normal `n`: world `+x` made perpendicular to
/// it (`+z` where the normal runs along `x`), `w` at 255 so the shader's bitangent is
/// `cross(n, t)` unflipped.
fn packed_tangent(n: [f32; 3]) -> u32 {
    let along = |axis: [f32; 3]| {
        let d = n[0] * axis[0] + n[1] * axis[1] + n[2] * axis[2];
        normalised([axis[0] - n[0] * d, axis[1] - n[1] * d, axis[2] - n[2] * d])
    };
    let t = along([1.0, 0.0, 0.0])
        .or_else(|| along([0.0, 0.0, 1.0]))
        .unwrap_or([1.0, 0.0, 0.0]);
    let byte = |v: f32| ((v * 0.5 + 0.5) * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
    u32::from_le_bytes([byte(t[0]), byte(t[1]), byte(t[2]), 255])
}

/// Whether `model` carries an enhanced water entry, so the circuit's sky is wanted as the
/// reflection map (`sky_paraboloid::from_mesh`).
#[must_use]
pub fn is_tagged(model: &Model) -> bool {
    model
        .emissive
        .iter()
        .any(|entry| entry.program == [PROGRAM_WATER, ENHANCED_KIND])
}

/// Draws every opaque batch of `model` that binds one of `names` as enhanced water.
///
/// Per tagged texture slot: a three-entry glow-table block (the colour is the texture's own
/// mean), the generated normal map bound third. Per tagged vertex: the glow entry in its
/// `slots`, a tangent, and, where the batch authored no normals (the loader faces such a
/// vertex at `+z`), a normal from its triangles, turned to face up.
pub fn enhanced(model: &mut Model, names: &[&str]) -> Report {
    let mut report = Report::default();
    report.missing = names
        .iter()
        .filter(|name| {
            !model
                .textures
                .iter()
                .flatten()
                .any(|texture| names_match(&texture.label, name))
        })
        .map(|name| (*name).to_string())
        .collect();
    let map = Arc::new(water_normal::texture(SEED));
    let mut entry_of: Vec<Option<u32>> = vec![None; model.textures.len()];
    for (slot, texture) in model.textures.iter().enumerate() {
        let Some(texture) = texture else { continue };
        if !names.iter().any(|name| names_match(&texture.label, name)) {
            continue;
        }
        let Some(colour) = mean_colour(texture) else {
            report.unread += 1;
            continue;
        };
        let [first, second] = taps();
        let block = sea_block(
            ENHANCED_KIND,
            colour,
            REFLECTIVITY,
            SPECULAR_POWER,
            SPECULAR_COLOUR,
            [&first, &second],
        );
        let Some(at) = lay(&mut model.emissive, &block) else {
            report.unread += 1;
            continue;
        };
        entry_of[slot] = Some(u32::try_from(at + 1).unwrap_or(0));
        report.slots += 1;
    }
    if report.slots == 0 {
        return report;
    }
    model.pad_masks.resize(model.textures.len(), None);
    for (slot, entry) in entry_of.iter().enumerate() {
        if entry.is_some() {
            model.pad_masks[slot] = Some(map.clone());
        }
    }
    let ranges: Vec<(std::ops::Range<u32>, u32)> = model
        .draws
        .iter()
        .filter_map(|draw| {
            let entry = entry_of.get(draw.texture?).copied().flatten()?;
            Some((draw.range.clone(), entry))
        })
        .collect();
    for (range, entry) in ranges {
        let triangles = &model.indices[range.start as usize..range.end as usize];
        let (Some(&first), Some(&last)) = (triangles.iter().min(), triangles.iter().max()) else {
            continue;
        };
        let mut accumulated = vec![[0.0f32; 3]; (last - first + 1) as usize];
        for tri in triangles.chunks_exact(3) {
            let p = |i: u32| model.vertices[i as usize].position;
            let (a, b, c) = (p(tri[0]), p(tri[1]), p(tri[2]));
            let mut n = cross(
                [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
                [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
            );
            if n[1] < 0.0 {
                n = [-n[0], -n[1], -n[2]];
            }
            for &i in tri {
                let slot = &mut accumulated[(i - first) as usize];
                for k in 0..3 {
                    slot[k] += n[k];
                }
            }
        }
        for (offset, geometric) in accumulated.iter().enumerate() {
            let vertex = &mut model.vertices[first as usize + offset];
            if vertex.normal == [0.0, 0.0, 1.0]
                && let Some(n) = normalised(*geometric)
            {
                vertex.normal = n;
            }
            vertex.tangent = packed_tangent(vertex.normal);
            vertex.slots = (vertex.slots & slots::ROLE_MASK) | (entry << slots::MATERIAL_SHIFT);
            report.vertices += 1;
        }
        report.draws += 1;
    }
    report
}

#[cfg(test)]
mod tests;
