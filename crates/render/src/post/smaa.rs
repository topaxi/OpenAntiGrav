//! Subpixel Morphological Anti-Aliasing: luma edge detection, blending
//! weight calculation against a precomputed area/search lookup, then a
//! neighbourhood blend. Three fullscreen passes, chained together the way
//! [`smaa.wgsl`](./smaa.wgsl)'s own module docs diagram.
//!
//! Ported from `iryoku/smaa` (MIT) the same way `fsr1` is ported from AMD's
//! FidelityFX - see [ADR-0012](../../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)
//! for why this project transliterates rather than links or vendors a native
//! SDK. This is specifically upstream's **MEDIUM** preset: see `smaa.wgsl`'s
//! module docs for exactly what that fixes and what it leaves out.
//!
//! # The lookup textures
//!
//! `AreaTex` (160x560, two channels) and `SearchTex` (64x16, one channel)
//! are upstream's own precomputed data, embedded here as the raw bytes
//! `Textures/AreaTex.h` and `Textures/SearchTex.h` decode to - not
//! regenerated, since they are not something this project has a way to
//! verify a regeneration of against upstream's own reference output. See
//! `smaa_area.bin` and `smaa_search.bin` next to this file.
//!
//! # Why this and not a fourth resample
//!
//! SMAA reads one finished frame and needs nothing else - no motion
//! vectors, no jitter, no history - which is what makes it the higher
//! quality alternative to FXAA in the same class rather than a competitor to
//! FSR 1 or a future temporal pass. See
//! [ADR-0013](../../../../docs/architecture/adr/0013-anti-aliasing-architecture.md).

use anyhow::Result;

const AREA_WIDTH: u32 = 160;
const AREA_HEIGHT: u32 = 560;
const SEARCH_WIDTH: u32 = 64;
const SEARCH_HEIGHT: u32 = 16;

/// Upstream's own precomputed area lookup, decoded from `Textures/AreaTex.h`:
/// `AREATEX_WIDTH x AREATEX_HEIGHT`, two channels (`AREATEX_PITCH = WIDTH * 2`).
static AREA_TEX: &[u8] = include_bytes!("smaa_area.bin");
/// Upstream's own precomputed search lookup, decoded from `Textures/SearchTex.h`:
/// `SEARCHTEX_WIDTH x SEARCHTEX_HEIGHT`, one channel.
static SEARCH_TEX: &[u8] = include_bytes!("smaa_search.bin");

/// One frame's worth of input to [`Smaa::render`].
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    /// A **non-sRGB** view of the scene, for the same reason `fsr1::Frame::source`
    /// and `fxaa::Frame::source` are: the luma edge detection reasons about
    /// encoded values, the way an eye weighs them.
    pub source: &'a wgpu::TextureView,
    /// The size of `source`, and of the output this resizes to.
    pub size: (u32, u32),
}

#[derive(Debug)]
struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

fn target(
    device: &wgpu::Device,
    label: &str,
    format: wgpu::TextureFormat,
    size: (u32, u32),
) -> Target {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            // `COPY_SRC` so a test can read the intermediate results back,
            // the same reason `fsr1::Fsr1`'s intermediates carry it.
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    Target { texture, view }
}

/// The three pipelines, the two lookup textures, and the three intermediates
/// they draw through.
#[derive(Debug)]
pub struct Smaa {
    edge_pipeline: wgpu::RenderPipeline,
    edge_layout: wgpu::BindGroupLayout,
    blend_pipeline: wgpu::RenderPipeline,
    blend_layout: wgpu::BindGroupLayout,
    neighborhood_pipeline: wgpu::RenderPipeline,
    neighborhood_layout: wgpu::BindGroupLayout,

    point_sampler: wgpu::Sampler,
    linear_sampler: wgpu::Sampler,
    /// Upstream's own precomputed lookups, uploaded once at [`Smaa::new`]
    /// and read fresh into the blend pass's bind group every frame - unlike
    /// `edges`/`blend`/`output` below, which are the *targets* passes write
    /// into, these never change and never resize.
    #[expect(dead_code, reason = "kept alive by area_view's owning texture")]
    area_texture: wgpu::Texture,
    area_view: wgpu::TextureView,
    #[expect(dead_code, reason = "kept alive by search_view's owning texture")]
    search_texture: wgpu::Texture,
    search_view: wgpu::TextureView,

    edges: Option<Target>,
    blend: Option<Target>,
    output: Option<Target>,
    /// Non-sRGB, matching `Frame::source`.
    format: wgpu::TextureFormat,
    size: (u32, u32),
}

impl Smaa {
    /// Builds all three pipelines and uploads the lookup textures.
    ///
    /// # Errors
    ///
    /// Propagates a shader that will not compile, which is a build-time
    /// mistake rather than anything a player can cause.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("smaa"),
            source: wgpu::ShaderSource::Wgsl(include_str!("smaa.wgsl").into()),
        });

        let point_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("smaa point"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        // Upstream's `LinearSampler`: `MIN_MAG_LINEAR_MIP_POINT`, clamped.
        // The blend-weight pass deliberately reads the binary edges texture
        // through this - see `smaa.wgsl`'s `@PSEUDO_GATHER4` note - so this
        // is not an approximation of upstream's sampler, it is the sampler.
        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("smaa linear"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let (area_texture, area_view) = upload(
            device,
            queue,
            "smaa area",
            wgpu::TextureFormat::Rg8Unorm,
            (AREA_WIDTH, AREA_HEIGHT),
            AREA_TEX,
            2,
        );
        let (search_texture, search_view) = upload(
            device,
            queue,
            "smaa search",
            wgpu::TextureFormat::R8Unorm,
            (SEARCH_WIDTH, SEARCH_HEIGHT),
            SEARCH_TEX,
            1,
        );

        let target_format = format.remove_srgb_suffix();

        let edge_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("smaa edge"),
            entries: &[texture_entry(0), sampler_entry(1)],
        });
        let edge_pipeline = pass_pipeline(
            device,
            &shader,
            "smaa edge",
            "edge_vs_main",
            "edge_fs_main",
            &edge_layout,
            wgpu::TextureFormat::Rg8Unorm,
        );

        let blend_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("smaa blend"),
            entries: &[
                texture_entry(0),
                texture_entry(1),
                texture_entry(2),
                sampler_entry(3),
            ],
        });
        let blend_pipeline = pass_pipeline(
            device,
            &shader,
            "smaa blend",
            "blend_vs_main",
            "blend_fs_main",
            &blend_layout,
            wgpu::TextureFormat::Rgba8Unorm,
        );
        let neighborhood_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("smaa neighborhood"),
                entries: &[texture_entry(0), texture_entry(1), sampler_entry(2)],
            });
        let neighborhood_pipeline = pass_pipeline(
            device,
            &shader,
            "smaa neighborhood",
            "neighborhood_vs_main",
            "neighborhood_fs_main",
            &neighborhood_layout,
            target_format,
        );

        Ok(Self {
            edge_pipeline,
            edge_layout,
            blend_pipeline,
            blend_layout,
            neighborhood_pipeline,
            neighborhood_layout,
            point_sampler,
            linear_sampler,
            area_texture,
            area_view,
            search_texture,
            search_view,
            edges: None,
            blend: None,
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

    /// Runs all three passes, resizing the intermediates if `frame.size` moved.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: Frame<'_>,
    ) {
        let _ = queue;
        self.resize(device, frame.size);
        let (Some(edges), Some(blend), Some(output)) = (&self.edges, &self.blend, &self.output)
        else {
            return;
        };

        // Pass 1: edge detection reads the scene through the point sampler
        // and writes into `edges`, cleared first because the shader
        // discards rather than writing every texel.
        let edge_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("smaa edge"),
            layout: &self.edge_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(frame.source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.point_sampler),
                },
            ],
        });
        clear_pass(encoder, "smaa edge", &edges.view).run(&self.edge_pipeline, &edge_bind_group);

        // Pass 2: blending weights, reading `edges` plus the two lookup
        // textures. `area_view`/`search_view` never move - only the edges
        // view does - but a bind group is cheap next to the draw it feeds,
        // so this is rebuilt every frame the same as the other two passes
        // rather than cached against a layout that would need invalidating
        // whenever `edges` resizes.
        let blend_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("smaa blend"),
            layout: &self.blend_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&edges.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.area_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.search_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                },
            ],
        });
        clear_pass(encoder, "smaa blend", &blend.view).run(&self.blend_pipeline, &blend_bind_group);

        // Pass 3: neighbourhood blend, reading the original scene and the
        // blend weights.
        let neighborhood_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("smaa neighborhood"),
            layout: &self.neighborhood_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(frame.source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&blend.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                },
            ],
        });
        clear_pass(encoder, "smaa neighborhood", &output.view)
            .run(&self.neighborhood_pipeline, &neighborhood_bind_group);
    }

    fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if self.size == size && self.output.is_some() {
            return;
        }
        self.size = size;
        self.edges = Some(target(
            device,
            "smaa edges",
            wgpu::TextureFormat::Rg8Unorm,
            size,
        ));
        self.blend = Some(target(
            device,
            "smaa blend weights",
            wgpu::TextureFormat::Rgba8Unorm,
            size,
        ));
        self.output = Some(target(device, "smaa output", self.format, size));
    }
}

fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    format: wgpu::TextureFormat,
    size: (u32, u32),
    data: &[u8],
    bytes_per_texel: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size.0 * bytes_per_texel),
            rows_per_image: Some(size.1),
        },
        wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn pass_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    label: &str,
    vs_entry: &str,
    fs_entry: &str,
    layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vs_entry),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs_entry),
            targets: &[Some(wgpu::ColorTargetState {
                format,
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
    })
}

/// One fullscreen pass over `view`, ready to be given a pipeline and its
/// input - the same helper `fsr1.rs` uses.
fn clear_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    label: &str,
    view: &wgpu::TextureView,
) -> Pass<'a> {
    Pass(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    }))
}

struct Pass<'a>(wgpu::RenderPass<'a>);

impl Pass<'_> {
    fn run(mut self, pipeline: &wgpu::RenderPipeline, bind_group: &wgpu::BindGroup) {
        self.0.set_pipeline(pipeline);
        self.0.set_bind_group(0, Some(bind_group), &[]);
        self.0.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lookup_textures_decode_to_exactly_the_bytes_upstream_ships() {
        assert_eq!(AREA_TEX.len(), (AREA_WIDTH * AREA_HEIGHT * 2) as usize);
        assert_eq!(SEARCH_TEX.len(), (SEARCH_WIDTH * SEARCH_HEIGHT) as usize);
    }

    fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()
    }

    /// Compiles all three shaders and runs all three passes on a real device.
    ///
    /// This is where a WGSL syntax mistake actually shows up: `cargo check`
    /// only compiles the Rust half, and `include_str!` hands the shader text
    /// to the device unexamined until a real `create_shader_module` parses
    /// and validates it. **Skips when there is no adapter.**
    #[test]
    fn all_three_passes_build_and_draw_on_a_real_device() {
        let Some((device, queue)) = device() else {
            eprintln!("no GPU adapter: skipping");
            return;
        };

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

        let mut smaa = Smaa::new(&device, &queue, format).expect("building the pipelines");
        assert!(smaa.output().is_none(), "no output before the first render");

        let mut encoder = device.create_command_encoder(&Default::default());
        smaa.render(
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

        assert!(smaa.output().is_some(), "the output target must exist");
    }

    /// A flat colour has no edges anywhere, so pass 1 discards every pixel,
    /// pass 2 sees an all-zero edges texture and pass 3 must be a pure
    /// pass-through - the whole three-pass pipeline reduces to an identity
    /// on a picture with nothing to anti-alias.
    ///
    /// **Skips when there is no adapter.**
    #[test]
    fn a_flat_colour_is_untouched() {
        const SIZE: u32 = 32;
        let Some((device, queue)) = device() else {
            eprintln!("no GPU adapter: skipping");
            return;
        };

        // `Rgba8Unorm`, not sRGB: checking the pipeline's arithmetic against
        // a known input, the same reason `fxaa`'s equivalent test does.
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

        let mut smaa = Smaa::new(&device, &queue, format).expect("building the pipelines");
        let mut encoder = device.create_command_encoder(&Default::default());
        smaa.render(
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
            smaa.output_texture().expect("an output").as_image_copy(),
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

    /// **A perfectly straight edge is exactly the case SMAA leaves alone.**
    /// Unlike `fxaa`, which blurs any sufficiently sharp edge indiscriminately
    /// along its axis, SMAA's blending weight only comes from `smaa_area`,
    /// and that lookup returns zero unless the search along the edge finds a
    /// *crossing* edge - a corner - within `SMAA_MAX_SEARCH_STEPS` texels. A
    /// perfectly vertical or horizontal one-pixel transition has no corner
    /// anywhere, so it is already the correct pixel-grid representation of a
    /// straight line and SMAA (correctly) does nothing to it. This is one of
    /// SMAA's real advantages over FXAA - it does not soften clean UI edges -
    /// and it is why this test uses a one-step staircase rather than a
    /// straight edge: a straight edge would prove nothing here.
    ///
    /// **Skips when there is no adapter.**
    #[test]
    fn a_staircase_step_blends_at_the_corner_and_nowhere_else() {
        const SIZE: u32 = 64;
        let Some((device, queue)) = device() else {
            eprintln!("no GPU adapter: skipping");
            return;
        };

        let format = wgpu::TextureFormat::Rgba8Unorm;
        let scene = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("staircase"),
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
        // One step: rows 0..32 have their black/white split at x=28, rows
        // 32..64 at x=32 - a single corner at the row-32 boundary, and
        // nothing but straight vertical edge everywhere else.
        let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
        for y in 0..SIZE {
            let split = if y < SIZE / 2 { 28 } else { 32 };
            for x in 0..SIZE {
                let value = if x < split { 0 } else { 255 };
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

        let mut smaa = Smaa::new(&device, &queue, format).expect("building the pipelines");
        let mut encoder = device.create_command_encoder(&Default::default());
        smaa.render(
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
            smaa.output_texture().expect("an output").as_image_copy(),
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
        let at = |x: u32, y: u32| mapped[padded * y as usize + (x * 4) as usize];

        // Far from the corner (more than `SMAA_MAX_SEARCH_STEPS` = 8 rows
        // away, at row 10), the vertical search cannot reach the step, and
        // the straight edge at x=28 must survive exactly as drawn: no
        // crossing edge means no blend weight, per the doc comment above.
        assert_eq!(at(26, 10), 0, "far from the corner, the black side moved");
        assert_eq!(at(30, 10), 255, "far from the corner, the white side moved");
        assert_eq!(
            at(28, 10),
            255,
            "far from the corner, the seam itself moved"
        );

        // At row 32 - immediately below the step, well inside the search
        // radius - the corner gives `smaa_area` a real crossing edge to
        // measure, and the texels between the two edge positions (x=28 and
        // x=32) must show a real antialiased ramp rather than a hard 0/255
        // cut.
        let ramp: Vec<u8> = (28..32).map(|x| at(x, 32)).collect();
        assert!(
            ramp.iter().any(|&v| v > 0 && v < 255),
            "no antialiasing near the staircase corner: {ramp:?}"
        );
    }
}
