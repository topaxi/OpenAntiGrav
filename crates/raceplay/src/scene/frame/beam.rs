//! The LeachBeam ribbon's own per-frame upload and draw, pulled out of
//! `frame.rs` under the 1,000-line rule in `scripts/check-file-size.py`, the
//! seam `frame/shadow.rs` already set - a move, with no behaviour change.
//! `write_leach_ball` and `draw_leach_ball` joined it 2026-09-25: a second
//! LeachBeam drawable, not a move.

use oag_core::math::Mat4;
use oag_mesh::mesh_render;

use crate::{Race, Uploads};

impl super::super::Scene {
    /// Uploads this frame's ribbon geometry, or nothing when
    /// [`super::super::Scene::beam`] is `None` (the texture did not decode)
    /// or [`Race::leach_beam_ribbon_vertices`] is empty (no locked beam this
    /// tick) - either way `Pipeline::draw` then draws nothing.
    ///
    /// Reads the camera's own right and up out of the view matrix, the same
    /// way `frame.rs` does for its sprites: `LeachBeam_BuildStrip` widens its
    /// two strips along view-space `x` and `y`.
    pub(super) fn upload_beam(
        &self,
        race: &Race,
        queue: &dyn oag_gpu::writes::Writes,
        vp: &[[f32; 4]; 4],
    ) {
        if let Some(beam) = &self.beam {
            let camera = race.view();
            let right =
                oag_core::math::Vec3::new(camera.x_axis.x, camera.y_axis.x, camera.z_axis.x);
            let up = oag_core::math::Vec3::new(camera.x_axis.y, camera.y_axis.y, camera.z_axis.y);
            // HD draws its own strip (`Scene::ribbons`) where it authors one.
            let vertices = if race.has_leach_strip() {
                Vec::new()
            } else {
                race.leach_beam_ribbon_vertices(right, up)
            };
            beam.borrow_mut().upload(queue, vp, &vertices);
        }
    }

    /// Writes this tick's transform onto the LeachBall models, one per live
    /// locked beam in pool order, and returns how many it wrote for the bounded
    /// draw below. Zero when [`super::super::Scene::leach_ball`] is empty (the
    /// model did not load) or no beam is locked.
    ///
    /// **No previous-frame matrix tracked**, the same choice
    /// [`super::super::Scene::write_plasma_blasts`] already makes for its
    /// own short-lived pool: `prev_mvp` is this frame's own
    /// `view_projection * matrix`, which zeroes the shader's velocity term
    /// rather than smearing from a frame this slot never recorded.
    /// Chosen, not measured - the ball reaches its own top speed inside one
    /// drain trip's period (`0.3`-`1.0` s), well inside a velocity buffer's
    /// blur window, so an unsmeared frame at each reveal is the safer
    /// default over an invented "previous" pose.
    pub(super) fn write_leach_ball(
        &self,
        race: &Race,
        queue: &Uploads<'_>,
        view_projection: Mat4,
        weapon_scene: &mesh_render::Scene,
    ) -> usize {
        let matrices = (0..oag_weapons::projectile::leach_beam::POOL_SIZE)
            .filter_map(|entry| race.leach_ball_model_matrix(entry));
        let mut written = 0;
        for (drawable, matrix) in self.leach_ball.iter().zip(matrices) {
            let mvp = view_projection * matrix;
            drawable.write(queue, view_projection, matrix, mvp);
            super::super::weapon_models::write_fog(drawable, queue, weapon_scene);
            written += 1;
        }
        written
    }

    /// Draws the first `written` LeachBall models, [`Self::write_leach_ball`]'s
    /// own return.
    pub(super) fn draw_leach_balls<'a>(
        &'a self,
        written: usize,
        list: &mut super::super::record::DrawList<'a>,
    ) {
        for drawable in self.leach_ball.iter().take(written) {
            list.mesh(drawable);
        }
    }
}
