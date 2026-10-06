//! The particle draw pipeline and its two blend states.
//!
//! Split out of `psys.rs`, which is past the 1,000-line rule and ratcheted;
//! a move, plus the sprite sheet binding that arrived with it.

use super::sprite::{SHEET_SIZE, Sheet};
use super::{MAX_INSTANCES, MAX_PARTICLES};
use oag_mesh::mesh::GpuVertex;

mod distort;

/// The additive blend - the original's blend class 2, `BlendFunc(ADD,
/// SRC_ALPHA, FIX 0xffffff)`, used by the three bright emitters. The same
/// shape [`crate::exhaust::BLEND`] uses, and for the same reason: additive
/// keeps overlapping sparks reading as *brighter* rather than as one
/// occluding another.
pub const BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// The over blend - the original's blend class 3, `BlendFunc(ADD, SRC_ALPHA,
/// ONE_MINUS_SRC_ALPHA)`, used by the smoke. This is what lets a *dark*
/// smoke colour darken the scene behind it; drawn additively it would be
/// nearly invisible, which is exactly how the missing smoke bug looked.
pub const BLEND_ALPHA_OVER: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
};

/// The maximum vertices [`Pipeline`]'s buffer holds: one quad for every
/// particle a full [`super::Stage`] can hold.
///
/// Derived rather than picked. A buffer sized for one [`super::System`] would
/// silently drop whole effects off a busy grid - the truncation in
/// [`Pipeline::upload`] cuts at a vertex, so an over-long frame loses the
/// tail of the last quads and reads as an explosion that never happened.
pub const MAX_VERTICES: usize = MAX_INSTANCES * MAX_PARTICLES * 6;

/// The particle draw pipeline - two of them, one per blend class, sharing
/// the shader and layout.
///
/// One texture binds for every effect: the [`Sheet`] every loaded sprite is
/// packed into, which [`Pipeline::sync_sheet`] keeps current. An emitter
/// with no sprite of its own draws the procedural radial falloff instead -
/// see [`super::sprite`]. Otherwise it matches `mesh_render`'s pipeline
/// the same way the exhaust does: same target format, same
/// [`oag_mesh::mesh_render::DEPTH_FORMAT`], depth-tested but not
/// depth-writing, for the same transparency-ordering reason
/// `exhaust::Pipeline` documents.
#[derive(Debug)]
pub struct Pipeline {
    additive: wgpu::RenderPipeline,
    alpha_over: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// The [`Sheet`]'s texels on the GPU, transparent until the first sync.
    sheet: wgpu::Texture,
    /// [`Sheet::generation`] of what [`Self::sheet`] holds.
    sheet_generation: Option<u64>,
    additive_vertices: wgpu::Buffer,
    alpha_vertices: wgpu::Buffer,
    /// Vertices actually uploaded by the last [`Pipeline::upload`].
    additive_count: u32,
    alpha_count: u32,
    /// The screen flash, which shares this pass's targets - see
    /// [`crate::flash`].
    flash: crate::flash::Pipeline,
    /// Blend class 8's offset target - see [`distort`].
    distort: distort::Distort,
}

impl Pipeline {
    /// Builds the pipeline pair.
    ///
    /// `format` must be the target the caller's render pass writes, and
    /// `sample_count` must match its multisample state - see
    /// `mesh_render::build`.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
        velocity: oag_mesh::mesh_render::Velocity,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("psys"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/psys.wgsl")).into(),
            ),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("psys uniforms"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("psys"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });

        let build = |label: &str, blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GpuVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2,
                            4 => Float32
                        ],
                    })],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    // The velocity target, when the race adds one, rides
                    // along **write-masked empty** for the reason the
                    // exhaust's does: a particle quad is rebuilt from scratch
                    // every draw and writes no depth, so the velocity at its
                    // pixels stays the surface's behind it. See
                    // `mesh_render::Velocity`.
                    targets: &{
                        let mut targets = vec![Some(wgpu::ColorTargetState {
                            format,
                            blend: Some(blend),
                            // Colour only - the original's particle draw path
                            // (`FUN_08915fd0`) calls `Bloom_SetPixelMask(g_bloom, 0)`,
                            // protecting the glow mask. See `oag_post::bloom`.
                            write_mask: wgpu::ColorWrites::COLOR,
                        })];
                        targets.extend(velocity.target(true));
                        targets
                    },
                    compilation_options: oag_mesh::mesh_render::fragment_options(format),
                }),
                primitive: wgpu::PrimitiveState {
                    // A camera-facing quad has no meaningful winding: the
                    // basis it is built from flips as the camera orbits.
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: oag_mesh::mesh_render::DEPTH_FORMAT,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: sample_count,
                    ..Default::default()
                },
                multiview_mask: None,
                cache: None,
            })
        };
        let additive = build("psys additive", BLEND);
        let alpha_over = build("psys alpha-over", BLEND_ALPHA_OVER);

        let distort = distort::Distort::new(device, &pipeline_layout, sample_count);

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("psys uniforms"),
            size: oag_mesh::mesh_render::UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // `Rgba8Unorm`, not sRGB: the sprite's bytes are the GE's own and are
        // modulated in the space they were authored in, like every other
        // texture on a gamma target (ADR-0020).
        let sheet = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("psys sprite sheet"),
            size: wgpu::Extent3d {
                width: SHEET_SIZE,
                height: SHEET_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = sheet.create_view(&wgpu::TextureViewDescriptor::default());
        // Bilinear, clamped: **chosen, not measured.** The particle path sets
        // no filter of its own and inherits whatever the frame left; the
        // sheet's padding and the half-texel inset in `Atlas::cell` are what
        // keep a cell from reading its neighbour.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("psys sprite sheet"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("psys uniforms"),
            layout: &layout,
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
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let buffer = |label: &str| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (MAX_VERTICES * std::mem::size_of::<GpuVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let additive_vertices = buffer("psys additive vertices");
        let alpha_vertices = buffer("psys alpha-over vertices");

        Self {
            flash: crate::flash::Pipeline::new(device, format, sample_count, velocity),
            additive,
            alpha_over,
            uniforms,
            bind_group,
            sheet,
            sheet_generation: None,
            additive_vertices,
            alpha_vertices,
            additive_count: 0,
            alpha_count: 0,
            distort,
        }
    }

    /// Uploads `sheet`'s texels if they changed since the last call - once
    /// per race in practice, since a race loads its effects up front.
    pub fn sync_sheet(&mut self, queue: &wgpu::Queue, sheet: &Sheet) {
        if self.sheet_generation == Some(sheet.generation()) {
            return;
        }
        if let Some(pixels) = sheet.pixels() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.sheet,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(SHEET_SIZE * 4),
                    rows_per_image: Some(SHEET_SIZE),
                },
                wgpu::Extent3d {
                    width: SHEET_SIZE,
                    height: SHEET_SIZE,
                    depth_or_array_layers: 1,
                },
            );
        }
        self.sheet_generation = Some(sheet.generation());
    }

    /// Uploads this frame's camera matrix and both blend classes' geometry,
    /// as [`super::System::vertices`] returns them.
    ///
    /// Takes `&mut self` only for the vertex counts; the writes go through
    /// `queue`, the same split [`crate::exhaust::Pipeline::upload`] uses.
    pub fn upload(
        &mut self,
        queue: &wgpu::Queue,
        view_projection: &[[f32; 4]; 4],
        additive: &[GpuVertex],
        alpha_over: &[GpuVertex],
    ) {
        let mut block = [[0.0f32; 4]; 8];
        block[..4].copy_from_slice(view_projection);
        block[4] = [1.0, 0.0, 0.0, 0.0];
        block[5] = [0.0, 1.0, 0.0, 0.0];
        block[6] = [0.0, 0.0, 1.0, 0.0];
        block[7] = [0.0, 0.0, 0.0, 1.0];
        queue.write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&block));

        // A `Stage` cannot produce more than this, so an overflow means the
        // buffer and the pool have drifted apart rather than that the scene
        // is busy - worth failing on in a debug build instead of quietly
        // losing the tail.
        debug_assert!(
            additive.len() <= MAX_VERTICES && alpha_over.len() <= MAX_VERTICES,
            "particle vertices past the buffer: {} additive, {} alpha, cap {MAX_VERTICES}",
            additive.len(),
            alpha_over.len(),
        );
        let n = additive.len().min(MAX_VERTICES);
        queue.write_buffer(
            &self.additive_vertices,
            0,
            bytemuck::cast_slice(&additive[..n]),
        );
        self.additive_count = n as u32;

        let n = alpha_over.len().min(MAX_VERTICES);
        queue.write_buffer(
            &self.alpha_vertices,
            0,
            bytemuck::cast_slice(&alpha_over[..n]),
        );
        self.alpha_count = n as u32;
    }

    /// Uploads this frame's blend class 8 quads, [`super::Stage::extend_distort_vertices`].
    pub fn upload_distort(&mut self, queue: &wgpu::Queue, vertices: &[GpuVertex]) {
        self.distort.upload(queue, vertices);
    }

    /// Draws the frame's offsets into their own target, **after** the scene
    /// pass has closed, and returns the texture the composite reads; `None`
    /// when the frame drew none. `depth` is the scene's depth attachment,
    /// `size` its extent and `viewport` the rectangle the scene pass drew -
    /// see [`distort`] for the executable's law and what differs from it.
    pub fn encode_distort(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        depth: &wgpu::TextureView,
        size: (u32, u32),
        viewport: (f32, f32, f32, f32),
    ) -> Option<&wgpu::TextureView> {
        self.distort
            .encode(device, encoder, &self.bind_group, depth, size, viewport)
    }

    /// The texture behind [`Self::encode_distort`]'s view, for a readback. It
    /// holds the last frame that drew anything; `None` before the first.
    #[must_use]
    pub fn offset_texture(&self) -> Option<&wgpu::Texture> {
        self.distort.texture()
    }

    /// This frame's screen flash, [`crate::flash::ScreenFlash::colour`].
    pub fn upload_flash(&mut self, queue: &wgpu::Queue, colour: Option<[f32; 4]>) {
        self.flash.upload(queue, colour);
    }

    /// Draws the screen flash, if one runs - **last** in the scene pass, so
    /// it washes over everything the pass drew and nothing the HUD does.
    pub fn draw_flash(&self, pass: &mut wgpu::RenderPass<'_>) {
        self.flash.draw(pass);
    }

    /// Draws into a pass the caller already opened.
    ///
    /// Must be issued **after** the opaque geometry, for the same
    /// depth-write-off reason as [`crate::exhaust::Pipeline::draw`]. The
    /// alpha-over smoke draws first, then the additive sparks on top -
    /// the original interleaves them in particle order, which two batched
    /// draws cannot reproduce exactly; additive-last is the closer
    /// approximation since adding light commutes.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.alpha_count > 0 {
            pass.set_pipeline(&self.alpha_over);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.alpha_vertices.slice(..));
            pass.draw(0..self.alpha_count, 0..1);
        }
        if self.additive_count > 0 {
            pass.set_pipeline(&self.additive);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.additive_vertices.slice(..));
            pass.draw(0..self.additive_count, 0..1);
        }
    }
}
