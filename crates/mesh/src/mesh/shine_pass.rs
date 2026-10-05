//! The CPU half of the hull's extra pass: which batches the pass draws, and the
//! unlit-white model it draws them on. The pass itself - its blend state, its
//! per-frame environment-map coordinates - is `oag_render::shine`; this half is
//! pure mesh analysis, so the loader that builds a race's hulls (`oag-livery`)
//! reaches it without depending on the renderer.

use super::Model;

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
    plain(&mut model);
    Some(model)
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
