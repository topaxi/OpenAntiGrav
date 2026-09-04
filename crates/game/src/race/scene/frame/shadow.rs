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
