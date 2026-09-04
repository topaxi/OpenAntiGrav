//! Bind group 2's shadow map: the texture every lit pipeline samples, and the
//! placeholder that stands in when nothing casts.
//!
//! The receiving half of Wipeout HD's model-shadow path. What the map *holds*
//! is coverage rather than depth - see
//! [`super::ShadowMap`](super::uniforms::ShadowMap), which carries the
//! microcode that settles it - so there is no comparison sampler here and no
//! depth format: an ordinary filtered `R8Unorm` read, exactly like the Zone
//! stage texture two bindings above it.

/// The layout entries this adds to the scene group: a texture and a sampler.
pub(super) fn layout_entries() -> [wgpu::BindGroupLayoutEntry; 2] {
    [
        wgpu::BindGroupLayoutEntry {
            binding: 6,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 7,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        },
    ]
}

/// What [`resources`] builds.
pub(super) struct Resources {
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

/// The view and sampler to bind, given the caller's map or none.
///
/// **Black where there is none**, on the same reasoning
/// [`super::zone::resources`] gives for its own placeholder: an absence should
/// read as no shadow rather than as a sheet of one, and `Scene::shadow`'s zero
/// strength multiplies it out regardless.
///
/// **Clamped to the edge, and that matters here**: the sample is projective,
/// so a receiver outside the map's own frustum lands off the edge. Under
/// `Repeat` it would pick up a craft's shadow from the other side of the map -
/// a second, wrong shadow somewhere down the track - where clamping smears the
/// border texel, which the caster pass keeps at zero.
pub(super) fn resources(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    map: Option<&wgpu::TextureView>,
) -> Resources {
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("shadow map"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let view = match map {
        Some(view) => view.clone(),
        None => {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("shadow map placeholder"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: crate::shadow::map::FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                texture.as_image_copy(),
                &[0u8],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(1),
                    rows_per_image: Some(1),
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            texture.create_view(&wgpu::TextureViewDescriptor::default())
        }
    };
    Resources { view, sampler }
}
