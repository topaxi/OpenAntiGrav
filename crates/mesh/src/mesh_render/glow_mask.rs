//! Which of a model's draws may write the scene target's alpha channel.

/// Whether a model's draws may write the scene target's alpha channel, which
/// `oag_post::bloom` reads as its glow mask.
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
    /// **The PS2's mask: what the fragment's own alpha is.** Read off a GS
    /// dump of Moa Therma, replayed with a readback draw appended
    /// (`docs/rendering/ps2-bloom.md`, "The mask, measured"): the frame clears
    /// to alpha `0`, opaque batches leave alpha alone, and the nine groups that
    /// write it draw `_GLOW`-textured batches with `FBMSK = 0` and the texture
    /// function on `MODULATE`, so what lands is the texel's alpha times the
    /// vertex colour's. A batch with the glow bits ([`crate::mesh::slots::GLOW_BATCH`])
    /// writes that; every other batch writes `0`, the clear's own value, which
    /// is the same picture because the original's opaque batches never touch
    /// alpha and its stamps run after them, depth-tested.
    ///
    /// The structure is [`Self::Stamped`]'s - opaque and cutout pipelines write
    /// it, blended ones leave it to a second stamp draw - and only the value
    /// differs, which `mesh.wesl`'s `glow_texel` override switches.
    StampedByTexel,
}

impl GlowMask {
    /// Whether this choice stamps the mask through the stamp machinery,
    /// whichever rule gives the value.
    #[must_use]
    pub fn stamps(self) -> bool {
        matches!(self, Self::Stamped | Self::StampedByTexel)
    }

    /// The colour write mask this choice implies.
    #[must_use]
    pub fn writes(self) -> wgpu::ColorWrites {
        match self {
            Self::Protected => wgpu::ColorWrites::COLOR,
            Self::Written | Self::Stamped | Self::StampedByTexel => wgpu::ColorWrites::ALL,
        }
    }

    /// The colour write mask for this choice's **blended** pipeline, where
    /// [`Self::Stamped`] writes colour only: its mask is written by the
    /// separate stamp draw - see that variant.
    #[must_use]
    pub fn blend_writes(self) -> wgpu::ColorWrites {
        match self {
            Self::Stamped | Self::StampedByTexel => wgpu::ColorWrites::COLOR,
            other => other.writes(),
        }
    }
}
