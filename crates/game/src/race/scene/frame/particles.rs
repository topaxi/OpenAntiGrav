//! The `.pob` particle upload - collision sparks and the stage's effects -
//! pulled out of `frame.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, the seam `frame/beam.rs` and
//! `frame/shadow.rs` already set - a move, with no behaviour change.

use oag_core::math::Vec3;
use oag_render::mesh::GpuVertex;

use crate::race::Race;

impl super::super::Scene {
    /// Gathers and uploads this frame's particle geometry.
    ///
    /// The hull's collision sparks and the stage's rocket effects share one
    /// pipeline and one pair of buffers: both are `.pob` particles in the
    /// same two blend classes, so a second pipeline would buy nothing.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn upload_particles(
        &self,
        race: &Race,
        queue: &wgpu::Queue,
        vp: &[[f32; 4]; 4],
        right: Vec3,
        up: Vec3,
        additive: &mut Vec<GpuVertex>,
        alpha: &mut Vec<GpuVertex>,
    ) {
        race.extend_spark_vertices(additive, alpha, right, up);
        race.extend_stage_vertices(additive, alpha, right, up);
        self.sparks.borrow_mut().upload(queue, vp, additive, alpha);
    }
}
