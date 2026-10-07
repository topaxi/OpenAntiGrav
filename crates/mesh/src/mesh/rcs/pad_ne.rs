//! HD's pad materials: the `_ne` mask as a third bound texture.
//!
//! Both pad fragment programs sample the `_ne` file (sampler `0xa2d555b9`,
//! unit 1) and use it twice: its RGB, decoded `x * 2 - 1`, is a tangent-space
//! normal that replaces the vertex normal in the program's own `N.L` and
//! `N.H`, and its alpha - a mask over exactly the pad's light bars - gates one
//! additive term, `_ne.a * colour`, added after the light.
//! `docs/rendering/pads.md` carries the register trace. The colour is the
//! material's own authored value for the parameter that term's inline
//! constant is patched by ([`oag_rcs::rcsmaterial::fragment::Program::alpha_gated_parameter`]):
//! `W_Cycle` on a `Weapon Pad`, `Colour` or one of two unnamed hashes on a
//! `Speedup Pad`, per instance and per circuit.
//!
//! **The lightmap stays bound.** [`super::skin::picks`] gives the second slot
//! to the lightmap wherever it sits, because the pad's program reads it too
//! (unit 2). The mask is a third binding, `Model::pad_masks`, so neither
//! trades the other away.
//!
//! Called from [`super::pads`] alone: another material that samples the same
//! hash (`diffuse_normal_specular`, 37 paths, `emissive`'s own table of
//! normal-map samplers) is a different program and is not read here.

use std::sync::Arc;

use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{Emissive, ModelTexture, TextureSlots, slots};

use super::emissive::EMISSIVE_LIMIT;
use super::skin::decode_texture;
use super::{Report, Textures};

/// The `_ne` sampler's name hash, bound to fragment unit 1 on both pad
/// programs. `docs/rendering/pads.md`, "What binds the `_ne` file".
pub(super) const NE_SAMPLER: u32 = 0xa2d5_55b9;

/// Binds each pad material's `_ne` mask and writes [`slots::PAD_NE`] and its
/// colour's [`Emissive`] index into `packed`.
///
/// `table` is the model's own glow table; a pad colour is deduplicated into
/// it by value beside whatever [`super::emissive::emissive`] put there.
///
/// `only` limits the binding to the material slots it marks, `None` for all:
/// the scene pass names the pad materials its unreferenced chunks use and the
/// `diffuse_normal_specular_emmissive` family (rails, start line), whose lit
/// program reads `_ne` the same way (`pads::is_light_bar_material`); every
/// other material that samples the same hash is refused by the program match
/// below.
pub(super) fn pad_ne(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    packed: &mut [u32],
    table: &mut Vec<Emissive>,
    only: Option<&[bool]>,
    report: &mut Report,
) -> TextureSlots {
    let mut out: TextureSlots = vec![None; model.materials.len()];
    let mut blobs: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();
    let mut decoded: std::collections::HashMap<String, Option<Arc<ModelTexture>>> =
        Default::default();

    for (slot, material) in model.materials.iter().enumerate() {
        if only.is_some_and(|only| !only.get(slot).copied().unwrap_or(false)) {
            continue;
        }
        let Some(path) = material
            .samplers
            .iter()
            .find(|(hash, path)| *hash == NE_SAMPLER && path.is_some())
            .and_then(|(_, path)| path.clone())
        else {
            continue;
        };
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(blob) = blobs
            .entry(material.name.clone())
            .or_insert_with(|| textures(&format!("/{}", material.name)))
            .clone()
        else {
            continue;
        };
        let Some(program) = rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset)
        else {
            continue;
        };
        // The program has to use the mask the way the pads' do: the `_ne`
        // sampler on a unit, whose alpha gates an accumulate.
        let Some(unit) = program
            .declared
            .samplers
            .iter()
            .find(|&&(hash, _)| hash == NE_SAMPLER)
            .and_then(|&(_, unit)| u8::try_from(unit).ok())
        else {
            continue;
        };
        let Some(parameter) = program.alpha_gated_parameter(unit) else {
            continue;
        };
        let colour = material.parameters.iter().find(|p| p.hash == parameter);
        let texture = decoded
            .entry(path.clone())
            .or_insert_with(|| {
                textures(&format!("/{path}"))
                    .and_then(|bytes| decode_texture(&path, &bytes))
                    .map(Arc::new)
            })
            .clone();
        let (Some(colour), Some(texture)) = (colour, texture) else {
            report.pad_ne_unread += 1;
            continue;
        };
        let layer = Emissive {
            tint: [colour.value[0], colour.value[1], colour.value[2]],
            offset: 0.0,
            scale: 1.0,
            rate: 0.0,
        };
        let index = table.iter().position(|seen| *seen == layer).or_else(|| {
            (table.len() + 1 < EMISSIVE_LIMIT).then(|| {
                table.push(layer);
                table.len() - 1
            })
        });
        let Some(index) = index else {
            report.pad_ne_unread += 1;
            continue;
        };
        out[slot] = Some(texture);
        if let Some(word) = packed.get_mut(slot) {
            *word |= slots::PAD_NE;
            *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
        }
        report.pad_ne_bound += 1;
    }
    out
}
