//! The track as a craft's sun-occlusion map is rendered from it, and the
//! diagnostic listing behind `OAG_DUMP_SUN_OCCLUSION` - see
//! `oag_render::shadow::occlusion` and `race::scene::frame::shadow`.
//!
//! Split out of `drawable.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use oag_core::math::{Mat4, Vec3};

use super::Drawable;

impl Drawable {
    /// This drawable as the track a craft's sun-occlusion map is rendered
    /// from: its buffers, its opaque, cutout and transparent draw calls with
    /// their bounds, and its own material bind groups - see
    /// `oag_render::shadow::occlusion::Track`. Borrowed, like
    /// [`Drawable::caster`].
    pub(in crate::race) fn occlusion_track(&self) -> oag_render::shadow::occlusion::Track<'_> {
        oag_render::shadow::occlusion::Track {
            vertices: &self.vertices,
            indices: &self.indices,
            draws: &self.model.draws,
            cutouts: &self.model.alpha_tested_draws,
            transparent: &self.model.transparent_draws,
            materials: &self.textures,
            model: Mat4::IDENTITY,
        }
    }

    /// One line per draw call of this model that `oag_render::shadow::occlusion`
    /// would draw for a craft at `centre` under `sun`: what a sun-occlusion
    /// map is made of, for the `OAG_DUMP_SUN_OCCLUSION` knob.
    pub(in crate::race) fn describe_occlusion_draws(&self, centre: Vec3, sun: Vec3) -> Vec<String> {
        self.model
            .draws
            .iter()
            .map(|draw| (draw, "opaque"))
            .chain(self.model.alpha_tested_draws.iter().map(|draw| (draw, "cutout")))
            .chain(self.model.transparent_draws.iter().map(|draw| (draw, "blend")))
            .filter(|(draw, _)| oag_render::shadow::occlusion::within(draw, centre, sun))
            .map(|(draw, kind)| {
                let label = draw
                    .texture
                    .and_then(|index| self.model.textures.get(index))
                    .and_then(Option::as_ref)
                    .map_or("untextured", |texture| texture.label.as_str());
                let lightmap = draw
                    .texture
                    .and_then(|index| self.model.lightmaps.get(index))
                    .and_then(Option::as_ref)
                    .map_or("-", |texture| texture.label.as_str());
                let slots = draw
                    .texture
                    .and_then(|index| self.model.material_slots.get(index))
                    .copied()
                    .unwrap_or(oag_mesh::mesh::slots::DEFAULT);
                let variant = draw
                    .texture
                    .and_then(|index| self.model.material_variants.get(index))
                    .and_then(Option::as_ref)
                    .map_or_else(|| "?".to_string(), |variant| format!("{:#x}", variant.feature_hash));
                let mask = self.model.vertices[self.model.indices[draw.range.start as usize] as usize]
                    .sun_mask;
                let [x, y, z] = draw.bounds.centre;
                format!(
                    "{kind} chunk {:?} {label} lightmap={lightmap} slots={slots:#x} variant={variant} sun_mask={mask} centre=({x:.1},{y:.1},{z:.1}) r={:.1} tris={}",
                    draw.chunk,
                    draw.bounds.radius,
                    draw.range.len() / 3
                )
            })
            .collect()
    }
}
