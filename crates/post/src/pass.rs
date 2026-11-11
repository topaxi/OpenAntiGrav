//! The full-screen passes the Omega and WO chains are built from: a render pipeline over
//! the bufferless triangle (or a bufferless mesh a vertex entry generates), a pass that
//! loads what is there and draws it into a rectangle, a pass that only clears, and the
//! colour target a pass writes.

/// A render pipeline that draws the full-screen triangle (`vs_main`) and shades it with
/// `entry` into `target`.
pub(crate) fn fullscreen_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    label: &str,
    entry: &str,
    target: wgpu::ColorTargetState,
) -> wgpu::RenderPipeline {
    mesh_pipeline(device, shader, layout, label, "vs_main", entry, target)
}

/// A render pipeline whose vertices `vertex` generates from the vertex index alone, shaded
/// with `entry` into `target`.
pub(crate) fn mesh_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    label: &str,
    vertex: &str,
    entry: &str,
    target: wgpu::ColorTargetState,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vertex),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(entry),
            targets: &[Some(target)],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// A colour target that replaces what it holds.
pub(crate) fn replacing(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    }
}

/// One full-screen pass with no blending, loading what is there.
pub(crate) fn draw(
    encoder: &mut wgpu::CommandEncoder,
    label: &'static str,
    pipeline: &wgpu::RenderPipeline,
    groups: &[&wgpu::BindGroup],
    target: &wgpu::TextureView,
    origin: (f32, f32),
    rect: (u32, u32),
) {
    draw_with(
        encoder,
        label,
        pipeline,
        groups,
        target,
        wgpu::LoadOp::Load,
        origin,
        rect,
        3,
    );
}

/// [`draw`] for a [`mesh_pipeline`]'s `vertices` vertices.
#[expect(clippy::too_many_arguments, reason = "one pass's whole description")]
pub(crate) fn draw_mesh(
    encoder: &mut wgpu::CommandEncoder,
    label: &'static str,
    pipeline: &wgpu::RenderPipeline,
    groups: &[&wgpu::BindGroup],
    target: &wgpu::TextureView,
    origin: (f32, f32),
    rect: (u32, u32),
    vertices: u32,
) {
    draw_with(
        encoder,
        label,
        pipeline,
        groups,
        target,
        wgpu::LoadOp::Load,
        origin,
        rect,
        vertices,
    );
}

/// [`draw`], with the target's `load` op and the vertex count chosen by the caller.
#[expect(clippy::too_many_arguments, reason = "one pass's whole description")]
pub(crate) fn draw_with(
    encoder: &mut wgpu::CommandEncoder,
    label: &'static str,
    pipeline: &wgpu::RenderPipeline,
    groups: &[&wgpu::BindGroup],
    target: &wgpu::TextureView,
    load: wgpu::LoadOp<wgpu::Color>,
    origin: (f32, f32),
    rect: (u32, u32),
    vertices: u32,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    oag_gpu::perfprobe::marks::mark(&mut pass, label);
    pass.set_pipeline(pipeline);
    pass.set_viewport(origin.0, origin.1, rect.0 as f32, rect.1 as f32, 0.0, 1.0);
    for (index, group) in groups.iter().enumerate() {
        pass.set_bind_group(index as u32, *group, &[]);
    }
    pass.draw(0..vertices, 0..1);
    oag_gpu::perfprobe::marks::close(&mut pass);
}

/// A pass that clears `target` to opaque black and draws nothing.
pub(crate) fn clear(
    encoder: &mut wgpu::CommandEncoder,
    label: &'static str,
    target: &wgpu::TextureView,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    oag_gpu::perfprobe::marks::mark(&mut pass, label);
    oag_gpu::perfprobe::marks::close(&mut pass);
}

/// One colour texture and its view.
#[derive(Debug)]
pub(crate) struct Target {
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "held so the view stays valid; the tests read it")
    )]
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
}

impl Target {
    pub(crate) fn new(
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
            // `COPY_DST` so a test can lay known pixels into the scene.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view }
    }
}
