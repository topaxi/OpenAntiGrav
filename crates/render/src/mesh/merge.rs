//! [`merge`]: concatenating several models into one buffer pair.
//!
//! Split out of `mesh.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. It sits apart
//! cleanly because it is the one function here that takes [`Model`]s rather than
//! bytes - nothing in it knows what a `.vex` is.

use super::*;

/// Concatenates several models into one buffer pair.
///
/// Used to draw more than one thing at a time without the pipeline learning
/// about scenes: `oag-view --collision --with-spline` overlays the collision
/// soup on the driveable ribbon so the two can be compared in place.
///
/// Index and texture-slot references are rebased, since both are positional.
/// Empty models are skipped rather than contributing an empty draw call.
///
/// This is a *concatenation*, not a scene: the models share one depth buffer and
/// one opaque pipeline, so a solid model will occlude anything inside it. That is
/// why the collision view draws outlines by default; see
/// [`crate::collision`].
#[must_use]
pub fn merge(label: &str, models: Vec<Model>) -> Model {
    let mut out = Model {
        // `merge` folds several sources into one buffer pair, and a flap's
        // vertex range (like `node_vertex_ranges`, dropped below for the
        // same reason) is only meaningful against the source it came from -
        // the same reason `DrawCall::node` is documented as ambiguous here.
        airbrakes: [None, None],
        label: label.to_string(),
        vertices: Vec::new(),
        indices: Vec::new(),
        draws: Vec::new(),
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: Vec::new(),
        lightmaps: Vec::new(),
        centre: [0.0; 3],
        radius: 1.0,
        mesh_count: 0,
        anim_tracks: Vec::new(),
        node_vertex_ranges: Vec::new(),
    };

    for model in models {
        if model.vertices.is_empty() || model.indices.is_empty() {
            continue;
        }
        let vertex_base = out.vertices.len() as u32;
        let index_base = out.indices.len() as u32;
        let texture_base = out.textures.len();
        // Positional like the texture slots, and rebased the same way, except
        // that 0 is "no transform" rather than a slot and has to stay 0.
        let anim_base = u32::try_from(out.anim_tracks.len()).unwrap_or(0);
        out.anim_tracks.extend(model.anim_tracks);
        // Concatenating two models can push the total past what the shader's
        // table holds, where the builder's own ceiling only bounds one source.
        // Rebased indices past it fall back to "no transform" rather than
        // pointing off the end.
        let limit = u32::try_from(ANIM_TRACK_LIMIT).unwrap_or(u32::MAX);
        out.anim_tracks.truncate(ANIM_TRACK_LIMIT - 1);

        out.vertices.extend(model.vertices.into_iter().map(|mut v| {
            if v.anim != 0 {
                // 0, not a wrap: an index that no longer has a track behind it
                // must draw unanimated, where a modulo would quietly hand it
                // some *other* surface's animation.
                let rebased = v.anim.saturating_add(anim_base);
                v.anim = if rebased < limit { rebased } else { 0 };
            }
            v
        }));
        out.indices
            .extend(model.indices.iter().map(|i| i + vertex_base));
        let rebase = |d: DrawCall| DrawCall {
            range: (d.range.start + index_base)..(d.range.end + index_base),
            texture: d.texture.map(|t| t + texture_base),
            bounds: d.bounds,
            node: d.node,
            blend: d.blend,
            blend_state: d.blend_state,
            layer: d.layer,
            culled: d.culled,
        };
        out.draws.extend(model.draws.into_iter().map(rebase));
        out.alpha_tested_draws
            .extend(model.alpha_tested_draws.into_iter().map(rebase));
        out.transparent_draws
            .extend(model.transparent_draws.into_iter().map(rebase));
        out.textures.extend(model.textures);
        out.mesh_count += model.mesh_count;
    }

    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for v in &out.vertices {
        for i in 0..3 {
            lo[i] = lo[i].min(v.position[i]);
            hi[i] = hi[i].max(v.position[i]);
        }
    }
    if !out.vertices.is_empty() {
        out.centre = [
            (lo[0] + hi[0]) * 0.5,
            (lo[1] + hi[1]) * 0.5,
            (lo[2] + hi[2]) * 0.5,
        ];
        out.radius = (0..3)
            .map(|i| (hi[i] - lo[i]) * 0.5)
            .fold(0.0f32, f32::max)
            .max(0.001);
    }
    // The original queues every model's batch sets into **one** queue and sorts
    // that, so a merge is exactly the case where sorting across sources is the
    // faithful thing rather than an over-reach - see `Model::sort_by_layer`.
    out.sort_by_layer();
    out
}

#[cfg(test)]
mod merge_tests {
    use super::*;

    fn model(label: &str, vertices: usize, textures: usize) -> Model {
        Model {
            airbrakes: [None, None],
            node_vertex_ranges: Vec::new(),
            label: label.to_string(),
            vertices: (0..vertices)
                .map(|k| GpuVertex {
                    position: [k as f32, 0.0, 0.0],
                    normal: [0.0, 1.0, 0.0],
                    colour: [1.0; 4],
                    texcoord: [0.0; 2],
                    lightmap_texcoord: [0.0, 0.0],
                    lit: 1.0,
                    anim: 0,
                })
                .collect(),
            indices: (0..vertices as u32).collect(),
            draws: vec![DrawCall {
                blend: None,
                blend_state: None,
                layer: oag_formats::vex::LAYER_DEFAULT,
                culled: false,
                range: 0..vertices as u32,
                texture: (textures > 0).then_some(0),
                bounds: Bounds {
                    centre: [0.0; 3],
                    radius: 1.0,
                },
                node: None,
            }],
            alpha_tested_draws: Vec::new(),
            transparent_draws: Vec::new(),
            textures: (0..textures).map(|_| None).collect(),
            lightmaps: Vec::new(),
            centre: [0.0; 3],
            radius: 1.0,
            anim_tracks: Vec::new(),
            mesh_count: 1,
        }
    }

    /// Indices are positions in a shared buffer, so the second model's have to
    /// be rebased or it draws the first model's geometry twice.
    #[test]
    fn indices_are_rebased_onto_the_combined_buffer() {
        let out = merge("both", vec![model("a", 3, 0), model("b", 3, 0)]);
        assert_eq!(out.vertices.len(), 6);
        assert_eq!(out.indices, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(out.draws[1].range, 3..6);
    }

    /// A material names a texture by its ordinal, so the slot index has to move
    /// with the slots or the second model draws the first model's skin.
    #[test]
    fn texture_slots_are_rebased_too() {
        let out = merge("both", vec![model("a", 3, 2), model("b", 3, 1)]);
        assert_eq!(out.textures.len(), 3);
        assert_eq!(out.draws[1].texture, Some(2));
    }

    #[test]
    fn empty_models_contribute_nothing() {
        let out = merge("one", vec![model("a", 3, 0), model("empty", 0, 0)]);
        assert_eq!(out.draws.len(), 1);
        assert_eq!(out.vertices.len(), 3);
    }

    #[test]
    fn merging_nothing_is_still_a_usable_model() {
        let out = merge("none", Vec::new());
        assert!(out.vertices.is_empty());
        assert!(out.radius > 0.0);
    }
}
