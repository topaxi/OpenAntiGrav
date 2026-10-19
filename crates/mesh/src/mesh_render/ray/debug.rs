//! The `--ray-debug` picture: a compute pass that casts one primary ray per pixel into a
//! [`super::Scene`] and colours the hit, read back as RGBA8.
//!
//! It touches no mesh pipeline and no bind group of the raster, so it is the same code
//! whether or not a tier uses the structures for anything else, and a frame that does not
//! ask for it pays nothing.

use wgpu::util::DeviceExt as _;

use super::Scene;

/// The longest primary ray, in world units: past any circuit's extent.
pub const MAX_RAY_LENGTH: f32 = 20_000.0;

const SHADER: &str = include_str!(concat!(env!("OUT_DIR"), "/ray_debug.wgsl"));

/// What the TLAS holds this frame: the circuit's animated node matrices and each craft's
/// hull index and model matrix (see [`Scene::update`]).
#[derive(Clone, Copy, Debug)]
pub struct Frame<'a> {
    pub nodes: &'a [[f32; 16]],
    pub craft: &'a [(usize, [f32; 16])],
}

/// The pipeline. Created only on a device with the tier: the module declares `ray_query`.
#[derive(Debug)]
pub struct Debug {
    pipeline: wgpu::ComputePipeline,
}

impl Debug {
    /// # Panics
    ///
    /// On a device without [`super::FEATURES`], which is a caller bug: check
    /// [`super::enabled`] first.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        assert!(
            super::enabled(device),
            "the ray tier needs its device feature"
        );
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ray debug"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ray debug"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self { pipeline }
    }

    /// Casts the picture's rays and returns `width * height` RGBA8 texels, row 0 on top.
    /// `inverse_view_projection` is column-major with `0..1` depth, as glam's
    /// `rh::proj::directx` pair makes it.
    #[must_use]
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &mut Scene,
        frame: &Frame<'_>,
        inverse_view_projection: [f32; 16],
        (width, height): (u32, u32),
    ) -> Vec<u8> {
        let Frame { nodes, craft } = *frame;
        let mut params = [0.0f32; 20];
        params[..16].copy_from_slice(&inverse_view_projection);
        params[16] = MAX_RAY_LENGTH;
        let uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ray debug params"),
            contents: bytemuck::cast_slice(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ray debug"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        scene.update(&mut encoder, nodes, craft);
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ray debug"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: scene.tlas().as_binding(),
                },
            ],
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ray debug"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }
        let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ray debug readback"),
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |result| {
            result.expect("the readback maps");
        });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let mapped = slice.get_mapped_range().expect("the readback range");
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height as usize {
            let start = y * row as usize;
            out.extend_from_slice(&mapped[start..start + (width * 4) as usize]);
        }
        out
    }
}
