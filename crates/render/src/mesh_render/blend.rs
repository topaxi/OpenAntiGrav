//! The blend equations `Model::transparent_draws` is drawn with, and the choice
//! between them.
//!
//! Split out of [`super`] rather than living beside the pipelines it describes,
//! because it is the one part of that module that is a *decode* - two of these
//! constants are `Gfx_BuildBatchStateList`'s own `Gu_BlendFunc` calls, and the
//! third path exists because Wipeout HD's materials author a factor pair
//! instead of naming a class.

/// The blend for a transparent batch of class
/// [`oag_formats::vex::BlendClass::AlphaOver`] - `pass_mask & 0x100`.
///
/// **Recovered.** `Gfx_BuildBatchStateList`'s `0x100` branch programs
/// `Gu_Enable(GU_BLEND)` with `Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA,
/// GU_ONE_MINUS_SRC_ALPHA, 0, 0)` and disables the colour test. The colour
/// factors here are that call. This constant's doc used to say the opposite -
/// "not recovered ... a plausible reading" - and it happened to be right; what
/// was wrong was applying it to **every** transparent batch. See
/// [`ADDITIVE_BLEND`] and [`oag_formats::vex::BlendClass`].
///
/// **The alpha factors are still ours**, and deliberately unchanged: PSP
/// blending is RGB-only, so the original's call says nothing about the alpha
/// channel and there is nothing to copy.
pub const TRANSPARENT_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
};

/// The blend for a transparent batch of class [`oag_formats::vex::BlendClass::Additive`] -
/// `pass_mask & 0x200`.
///
/// **Recovered, and identical to [`crate::exhaust::BLEND`].**
/// `Gfx_BuildBatchStateList`'s `0x200` branch programs
/// `Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA, GU_FIX, 0x000000, 0xffffff)` - a fixed
/// destination factor of white, i.e. `src * srcAlpha + dst`. That is the same
/// equation `ExhaustFlare_BuildDisplayList` programs for the engine flare, so
/// **the flare, the boost plume and every other `0x200` batch in the game
/// share one blend**, which was not known until the `0x0700` class was split.
///
/// Alpha matches the colour factors here rather than following
/// [`TRANSPARENT_BLEND`]'s split, for the same reason `exhaust::BLEND` does:
/// a target later read as premultiplied should not disagree with its own
/// colour channels.
pub const ADDITIVE_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// The pipelines a [`crate::mesh::Model::transparent_draws`] loop chooses between, borrowed
/// from wherever a caller keeps them.
///
/// **A borrowed view rather than a field group**, because the three callers that
/// draw a `Model` - the viewer, the offscreen capture and the race - each own
/// these as their own fields and are otherwise unrelated. What it buys is that
/// [`Self::select`] is written once: the choice used to be a `match` on
/// `DrawCall::blend` copied into all three, and a fourth case added to one of
/// them would silently not exist in the others.
#[derive(Debug, Clone, Copy)]
pub struct TransparentPipelines<'a> {
    /// [`super::Built::blend_pipeline`].
    pub alpha_over: &'a [wgpu::RenderPipeline; 2],
    /// [`super::Built::additive_pipeline`].
    pub additive: &'a [wgpu::RenderPipeline; 2],
    /// [`super::Built::unblended_pipeline`].
    pub unblended: &'a [wgpu::RenderPipeline; 2],
    /// [`super::Built::authored_pipelines`].
    pub authored: &'a [(wgpu::BlendState, [wgpu::RenderPipeline; 2])],
}

impl<'a> TransparentPipelines<'a> {
    /// The pipeline one transparent draw is submitted through.
    ///
    /// **A file's own equation wins over a class.** `DrawCall::blend_state` is
    /// set only where the model's file authors a factor pair - Wipeout HD's
    /// material table - and there the pair *is* the answer; `DrawCall::blend`
    /// then says no more than that the draw is transparent. Pulse's batches
    /// leave it `None` and fall through to the recovered classes, which is the
    /// path every existing picture takes.
    ///
    /// A state with no pipeline cannot happen for a model this `Built` was made
    /// from - [`super::build`] creates one per distinct state it finds - and falls back
    /// to [`Self::alpha_over`] rather than panicking, for the caller that pairs
    /// a `Built` with a different model.
    #[must_use]
    pub fn select(&self, draw: &crate::mesh::DrawCall) -> &'a wgpu::RenderPipeline {
        let set = match draw.blend_state {
            Some(state) => self
                .authored
                .iter()
                .find(|(seen, _)| *seen == state)
                .map_or(self.alpha_over, |(_, pipelines)| pipelines),
            None => match draw.blend {
                Some(oag_formats::vex::BlendClass::Additive) => self.additive,
                Some(oag_formats::vex::BlendClass::None) => self.unblended,
                Some(oag_formats::vex::BlendClass::AlphaOver) | None => self.alpha_over,
            },
        };
        &set[usize::from(draw.culled)]
    }
}
