//! The circuit's caustic pass: Pulse's flagged batches redrawn under the ripple frame the world
//! clock picks (`oag_mesh::mesh::caustic_pass`).
//!
//! The pass is the circuit's shine pass over again - a drawable over the batches' own vertices
//! that redraws what the circuit shows - with sixteen sets of draws, one per picture, of which
//! [`TrackCaustic::frame_draws`] hands the recording the set the clock names. Its coordinates are
//! a function of position alone, so nothing is streamed.

use super::*;

/// `GU_ADD`, `GU_SRC_ALPHA` / `GU_FIX 0xffffff`: what `Gfx_BuildBatchStateList` programs for the
/// pass's `pass_mask` `0x202` (`BlendClass::Additive`, `docs/ghidra/functions/psp-pulse-usa/
/// mesh-draw.md`). The alpha channel is the bloom's glow mask and is left as it was, as the
/// shine pass leaves it.
pub(super) const BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// A circuit's caustic pass: its drawable, and for each draw the circuit draw it redraws.
#[derive(Debug)]
pub(super) struct TrackCaustic {
    drawable: Drawable,
    sources: Vec<usize>,
    per_frame: usize,
}

impl TrackCaustic {
    /// `None` when the circuit's flagged batches have no frames (the setting is `off`, the
    /// title or the disc has none) or none is drawable.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        track: &Model,
        format: wgpu::TextureFormat,
        store: oag_gpu::formats::SceneStore,
        anisotropy: Anisotropy,
        sample_count: u32,
        zone_art: &mesh_render::zone::StageArt,
        shadow_maps: mesh_render::ShadowMaps<'_>,
    ) -> Result<Option<Self>> {
        let Some(pass) = oag_mesh::mesh::caustic_pass::build(track) else {
            return Ok(None);
        };
        let drawable = Drawable::new_with(
            device,
            queue,
            pass.model,
            format,
            store,
            anisotropy,
            sample_count,
            mesh_render::Depth::Overlay,
            BLEND,
            mesh_render::GlowMask::Written,
            zone_art,
            shadow_maps,
            mesh_render::ShadowReceiver::Never,
            mesh_render::Texcoords::Interleaved,
            false,
            false,
            mesh_render::Velocity::Write,
            true,
        )?;
        Ok(Some(Self {
            drawable,
            sources: pass.sources,
            per_frame: pass.per_frame,
        }))
    }

    /// The pass's drawable, and the circuit draws the frame at `seconds` redraws, with the index
    /// of the first of its draws: what recording reads.
    pub(super) fn frame_draws(&self, seconds: f32) -> (&Drawable, &[usize], usize) {
        let first = oag_mesh::mesh::caustic_pass::frame(seconds) * self.per_frame;
        (
            &self.drawable,
            &self.sources[first..first + self.per_frame],
            first,
        )
    }

    /// Moves the pass's matrices onto the shared buffer.
    pub(super) fn share_uniforms(&mut self, device: &wgpu::Device, blocks: &mut UniformBlocks) {
        self.drawable.share_uniforms(device, blocks);
    }

    /// Moves the pass onto the shine block: black fog, an additive pass fading with distance.
    pub(super) fn share_scene(&mut self, device: &wgpu::Device, blocks: &mut SceneBlocks) {
        self.drawable.share_scene(device, blocks, SceneKind::Shine);
    }
}

impl Scene {
    /// Writes the pass's matrices for this view.
    pub(super) fn write_caustic(&self, queue: &Uploads<'_>, view_projection: Mat4, prev_vp: Mat4) {
        if let Some(caustic) = &self.track_caustic {
            caustic
                .drawable
                .write(queue, view_projection, Mat4::IDENTITY, prev_vp);
        }
    }

    /// What the recording draws of the pass this frame.
    pub(super) fn caustic_frame(&self) -> Option<(&Drawable, &[usize], usize)> {
        self.track_caustic
            .as_ref()
            .map(|c| c.frame_draws(self.anim_clock.get()))
    }
}
