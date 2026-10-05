//! The scene pass's own attachments: the depth buffer and, under MSAA, the
//! multisampled colour target it resolves from.
//!
//! Split out of `frame.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. The seam is
//! the one `frame/craft.rs` already used: these two are the only things in
//! that file that build a resource rather than encoding a frame.

use crate::mesh_render;

pub(crate) fn depth_texture(
    device: &wgpu::Device,
    size: (u32, u32),
    sample_count: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("race depth"),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: mesh_render::DEPTH_FORMAT,
        // `TEXTURE_BINDING` because the motion blur pass reads the depth the
        // scene just wrote to reproject each pixel against the previous
        // camera - see `oag_post::motion_blur`. Declared even at MSAA
        // sample counts, where that pass does not run: the flag costs
        // nothing, and a conditional usage would be one more thing the two
        // call sites could disagree about.
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

/// The multisampled colour attachment MSAA draws into, resolved into the
/// caller's own target at the end of [`Scene::render`]'s one pass.
///
/// `None` at `sample_count` 1: a single-sample scene draws straight into the
/// caller's view and there is nothing here to resolve.
pub(crate) fn msaa_color_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: (u32, u32),
    sample_count: u32,
) -> Option<wgpu::Texture> {
    (sample_count > 1).then(|| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("race msaa colour"),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    })
}
