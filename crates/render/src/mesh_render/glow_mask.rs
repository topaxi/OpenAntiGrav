//! Which of a model's draws may write the scene target's alpha channel.

/// Whether a model's draws may write the scene target's alpha channel, which
/// [`crate::post::bloom`] reads as its glow mask.
///
/// **The original writes that channel only through the GE stencil**, which
/// keeps its value in the framebuffer's alpha and never blends it. What a
/// surface stamps is decided per batch - `pass_mask & 0xc0` and the blend
/// class, in `Gfx_BuildBatchStateList` (`0x0891f890`) - and was measured out
/// of EDRAM on a live race: see `docs/rendering/glow-mask.md` and
/// [`Self::Stamped`]. An earlier reading here, that the plume's draw path
/// writes the mask and a hull's does not, took `Gu_PixelMask(0)` for a write;
/// opening the channel writes nothing without a stencil op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlowMask {
    /// Colour only - the default wherever the original's stencil stamp has
    /// not been measured. A Pulse PSP model is [`Self::Stamped`] instead,
    /// whatever its caller asks - see [`crate::mesh::Model::stamps_glow`].
    Protected,
    /// Alpha reaches the target, so this model's fragments feed the bloom.
    Written,
    /// **The original's own stencil stamp, per batch.** Every opaque and
    /// alpha-tested draw writes the constant its batch names -
    /// [`crate::mesh::GpuVertex::glow`], read by `crate::mesh::glow` - in
    /// place of its alpha. That is Pulse on the PSP, measured out of EDRAM;
    /// see `docs/rendering/glow-mask.md`.
    ///
    /// **A transparent batch with the glow bits stamps too**, its texture's
    /// byte, and [`crate::mesh_render::Built::stamp_pipeline`] is how: a blend
    /// state cannot write a constant alpha while the colour blend reads the
    /// texel's own, so the draw is submitted a second time with colour masked
    /// off and the alpha test of its own batch. The original does it in one
    /// draw, with `StencilFunc(ALWAYS, ref)` and `StencilOp(KEEP, KEEP,
    /// REPLACE)` left on under the blend - read off a GE dump of Outpost 7
    /// (`glow-mask.md`, "Transparent batches stamp").
    Stamped,
}

impl GlowMask {
    /// The colour write mask this choice implies.
    #[must_use]
    pub fn writes(self) -> wgpu::ColorWrites {
        match self {
            Self::Protected => wgpu::ColorWrites::COLOR,
            Self::Written | Self::Stamped => wgpu::ColorWrites::ALL,
        }
    }

    /// The colour write mask for this choice's **blended** pipeline, where
    /// [`Self::Stamped`] writes colour only: its mask is written by the
    /// separate stamp draw - see that variant.
    #[must_use]
    pub fn blend_writes(self) -> wgpu::ColorWrites {
        match self {
            Self::Stamped => wgpu::ColorWrites::COLOR,
            other => other.writes(),
        }
    }
}
