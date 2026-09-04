//! Both shadow tiers' per-frame geometry: where each craft's blob quad goes,
//! and what its authored hull projects to.
//!
//! Split out of `frame.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, the seam `frame/craft.rs` and
//! `frame/attachments.rs` already set; a move, with no behaviour change.

use oag_core::math::Vec3;
use oag_render::mesh::GpuVertex;
use oag_render::shadow::Placement;

use crate::race::Race;

/// How dark a fully covered texel of the shadow map draws the track.
///
/// **Ours, and the same kind of number `HULL_DARKNESS` is.** Wipeout HD's own
/// track material computes `1 - shadow` into the fragment's *alpha* and a later
/// compositing pass consumes it; that pass is unread, so how dark the result
/// lands is not something this project can read off the disc. `0.5` is chosen
/// to sit where the craft reads as shadowed without the road going black under
/// it, and it is the first thing to replace once the compositing pass is read.
const MAP_STRENGTH: f32 = 0.5;

/// How far past the grid's own bounds the light's view is fitted.
///
/// **Ours.** The map covers the casters and nothing else - a track-sized
/// orthographic box at `shadow::map::SIZE` is metres per texel - and a small
/// margin keeps a craft that is banking or airborne inside it rather than
/// clipping its own shadow at the border.
const FIT_MARGIN: f32 = 6.0;

impl super::super::Scene {
    /// The quads and the projected hulls this frame draws, for the tier the
    /// caller is on.
    ///
    /// Both lists are empty at `off`, and each tier fills exactly one of them:
    /// they are alternatives, not layers.
    pub(super) fn shadow_geometry(
        &self,
        race: &Race,
        shadows: crate::display::Shadows,
    ) -> (Vec<Placement>, Vec<GpuVertex>) {
        // The blob shadows, gathered here with the rest of the per-frame
        // geometry and uploaded whether or not the tier is on - `upload` with
        // no placements clears the runs, which is what makes turning the row
        // off take effect on the next frame rather than leaving the last
        // frame's quads in the buffer.
        let placements = if shadows.draws() {
            race.shadow_placements()
        } else {
            Vec::new()
        };
        // The `original` tier draws the craft's own authored hull instead of a
        // textured quad, projected onto the same surface the placement found.
        // A slot whose model authors no hull contributes nothing rather than
        // falling back to a blob: a tier that silently becomes another tier is
        // how a missing feature stops being noticed. See
        // `oag_render::shadow::hull_triangles`, and
        // `oag_pulse::shadow::AUTHORED_AXIS` for the direction.
        let mut hull_vertices = Vec::new();
        if shadows == crate::display::Shadows::Original {
            for placement in &placements {
                let Some(Some(hull)) = self.shadow_hulls.get(placement.silhouette) else {
                    continue;
                };
                oag_render::shadow::hull_triangles(
                    &oag_render::shadow::Cast {
                        hull,
                        model: race.ship_model_matrix_of(placement.silhouette),
                        axis: Vec3::from_array(oag_pulse::shadow::AUTHORED_AXIS),
                        contact: placement.contact,
                        normal: placement.normal,
                        strength: placement.strength,
                    },
                    &mut hull_vertices,
                );
            }
        }
        // At `original` the quads are not drawn at all: the two tiers are
        // alternatives, not layers.
        let quads: &[Placement] = if shadows == crate::display::Shadows::Blob {
            &placements
        } else {
            &[]
        };
        (quads.to_vec(), hull_vertices)
    }
}

impl super::super::Scene {
    /// The scene uniform's shadow block for this frame.
    ///
    /// [`ShadowMap::off`] unless the `original` tier is on *and* the map has
    /// something in it: a strength of zero is what makes the sample every
    /// pipeline carries inert, so a title with no casters draws exactly as it
    /// did before the map existed.
    pub(super) fn shadow_uniform(
        &self,
        shadows: crate::display::Shadows,
    ) -> oag_render::mesh_render::ShadowMap {
        let map = self.shadow_map.borrow();
        if shadows != crate::display::Shadows::Original || map.casters() == 0 {
            return oag_render::mesh_render::ShadowMap::off();
        }
        oag_render::mesh_render::ShadowMap {
            matrix: map.matrix().to_cols_array_2d(),
            strength: MAP_STRENGTH,
            _pad: [0.0; 3],
        }
    }

    /// Renders every craft into the shadow map, for a title whose `original`
    /// tier is a shadow map.
    ///
    /// **Wipeout HD only, and by data rather than by name**: the casters are
    /// the craft, the receiver is the track, and the tier draws nothing at all
    /// where the craft author their own occluder hulls instead - that is
    /// Pulse's mechanism and `hull_triangles` is where it lives. A title with
    /// neither gets an empty map and the report has already said so.
    ///
    /// Returns how many casters it drew, so a caller can tell a cleared map
    /// from one that was never rendered.
    pub(super) fn render_shadow_map(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        race: &Race,
        shadows: crate::display::Shadows,
    ) -> usize {
        let mut map = self.shadow_map.borrow_mut();
        // A hull the craft authors is Pulse's mechanism; if any slot has one,
        // this title shadows that way and not this one.
        let hulls = self.shadow_hulls.iter().any(Option::is_some);
        if shadows != crate::display::Shadows::Original || hulls {
            map.render(queue, encoder, &default_fit(), &[]);
            return 0;
        }
        let drawn = usize::from(race.ship_count());
        let mut casters = Vec::with_capacity(drawn);
        let mut centre = Vec3::ZERO;
        let mut count = 0.0;
        for slot in 0..drawn.min(self.ships.len()) {
            if !race.ship_active(slot) {
                continue;
            }
            let model = race.ship_model_matrix_of(slot);
            casters.push(self.ships[slot].caster(model));
            centre += model.transform_point3(Vec3::ZERO);
            count += 1.0;
        }
        if casters.is_empty() {
            map.render(queue, encoder, &default_fit(), &[]);
            return 0;
        }
        centre /= count;
        // Fitted to the grid's own spread **plus each craft's own size**, and
        // not to the track - see `shadow::map::Fit`. Leaving the hull's radius
        // out is not a small error: with one craft it makes the box smaller
        // than the caster, and the shadow comes out the size of the overlap.
        let mut radius = FIT_MARGIN;
        for slot in 0..drawn.min(self.ships.len()) {
            if !race.ship_active(slot) {
                continue;
            }
            let model = race.ship_model_matrix_of(slot);
            let at = model.transform_point3(Vec3::ZERO);
            // The matrix carries the craft's own render scale, so the world
            // radius is the model's through it - see
            // `oag_render::exhaust::CRAFT_ROW_SCALE`.
            let scale = model.x_axis.truncate().length();
            let hull = self.ships[slot].radius() * scale;
            radius = radius.max((at - centre).length() + hull + FIT_MARGIN);
        }
        let fit = oag_render::shadow::map::Fit {
            centre,
            radius,
            // The circuit's own sun, which HD authors in its `.envsettings`
            // and this renderer already lights through. Not a direction of
            // this module's own.
            towards_light: Vec3::from_array(self.light.direction),
        };
        map.render(queue, encoder, &fit, &casters);
        casters.len()
    }
}

/// The fit an empty pass uses: anywhere, one unit across, straight down.
///
/// A cleared map needs a projection for nothing to be drawn with, and this is
/// the one that cannot accidentally cover a receiver - `shadow_uniform` returns
/// `off` for the same frame regardless.
fn default_fit() -> oag_render::shadow::map::Fit {
    oag_render::shadow::map::Fit {
        centre: Vec3::ZERO,
        radius: 1.0,
        towards_light: Vec3::Y,
    }
}
