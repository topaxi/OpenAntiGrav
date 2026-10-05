//! Bind group 1's layout, split out of `mesh_render.rs` under the 1,000-line rule.

/// Bind group 1's layout: a material's albedo, the sampler both textures
/// share, and its lightmap - what every mesh pipeline binds per material slot.
///
/// A function rather than an inline descriptor because a second pipeline
/// draws through the same bind groups: the sun-occlusion pass
/// (`oag_render::shadow::occlusion`) takes a track drawable's own material bind
/// groups and needs a layout equal to the one they were built against. Two
/// layouts built from one descriptor are equal in wgpu's eyes; two written
/// out by hand drift.
#[must_use]
pub fn material_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("albedo"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // The lightmap shares the albedo's sampler: it is the same
            // filtering on the same kind of texture, and a second sampler would
            // be a second thing to keep in step for no difference in the
            // picture.
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // HD's pad mask, `_ne`: sampled only where `slots::PAD_NE` is
            // set, and a flat placeholder everywhere else.
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // HD's magstrip wave, sampled only where `slots::MAG_WAVE` is set.
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
        ],
    })
}
