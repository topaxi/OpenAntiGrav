//! Bind group 2's Zone half: the stage texture the Zone material variant
//! samples, and the sampler it reads it through.
//!
//! Split out of `mesh_render` rather than written inline for one reason worth
//! stating: the scene group is now two unrelated things in one binding set -
//! the fog and light *buffer*, which every title writes, and a *texture* only
//! a Zone race on Wipeout HD/Fury ever has. Keeping the second one here means
//! the first reads as it always did.
//!
//! See [`super::Zone`] for what the shader does with it, which terms of the
//! recovered rule are deliberately absent, and the evidence for both.

use std::sync::Arc;

use super::texture;
use crate::mesh::ModelTexture;

/// The two layout entries the Zone half of bind group 2 adds: the stage
/// texture at binding 1 and its own sampler at binding 2.
///
/// A sampler of its own rather than the albedo's, because the coordinate is
/// one the shader builds - `zoneColourTint.xy * (1 - meshUV)` - and runs
/// outside `[0, 1]` wherever the file's own scale does.
pub(super) fn layout_entries() -> [wgpu::BindGroupLayoutEntry; 2] {
    [
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        },
    ]
}

/// The sampler and the view to bind, for a model that has a Zone stage
/// texture and for one that does not.
///
/// **Black, not white, where there is none.** [`super::Scene::off`] leaves
/// `zone.enabled` at zero so the sample is multiplied out anyway, and a black
/// placeholder means even a caller that writes a Zone uniform without a
/// texture adds nothing rather than adding a white sheet - the failure this
/// project wants from a missing asset is an absence.
pub(super) fn resources(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    anisotropy: super::Anisotropy,
    stage: Option<&Arc<ModelTexture>>,
) -> (wgpu::Sampler, wgpu::TextureView) {
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("zone"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        anisotropy_clamp: anisotropy.clamp(),
        ..Default::default()
    });
    let view = match stage {
        Some(texture) => texture::upload(
            device,
            queue,
            texture,
            device
                .features()
                .contains(wgpu::Features::TEXTURE_COMPRESSION_BC),
        ),
        None => texture::upload_rgba(device, queue, 1, 1, &[0, 0, 0, 255], "no zone stage"),
    };
    (sampler, view)
}
