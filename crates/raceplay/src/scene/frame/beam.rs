//! The LeachBeam ribbon's own per-frame upload and draw, pulled out of
//! `frame.rs` under the 1,000-line rule in `scripts/check-file-size.py`, the
//! seam `frame/shadow.rs` already set - a move, with no behaviour change.
//! `write_leach_ball` and `draw_leach_ball` joined it 2026-09-25: a second
//! LeachBeam drawable, not a move.

use oag_core::math::Mat4;
use oag_mesh::mesh_render;

use crate::{Race, SceneStats};

impl super::super::Scene {
    /// Uploads this frame's ribbon geometry, or nothing when
    /// [`super::super::Scene::beam`] is `None` (the texture did not decode)
    /// or [`Race::leach_beam_ribbon_vertices`] is empty (no locked beam this
    /// tick) - either way `Pipeline::draw` then draws nothing.
    ///
    /// Reads the camera's own right and up out of the view matrix, the same
    /// way `frame.rs` does for its sprites: `LeachBeam_BuildStrip` widens its
    /// two strips along view-space `x` and `y`.
    pub(super) fn upload_beam(&self, race: &Race, queue: &wgpu::Queue, vp: &[[f32; 4]; 4]) {
        if let Some(beam) = &self.beam {
            let camera = race.view();
            let right =
                oag_core::math::Vec3::new(camera.x_axis.x, camera.y_axis.x, camera.z_axis.x);
            let up = oag_core::math::Vec3::new(camera.x_axis.y, camera.y_axis.y, camera.z_axis.y);
            let vertices = race.leach_beam_ribbon_vertices(right, up);
            beam.borrow_mut().upload(queue, vp, &vertices);
        }
    }

    /// Draws the ribbon into a pass the caller already opened, after the
    /// hull's own depth is present to occlude it - the same ordering
    /// [`super::super::Scene::exhaust`] and [`super::super::Scene::sparks`]
    /// need and for the same reason.
    pub(in crate::scene) fn draw_beam(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(beam) = &self.beam {
            beam.borrow().draw(pass);
        }
    }

    /// Writes this tick's transform onto the LeachBall's own model, or
    /// leaves it unwritten - and reports so, for the bounded draw below -
    /// when [`super::super::Scene::leach_ball`] is `None` (the model did not
    /// load) or [`Race::leach_ball_model_matrix`] is `None` (no beam locked).
    ///
    /// **No previous-frame matrix tracked**, the same choice
    /// [`super::super::Scene::write_plasma_blasts`] already makes for its
    /// own short-lived pool: `prev_mvp` is this frame's own
    /// `view_projection * matrix`, which zeroes the shader's velocity term
    /// rather than smearing from a frame this single slot never recorded.
    /// Chosen, not measured - the ball reaches its own top speed inside one
    /// drain trip's period (`0.3`-`1.0` s), well inside a velocity buffer's
    /// blur window, so an unsmeared frame at each reveal is the safer
    /// default over an invented "previous" pose.
    pub(super) fn write_leach_ball(
        &self,
        race: &Race,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        weapon_scene: &mesh_render::Scene,
    ) -> bool {
        let (Some(drawable), Some(matrix)) = (&self.leach_ball, race.leach_ball_model_matrix())
        else {
            return false;
        };
        let mvp = view_projection * matrix;
        drawable.write(queue, view_projection, matrix, mvp);
        super::super::weapon_models::write_fog(drawable, queue, weapon_scene);
        true
    }

    /// Draws the LeachBall's own model, when [`Self::write_leach_ball`]'s
    /// own return says it was written this frame.
    pub(super) fn draw_leach_ball(
        &self,
        active: bool,
        pass: &mut wgpu::RenderPass<'_>,
        stats: &mut SceneStats,
    ) {
        if active && let Some(drawable) = &self.leach_ball {
            stats.add(drawable.draw(pass, None, None, None, None));
        }
    }
}
