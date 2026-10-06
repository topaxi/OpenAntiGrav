//! The `.pob` particle upload - collision sparks and the stage's effects -
//! pulled out of `frame.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, the seam `frame/beam.rs` and
//! `frame/shadow.rs` already set - a move, with no behaviour change.

use oag_core::math::Vec3;
use oag_mesh::mesh::GpuVertex;

use crate::Race;

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
        let mut pipeline = self.sparks.borrow_mut();
        // The effects' own sprites - a no-op after the first frame of a race.
        pipeline.sync_sheet(queue, race.view.effects.sheet());
        pipeline.upload(queue, vp, additive, alpha);
        let mut distort = Vec::new();
        race.extend_distort_vertices(&mut distort, right, up);
        pipeline.upload_distort(queue, &distort);
        let flash = race.view.screen_flash.as_ref().and_then(|f| f.colour());
        pipeline.upload_flash(queue, flash);
    }
}

impl super::super::Scene {
    /// Omega's tone map and composite over the frame the race pass just
    /// drew, with the offsets blend class 8 wrote this frame.
    ///
    /// The offset pass runs here, after the scene pass has closed, because it
    /// reads that pass's depth attachment: the executable's pass 9 follows its
    /// scene pass the same way. A frame with no class 8 particle alive draws
    /// no pass and the composite samples a zero texture.
    pub(super) fn run_omega(
        &self,
        omega: &oag_post::omega_tonemap::Chain,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
    ) {
        let depth = self.depth.size();
        let mut sparks = self.sparks.borrow_mut();
        let offsets = sparks.encode_distort(
            device,
            encoder,
            &self.attachment_views.depth,
            (depth.width, depth.height),
            viewport,
        );
        let rect = (viewport.2 as u32, viewport.3 as u32);
        omega.run(
            device,
            queue,
            encoder,
            view,
            (viewport.0, viewport.1),
            rect,
            offsets,
        );
    }
}
