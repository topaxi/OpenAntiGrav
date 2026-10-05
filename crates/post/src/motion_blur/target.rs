//! One of the chain's intermediate targets. Split out of `motion_blur.rs`
//! under the 1,000-line rule in `scripts/check-file-size.py`; a move, with
//! no behaviour change.

/// A render-and-sample scratch target.
#[derive(Debug)]
pub(super) struct Target {
    #[expect(dead_code, reason = "held so the view stays valid")]
    texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
}

impl Target {
    pub(super) fn new(
        device: &wgpu::Device,
        label: &str,
        format: wgpu::TextureFormat,
        (width, height): (u32, u32),
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
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
        Self { texture, view }
    }
}
