//! The `.pob` particle upload - collision sparks and the stage's effects -
//! pulled out of `frame.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, the seam `frame/beam.rs` and
//! `frame/shadow.rs` already set - a move, with no behaviour change.

use oag_core::math::Vec3;
use oag_mesh::mesh::GpuVertex;

use crate::Race;

/// Vertices per particle quad: two triangles, `oag_fx::psys`'s `quad`.
const QUAD: usize = 6;

/// Removes, in place and in order, every quad whose colour alpha is exactly 0 at all six vertices.
///
/// A rocket that strikes a craft near the camera stacks dozens of screens of particle fill at
/// full resolution (`docs/rendering/particle-fill.md`), and a fading particle spends the end of
/// its life at colour alpha exactly 0, still drawn. `psys.wesl` multiplies its output alpha by
/// the interpolated vertex alpha, so such a quad writes `dst` back under both of its blends (the
/// additive's `rgb * a`, the over's `a` and `1 - a`) and writes no depth or velocity. One live
/// corner keeps the quad. Order is kept, because the over blend is not commutative. A list that
/// is not whole quads is left alone.
fn drop_dead_quads(vertices: &mut Vec<GpuVertex>) {
    if !vertices.len().is_multiple_of(QUAD) {
        return;
    }
    let mut kept = 0;
    for start in (0..vertices.len()).step_by(QUAD) {
        if vertices[start..start + QUAD]
            .iter()
            .all(|v| v.colour[3] == 0.0)
        {
            continue;
        }
        vertices.copy_within(start..start + QUAD, kept);
        kept += QUAD;
    }
    vertices.truncate(kept);
}

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
        queue: &dyn oag_gpu::writes::Writes,
        vp: &[[f32; 4]; 4],
        right: Vec3,
        up: Vec3,
        additive: &mut Vec<GpuVertex>,
        alpha: &mut Vec<GpuVertex>,
    ) {
        race.extend_spark_vertices(additive, alpha, right, up);
        race.extend_stage_vertices(additive, alpha, right, up);
        drop_dead_quads(additive);
        drop_dead_quads(alpha);
        let mut pipeline = self.sparks.borrow_mut();
        // The effects' own sprites - a no-op after the first frame of a race.
        pipeline.sync_sheet(queue, race.view.effects.sheet());
        pipeline.upload(queue, vp, race.view.draw_alpha(), additive, alpha);
        let mut underlay = Vec::new();
        race.extend_underlay_vertices(&mut underlay, right, up);
        drop_dead_quads(&mut underlay);
        pipeline.upload_underlay(queue, &underlay);
        let mut distort = Vec::new();
        race.extend_distort_vertices(&mut distort, right, up);
        pipeline.upload_distort(queue, &distort);
        let flash = race.screen_wash();
        pipeline.upload_flash(queue, flash);
        // The hit static's noise is the race's, written once; the strips are this view's.
        if let Some(noise) = race.cockpit_hit_noise() {
            pipeline.sync_cockpit_hit(queue, noise);
        }
        pipeline.upload_cockpit_hit(queue, race.cockpit_hit_strips());
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
    ///
    /// `distort` is false for the one run a split screen makes over all its views:
    /// the offset pass reads the depth attachment, which holds the last view's
    /// depth alone by then, so that run draws no offsets.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::scene) fn run_omega(
        &self,
        omega: &oag_post::omega_tonemap::Chain,
        device: &wgpu::Device,
        queue: &dyn oag_gpu::writes::Writes,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        distort: bool,
    ) {
        let depth = self.depth.size();
        let mut sparks = self.sparks.borrow_mut();
        let offsets = if distort {
            sparks.encode_distort(
                device,
                encoder,
                &self.attachment_views.depth,
                (depth.width, depth.height),
                viewport,
            )
        } else {
            None
        };
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

#[cfg(test)]
mod tests {
    use super::{GpuVertex, QUAD, drop_dead_quads};

    /// One quad whose six corners all carry `colour`, tagged by `x` so order can be read back.
    fn quad(x: f32, colour: [f32; 4]) -> [GpuVertex; QUAD] {
        [GpuVertex {
            position: [x, 0.0, 0.0],
            colour,
            ..bytemuck::Zeroable::zeroed()
        }; QUAD]
    }

    fn tags(vertices: &[GpuVertex]) -> Vec<f32> {
        vertices.chunks(QUAD).map(|q| q[0].position[0]).collect()
    }

    #[test]
    fn a_quad_with_alpha_zero_is_dropped_and_the_rest_keep_their_order() {
        let mut vertices: Vec<GpuVertex> = [
            quad(1.0, [1.0, 0.5, 0.2, 0.8]),
            quad(2.0, [1.0, 0.5, 0.2, 0.0]),
            // Black smoke with alpha darkens what is behind it under the over blend.
            quad(3.0, [0.0, 0.0, 0.0, 0.7]),
            quad(4.0, [1.0, 1.0, 1.0, 0.0]),
            quad(5.0, [0.3, 0.3, 0.3, 0.1]),
        ]
        .concat();
        drop_dead_quads(&mut vertices);
        assert_eq!(tags(&vertices), [1.0, 3.0, 5.0]);
    }

    #[test]
    fn one_live_corner_keeps_the_quad() {
        let mut fading = quad(2.0, [1.0, 1.0, 1.0, 0.0]);
        fading[5].colour[3] = 0.25;
        let mut vertices = fading.to_vec();
        drop_dead_quads(&mut vertices);
        assert_eq!(tags(&vertices), [2.0]);
    }

    #[test]
    fn a_list_that_is_not_whole_quads_is_left_alone() {
        let mut ragged = quad(1.0, [1.0, 1.0, 1.0, 0.0]).to_vec();
        ragged.pop();
        drop_dead_quads(&mut ragged);
        assert_eq!(ragged.len(), QUAD - 1);
    }
}
