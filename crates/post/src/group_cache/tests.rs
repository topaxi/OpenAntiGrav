use super::*;

fn view(device: &wgpu::Device, size: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("group cache test"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

/// The group is built once for a view however often it is asked for, and again
/// for a different view (a resize makes a new texture, so a new view) - and a
/// return to the first view is a rebuild too, never a stale group.
#[test]
fn a_group_is_rebuilt_only_when_the_view_changes() {
    let Some((device, _queue)) = crate::split_fixture::device() else {
        return;
    };
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }],
    });
    let build = |v: &wgpu::TextureView| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(v),
            }],
        })
    };
    let cache = ViewGroup::default();
    let a = view(&device, 8);
    let b = view(&device, 16);
    for _ in 0..3 {
        cache.get(&a, || build(&a));
    }
    assert_eq!(cache.builds(), 1, "the same view reuses the group");
    cache.get(&b, || build(&b));
    assert_eq!(cache.builds(), 2, "a new view rebuilds");
    cache.get(&b, || build(&b));
    assert_eq!(cache.builds(), 2);
    cache.get(&a, || build(&a));
    assert_eq!(cache.builds(), 3, "going back to the first view rebuilds");
}
