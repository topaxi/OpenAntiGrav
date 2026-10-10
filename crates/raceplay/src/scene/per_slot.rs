//! One [`Drawable`] per grid slot out of each slot's livery - the shape the
//! shield shell and the absorb hull overlay share.
//!
//! Split out of `scene.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; the shield's loop moved here unchanged.

use super::*;

/// Everything every per-slot drawable is built with besides its model.
pub(super) struct Build<'a> {
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) liveries: &'a [oag_livery::Livery],
    pub(super) format: wgpu::TextureFormat,
    pub(super) anisotropy: Anisotropy,
    pub(super) sample_count: u32,
    pub(super) zone_art: &'a mesh_render::zone::StageArt,
    pub(super) shadow_maps: mesh_render::ShadowMaps<'a>,
}

impl Build<'_> {
    /// The absorb hull overlay per slot: the hull again, unshadowed, tested
    /// `LessEqual` against its own depth, through
    /// [`oag_fx::hull_overlay::BLEND`] - see `absorb_overlay`.
    pub(super) fn absorb_overlays(&self) -> Result<Vec<Option<Drawable>>> {
        self.drawables(
            |l| l.absorb_overlay.clone(),
            oag_fx::hull_overlay::BLEND,
            mesh_render::Depth::Overlay,
            mesh_render::ShadowReceiver::Never,
        )
    }

    /// The LeachBeam hull overlay per slot: the same drawable shape as
    /// [`Self::absorb_overlays`] under `leachbeam_surface.mip` - one draw
    /// routine, `HullOverlay_Submit`, serves both - see `leach_overlay`.
    pub(super) fn leach_overlays(&self) -> Result<Vec<Option<Drawable>>> {
        self.drawables(
            |l| l.leach_overlay.clone(),
            oag_fx::hull_overlay::BLEND,
            mesh_render::Depth::Overlay,
            mesh_render::ShadowReceiver::Never,
        )
    }

    /// The hull's extra pass per slot: the `0x2000` batches again under their
    /// second texture, tested `LessEqual` against the hull's own depth and
    /// added to it through [`oag_render::shine::BLEND`] - see `shine`.
    pub(super) fn shines(&self) -> Result<Vec<Option<Drawable>>> {
        self.drawables(
            |l| l.shine.clone(),
            oag_render::shine::BLEND,
            mesh_render::Depth::Overlay,
            mesh_render::ShadowReceiver::Never,
        )
    }

    /// One drawable per slot from the model `pick` takes out of that slot's
    /// livery, blended with `blend` and written into the glow mask;
    /// `None` for a slot with no model or an empty one. A grid wider than the
    /// liveries repeats the last, as the hulls do.
    pub(super) fn drawables(
        &self,
        pick: impl Fn(&oag_livery::Livery) -> Option<Model>,
        blend: wgpu::BlendState,
        depth: mesh_render::Depth,
        receiver: mesh_render::ShadowReceiver,
    ) -> Result<Vec<Option<Drawable>>> {
        (0..GRID_SLOTS as usize)
            .map(|slot| {
                let livery = &self.liveries[slot.min(self.liveries.len().saturating_sub(1))];
                let Some(model) = pick(livery).filter(|model| !model.indices.is_empty()) else {
                    return Ok(None);
                };
                Drawable::new_attached(
                    self.device,
                    self.queue,
                    model,
                    self.format,
                    self.anisotropy,
                    self.sample_count,
                    depth,
                    blend,
                    mesh_render::GlowMask::Written,
                    self.zone_art,
                    self.shadow_maps,
                    receiver,
                )
                .map(Some)
            })
            .collect()
    }
}
