//! Subpixel Morphological Anti-Aliasing: luma edge detection, blending
//! weight calculation against a precomputed area/search lookup, then a
//! neighbourhood blend. Three fullscreen passes, chained together the way
//! [`smaa.wesl`](../shaders/smaa.wesl)'s own module docs diagram.
//!
//! Ported from `iryoku/smaa` (MIT) the same way `fsr1` is ported from AMD's
//! FidelityFX - see [ADR-0012](../../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)
//! for why this project transliterates rather than links or vendors a native
//! SDK. This is specifically upstream's **MEDIUM** preset: see `smaa.wesl`'s
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

use super::{sampler_entry, texture_entry, uniform_entry};

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
    /// The size of the resource behind `source`, and of the intermediates this
    /// resizes to. **The allocation, not the drawn rectangle** - keying three
    /// targets off this is what stops a moving render extent rebuilding all of
    /// them several times a second.
    pub size: (u32, u32),
    /// The rectangle of `source` that was drawn, `<= size` on both axes. Every
    /// pass draws into the same rectangle of its own target, so `edges` and
    /// `blend` carry it too and one scale covers the chain.
    pub viewport: (u32, u32),
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

/// The one uniform all three passes share: which sub-rectangle of the
/// allocation-sized targets was actually drawn.
///
/// SMAA had no uniform at all until dynamic resolution needed one - every
/// number the shader wants it derives from `textureDimensions`, which returns
/// the *resource* and so stays correct here. What is not derivable is the
/// mapping from the fullscreen triangle's `0..1` onto the drawn rectangle, and
/// that is all this carries. Sixteen bytes, a uniform binding's minimum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Constants {
    uv_scale: [f32; 2],
    uv_max: [f32; 2],
}

impl Constants {
    fn new(viewport: (u32, u32), size: (u32, u32)) -> Self {
        let (uv_scale, uv_max) = super::sub_rectangle(viewport, size);
        Self { uv_scale, uv_max }
    }
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

    constants: wgpu::Buffer,
    written: Option<Constants>,

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
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/smaa.wgsl")).into(),
            ),
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
        // through this - see `smaa.wesl`'s `@PSEUDO_GATHER4` note - so this
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
            entries: &[texture_entry(0), sampler_entry(1), uniform_entry(4)],
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
                uniform_entry(4),
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
                entries: &[
                    texture_entry(0),
                    texture_entry(1),
                    sampler_entry(2),
                    uniform_entry(4),
                ],
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

        let constants = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("smaa constants"),
            size: size_of::<Constants>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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
            constants,
            written: None,
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
        self.resize(device, frame.size);

        // Four floats, moved only by a resolution controller and a window
        // resize. Uploaded when they change, the idiom `fxaa` and `fsr1` use.
        let wanted = Constants::new(frame.viewport, frame.size);
        if self.written != Some(wanted) {
            queue.write_buffer(&self.constants, 0, bytemuck::bytes_of(&wanted));
            self.written = Some(wanted);
        }

        let (Some(edges), Some(blend), Some(output)) = (&self.edges, &self.blend, &self.output)
        else {
            return;
        };

        // Pass 1: edge detection reads the scene through the point sampler
        // and writes into `edges`, cleared first because the shader
        // discards rather than writing every texel.
        let edge_bind_group = oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
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
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: self.constants.as_entire_binding(),
                    },
                ],
            },
        );
        clear_pass(encoder, "smaa edge", &edges.view, frame.viewport)
            .run(&self.edge_pipeline, &edge_bind_group);

        // Pass 2: blending weights, reading `edges` plus the two lookup
        // textures. `area_view`/`search_view` never move - only the edges
        // view does - but a bind group is cheap next to the draw it feeds,
        // so this is rebuilt every frame the same as the other two passes
        // rather than cached against a layout that would need invalidating
        // whenever `edges` resizes.
        let blend_bind_group = oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
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
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: self.constants.as_entire_binding(),
                    },
                ],
            },
        );
        clear_pass(encoder, "smaa blend", &blend.view, frame.viewport)
            .run(&self.blend_pipeline, &blend_bind_group);

        // Pass 3: neighbourhood blend, reading the original scene and the
        // blend weights.
        let neighborhood_bind_group = oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
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
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: self.constants.as_entire_binding(),
                    },
                ],
            },
        );
        clear_pass(encoder, "smaa neighborhood", &output.view, frame.viewport)
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
    viewport: (u32, u32),
) -> Pass<'a> {
    let mut pass = Pass(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
    }));
    // The clear covers the whole attachment - `LoadOp::Clear` is not
    // viewport-restricted - and the draw covers the drawn rectangle. That is
    // deliberate and is what the `search_*` loops in `smaa.wesl` lean on: the
    // edges outside the viewport are zero, so a search terminates there
    // exactly as it does at the frame border.
    pass.0.set_viewport(
        0.0,
        0.0,
        viewport.0.max(1) as f32,
        viewport.1.max(1) as f32,
        0.0,
        1.0,
    );
    pass
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
mod tests;
