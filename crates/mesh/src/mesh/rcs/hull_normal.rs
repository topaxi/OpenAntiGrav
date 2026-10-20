//! HD's ship hulls sample their `_n` normal map.
//!
//! A hull's lit fragment program (`diffuse_with_specular_from_alpha_n`,
//! `carbonfibre`, `nitro_body_new` and kin) reads texture unit 1 as a
//! tangent-space normal and builds the lighting normal from it:
//!
//! ```text
//! N = (2 t.x - 1) T + (2 t.y - 1) B + (2 t.z - 1) N0         then normalised
//! B = cross(N0, T) * T.w                                     the vertex program
//! ```
//!
//! with `T` the chunk's authored `tangent` attribute (`2b - 1` on its first
//! three bytes, the fourth byte `w` the factor on `B`) and `N0` the vertex
//! normal. The normalised `N` then feeds every term the program lights with:
//! the sun's `N.L`, the specular's `N.H`, the probe lookups. Plain RGB, not
//! DXT5nm. `docs/rendering/hd-ship-materials.md`, "Hull normal maps".
//!
//! This binds the picture in the third slot ([`Model::pad_masks`]) and marks
//! the material, so `mesh::rcs::emit` fills [`GpuVertex::tangent`] for its
//! chunks; `shade.wesl` reads both. Every vertex of every other material
//! carries a zero tangent, which is the shader's off switch.
//!
//! **Scope: opaque ship materials whose program reads unit 1 as a normal.** A
//! material whose normal sits on another unit (`detonator_shield`'s unit 2,
//! `detonator_ship_rich_iridescent`'s unit 0) or that blends (the canopy glass,
//! `nitro_perspex_new`) is a different program and is left as it drew; the
//! circuits' own normal-mapped materials are not ships and are not touched.

use std::sync::Arc;

use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{ModelTexture, TextureSlots};

use super::skin::decode_texture;
use super::{Report, Textures};

/// The fragment unit the hull programs read their normal map on.
const NORMAL_UNIT: u32 = 1;

/// Binds each ship hull material's normal map and returns, per material slot,
/// whether its chunks carry the tangent.
pub(super) fn hull_normal(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    report: &mut Report,
) -> (TextureSlots, Vec<bool>) {
    let mut maps: TextureSlots = vec![None; model.materials.len()];
    let mut marked = vec![false; model.materials.len()];
    let mut decoded: std::collections::HashMap<String, Option<Arc<ModelTexture>>> =
        Default::default();
    for (slot, material) in model.materials.iter().enumerate() {
        if !material.name.contains("/ships/") || material.blend() != rcsmodel::Blend::Opaque {
            continue;
        }
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(blob) = textures(&format!("/{}", material.name)) else {
            continue;
        };
        let (Some(declared), Some(program)) = (
            rcsmaterial::Declared::parse(&blob, variant.fragment.offset),
            rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset),
        ) else {
            continue;
        };
        let Some(&(hash, _)) = declared
            .samplers
            .iter()
            .find(|&&(_, unit)| unit == NORMAL_UNIT)
        else {
            continue;
        };
        if !program.samples_as_normal(NORMAL_UNIT as u8) {
            continue;
        }
        let Some(path) = material
            .samplers
            .iter()
            .find(|(h, p)| *h == hash && p.is_some())
            .and_then(|(_, p)| p.clone())
        else {
            report.hull_normal_unread += 1;
            continue;
        };
        let texture = decoded
            .entry(path.clone())
            .or_insert_with(|| {
                textures(&format!("/{path}"))
                    .and_then(|bytes| decode_texture(&path, &bytes))
                    .map(Arc::new)
            })
            .clone();
        let Some(texture) = texture else {
            report.hull_normal_unread += 1;
            continue;
        };
        maps[slot] = Some(texture);
        marked[slot] = true;
        report.hull_normal_bound += 1;
    }
    (maps, marked)
}
