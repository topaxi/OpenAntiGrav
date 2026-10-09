//! The blit's uniform: the grade, and how much of the source to read.
//!
//! Split out of `upscale.rs` for the 1,000-line file ratchet
//! (`just check-size`) rather than because the two are conceptually apart -
//! this is the parent module's own uniform and nothing else uses it.

use oag_display::display::{Brightness, Gamma};

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
    /// `oag_post`. Since [ADR-0020](../../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md)
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
    /// entry in the shared `oag_post::fullscreen_layout` that the five passes in
    /// `oag_post` have no use for.
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
/// [`super::Framebuffer::new`] has no queue and should not need one: building a
/// framebuffer stays a device-only operation, which is what lets a capture path
/// build one without a frame loop around it. [`oag_gpu::init_buffer`] maps on
/// native and writes through the browser's registered queue on the web.
pub(super) fn grade_buffer(device: &wgpu::Device, label: &str, graded: Grade) -> wgpu::Buffer {
    oag_gpu::init_buffer::init(
        device,
        label,
        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        bytemuck::bytes_of(&graded),
    )
}

/// Which rectangle of the bound source the blit reads.
///
/// **The question is which size the bound view is, not whether a pass ran.**
/// FSR 1 resolves to the presentation rectangle and hands back a texture
/// exactly that size, so its output is the whole thing. FXAA and SMAA draw at
/// scene resolution into an allocation-sized target with the extent in its
/// corner, so their output owes the same sub-rectangle the scene target does -
/// and `source.is_some()` cannot tell those two apart.
///
/// Its own function because both arms are identical while the extent is the
/// allocation, which is every frame until a controller moves it: no
/// `--presented` capture can distinguish a correct reading of this from a
/// wrong one, so a test on the decision itself is the only guard there is.
pub(super) fn resolved_source(
    upscaled: bool,
    extent: (u32, u32),
    allocation: (u32, u32),
) -> Source {
    if upscaled {
        Source::WHOLE
    } else {
        Source::of(extent, allocation)
    }
}

/// Whether resolving `scene` to `rect` is a magnification, which is the only
/// thing a spatial upscaler is for.
///
/// FSR 1 is a **magnifier**, and upstream says so: EASU's contract is that the
/// output is larger than the input. Handed a render scale above 100 % it is
/// being asked to minify, and its twelve taps then step more than one input
/// texel apart and undersample - so it re-introduces exactly the aliasing the
/// supersampling was there to remove. Measured on a trackside fence at 200 %:
/// the bilinear blit resolves the mesh smoothly and FSR 1 turns it crunchy.
///
/// So the setting is honoured where it means something and quietly not where it
/// would only do harm. A row that silently degrades the picture at three of the
/// six render scales it sits next to would be worse than one that does nothing
/// at those three.
/// Either axis and not both: a render scale applies to both, but a clamped
/// target or an odd rectangle can leave one axis equal while the other is
/// short, and one short axis is still something to reconstruct. On the equal
/// axis EASU then steps exactly one input texel per output texel, which is the
/// one-to-one case - it reconstructs nothing there, but it also cannot produce
/// the undersampling artefact above, which needs a step *greater* than one.
pub fn magnifies(scene: (u32, u32), rect: (u32, u32)) -> bool {
    scene.0 < rect.0 || scene.1 < rect.1
}

/// The blit's one bind group: a source view, the sampler and the grade.
pub(super) fn bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    grade: &wgpu::Buffer,
    view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    // Through the probe rather than `device.create_bind_group` directly, the
    // same as the five sites in `oag_post`. This is the sixth, and the
    // only one outside that module - which is why a sweep of `post/` alone
    // missed it. A counting passthrough: the same bind group, and nothing at
    // all without the `perf-probe` feature.
    //
    // It counts three callers, and only two of them are per-frame:
    // `resolve_scene` binds an upscaler's or a post-process's output every
    // frame it runs one, while `target` and `output` bind once per resize. A
    // resize is rare enough that the count still reads as per-frame churn.
    oag_gpu::perfprobe::bind_group(
        device,
        &wgpu::BindGroupDescriptor {
            label: Some("upscale"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: grade.as_entire_binding(),
                },
            ],
        },
    )
}
