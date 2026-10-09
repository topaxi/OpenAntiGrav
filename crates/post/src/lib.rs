//! Post-processing that runs between a scene and the surface.
//!
//! Everything here reads a finished offscreen frame and writes another one; none
//! of it knows what drew the frame or where the result is going. That is what
//! lets the game's composition root own the blit, the aspect bars and the
//! brightness grade while the upscalers live in the renderer, reachable from
//! `oag-view` too.
//!
//! # Colour space
//!
//! **The upscalers here work in perceptual space** - on sRGB-encoded values, not
//! on linear light. FSR 1's edge detection reasons about luma differences the
//! way an eye weighs them, and feeding it linear light makes it misjudge which
//! edges matter in shadow.
//!
//! The scene target's *format* is still sRGB, so everything that draws into it
//! keeps encoding on write exactly as it does onto a window. What changes is how
//! the upscaler *reads* it: through a non-sRGB view of the same texture, which
//! returns the stored bytes without the hardware's decode. The three consumers
//! of a frame therefore want three different things, and it is worth naming them
//! because getting one wrong costs an encode either way:
//!
//! | Consumer | Wants | How it gets it |
//! | --- | --- | --- |
//! | A PNG readback | sRGB-encoded bytes | the texture's own bytes, copied |
//! | FSR 1 (here) | sRGB-encoded samples | a non-sRGB view |
//! | The blit's grade | linear light | an sRGB view, or an explicit decode |
//!
//! FSR 3.1, when it lands, wants a fourth thing - linear light with its own
//! tonemapping either side of accumulation - which is why the scene target's
//! encoding is a per-upscaler decision rather than a setting made once.

pub mod bloom;
pub mod fsr1;
pub mod fsr3;
pub mod fxaa;
pub mod hd_bloom;
pub mod hd_zoom;
pub mod jitter;
pub mod motion_blur;
pub mod omega_tonemap;
pub mod ps2_bloom;
pub mod screen;
pub mod smaa;

/// A sampled 2D colour texture, at `binding`, visible to the fragment stage.
///
/// Every pass here reads at least one, and three of them had written this
/// thirteen-line literal out by hand: `smaa` as a module function, `hd_bloom`
/// as a local closure identical to it, and `bloom`, `fsr1`, `fxaa` and the
/// game's blit inline inside a larger descriptor.
#[must_use]
pub fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

/// A filtering sampler at `binding`, visible to the fragment stage.
#[must_use]
pub fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

/// A uniform buffer at `binding`, visible to the fragment stage.
///
/// `min_binding_size` is left unset rather than given the struct's size: the
/// passes here each bind a different uniform, and stating the size in the
/// layout as well as in the shader is a second place for it to be wrong.
#[must_use]
pub fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// The layout a fullscreen pass over one source texture wants: the texture at
/// `0`, a filtering sampler at `1`, and the pass's own uniform at `2`.
///
/// Four passes had this written out identically - `bloom`, `fsr1`, `fxaa` and
/// `oag-game`'s blit - differing only in the label, and a fifth (`hd_bloom`)
/// wants the same first three bindings with two more textures after them, so it
/// composes the entries above instead. Sharing the shape is what makes the two
/// facts that matter about it checkable in one place: **the binding numbers
/// have to agree with every `@group(0) @binding(n)` in the matching WGSL**, and
/// the sampler is `Filtering`, which is what pairs with `filterable: true` on
/// the texture. Neither is visible at a call site that only passes a label.
#[must_use]
pub fn fullscreen_layout(device: &wgpu::Device, label: &str) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &[texture_entry(0), sampler_entry(1), uniform_entry(2)],
    })
}

/// The UV scale and clamp for a `viewport` drawn inside a resource of `size`.
///
/// Every scene-resolution pass here reads a texture that since
/// [ADR-0037](../../../../docs/architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md)
/// may be larger than the rectangle drawn into it: the target is allocated at
/// the `render_scale` ceiling and a controller draws a sub-rectangle of it.
/// The scale maps the fullscreen triangle's `0..1` onto that rectangle; the
/// clamp keeps a tap half a texel inside its outer edge, because the texels
/// past it hold whatever the clear left there rather than picture.
///
/// **Exactly `1.0` on both when the two are equal, by construction**, which is
/// what keeps every pass byte-identical while nothing moves the extent. An
/// unconditional `(viewport - 0.5) / size` would compress the read by half a
/// texel across the whole frame even at the ceiling, shifting every interior
/// sample. `oag_game::upscale::blit::Source` makes the same argument for the
/// final blit and is where this shape was first written.
#[must_use]
pub fn sub_rectangle(viewport: (u32, u32), size: (u32, u32)) -> ([f32; 2], [f32; 2]) {
    // Per axis, because only one of the two may be short: the allocation is
    // clamped against the device's maximum texture dimension independently on
    // each, so a wide window at a high scale can hit the ceiling on one and
    // not the other.
    let axis = |drawn: u32, allocated: u32| {
        if drawn >= allocated || allocated == 0 {
            return (1.0, 1.0);
        }
        let allocated = allocated as f32;
        (drawn as f32 / allocated, (drawn as f32 - 0.5) / allocated)
    };
    let (scale_x, max_x) = axis(viewport.0, size.0);
    let (scale_y, max_y) = axis(viewport.1, size.1);
    ([scale_x, scale_y], [max_x, max_y])
}
