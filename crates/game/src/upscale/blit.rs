//! The blit's uniform: the grade, and how much of the source to read.
//!
//! Split out of `upscale.rs` for the 1,000-line file ratchet
//! (`just check-size`) rather than because the two are conceptually apart -
//! this is the parent module's own uniform and nothing else uses it.

use anyhow::{Context, Result};

use crate::display::{Brightness, Gamma};

/// What the blit does to the picture on its way onto the surface, as the
/// shader's uniform expects it.
///
/// `repr(C)` and sixteen bytes: a uniform binding has a minimum size, and the
/// two floats alone are half of it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Grade {
    pub(super) brightness: f32,
    pub(super) exponent: f32,
    /// Whether the blit's source holds sRGB-encoded values the shader has to
    /// decode itself, as `1.0` or `0.0`.
    ///
    /// One only when `self.format.is_srgb()` - the offscreen target is bound
    /// through an sRGB view and arrives already decoded by the sampler, while
    /// an upscaler's or FXAA/SMAA's output is deliberately read through the
    /// *non*-sRGB twin because that pass works in perceptual space and a
    /// decode on every internal read would be both wrong and paid twice. See
    /// `oag_render::post`. Since [ADR-0020](../../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md)
    /// forced every real `format` here non-sRGB, `twin == format` and this is
    /// always zero in production - "a post-process ran" is not the same fact
    /// as "the source needs decoding", and treating them as one is the bug
    /// that made FXAA, SMAA and a magnifying FSR 1 darken the picture.
    pub(super) decode: f32,
    pub(super) padding: f32,
    /// What fraction of the bound source the blit reads, and how far into it
    /// the sampler may go - [`Source`], flattened into the uniform.
    ///
    /// Here rather than in a binding of its own because it rides the same
    /// buffer, the same write and the same "only when it changed" comparison
    /// the grade already had, and a second uniform binding would mean a second
    /// entry in the shared `oag_render::post::fullscreen_layout` that the five passes in
    /// `oag_render::post` have no use for.
    pub(super) uv_scale: [f32; 2],
    pub(super) uv_max: [f32; 2],
}

impl Grade {
    /// A grade that reads the whole of whatever is bound.
    ///
    /// The right answer for every source but one: an upscaler's output and a
    /// post-process's output are both exactly the size they were asked for,
    /// and the presentation target is the surface's. Only the scene target can
    /// be larger than what was drawn into it - see [`super::Framebuffer::set_extent`].
    pub(super) fn new(brightness: Brightness, gamma: Gamma, decode: bool) -> Self {
        Self {
            brightness: brightness.factor(),
            exponent: gamma.exponent(),
            decode: f32::from(u8::from(decode)),
            padding: 0.0,
            uv_scale: Source::WHOLE.uv_scale,
            uv_max: Source::WHOLE.uv_max,
        }
    }

    /// The same grade, reading `source` of the bound texture instead of all of
    /// it.
    pub(super) fn reading(self, source: Source) -> Self {
        Self {
            uv_scale: source.uv_scale,
            uv_max: source.uv_max,
            ..self
        }
    }
}

/// How much of a blit's source texture was actually drawn into.
///
/// One for the whole texture, less when the scene target is allocated at the
/// `render_scale` ceiling and a smaller rectangle of it was rendered this
/// frame - the arrangement
/// [ADR-0037](../../../../docs/architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md)
/// decides on. See [`docs/rendering/dynamic-resolution.md`](../../../../docs/rendering/dynamic-resolution.md).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Source {
    /// What to multiply the fullscreen triangle's `0..1` coordinate by.
    pub(super) uv_scale: [f32; 2],
    /// The furthest the sampler may be asked for, half a texel inside the
    /// drawn rectangle's outer edge.
    ///
    /// Without it a linear tap at the rectangle's edge reaches the first texel
    /// *outside* it, which holds whatever the last larger frame left there -
    /// the classic dynamic-resolution artefact, a stale fringe down the right
    /// and bottom.
    pub(super) uv_max: [f32; 2],
}

impl Source {
    /// All of it, exactly - `1.0` on both fields rather than an inset that
    /// evaluates close to it.
    ///
    /// Load-bearing: an unconditional `(extent - 0.5) / allocation` would
    /// compress the read by half a texel across the whole destination even
    /// when the two sizes are equal, shifting every interior sample and
    /// changing the bytes of every `--presented` capture for a change that is
    /// meant to be structurally inert.
    pub(super) const WHOLE: Self = Self {
        uv_scale: [1.0, 1.0],
        uv_max: [1.0, 1.0],
    };

    /// The rectangle `extent` occupies inside a texture of `allocation`.
    pub(super) fn of(extent: (u32, u32), allocation: (u32, u32)) -> Self {
        if extent == allocation {
            return Self::WHOLE;
        }
        // Per axis, because only one of the two may be short: `target_size`
        // clamps each independently against the device's maximum texture
        // dimension, so a wide window at a high scale can hit the ceiling on
        // one axis and not the other.
        let axis = |drawn: u32, allocated: u32| {
            if drawn >= allocated {
                return (1.0, 1.0);
            }
            let allocated = allocated as f32;
            (drawn as f32 / allocated, (drawn as f32 - 0.5) / allocated)
        };
        let (scale_x, max_x) = axis(extent.0, allocation.0);
        let (scale_y, max_y) = axis(extent.1, allocation.1);
        Self {
            uv_scale: [scale_x, scale_y],
            uv_max: [max_x, max_y],
        }
    }
}

/// A grade uniform, filled at creation rather than through the queue.
///
/// Through the mapping because [`super::Framebuffer::new`] has no queue and should not
/// need one: building a framebuffer stays a device-only operation, which is
/// what lets a capture path build one without a frame loop around it.
pub(super) fn grade_buffer(
    device: &wgpu::Device,
    label: &str,
    graded: Grade,
) -> Result<wgpu::Buffer> {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: std::mem::size_of::<Grade>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: true,
    });
    buffer
        .slice(..)
        .get_mapped_range_mut()
        .context("mapping the grade buffer")?
        .copy_from_slice(bytemuck::bytes_of(&graded));
    buffer.unmap();
    Ok(buffer)
}
