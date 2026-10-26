//! The CPU half of the hull's extra pass: which batches the pass draws, and the
//! unlit-white model it draws them on. The pass itself - its blend state, its
//! per-frame environment-map coordinates - is `oag_render::shine`; this half is
//! pure mesh analysis, so the loader that builds a race's hulls (`oag-livery`)
//! reaches it without depending on the renderer.

use super::{Emissive, Model, TextureSlots};

/// The extra pass's model for `hull`: only its [`Model::shine_draws`], on the
/// hull's own vertices and indices, unlit and white, through the second
/// texture each draw already names. `None` for a hull that authors no such
/// batch, which is every model but a Pulse hull.
///
/// The texture coordinates are the authored ones until `oag_render::shine::write` replaces them
/// each frame, because they depend on the ship's rotation.
#[must_use]
pub fn build(hull: &Model) -> Option<Model> {
    if hull.shine_draws.is_empty() {
        return None;
    }
    let mut model = hull.clone();
    model.label = format!("{} (shine pass)", hull.label);
    model.draws = std::mem::take(&mut model.shine_draws);
    let lightmaps = std::mem::take(&mut model.lightmaps);
    plain(&mut model);
    mask(&mut model, lightmaps);
    Some(model)
}

/// A texture slot for an extra-pass draw its ordinary pass weights by its own
/// texture's alpha ([`oag_vex::vex::Batch::weights_extra_pass_by_alpha`]): a
/// copy of the `second` (environment) texture's slot whose `lightmaps` entry is
/// the batch's `base` texture, so [`build`] can bind the base as the draw's
/// second picture. One slot per pair, reused by every batch that shares it.
pub fn masked_slot(
    textures: &mut TextureSlots,
    lightmaps: &mut TextureSlots,
    second: usize,
    base: usize,
) -> usize {
    let same = |a: &Option<std::sync::Arc<super::ModelTexture>>,
                b: &Option<std::sync::Arc<super::ModelTexture>>| match (a, b) {
        (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
        _ => false,
    };
    let (env, mask) = (textures[second].clone(), textures[base].clone());
    if let Some(slot) = (0..lightmaps.len())
        .find(|&slot| same(&textures[slot], &env) && same(&lightmaps[slot], &mask))
    {
        return slot;
    }
    textures.push(env);
    lightmaps.resize(textures.len(), None);
    lightmaps[textures.len() - 1] = mask;
    textures.len() - 1
}

/// Whether `draw` of a [`build`] result is weighted by its batch texture's alpha.
#[must_use]
pub fn is_masked(model: &Model, draw: &super::DrawCall) -> bool {
    draw.texture
        .and_then(|slot| model.lightmaps.get(slot))
        .is_some_and(Option::is_some)
}

/// Hands the draws [`masked_slot`] paired with a base texture their weight: the
/// base bound as the second picture, each such draw's vertices carrying their
/// authored coordinate in `lightmap_texcoord` (the environment map overwrites
/// `texcoord` every frame) and the glow-table entry whose
/// [`super::PROGRAM_SHINE_MASK`] mode `shade.wesl` multiplies the picture by
/// that texture's alpha under.
pub fn mask(model: &mut Model, lightmaps: TextureSlots) {
    let masked = |slot: Option<usize>| {
        slot.and_then(|s| lightmaps.get(s))
            .is_some_and(Option::is_some)
    };
    if !model.draws.iter().any(|d| masked(d.texture)) {
        return;
    }
    model.emissive = vec![Emissive {
        tint: [1.0; 3],
        offset: 0.0,
        scale: 1.0,
        rate: 0.0,
        program: [super::PROGRAM_SHINE_MASK, 0.0],
    }];
    let packed = super::slots::DEFAULT | (1 << super::slots::MATERIAL_SHIFT);
    for draw in model.draws.iter().filter(|d| masked(d.texture)) {
        for &index in &model.indices[draw.range.start as usize..draw.range.end as usize] {
            let vertex = &mut model.vertices[index as usize];
            vertex.lightmap_texcoord = vertex.texcoord;
            vertex.slots = packed;
        }
    }
    model.lightmaps = lightmaps;
}

/// Drops the weights from a circuit model whose extra pass is to draw unweighted:
/// [`oag_render::shine::build_track`] reads them off [`Model::lightmaps`], and a
/// weighted draw's slot then reads as the plain environment texture it copies.
pub fn unweight(model: &mut Model) {
    model.lightmaps.clear();
}

/// Turns `model` into the extra pass's: unlit white vertices, no animation,
/// glow or flame, and only its [`Model::draws`] left to draw.
pub fn plain(model: &mut Model) {
    for vertex in &mut model.vertices {
        // A vertex that authors a colour keeps it: the pass is lit by nothing
        // and the texture is modulated by what the vertex carries, as the
        // original's is. One that authors none (a hull's) is white.
        if vertex.lit != 0.0 {
            vertex.colour = [1.0; 4];
        }
        vertex.lit = 0.0;
        vertex.anim = 0;
        vertex.glow = 0.0;
    }
    model.alpha_tested_draws.clear();
    model.transparent_draws.clear();
    model.lightmaps.clear();
    model.material_slots.clear();
    model.material_specular_exponent.clear();
    model.material_glow.clear();
    model.glow_from_program = false;
    model.material_colour_factor.clear();
    model.material_quake.clear();
    model.material_variants.clear();
    model.material_anim.clear();
    model.anim_tracks.clear();
    model.emissive.clear();
    model.vertex_colour_is_light = false;
    model.stamps_glow = false;
    model.flame = None;
}

#[cfg(test)]
mod tests;
