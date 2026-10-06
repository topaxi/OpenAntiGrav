//! The flash's draw: one full-screen quad, additive, colour only.

use oag_mesh::mesh_render::{DEPTH_FORMAT, Velocity, fragment_options};

/// The flash pipeline and its one-colour uniform.
#[derive(Debug)]
pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// Whether the last [`Pipeline::upload`] had anything to draw.
    visible: bool,
}

impl Pipeline {
    /// Builds the pipeline for a pass writing `format` at `sample_count`,
    /// with the race's velocity target when `velocity` says so.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
        velocity: Velocity,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("screen flash"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/flash.wgsl")).into(),
            ),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("screen flash"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("screen flash"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let mut targets = vec![Some(wgpu::ColorTargetState {
            format,
            // `BlendFunc(ADD, SRC_ALPHA, FIX 0xffffff)`, the same factors as
            // the particles' additive class.
            blend: Some(crate::psys::BLEND),
            // `ScreenFlash_Draw` clears the bloom's glow-mask write first.
            write_mask: wgpu::ColorWrites::COLOR,
        })];
        targets.extend(velocity.target(true));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("screen flash"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &targets,
                compilation_options: fragment_options(format),
            }),
            primitive: wgpu::PrimitiveState::default(),
            // Depth test off, as `ScreenFlash_Draw` sets it; the attachment
            // is still bound, so the state names it.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screen flash"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("screen flash"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        Self {
            pipeline,
            uniforms,
            bind_group,
            visible: false,
        }
    }

    /// This frame's colour, [`super::ScreenFlash::colour`]; `None` draws
    /// nothing.
    pub fn upload(&mut self, queue: &wgpu::Queue, colour: Option<[f32; 4]>) {
        self.visible = colour.is_some();
        if let Some(colour) = colour {
            queue.write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&colour));
        }
    }

    /// Draws the quad into a pass the caller opened, after everything the
    /// flash should wash over.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if !self.visible {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
