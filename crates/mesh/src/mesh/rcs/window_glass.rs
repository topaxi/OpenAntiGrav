//! Which scenery glass reads the circuit's sky reflection map.
//!
//! `dc_windows_a`'s lit block (`@0x43d0`, traced `hd-one-pair-moa`) is
//! `rgb = fog(reflection * 0.8 + light * Texture1 + specular)`, the reflection being
//! `paraboloidReflectionTex` read at `reflect(-V, N)` through the hull's own dual-paraboloid
//! address, tinted by the engine's `iblScalePower` (`0x7480de6d`) and scaled by the material's
//! own weight, parameter `0x5e5b1937` (`0.8` on every instance of the family in Moa Therma).
//! The map is the circuit's sky (`mesh::sky_paraboloid`); this module tags the slots whose
//! program is that block, the way `probe_lit` tags a hull's, and `shade.wesl` adds the term.
//!
//! **Left out, named**: the program perturbs `N` with a tangent-space normal map (`Texture` at
//! unit 0) before the reflection and the specular, which a track vertex carries no frame for in
//! this renderer, so the reflection reads the vertex normal. A slot that already holds a glow
//! index keeps it and is counted, not overwritten.

use oag_rcs::rcsmaterial;
use oag_rcs::rcsmodel;

use super::{Report, Textures, slots};
use crate::mesh::Emissive;

/// `~crc32("paraboloidReflectionTex")`.
const REFLECTION_SAMPLER: u32 = 0x9edd_3243;
/// The material's weight on the reflection, `0.8`.
const WEIGHT: u32 = 0x5e5b_1937;
/// The families whose lit block is the one traced above.
const FAMILIES: &[&str] = &["dc_windows_a"];

pub use crate::mesh::WINDOW_REFLECTION_RATE;

pub(super) fn window_glass(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    table: &mut Vec<Emissive>,
    report: &mut Report,
) {
    let mut cache: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();
    for (slot, material) in model.materials.iter().enumerate() {
        if !FAMILIES.contains(&super::emissive::family(material)) {
            continue;
        }
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        let blob = cache
            .entry(material.name.clone())
            .or_insert_with(|| textures(&format!("/{}", material.name)))
            .clone();
        let Some(blob) = blob else { continue };
        let Some(declared) = rcsmaterial::Declared::parse(&blob, variant.fragment.offset) else {
            continue;
        };
        if !declared
            .samplers
            .iter()
            .any(|(hash, _)| *hash == REFLECTION_SAMPLER)
        {
            continue;
        }
        let Some(weight) = material
            .parameters
            .iter()
            .find(|p| p.hash == WEIGHT)
            .map(|p| p.value[0])
        else {
            report.window_glass_unread += 1;
            continue;
        };
        if slots::material_index(*word) != 0 {
            report.window_glass_blocked += 1;
            continue;
        }
        let layer = Emissive {
            tint: [weight, 0.0, 0.0],
            offset: 0.0,
            scale: 1.0,
            rate: WINDOW_REFLECTION_RATE,
            program: [0.0; 2],
        };
        let index = table.iter().position(|seen| *seen == layer).or_else(|| {
            (table.len() + 1 < super::EMISSIVE_LIMIT).then(|| {
                table.push(layer);
                table.len() - 1
            })
        });
        let Some(index) = index else {
            report.window_glass_blocked += 1;
            continue;
        };
        *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        report.window_glass_bound += 1;
    }
}
