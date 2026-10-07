//! Bind group 2's shadow map: the texture every lit pipeline samples, and the
//! placeholder that stands in when nothing casts.
//!
//! The receiving half of Wipeout HD's model-shadow path. What the map *holds*
//! is coverage rather than depth - see
//! [`super::ShadowMap`](super::uniforms::ShadowMap), which carries the
//! microcode that settles it - so there is no comparison sampler here and no
//! depth format: an ordinary filtered `R8Unorm` read, exactly like the Zone
//! stage texture two bindings above it.
//!
//! The formats of the four shadow textures (the coverage map, its depth
//! sibling, the per-craft sun-occlusion array and the self-shadow array) live
//! here because this module creates them; `oag_render::shadow` renders into
//! them and imports the same values.

/// The coverage map's texel format (`oag_render::shadow::map`).
///
/// One channel, because the receiver reads one - `TXP R1.x` takes `.x` and
/// nothing else. Unorm rather than float: a coverage value is `0..1` by
/// construction and the sampler filters it for free.
pub const COVERAGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;

/// The depth map's format, for the `mapped` tier.
///
/// **A second target rather than a second meaning for the first.** The
/// coverage map answers "is anything between this point and the light", which
/// is all Wipeout HD's own track shadows need; a cascaded shadow map answers
/// "what is the *nearest* thing", which needs real depth and a depth test to
/// resolve overlapping casters. Storing depth in the `R8Unorm` target would
/// quantise a whole circuit's depth range to 256 steps.
pub const SHADOW_DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// The sun-occlusion maps' texel format: one channel, `0..1`, filtered. What the receiver
/// reads is one channel (`TXP R0.z ... unit2` takes `.z` alone).
pub const OCCLUSION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;

/// One layer per craft the grid can hold - `oag_gameplay::MAX_SHIPS`, stated
/// as a literal because this crate depends on no gameplay crate; the same
/// number `oag_render::shadow::map::MAX_CASTERS` is.
pub const OCCLUSION_LAYERS: u32 = 8;

/// The self-shadow maps' texel format: real depth, for the same reason
/// [`SHADOW_DEPTH_FORMAT`] is - a comparison needs it, and eight bits
/// over a 140-unit-deep box would be half a unit per step.
pub const SELF_SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// The layout entries this adds to the scene group: the coverage map and its
/// filtering sampler, then the depth map and its non-filtering one, then the
/// per-craft sun-occlusion array at binding 13 - which shares the coverage
/// map's sampler, being the same kind of clamped, filtered `R8Unorm` read -
/// and the per-craft self-shadow depth array at binding 14, which shares
/// the depth map's non-filtering sampler for the reason that one exists.
///
/// **Two textures rather than one reused**, because the two tiers store
/// different things: coverage in eight bits, which filters, and depth in
/// thirty-two, which must not be - a filtered depth is an average of two
/// surfaces and belongs to neither, so a comparison against it shadows a band
/// along every silhouette.
///
/// Binding 15 is not a shadow map: it is the behind-the-glass target
/// (`super::ShadowMaps::behind_glass`), read through the coverage map's
/// clamped filtering sampler, which is the original's own sampler state for it
/// (clamp to edge, linear, `0x60030303`/`0x02022000` on Vineta K's capture).
pub(super) fn layout_entries() -> [wgpu::BindGroupLayoutEntry; 7] {
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
        wgpu::BindGroupLayoutEntry {
            binding: 8,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 9,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 13,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 14,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 15,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
    ]
}

/// What [`resources`] builds.
#[derive(Debug)]
pub(super) struct Resources {
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub depth_view: wgpu::TextureView,
    pub depth_sampler: wgpu::Sampler,
    /// The sun-occlusion array, or a one-layer black placeholder.
    pub occlusion_view: wgpu::TextureView,
    /// The self-shadow depth array, or a one-layer far placeholder.
    pub self_shadow_view: wgpu::TextureView,
    /// The behind-the-glass target, or a black placeholder.
    pub behind_glass_view: wgpu::TextureView,
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
    maps: super::ShadowMaps<'_>,
) -> Resources {
    let super::ShadowMaps {
        coverage: map,
        depth: depth_map,
        occlusion,
        self_shadow,
        behind_glass,
    } = maps;
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
                format: COVERAGE_FORMAT,
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
    // Non-filtering, and that is not a detail: a linearly filtered depth is the
    // average of two surfaces and belongs to neither, so comparing against it
    // shadows a band along every silhouette. Softening belongs in the
    // comparison - several taps, each compared - not in the sample.
    let depth_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("shadow depth map"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let depth_view = match depth_map {
        Some(view) => view.clone(),
        None => {
            // Cleared to the far plane, which is "nothing between here and the
            // light" - the same absence the coverage placeholder's zero is.
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("shadow depth placeholder"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: SHADOW_DEPTH_FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            texture.create_view(&wgpu::TextureViewDescriptor::default())
        }
    };
    let occlusion_view = match occlusion {
        Some(view) => view.clone(),
        None => {
            // Black, one layer: no hull names a layer without a map bound,
            // so this is never sampled; it exists so the layout is one
            // layout.
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("sun occlusion placeholder"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: OCCLUSION_FORMAT,
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
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            })
        }
    };
    let self_shadow_view = match self_shadow {
        Some(view) => view.clone(),
        None => {
            // One layer, never sampled for the reason the occlusion
            // placeholder is not; a fresh depth texture's contents are
            // unspecified, and that is fine for a binding nothing reads.
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("self shadow placeholder"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: SELF_SHADOW_FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            })
        }
    };
    // Black: a glass with no target behind it shows its own lit colour alone.
    // Only Vineta K's glass reads it, and that circuit always binds the real
    // target, so this is what every other drawable binds and never samples.
    let behind_glass_view = match behind_glass {
        Some(view) => view.clone(),
        None => {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("behind the glass placeholder"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: COVERAGE_FORMAT,
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
    Resources {
        view,
        sampler,
        depth_view,
        depth_sampler,
        occlusion_view,
        self_shadow_view,
        behind_glass_view,
    }
}
