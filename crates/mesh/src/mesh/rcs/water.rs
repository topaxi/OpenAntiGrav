//! HD's water family: a program that reads its picture as a normal map.
//!
//! `water_test_2` and `water_noref` (Vineta K's sea) declare a texture,
//! `waves2.gtf`, and read it only as a normal: three taps, each decoded
//! `2 * tap - 1` and summed into a perturbed normal
//! (`docs/formats/rcsmaterial.md`, "Water family"). Their colour is never a
//! picture; it is the vertex colour times the light plus a lookup in the
//! engine's `paraboloidReflectionTex`. Binding `waves2.gtf` as the albedo, as
//! the generic reading did, drew the normal map as a blue sheet.
//!
//! The classifier is by what the resolved program does, not by name: it
//! declares the paraboloid probe, at least one other sampler is decoded as a
//! normal, and no declared sampler besides the lightmap is read as a colour.

use oag_rcs::rcsmaterial::fragment::Program;
use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{ModelTexture, Texels};

use super::{Report, TextureSlots, Textures};

/// `~crc32("paraboloidReflectionTex")`.
pub(super) const PARABOLOID: u32 = 0x9edd_3243;
/// `~crc32("PerMaterialEnvMap")` (exact preimage). 2048's `Environment_Load`
/// scans every material for it; HD's `.rcsmodel` records carry it too, **no HD
/// program declares it and the HD executable does not contain the string**,
/// so it is not read as HD's reflection source (`rcsmaterial.md`, "Water
/// family").
#[allow(dead_code)]
pub(super) const PER_MATERIAL_ENV_MAP: u32 = 0x8365_b1f3;

/// Whether a resolved program is the water shape described above.
pub(super) fn is_water(declared: &rcsmaterial::Declared, program: &Program) -> bool {
    if !declared.samplers.iter().any(|&(h, _)| h == PARABOLOID) {
        return false;
    }
    let others = || {
        declared
            .samplers
            .iter()
            .filter(|&&(h, _)| h != PARABOLOID && h != rcsmaterial::LIGHTMAP_SAMPLER)
            .map(|&(_, unit)| u8::try_from(unit).unwrap_or(u8::MAX))
    };
    others().any(|u| program.samples_as_normal(u))
        && others().all(|u| program.samples_as_normal(u) || !program.samples_colour(u))
}

/// `~crc32("constantAmbientColour")`, which only the lit-vertex-colour variant
/// declares.
const CONSTANT_AMBIENT: u32 = rcsmaterial::CONSTANT_AMBIENT;

/// Takes the normal map out of every water material's albedo.
///
/// Two programs share the shape and differ in what is left once the
/// reflection (the engine's `paraboloidReflectionTex`, not drawn) is taken
/// out:
///
/// - one that declares `constantAmbientColour` (`water_test_2`) computes
///   `vertexColour * (ambient + sun * N.L)` plus the reflection times the
///   vertex colour, so with no reflection it is **lit vertex colour**: no
///   picture, which the white placeholder gives;
/// - one that does not (`water_noref`) has no colour but the reflection and a
///   sun glint, so with no reflection it is **black plus the glint**: a black
///   picture, the glint left to the generic specular (**chosen, not
///   measured**), and fog on top.
pub(super) fn water(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    skins: &mut TextureSlots,
    report: &mut Report,
) {
    let mut black: Option<std::sync::Arc<ModelTexture>> = None;
    for (slot, material) in model.materials.iter().enumerate() {
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(blob) = textures(&format!("/{}", material.name)) else {
            continue;
        };
        let (Some(declared), Some(program)) = (
            rcsmaterial::Declared::parse(&blob, variant.fragment.offset),
            Program::parse(&blob, variant.fragment.offset),
        ) else {
            continue;
        };
        if !is_water(&declared, &program) {
            continue;
        }
        let lit_vertex_colour = declared.parameters.contains(&CONSTANT_AMBIENT);
        let Some(texture) = skins.get_mut(slot) else {
            continue;
        };
        *texture = if lit_vertex_colour {
            None
        } else {
            Some(
                black
                    .get_or_insert_with(|| {
                        std::sync::Arc::new(ModelTexture {
                            label: "water: no reflection, black".to_string(),
                            width: 1,
                            height: 1,
                            texels: Texels::Rgba8(vec![0, 0, 0, 255]),
                            mip_count: None,
                        })
                    })
                    .clone(),
            )
        };
        if lit_vertex_colour {
            report.water_lit_colour += 1;
        } else {
            report.water_glint_only += 1;
        }
    }
}
