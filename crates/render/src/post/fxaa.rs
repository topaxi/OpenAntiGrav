//! Fast Approximate Anti-Aliasing: one fullscreen pass, luma edge detection
//! then a directional blend. See [`fxaa.wgsl`](./fxaa.wgsl) for the shader
//! and [`crate::post`] for the colour-space convention it draws in.
//!
//! # Why this and not a transliteration
//!
//! [ADR-0012](../../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)
//! established transliterating a specific MIT-licensed upstream, attributed
//! and reproduced under `licences/`, as this project's route for a shader it
//! did not write - `fsr1.wgsl` and `smaa.wgsl` both are one. FXAA has no
//! single canonical upstream with licensing this project can clear the same
//! way: the widely distributed "FXAA 3.11" carries NVIDIA's own terms, and a
//! transliteration of it would need those cleared and reproduced exactly like
//! AMD's are. Rather than copy text this project cannot attribute correctly,
//! this is written from the technique's public description - luma-based edge
//! detection, then a blend across the detected edge - as its own
//! implementation.
//!
//! # What that costs
//!
//! A single quality tier: one 5-tap cross sample for edge detection, one
//! directional blend, no iterative search along the edge the way NVIDIA's own
//! shader performs to find where an edge ends. That is a real difference in
//! sharpness, not a bug - see [ADR-0013](../../../../docs/architecture/adr/0013-anti-aliasing-architecture.md)
//! for where this sits next to SMAA, which *is* a transliteration and keeps
//! more detail for roughly three times the cost.

use anyhow::Result;

/// The shader's uniform: the inverse render-target size the taps are spaced
/// by, and the two edge thresholds. `repr(C)` and sixteen bytes for the same
/// reason `upscale::Grade` is - a uniform binding has a minimum size, and
/// four floats is exactly it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Constants {
    inv_size: [f32; 2],
    threshold: f32,
    relative_threshold: f32,
}

impl Constants {
    /// The minimum local contrast that counts as an edge at all. Below this,
    /// on a flat plain, no amount of relative contrast changes the answer.
    const THRESHOLD: f32 = 0.0625;
    /// The same test again as a fraction of the local brightness, so a
    /// shadow does not need the same absolute contrast a highlight does to
    /// register as an edge worth blending.
    const RELATIVE_THRESHOLD: f32 = 0.125;

    fn new(size: (u32, u32)) -> Self {
        let (w, h) = (size.0.max(1) as f32, size.1.max(1) as f32);
        Self {
            inv_size: [1.0 / w, 1.0 / h],
            threshold: Self::THRESHOLD,
            relative_threshold: Self::RELATIVE_THRESHOLD,
        }
    }
}

/// One frame's worth of input to [`Fxaa::render`].
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    /// A **non-sRGB** view of the scene, for the same reason `fsr1::Frame::source`
    /// is: the luma test reasons about encoded values, the way an eye weighs
    /// them, and a linear view would misjudge which edges matter in shadow.
    pub source: &'a wgpu::TextureView,
    /// The size of `source`, and of the output this resizes to.
    pub size: (u32, u32),
}

/// The one pipeline, its uniform, and the target it resizes to match `Frame::size`.
#[derive(Debug)]
pub struct Fxaa {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    constants: wgpu::Buffer,
    written: Option<Constants>,
    output: Option<Target>,
    /// Non-sRGB, matching `Frame::source`: the blend must not decode or
    /// re-encode a value it is only passing through untouched.
    format: wgpu::TextureFormat,
    size: (u32, u32),
}

#[derive(Debug)]
struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Fxaa {
    /// Builds the pipeline against a surface `format`.
    ///
    /// The output target is built at the first [`render`](Self::render),
    /// which is the first time a size is known.
    ///
    /// # Errors
    ///
    /// Propagates a shader that will not compile, which is a build-time
    /// mistake rather than anything a player can cause.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fxaa"),
            source: wgpu::ShaderSource::Wgsl(include_str!("fxaa.wgsl").into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fxaa"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
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

        // Linear, unlike FSR 1's nearest-and-gather sampler: the blend tap
        // reads one texel in a cross pattern and wants ordinary bilinear
        // filtering there, not EASU's own weighting scheme.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fxaa"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fxaa"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let target_format = format.remove_srgb_suffix();
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fxaa"),
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
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let constants = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fxaa constants"),
            size: size_of::<Constants>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            pipeline,
            layout,
            sampler,
            constants,
            written: None,
            output: None,
            format: target_format,
            size: (0, 0),
        })
    }

    /// The blended frame, in **perceptual space** - the caller must decode
    /// it before grading. `None` until the first [`render`](Self::render).
    #[must_use]
    pub fn output(&self) -> Option<&wgpu::TextureView> {
        self.output.as_ref().map(|target| &target.view)
    }

    /// The texture behind [`output`](Self::output), for a readback.
    #[must_use]
    pub fn output_texture(&self) -> Option<&wgpu::Texture> {
        self.output.as_ref().map(|target| &target.texture)
    }

    /// Runs the one pass, resizing the target if `frame.size` moved.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: Frame<'_>,
    ) {
        self.resize(device, frame.size);

        let wanted = Constants::new(frame.size);
        if self.written != Some(wanted) {
            queue.write_buffer(&self.constants, 0, bytemuck::bytes_of(&wanted));
            self.written = Some(wanted);
        }

        let Some(output) = &self.output else { return };

        let bind_group = self.bind_group(device, frame.source);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fxaa"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &output.view,
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, Some(&bind_group), &[]);
        pass.draw(0..3, 0..1);
    }

    fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if self.size == size && self.output.is_some() {
            return;
        }
        self.size = size;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fxaa output"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            // `COPY_SRC` so a test can read the result back, the same reason
            // `fsr1::Fsr1`'s intermediates carry it.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.output = Some(Target { texture, view });
    }

    fn bind_group(&self, device: &wgpu::Device, source: &wgpu::TextureView) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fxaa"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.constants.as_entire_binding(),
                },
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_uniform_is_sixteen_bytes_and_holds_the_inverse_size() {
        assert_eq!(std::mem::size_of::<Constants>(), 16);
        let c = Constants::new((1920, 1080));
        assert_eq!(c.inv_size, [1.0 / 1920.0, 1.0 / 1080.0]);
        assert_eq!(c.threshold, Constants::THRESHOLD);
        assert_eq!(c.relative_threshold, Constants::RELATIVE_THRESHOLD);
    }

    #[test]
    fn a_degenerate_size_cannot_divide_by_zero() {
        let c = Constants::new((0, 0));
        assert!(c.inv_size.iter().all(|v| v.is_finite()), "{:?}", c.inv_size);
    }

    /// Compiles the shader and runs the pass on a real device.
    ///
    /// **Skips when there is no adapter**, so a green CI run is not evidence
    /// that it ran. Run it locally on real hardware.
    #[test]
    fn the_pass_builds_and_draws_on_a_real_device() {
        let instance = wgpu::Instance::default();
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
            eprintln!("no GPU adapter: skipping");
            return;
        };
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("requesting the device");

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let scene = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[format.remove_srgb_suffix()],
        });
        let source = scene.create_view(&wgpu::TextureViewDescriptor {
            format: Some(format.remove_srgb_suffix()),
            ..Default::default()
        });

        let mut fxaa = Fxaa::new(&device, format).expect("building the pipeline");
        assert!(fxaa.output().is_none(), "no output before the first render");

        let mut encoder = device.create_command_encoder(&Default::default());
        fxaa.render(
            &device,
            &queue,
            &mut encoder,
            Frame {
                source: &source,
                size: (64, 64),
            },
        );
        queue.submit(Some(encoder.finish()));
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("draining the queue");

        assert!(fxaa.output().is_some(), "the output target must exist");
    }

    /// A flat colour has no local contrast anywhere, so every pixel takes the
    /// early-out and the pass must be a pure pass-through.
    ///
    /// **Skips when there is no adapter.**
    #[test]
    fn a_flat_colour_is_untouched() {
        const SIZE: u32 = 32;
        let instance = wgpu::Instance::default();
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
            eprintln!("no GPU adapter: skipping");
            return;
        };
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("requesting the device");

        // `Rgba8Unorm`, not sRGB: this is checking the blend's arithmetic
        // against a known input, and an encode either way would put a
        // transfer function between what is written and what is asserted.
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let scene = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("flat"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mut pixels = vec![128u8; (SIZE * SIZE * 4) as usize];
        for pixel in pixels.chunks_mut(4) {
            pixel[3] = 255;
        }
        queue.write_texture(
            scene.as_image_copy(),
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE * 4),
                rows_per_image: Some(SIZE),
            },
            scene.size(),
        );
        let source = scene.create_view(&wgpu::TextureViewDescriptor::default());

        let mut fxaa = Fxaa::new(&device, format).expect("building the pipeline");
        let mut encoder = device.create_command_encoder(&Default::default());
        fxaa.render(
            &device,
            &queue,
            &mut encoder,
            Frame {
                source: &source,
                size: (SIZE, SIZE),
            },
        );

        let unpadded = (SIZE * 4) as usize;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded = unpadded.div_ceil(align) * align;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (padded * SIZE as usize) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            fxaa.output_texture().expect("an output").as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded as u32),
                    rows_per_image: Some(SIZE),
                },
            },
            wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("draining the queue");
        let mapped = readback.slice(..).get_mapped_range().expect("mapping");

        let row = SIZE / 2;
        let at = padded * row as usize + (SIZE / 2 * 4) as usize;
        assert_eq!(
            &mapped[at..at + 4],
            &[128, 128, 128, 255],
            "a flat colour must pass through untouched"
        );
    }

    /// A hard edge blends near the seam and leaves both flat sides alone -
    /// the whole point of the contrast threshold existing at all.
    ///
    /// **Skips when there is no adapter.**
    #[test]
    fn a_hard_edge_blends_only_near_the_seam() {
        const SIZE: u32 = 32;
        let instance = wgpu::Instance::default();
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
            eprintln!("no GPU adapter: skipping");
            return;
        };
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("requesting the device");

        let format = wgpu::TextureFormat::Rgba8Unorm;
        let scene = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("edge"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
        for y in 0..SIZE {
            for x in 0..SIZE {
                let value = if x < SIZE / 2 { 0 } else { 255 };
                let at = ((y * SIZE + x) * 4) as usize;
                pixels[at..at + 4].copy_from_slice(&[value, value, value, 255]);
            }
        }
        queue.write_texture(
            scene.as_image_copy(),
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE * 4),
                rows_per_image: Some(SIZE),
            },
            scene.size(),
        );
        let source = scene.create_view(&wgpu::TextureViewDescriptor::default());

        let mut fxaa = Fxaa::new(&device, format).expect("building the pipeline");
        let mut encoder = device.create_command_encoder(&Default::default());
        fxaa.render(
            &device,
            &queue,
            &mut encoder,
            Frame {
                source: &source,
                size: (SIZE, SIZE),
            },
        );

        let unpadded = (SIZE * 4) as usize;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded = unpadded.div_ceil(align) * align;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (padded * SIZE as usize) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            fxaa.output_texture().expect("an output").as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded as u32),
                    rows_per_image: Some(SIZE),
                },
            },
            wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("draining the queue");
        let mapped = readback.slice(..).get_mapped_range().expect("mapping");

        let row = SIZE / 2;
        let at = |x: u32| mapped[padded * row as usize + (x * 4) as usize];

        // Far from the seam, on either side, the pass leaves the flat plain
        // alone - no neighbour ever disagrees enough to cross the threshold.
        assert_eq!(at(2), 0, "the black side is not black");
        assert_eq!(at(SIZE - 3), 255, "the white side is not white");
        // At the seam itself, the two texels immediately either side see a
        // neighbour on the far side of the edge and must move off their
        // extreme value.
        let left_of_seam = at(SIZE / 2 - 1);
        let right_of_seam = at(SIZE / 2);
        assert!(
            left_of_seam > 0 || right_of_seam < 255,
            "neither texel at the seam moved: {left_of_seam} / {right_of_seam}"
        );
    }
}
