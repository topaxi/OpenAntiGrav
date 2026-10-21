//! Which hull materials read Wipeout HD's reflection probe.
//!
//! The original lights a hull with an `IBL` variant of its material (permutation
//! word `0x007`, `0x847` with the craft light and shadow map) where this renderer
//! resolves the constant-ambient one. The `IBL` variants of the materials that
//! shine - `diffuse_with_specular_from_alpha_n` and its kin - declare a second
//! picture, `paraboloidReflectionTex` (`0x9edd3243`), which the program adds as
//! `4 * probe(reflect(-V, N)) * (albedo.a * reflectivity)` after the albedo
//! multiply and before the fog (`docs/rendering/hd-ship-materials.md`, "Finding 4").
//! That term is what makes the original's hull silver where the livery's alpha says
//! metal.
//!
//! The material slot carries the answer in the glow table, the way the light cone
//! and the refraction do: an entry whose `rate` is [`REFLECTION_RATE`] and whose
//! `tint.x` is the material's own `reflectivity` (parameter `0xab31c2b1`, `0.5` on
//! Harimau). A slot that already holds a glow index keeps it and is counted, not
//! overwritten.

use oag_rcs::rcsmaterial::{self, Features};
use oag_rcs::rcsmodel;

use super::{Report, Textures, slots};
use crate::mesh::Emissive;

/// `~crc32("paraboloidReflectionTex")`.
const REFLECTION_SAMPLER: u32 = 0x9edd_3243;
/// The material's own reflectivity, the `0.5` the program multiplies the alpha by.
const REFLECTIVITY: u32 = 0xab31_c2b1;
/// The `IBL` permutation without the craft light, the shadow map or the SPU colours:
/// `Sun`, field 3 (`IBL`), `Spot0`, `HalfBright`, `SVC0`.
const IBL_PASS: u32 = 0b0000_0000_0111;

/// The `rate` of a glow-table entry that marks a reflective hull material. A value
/// no other entry reaches (`0`, `1`, `2` for water, [`crate::mesh::ZONE_EXEMPT_RATE`]).
pub use crate::mesh::REFLECTION_RATE;

pub(super) fn probe_lit(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    table: &mut Vec<Emissive>,
    report: &mut Report,
) {
    let key = Features::from_pass_word(IBL_PASS);
    let mut cache: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();
    for (slot, material) in model.materials.iter().enumerate() {
        if variants.get(slot).copied().flatten().is_none() {
            continue;
        }
        let Some(word) = packed.get_mut(slot) else {
            continue;
        };
        let blob = cache
            .entry(material.name.clone())
            .or_insert_with(|| textures(&format!("/{}", material.name)))
            .clone();
        let Some(blob) = blob else { continue };
        let Ok(parsed) = rcsmaterial::RcsMaterial::parse(&blob) else {
            continue;
        };
        let Some(ibl) = rcsmaterial::Class::ALL
            .into_iter()
            .find_map(|class| parsed.variant(class, key))
        else {
            continue;
        };
        let Some(declared) = rcsmaterial::Declared::parse(&blob, ibl.fragment.offset) else {
            continue;
        };
        if !declared
            .samplers
            .iter()
            .any(|(hash, _)| *hash == REFLECTION_SAMPLER)
        {
            continue;
        }
        let Some(reflectivity) = material
            .parameters
            .iter()
            .find(|p| p.hash == REFLECTIVITY)
            .map(|p| p.value[0])
        else {
            report.probe_lit_unread += 1;
            continue;
        };
        if slots::material_index(*word) != 0 {
            report.probe_lit_blocked += 1;
            continue;
        }
        let layer = Emissive {
            tint: [reflectivity, 0.0, 0.0],
            offset: 0.0,
            scale: 1.0,
            rate: REFLECTION_RATE,
            program: [0.0; 2],
        };
        let index = table.iter().position(|seen| *seen == layer).or_else(|| {
            (table.len() + 1 < super::EMISSIVE_LIMIT).then(|| {
                table.push(layer);
                table.len() - 1
            })
        });
        let Some(index) = index else {
            report.probe_lit_blocked += 1;
            continue;
        };
        *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        report.probe_lit_bound += 1;
    }
}
