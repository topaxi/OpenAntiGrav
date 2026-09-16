//! [`Clouds`]: `05_Track`'s cloud puffs on the GPU side of [`super::Scene`].
//!
//! The state and the maths - what a sprite is, how it rotates, what it draws
//! with what colour - are all `oag_render::cloud`'s; see that module's own
//! doc and `docs/ghidra/functions/psp-pulse-usa/clouds.md` for the evidence.
//! This is only the seam: building the pipeline once at load, and gating the
//! per-tick advance the way [`super::motion::MotionState`] already gates its
//! own "a frame rate above 60 Hz re-renders one tick" case, so the rotation
//! follows the simulation tick rather than how often [`super::Scene::render`]
//! happens to be called.
//!
//! Split into its own file so a new field on [`super::Scene`] costs this
//! module a handful of lines rather than `scene.rs` a dozen - see
//! `scripts/check-file-size.py`.

use super::*;

/// `05_Track`'s cloud pipeline and the tick its sprites last advanced at, or
/// nothing at all for every other circuit and for a ribbon build.
#[derive(Debug, Default)]
pub(super) struct Clouds {
    state: Option<(oag_render::cloud::Layer, oag_render::cloud::Pipeline)>,
    tick: Option<u64>,
}

impl Clouds {
    /// Builds the pipeline from what `crate::race::load::environment::cloud_layer`
    /// found, or stays empty. Wrapped in the `RefCell` [`super::Scene`] wants,
    /// so the call at the one site this has stays on one line.
    pub(super) fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        sample_count: u32,
        source: Option<(oag_render::cloud::Layer, FlareTexture)>,
    ) -> std::cell::RefCell<Self> {
        let state = source.map(|(layer, texture)| {
            let pipeline = oag_render::cloud::Pipeline::new(
                device,
                queue,
                format,
                &texture,
                sample_count,
                mesh_render::Velocity::Write,
            );
            (layer, pipeline)
        });
        std::cell::RefCell::new(Self { state, tick: None })
    }

    /// Advances at most once per simulation tick and uploads this frame's
    /// vertices. A no-op when this circuit authors no cloud node.
    pub(super) fn upload(
        &mut self,
        queue: &wgpu::Queue,
        view_projection: &[[f32; 4]; 4],
        tick: u64,
        right: Vec3,
        up: Vec3,
    ) {
        let Some((layer, pipeline)) = self.state.as_mut() else {
            return;
        };
        // `None` only on the very first call - nothing to advance *from* yet,
        // so the layer stays at the phase it was built with.
        let previous = self.tick.replace(tick);
        if previous.is_some() && previous != Some(tick) {
            layer.advance();
        }
        let mut vertices = Vec::new();
        layer.extend_vertices(&mut vertices, right, up);
        pipeline.upload(queue, view_projection, &vertices);
    }

    /// Draws into a pass the caller already opened. Depth-tested but not
    /// depth-writing, the same as `exhaust::Pipeline::draw` and for the same
    /// reason - see `oag_render::cloud::Pipeline::draw`.
    pub(super) fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some((_, pipeline)) = &self.state {
            pipeline.draw(pass);
        }
    }
}
