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
    /// What the scene target stores - see `oag_gpu::formats::SceneStore`.
    pub(super) store: oag_gpu::formats::SceneStore,
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
    /// liveries repeats the last, as the hulls do. A shield shell under its own
    /// lit program (`Model::shield_lit`) takes neither: the original masks the
    /// draw's alpha off and its program samples no shadow map
    /// (`docs/ghidra/functions/ps3-hdfury-eu/shield.md`, "The shell's draw state").
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
                let (glow, receiver) = mask_and_shadow(&model, receiver);
                Drawable::new(
                    self.device,
                    self.queue,
                    model,
                    self.format,
                    self.store,
                    self.anisotropy,
                    self.sample_count,
                    depth,
                    blend,
                    glow,
                    self.zone_art,
                    self.shadow_maps,
                    receiver,
                    mesh_render::Velocity::Attached,
                )
                .map(Some)
            })
            .collect()
    }
}

/// A per-slot model's glow mask and shadow receiver: written and `receiver`, except for a lit
/// shield shell.
fn mask_and_shadow(
    model: &Model,
    receiver: mesh_render::ShadowReceiver,
) -> (mesh_render::GlowMask, mesh_render::ShadowReceiver) {
    if model.shield_lit {
        (
            mesh_render::GlowMask::Protected,
            mesh_render::ShadowReceiver::Never,
        )
    } else {
        (mesh_render::GlowMask::Written, receiver)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lit_shield_shell_leaves_the_glow_mask_and_takes_no_shadow() {
        let mut model = crate::tests::model(0, 0);
        let mapped = mesh_render::ShadowReceiver::Mapped;
        assert_eq!(
            mask_and_shadow(&model, mapped),
            (mesh_render::GlowMask::Written, mapped)
        );
        model.shield_lit = true;
        assert_eq!(
            mask_and_shadow(&model, mapped),
            (
                mesh_render::GlowMask::Protected,
                mesh_render::ShadowReceiver::Never
            )
        );
    }
}
