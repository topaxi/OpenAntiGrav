//! Running the ported passes over a hand-built depth buffer and reading an
//! intermediate back, which is the only instrument this port has until
//! [`super::Pass::Accumulate`] lands and there is an actual output to look at.
//!
//! Its own file rather than more of `tests.rs`, under the 1,000-line rule in
//! `scripts/check-file-size.py` and because the harness below is a fixture the
//! next pass's tests will reuse rather than a test in itself.

use super::*;

/// A depth buffer, and everything the passes need alongside it.
///
/// The device and queue are kept because a readback needs them after the
/// dispatch has already happened - the pass ran in [`Scene::run`], and every
/// `read_r32` after it is a second submission against the same device.
pub(super) struct Scene {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub fsr3: Fsr3,
}

/// The camera the readbacks project through - the race's own near and far, so
/// that `get_view_space_depth` is exercised over the range it really sees.
pub(super) const CAMERA: Camera = Camera {
    near: 0.1,
    far: 1000.0,
    fov_y: std::f32::consts::FRAC_PI_4,
};

impl Scene {
    /// Runs every ported pass over a `render`-sized depth buffer holding
    /// `depth`, row-major. `None` when there is no adapter.
    ///
    /// **The depth texture is a real `Depth32Float`,** not a colour format
    /// standing in for one: binding a depth view as an unfilterable float is
    /// exactly the arrangement that would fail, and a colour texture would not
    /// exercise it. It is filled by rendering rather than by `write_texture`,
    /// because WebGPU does not allow a buffer-to-texture copy into a depth
    /// format - the fill is a fullscreen pass writing `@builtin(frag_depth)`
    /// from a storage buffer, which is the same picture by a longer road.
    pub(super) fn run(render: (u32, u32), depth: &[f32]) -> Option<Self> {
        assert_eq!(depth.len(), (render.0 * render.1) as usize);
        let upscale = (render.0 * 2, render.1 * 2);

        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).ok()?;

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fsr3 test depth"),
            size: wgpu::Extent3d {
                width: render.0,
                height: render.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&Default::default());
        fill_depth(&device, &queue, &depth_view, render, depth);

        let plain = |label, format, usage| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: render.0,
                        height: render.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let binding = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT;
        let colour = plain(
            "fsr3 test colour",
            wgpu::TextureFormat::Rgba16Float,
            binding,
        );
        let velocity = plain(
            "fsr3 test velocity",
            crate::mesh_render::VELOCITY_FORMAT,
            binding,
        );

        let mut fsr3 = Fsr3::new(&device).expect("the FSR 3.1 shaders must compile");
        let mut encoder = device.create_command_encoder(&Default::default());
        fsr3.render(
            &device,
            &queue,
            &mut encoder,
            Frame {
                colour: &colour,
                depth: &depth_view,
                velocity: &velocity,
                dispatch: Dispatch {
                    render,
                    max_render: render,
                    upscale,
                    // Zero, so that nothing in this fixture depends on which
                    // phase of the sequence it happened to land on.
                    jitter: (0.0, 0.0),
                    phase_count: crate::jitter::phases(render.0, upscale.0),
                    camera: CAMERA,
                    delta_time: 1.0 / 60.0,
                    reset: true,
                },
            },
        );
        queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("draining the queue");

        Some(Self {
            device,
            queue,
            fsr3,
        })
    }

    /// One `R32Float` intermediate, read back as `f32`s row-major.
    pub(super) fn read_r32(&self, texture: &wgpu::Texture) -> Vec<f32> {
        let size = texture.size();
        let unpadded = (size.width * 4) as usize;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded = unpadded.div_ceil(align) * align;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fsr3 readback"),
            size: (padded * size.height as usize) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded as u32),
                    rows_per_image: Some(size.height),
                },
            },
            size,
        );
        self.queue.submit([encoder.finish()]);
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("draining the queue");
        let mapped = readback.slice(..).get_mapped_range().expect("mapping");
        let mut out = Vec::with_capacity((size.width * size.height) as usize);
        for y in 0..size.height as usize {
            for x in 0..size.width as usize {
                let at = y * padded + x * 4;
                out.push(f32::from_le_bytes(mapped[at..at + 4].try_into().unwrap()));
            }
        }
        drop(mapped);
        readback.unmap();
        out
    }

    /// `GetViewSpaceDepthInMeters` on the CPU, from the same two constants the
    /// shader reads - so the check is against the transform this frame was
    /// dispatched with rather than against a second derivation of it.
    pub(super) fn view_space_metres(&self, device_depth: f32) -> f32 {
        let c = self.fsr3.constants().expect("a dispatched frame");
        let metres = c.device_to_view_depth[1] / (device_depth - c.device_to_view_depth[0]);
        metres.min(65504.0)
    }
}

/// Fills a depth attachment from a storage buffer of per-texel values.
///
/// A fullscreen triangle writing `@builtin(frag_depth)`, because WebGPU has no
/// buffer-to-texture copy into a depth format. `DepthCompare::Always` and a
/// write mask, so the value written is exactly the one in the buffer.
fn fill_depth(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &wgpu::TextureView,
    size: (u32, u32),
    depth: &[f32],
) {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("fsr3 test depth fill"),
        source: wgpu::ShaderSource::Wgsl(include_str!("fill_depth.wgsl").into()),
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("fsr3 test depth fill"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    let values = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fsr3 test depth values"),
        size: (depth.len() * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&values, 0, bytemuck::cast_slice(depth));
    let extent = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fsr3 test depth extent"),
        size: 16,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&extent, 0, bytemuck::cast_slice(&[size.0, size.1, 0, 0]));
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("fsr3 test depth fill"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: values.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: extent.as_entire_binding(),
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("fsr3 test depth fill"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("fsr3 test depth fill"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            targets: &[],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });

    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fsr3 test depth fill"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, Some(&group), &[]);
        pass.draw(0..3, 0..1);
    }
    queue.submit([encoder.finish()]);
}
