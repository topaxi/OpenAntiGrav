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
//! **`glass_texture_customr` is a pane, not a window** (block #5 `@0x2db0`, the variant
//! Modesto Heights' and Talon's Junction's slots resolve to, traced `hd-glass-families`):
//! `rgb = fog(a * light * T1.rgb + (1 - a) * (specular + k * reflection)) / 2`, alpha `a`, with
//! `T1` the texture at `Texture1` (`0x3bdc0403`, unit 0) and `a` its alpha, `k` the material's
//! `Refbrightness` (`0xab31c2b1`), drawn `One`/`OneMinusSrcAlpha`. So the pane's picture and
//! coverage are `T1` and nothing else: the `j_sky.gtf` its entry 0 names (`0x8365b1f3`) is
//! declared by none of the material's twelve fragment blocks, which [`pane_picture`] answers
//! for `skin::picks`. [`Mode::Pane`] rides `tint.y` of the entry.
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
/// `Texture1`, the pane's picture and coverage.
const TEXTURE1: u32 = 0x3bdc_0403;

/// How a family's program combines the reflection with the rest of its sum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// `dc_windows_a`: the reflection times the weight, added.
    Window,
    /// `glass_texture_customr`: the lit picture times `a`, the specular and the reflection
    /// times `1 - a`.
    Pane,
}

/// One family whose lit block reads the sky map, with the parameter its weight is.
struct Row {
    family: &'static str,
    weight: u32,
    mode: Mode,
}

/// The families whose lit blocks are traced above.
const FAMILIES: &[Row] = &[
    Row {
        family: "dc_windows_a",
        weight: 0x5e5b_1937,
        mode: Mode::Window,
    },
    Row {
        family: "glass_texture_customr",
        weight: 0xab31_c2b1,
        mode: Mode::Pane,
    },
];

fn row(material: &rcsmodel::Material) -> Option<&'static Row> {
    let family = super::emissive::family(material);
    FAMILIES.iter().find(|row| row.family == family)
}

/// Whether `material` is one of the families whose unit 1 is the sky map this module tags, which
/// the generic emissive reading must not take for an additive layer.
pub(super) fn reads_sky(material: &rcsmodel::Material) -> bool {
    row(material).is_some()
}

/// The entry a pane family's program samples as picture and coverage, `Texture1`'s, when the
/// resolved variant declares it and the material names a `.gtf` there.
pub(super) fn pane_picture(
    material: &rcsmodel::Material,
    declared: &rcsmaterial::Declared,
) -> Option<usize> {
    if row(material)?.mode != Mode::Pane
        || !declared.samplers.iter().any(|(hash, _)| *hash == TEXTURE1)
    {
        return None;
    }
    material
        .samplers
        .iter()
        .position(|(hash, path)| *hash == TEXTURE1 && path.is_some())
}

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
        let Some(row) = row(material) else {
            continue;
        };
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
            .find(|p| p.hash == row.weight)
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
            tint: [weight, f32::from(u8::from(row.mode == Mode::Pane)), 0.0],
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
