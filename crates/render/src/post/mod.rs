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
pub mod fxaa;
pub mod hd_bloom;
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
