//! Wipeout HD's additive glow layer, per material.
//!
//! The one shape that separates HD's emissive family from every other
//! two-texture material: `MAD H0.xyz, H0.wwww, H1, H0` - the albedo plus the
//! diffuse alpha times a tinted, scrolling sample from unit 1. `mesh.wgsl`
//! *selects* between its two textures everywhere else, so without this the
//! glow is simply absent rather than wrong.
//!
//! Its own module rather than more of [`super::skin`], because it asks a
//! different question of the same inputs: `skin::roles` reads which unit each
//! texture is for, and this reads what is done with one of them.
//!
//! # What it will not do
//!
//! **Never the lightmap.** A material whose second slot is the circuit's baked
//! atlas is refused outright, whatever its microcode says: adding a light term
//! and a sun-occlusion mask to the albedo paints a shadow map as a glow. That
//! is the same refusal `skin::roles` already makes for albedo and coverage, and
//! it is why 50 of the disc's accumulating slots get no entry here.
//!
//! # Reach
//!
//! **119 material slots across the 16 circuits**, from 51 on Modesto Heights
//! and 31 on Tech de Ra down to none at all on the four Zone tracks - measured
//! by `crates/render/examples/hd_emissive_reach.rs`, which joins the same three
//! conditions this module applies.

use oag_formats::{rcsmaterial, rcsmodel};

use crate::mesh::{Emissive, ModelTexture, slots};

use super::Textures;

/// The float3 the emissive sample is multiplied by, parameter `0xe8bcd7f5`.
const TINT: u32 = 0xe8bc_d7f5;
/// `a`, added to `v` before the scale: parameter `0x78256a45`.
const OFFSET: u32 = 0x7825_6a45;
/// `b`, what that sum is scaled by: parameter `0x78787596`.
const SCALE: u32 = 0x7878_7596;

/// None of the three has a preimage yet, so each is cited by hash. See
/// `crates/render/examples/hd_param_names.rs`, which named 64 of 300.
const _: () = ();

/// The glow table and each material's index into it, plus one.
///
/// `roles` is read for [`slots::SECOND_IS_LIGHTMAP`] and written with
/// [`slots::ADD_SECOND`] and the index, so it must already carry what
/// `skin::roles` produced.
///
/// Deduplicated by value, like the texture-transform tracks: a circuit's
/// animated materials collapse to a handful of distinct `(tint, rate)` pairs
/// and the shader's table stays small.
pub(super) fn emissive(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    seconds: &[Option<std::sync::Arc<ModelTexture>>],
    roles: &mut [u32],
    textures: Textures<'_>,
) -> Vec<Emissive> {
    let mut table: Vec<Emissive> = Vec::new();
    let mut cache: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();

    for (slot, material) in model.materials.iter().enumerate() {
        let Some(packed) = roles.get_mut(slot) else {
            continue;
        };
        // The three conditions, in the order that makes the cheapest one
        // first: there has to be a decoded second texture to add, it must not
        // be the baked atlas, and the program must actually add it.
        if !seconds.get(slot).is_some_and(Option::is_some) {
            continue;
        }
        if *packed & slots::SECOND_IS_LIGHTMAP != 0 {
            continue;
        }
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let blob = cache
            .entry(material.name.clone())
            .or_insert_with(|| textures(&format!("/{}", material.name)))
            .clone();
        let Some(blob) = blob else { continue };
        let Some(program) = rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset)
        else {
            continue;
        };
        // Unit 1 is where every material this reading has met binds its second
        // texture, and `skin::units` is what says so per material - but it
        // answers for the *`.gtf` slot*, and this asks about the microcode's
        // own unit. They agree on every accumulating material measured.
        if !program.accumulates(1) {
            continue;
        }

        let value = |hash: u32, default: f32| {
            material
                .parameters
                .iter()
                .find(|p| p.hash == hash)
                .map_or(default, |p| p.value[0])
        };
        let tint = material
            .parameters
            .iter()
            .find(|p| p.hash == TINT)
            .map_or([1.0, 1.0, 1.0], |p| [p.value[0], p.value[1], p.value[2]]);
        // **A missing scroll is a still layer, not a missing layer.** 24 of
        // the 311 records carrying a tint author no `a`/`b` pair at all; their
        // glow is drawn and simply does not move, which is what `b` of 0 would
        // give anyway. Defaulting the *tint* to white is the same trade in the
        // other direction and is what an unpatched constant already is.
        let layer = Emissive {
            tint,
            offset: value(OFFSET, 0.0),
            scale: value(SCALE, 0.0),
        };

        let index = table.iter().position(|seen| *seen == layer).or_else(|| {
            // Past the shader's table the surface draws without its glow
            // rather than the build failing - the same graceful direction the
            // texture-transform and node-transform ceilings take.
            (table.len() + 1 < EMISSIVE_LIMIT).then(|| {
                table.push(layer);
                table.len() - 1
            })
        });
        let Some(index) = index else { continue };
        *packed |= slots::ADD_SECOND;
        *packed |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
    }
    table
}

/// How many distinct glow layers one model may carry, matching `mesh.wgsl`'s
/// `Emissives` array.
///
/// Slot 0 is "no glow", so a model gets `EMISSIVE_LIMIT - 1` real entries.
///
/// **Measured**: the busiest circuit is Modesto Heights at 51 accumulating
/// slots, which deduplicate further, against 64 here - and 64 `vec4` pairs is
/// 2 KiB of uniform, well inside the 64 KiB binding every backend guarantees.
pub const EMISSIVE_LIMIT: usize = 64;
