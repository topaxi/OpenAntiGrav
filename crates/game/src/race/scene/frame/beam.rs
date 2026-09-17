//! The LeachBeam ribbon's own per-frame upload and draw, pulled out of
//! `frame.rs` under the 1,000-line rule in `scripts/check-file-size.py`, the
//! seam `frame/shadow.rs` already set - a move, with no behaviour change.

use crate::race::Race;

impl super::super::Scene {
    /// Uploads this frame's ribbon geometry, or nothing when
    /// [`super::super::Scene::beam`] is `None` (the texture did not decode)
    /// or [`Race::leach_beam_ribbon_vertices`] is empty (no locked beam this
    /// tick) - either way `Pipeline::draw` then draws nothing.
    ///
    /// World-space already, unlike every other pipeline this frame uploads,
    /// so it needs no `right`/`up` camera basis.
    pub(super) fn upload_beam(&self, race: &Race, queue: &wgpu::Queue, vp: &[[f32; 4]; 4]) {
        if let Some(beam) = &self.beam {
            let vertices = race.leach_beam_ribbon_vertices();
            beam.borrow_mut().upload(queue, vp, &vertices);
        }
    }

    /// Draws the ribbon into a pass the caller already opened, after the
    /// hull's own depth is present to occlude it - the same ordering
    /// [`super::super::Scene::exhaust`] and [`super::super::Scene::sparks`]
    /// need and for the same reason.
    pub(super) fn draw_beam(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(beam) = &self.beam {
            beam.borrow().draw(pass);
        }
    }
}
