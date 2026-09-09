//! The two textures a [`super::Framebuffer`] owns, and how each is built.
//!
//! Its own file rather than the foot of `upscale.rs`, under the 1,000-line rule
//! in `scripts/check-file-size.py`. The pair belong together: they are the
//! *scene* target and the *presentation* target, and the difference between
//! them - one has a non-sRGB twin view and the other does not - is the whole
//! reason both exist.

use anyhow::Result;

use super::{Grade, Output, bind, grade_buffer};
use oag_display::display::{Brightness, Gamma};

/// Builds the presentation-sized target and the bind group that reads it.
///
/// No non-sRGB twin, unlike [`target`]: nothing samples this one in perceptual
/// space. The upscalers and the post-process passes all read the *scene*
/// target, upstream of here, and the only pass that reads this one is the
/// graded blit onto the surface.
pub(super) fn output(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    format: wgpu::TextureFormat,
    size: (u32, u32),
) -> Result<Output> {
    let size = (size.0.max(1), size.1.max(1));
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("presentation target"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let graded = Grade::new(Brightness::NEUTRAL, Gamma::NEUTRAL, false);
    let grade = grade_buffer(device, "presentation grade", graded)?;
    let bind_group = bind(device, layout, sampler, &grade, &view);
    Ok(Output {
        view,
        bind_group,
        grade,
        graded,
        size,
    })
}

pub(super) fn target(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    grade: &wgpu::Buffer,
    format: wgpu::TextureFormat,
    size: (u32, u32),
) -> (
    wgpu::Texture,
    wgpu::TextureView,
    wgpu::TextureView,
    wgpu::BindGroup,
) {
    // The non-sRGB twin is declared here so an upscaler can take a view in it
    // later. Declaring a view format costs nothing when nobody asks for one,
    // and on some backends it is the difference between a texture that can be
    // reinterpreted at all and one that cannot - which is not a thing that can
    // be retrofitted to an already-created texture.
    let twin = format.remove_srgb_suffix();
    let view_formats: &[wgpu::TextureFormat] = if twin == format { &[] } else { &[twin] };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("upscale target"),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats,
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let perceptual = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("upscale target (perceptual)"),
        format: Some(twin),
        ..Default::default()
    });
    let bind_group = bind(device, layout, sampler, grade, &view);
    (texture, view, perceptual, bind_group)
}
