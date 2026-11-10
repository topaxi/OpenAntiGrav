//! The scene's seat of [`oag_fx::mod_shader::Stage`]: built once the race is known, uploaded per
//! view. The stage's draws are `Recorder::draw_effects`'.

use oag_core::math::Vec3;

use crate::Race;

use super::{Scene, mesh_render};

impl Scene {
    /// Builds the race's shaders; a race with none builds nothing and costs nothing. What the
    /// device refuses goes to the log and draws nothing.
    pub fn attach_mod_shaders(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        shaders: &[Option<oag_fx::mod_shader::Shader>],
    ) {
        *self.portal.borrow_mut() = None;
        *self.mod_shaders.borrow_mut() = (!shaders.is_empty()).then(|| {
            let stage = oag_fx::mod_shader::Stage::new(
                device,
                queue,
                self.pass_format,
                self.store(),
                self.sample_count(),
                mesh_render::Velocity::Write,
                shaders,
            );
            for failure in stage.failures() {
                log::warn!("mod {failure}");
            }
            stage
        });
    }

    /// Writes the view's surfaces, fill and scalars.
    pub(super) fn upload_mod_shaders(
        &self,
        race: &Race,
        queue: &dyn oag_gpu::writes::Writes,
        vp: &[[f32; 4]; 4],
    ) {
        let mut stage = self.mod_shaders.borrow_mut();
        let Some(stage) = stage.as_mut() else {
            return;
        };
        let camera = race.view();
        let right = Vec3::new(camera.x_axis.x, camera.y_axis.x, camera.z_axis.x);
        let up = Vec3::new(camera.x_axis.y, camera.y_axis.y, camera.z_axis.y);
        let eye = camera.inverse().w_axis.truncate();
        // The projection alone, for `eye_ray`: what is left of `vp` once the view is out.
        let projection = oag_core::math::Mat4::from_cols_array_2d(vp) * camera.inverse();
        let lens = [1.0 / projection.x_axis.x, 1.0 / projection.y_axis.y];
        stage.upload(
            queue,
            &oag_fx::mod_shader::Frame {
                view_projection: *vp,
                engine_view_projection: self.portal_engine_of(race),
                craft: (camera * race.craft_frame()).to_cols_array_2d(),
                lens,
                // The offset and the flyby are the world meshes' epoch; a mod counts race time.
                time: self.anim_clock.get() - race.world_clock_offset() - race.flyby_seconds(),
                draw_alpha: race.draw_alpha(),
                fill: race.mod_fill(),
                surfaces: &race.mod_surfaces(right, up, eye),
                params: &race.mod_shader_params(),
            },
        );
    }
}
