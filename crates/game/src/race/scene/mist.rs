//! The weather's mist overlay on the GPU side of [`super::Scene`]: built once
//! from the circuit's decoded `Tex`, uploaded each frame from
//! [`crate::race::scenery_fx::mist::Mist`]. See `oag_fx::mist`.

use super::*;

impl Scene {
    /// Builds the mist pipeline when `texture` is the circuit's decoded mist
    /// texture; a no-op on `None`, every circuit without one.
    pub fn attach_mist(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        texture: Option<&FlareTexture>,
    ) {
        *self.mist.borrow_mut() = texture.map(|texture| {
            oag_fx::mist::Pipeline::new(
                device,
                queue,
                format,
                texture,
                self.sample_count(),
                mesh_render::Velocity::Write,
            )
        });
    }

    /// Uploads this frame's two quads. `tan_half_fov` is the drawn camera's
    /// vertical half-angle tangent, `g_camera_tan_half_fov`'s counterpart.
    pub(super) fn upload_mist(&self, race: &Race, queue: &wgpu::Queue, tan_half_fov: f32) {
        let mut pipeline = self.mist.borrow_mut();
        let Some(pipeline) = pipeline.as_mut() else {
            return;
        };
        let vertices = race
            .view
            .scenery_fx
            .weather()
            .mist()
            .and_then(|mist| mist.vertices(tan_half_fov));
        pipeline.upload(queue, vertices.as_ref());
    }
}
