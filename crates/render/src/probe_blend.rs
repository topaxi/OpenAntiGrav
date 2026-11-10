//! The hull environment-probe blend, on the GPU.
//!
//! A Wipeout HD hull is lit from two maps that are a weighted blend of up to four of the
//! circuit's baked probes, and the weights move with the craft, so the blend is new every
//! frame (`oag_vex::probes`). This is that blend as one compute pass a frame: the decoded
//! probes sit in a source array, uploaded once each, and the pass writes every craft's two
//! layers of the array the hull shader samples.
//!
//! **The bytes are `oag_vex::probes::mix`'s.** The shader does the float arithmetic on the bits,
//! in integers, so no driver can fuse or reorder it (`shaders/probe_blend.wesl`), and the
//! tests hold the array to `mix` texel for texel. The hull shader reads the same array it
//! always did, so nothing about a hull's picture moves.

use oag_gpu::writes::Writes;
use oag_mesh::mesh_render::{ENV_PROBE_FORMAT, ENV_PROBE_LAYERS, ENV_PROBE_SIZE};

/// How many decoded probes the source array holds: 8 crafts blend four each, and 64 are
/// 4 MB.
pub const SLOTS: u32 = 64;

/// The most crafts one pass blends: every craft the array has layers for.
pub const JOBS: usize = ENV_PROBE_LAYERS as usize / 2;

/// The size of the irradiance map in the corner of its layer.
const IRRADIANCE_SIZE: (u32, u32) = (ENV_PROBE_SIZE.0 / 2, ENV_PROBE_SIZE.1 / 2);

/// `Job` in the shader: three `vec4<u32>`.
const JOB_BYTES: usize = 48;

/// One craft's blend: its taps' slots in the source array and their scales (weight over the
/// weights' sum, `oag_vex::probes::Taps::scales`). A tap of scale zero is left out, whatever
/// slot it names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Job {
    pub craft: u32,
    pub slots: [u32; 4],
    pub scales: [f32; 4],
}

/// The source array, the blended array and the pass between them.
#[derive(Debug)]
pub struct ProbeBlend {
    sources: wgpu::Texture,
    blended: wgpu::Texture,
    view: wgpu::TextureView,
    jobs: wgpu::Buffer,
    bind: wgpu::BindGroup,
    pipeline: wgpu::ComputePipeline,
}

impl ProbeBlend {
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        let layer = wgpu::Extent3d {
            width: ENV_PROBE_SIZE.0,
            height: ENV_PROBE_SIZE.1,
            depth_or_array_layers: 1,
        };
        let sources = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("environment probe sources"),
            size: wgpu::Extent3d {
                depth_or_array_layers: SLOTS * 2,
                ..layer
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        // A storage texture cannot be sRGB, so the array is the linear format and the hull
        // reads it through an sRGB view: the same bytes, decoded on fetch as before.
        let blended = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("environment probes"),
            size: wgpu::Extent3d {
                depth_or_array_layers: ENV_PROBE_LAYERS,
                ..layer
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: ENV_PROBE_FORMAT.remove_srgb_suffix(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[ENV_PROBE_FORMAT],
        });
        let array = |texture: &wgpu::Texture, format, usage| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                format: Some(format),
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                usage: Some(usage),
                ..Default::default()
            })
        };
        let view = array(
            &blended,
            ENV_PROBE_FORMAT,
            wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let target = array(
            &blended,
            blended.format(),
            wgpu::TextureUsages::STORAGE_BINDING,
        );
        let source = array(
            &sources,
            sources.format(),
            wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let jobs = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("environment probe blend jobs"),
            size: (JOBS * JOB_BYTES) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty,
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("environment probe blend"),
            entries: &[
                entry(
                    0,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                ),
                entry(
                    2,
                    wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: blended.format(),
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                    },
                ),
            ],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("environment probe blend"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: jobs.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&source),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&target),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("environment probe blend"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/probe_blend.wgsl")).into(),
            ),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("environment probe blend"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("environment probe blend"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("blend"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        Self {
            sources,
            blended,
            view,
            jobs,
            bind,
            pipeline,
        }
    }

    /// The blended array as the hull shader samples it, for `ShadowMaps::env_probes`.
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Puts one decoded probe in `slot`: its 64x32 irradiance map and its 128x64 reflection
    /// map, both `RGBA8`.
    pub fn write_source(
        &self,
        queue: &dyn Writes,
        slot: u32,
        irradiance: &[u8],
        reflection: &[u8],
    ) {
        let images = [(irradiance, IRRADIANCE_SIZE), (reflection, ENV_PROBE_SIZE)];
        for (which, (texels, size)) in images.into_iter().enumerate() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.sources,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: slot * 2 + which as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                texels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(size.0 * 4),
                    rows_per_image: Some(size.1),
                },
                wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    /// Blends `jobs` (at most [`JOBS`]) into their crafts' layers, as one pass in `encoder`.
    /// The sources a job names must be written through the same `queue` first.
    pub fn blend(&self, queue: &dyn Writes, encoder: &mut wgpu::CommandEncoder, jobs: &[Job]) {
        let jobs = &jobs[..jobs.len().min(JOBS)];
        if jobs.is_empty() {
            return;
        }
        let mut bytes = [0u8; JOBS * JOB_BYTES];
        for (job, out) in jobs.iter().zip(bytes.as_chunks_mut::<JOB_BYTES>().0) {
            let words = job
                .slots
                .into_iter()
                .chain(job.scales.map(f32::to_bits))
                .chain([job.craft]);
            for (word, at) in words.zip(out.as_chunks_mut::<4>().0) {
                *at = word.to_ne_bytes();
            }
        }
        queue.write_buffer(&self.jobs, 0, &bytes[..jobs.len() * JOB_BYTES]);
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("environment probe blend"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.dispatch_workgroups(
            ENV_PROBE_SIZE.0 / 8,
            ENV_PROBE_SIZE.1 / 8,
            jobs.len() as u32 * 2,
        );
    }

    /// One layer of the blended array, `RGBA8` rows of [`ENV_PROBE_SIZE`], for a test.
    #[must_use]
    pub fn read_back(&self, device: &wgpu::Device, queue: &wgpu::Queue, layer: u32) -> Vec<u8> {
        let (width, height) = ENV_PROBE_SIZE;
        let row = (width * 4).div_ceil(256) * 256;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("environment probe readback"),
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.blended,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
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
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("draining the queue");
        let mapped = slice.get_mapped_range().expect("mapping the readback");
        mapped
            .chunks(row as usize)
            .flat_map(|line| &line[..width as usize * 4])
            .copied()
            .collect()
    }
}

#[cfg(test)]
mod tests;
