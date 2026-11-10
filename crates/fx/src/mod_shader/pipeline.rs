//! [`Stage`]: the GPU half of [`super`]. One pipeline pair per shader, one vertex buffer the
//! frame's surfaces share, drawn in the race pass with the view's own [`Drawn`].

use std::ops::Range;

use oag_mesh::mesh::GpuVertex;
use oag_mesh::mesh_render::{DEPTH_FORMAT, Velocity, fragment_options};

use super::{Blend, Compiled, MAX_PARAMS};

/// Vertices one view's surfaces can hold; the rest of a frame's are cut off.
const MAX_VERTICES: usize = 16 * 1024;

/// `mod_shader.wesl`'s `ModFrame`.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_projection: [[f32; 4]; 4],
    engine_view_projection: [[f32; 4]; 4],
    craft: [[f32; 4]; 4],
    fill: [f32; 4],
    params: [f32; MAX_PARAMS],
    time: f32,
    between: f32,
    engine_bound: f32,
    pad: f32,
    lens: [f32; 2],
    pad_lens: [f32; 2],
}

const IDENTITY: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// A shader and the texture its header named, ready for [`Stage::new`].
#[derive(Debug, Clone)]
pub struct Shader {
    /// The module.
    pub compiled: Compiled,
    /// The header's PNG as `(width, height, rgba)`.
    pub texture: Option<(u32, u32, Vec<u8>)>,
}

/// A screen fill through a shader.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fill {
    /// Its place in [`Stage::new`]'s list.
    pub shader: usize,
    /// The colour, straight alpha.
    pub colour: [f32; 4],
}

/// One view's draws.
#[derive(Debug)]
pub struct Frame<'a> {
    pub view_projection: [[f32; 4]; 4],
    /// Seconds on the race's world clock.
    pub time: f32,
    /// [`crate::psys`]'s draw alpha.
    pub draw_alpha: f32,
    /// What `engine_uv` projects with in this view; `None` is no picture for this view.
    pub engine_view_projection: Option<[[f32; 4]; 4]>,
    /// `craft_frame`: the view's own craft in eye space, rigid, as it is drawn.
    pub craft: [[f32; 4]; 4],
    /// `eye_ray`'s scale: the tangents of half the view's field, across and up.
    pub lens: [f32; 2],
    /// The view's fill.
    pub fill: Option<Fill>,
    /// Triangles per shader.
    pub surfaces: &'a [(usize, Vec<GpuVertex>)],
    /// The scalars each shader was sent, by its place, over the defaults of its header; a
    /// `None` keeps the default and a place past the end keeps them all.
    pub params: &'a [[Option<f32>; MAX_PARAMS]],
}

/// What the last upload left to draw, as a value: a split screen records its views at once, so
/// each view keeps its own.
#[derive(Debug, Clone, Default)]
pub struct Drawn {
    runs: Vec<(usize, Range<u32>)>,
    fill: Option<usize>,
    /// The fill's shader is a backdrop's (`fill = "backdrop"`).
    backdrop: bool,
}

#[derive(Debug, Clone)]
struct Built {
    surface: wgpu::RenderPipeline,
    fill: wgpu::RenderPipeline,
    backdrop: wgpu::RenderPipeline,
    fringe: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

/// What [`Stage`]'s draws bind, cloned into a view's recording.
#[derive(Debug, Clone)]
pub struct Drawer {
    built: Vec<Option<Built>>,
    vertices: wgpu::Buffer,
}

impl Drawer {
    /// The surfaces, depth tested and writing no depth: with the effects.
    pub fn draw_surfaces(&self, pass: &mut wgpu::RenderPass<'_>, drawn: &Drawn) {
        if drawn.runs.is_empty() {
            return;
        }
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        for (shader, range) in &drawn.runs {
            if let Some(Some(built)) = self.built.get(*shader) {
                pass.set_pipeline(&built.surface);
                pass.set_bind_group(0, &built.bind_group, &[]);
                pass.draw(range.clone(), 0..1);
            }
        }
    }

    /// The fill, after every world layer. Of a backdrop, what [`Self::draw_backdrop`] left: the
    /// texels under half alpha, which hide nothing.
    pub fn draw_fill(&self, pass: &mut wgpu::RenderPass<'_>, drawn: &Drawn) {
        if let Some(Some(built)) = drawn.fill.and_then(|shader| self.built.get(shader)) {
            pass.set_pipeline(if drawn.backdrop {
                &built.fringe
            } else {
                &built.fill
            });
            pass.set_bind_group(0, &built.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    /// A backdrop fill, before the crafts: over the world drawn so far, writing a depth
    /// nearer than any of it where it is at least half there, so the world's later layers
    /// stay behind it.
    pub fn draw_backdrop(&self, pass: &mut wgpu::RenderPass<'_>, drawn: &Drawn) {
        if !drawn.backdrop {
            return;
        }
        if let Some(Some(built)) = drawn.fill.and_then(|shader| self.built.get(shader)) {
            pass.set_pipeline(&built.backdrop);
            pass.set_bind_group(0, &built.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    /// Whether the last upload left a backdrop to draw.
    #[must_use]
    pub fn has_backdrop(drawn: &Drawn) -> bool {
        drawn.backdrop && drawn.fill.is_some()
    }
}

/// One shader's state beside its pipelines.
#[derive(Debug)]
struct Slot {
    uniforms: wgpu::Buffer,
    mod_texture: wgpu::TextureView,
    defaults: [f32; MAX_PARAMS],
    backdrop: bool,
}

/// Every shader of a race. A place that did not build draws nothing.
#[derive(Debug)]
pub struct Stage {
    drawer: Drawer,
    slots: Vec<Option<Slot>>,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// The 1x1 transparent black an engine texture slot holds with nothing bound.
    fallback: wgpu::TextureView,
    drawn: Drawn,
    failures: Vec<String>,
}

fn one_pixel(device: &wgpu::Device, queue: &wgpu::Queue, rgba: [u8; 4]) -> wgpu::TextureView {
    texture(device, queue, (1, 1, &rgba))
}

fn texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    (width, height, rgba): (u32, u32, &[u8]),
) -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("mod shader texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: None,
        },
        size,
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn binding_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("mod shader"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            texture_entry(1),
            texture_entry(2),
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn state(blend: Blend) -> wgpu::BlendState {
    match blend {
        Blend::Alpha => crate::psys::BLEND_ALPHA_OVER,
        Blend::Add => crate::psys::BLEND,
    }
}

/// What a pipeline of a [`Stage`] targets: the race pass.
#[derive(Debug, Clone, Copy)]
struct Pass {
    format: wgpu::TextureFormat,
    store: oag_gpu::formats::SceneStore,
    sample_count: u32,
    velocity: Velocity,
}

fn pipelines(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    blend: Blend,
    target: Pass,
) -> [wgpu::RenderPipeline; 4] {
    let mut targets = vec![Some(wgpu::ColorTargetState {
        format: target.format,
        blend: Some(state(blend)),
        // The glow mask stays the surface's behind, as a particle leaves it.
        write_mask: wgpu::ColorWrites::COLOR,
    })];
    targets.extend(target.velocity.target(true));
    let build = |label,
                 (entry, fragment): (&str, &str),
                 buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
                 (compare, write)| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some(entry),
                buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some(fragment),
                targets: &targets,
                compilation_options: fragment_options(target.format, target.store),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(write),
                depth_compare: Some(compare),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: target.sample_count,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        })
    };
    let surface = build(
        "mod shader surface",
        ("vs_surface", "fs_main"),
        &[Some(wgpu::VertexBufferLayout {
            array_stride: size_of::<GpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &crate::psys::SURFACE_ATTRIBUTES,
        })],
        (wgpu::CompareFunction::Less, false),
    );
    let fill = build(
        "mod shader fill",
        ("vs_fill", "fs_main"),
        &[],
        (wgpu::CompareFunction::Always, false),
    );
    let backdrop = build(
        "mod shader backdrop",
        ("vs_fill", "fs_backdrop"),
        &[],
        (wgpu::CompareFunction::Always, true),
    );
    let fringe = build(
        "mod shader backdrop fringe",
        ("vs_fill", "fs_fringe"),
        &[],
        (wgpu::CompareFunction::Always, false),
    );
    [surface, fill, backdrop, fringe]
}

impl Stage {
    /// Builds every shader of `shaders`, each kept at its place; a `None`, or one the device
    /// refuses, stays a place that draws nothing ([`Self::failures`] names the second kind).
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        store: oag_gpu::formats::SceneStore,
        sample_count: u32,
        velocity: Velocity,
        shaders: &[Option<Shader>],
    ) -> Self {
        let layout = binding_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mod shader"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mod shader"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let fallback = one_pixel(device, queue, [0; 4]);
        let target = Pass {
            format,
            store,
            sample_count,
            velocity,
        };
        let mut built = Vec::with_capacity(shaders.len());
        let mut slots = Vec::with_capacity(shaders.len());
        let mut failures = Vec::new();
        for (place, shader) in shaders.iter().enumerate() {
            let Some(shader) = shader else {
                built.push(None);
                slots.push(None);
                continue;
            };
            let made = oag_gpu::shader_file::scoped(device, || {
                let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("mod shader"),
                    source: wgpu::ShaderSource::Wgsl(shader.compiled.wgsl.as_str().into()),
                });
                pipelines(
                    device,
                    &pipeline_layout,
                    &module,
                    shader.compiled.blend,
                    target,
                )
            });
            let [surface, fill, backdrop, fringe] = match made {
                Ok(pair) => pair,
                Err(why) => {
                    failures.push(format!("shader {place} left out: {why}; draws nothing"));
                    built.push(None);
                    slots.push(None);
                    continue;
                }
            };
            let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("mod shader"),
                size: size_of::<Uniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mod_texture = match &shader.texture {
                Some((width, height, rgba)) => texture(device, queue, (*width, *height, rgba)),
                None => one_pixel(device, queue, [255; 4]),
            };
            let slot = Slot {
                uniforms,
                mod_texture,
                defaults: shader.compiled.defaults,
                backdrop: shader.compiled.backdrop,
            };
            let bind_group = bind(device, &layout, &sampler, &slot, &fallback);
            built.push(Some(Built {
                surface,
                fill,
                backdrop,
                fringe,
                bind_group,
            }));
            slots.push(Some(slot));
        }
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mod shader vertices"),
            size: (MAX_VERTICES * size_of::<GpuVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            drawer: Drawer { built, vertices },
            slots,
            layout,
            sampler,
            fallback,
            drawn: Drawn::default(),
            failures,
        }
    }

    /// What the device refused at [`Self::new`], for the load log.
    #[must_use]
    pub fn failures(&self) -> &[String] {
        &self.failures
    }

    /// Fills every shader's engine texture slot with `view`, a texture of the scene's format;
    /// `None` empties it. Rebuilds the bind groups, so it belongs at a change of target, not in
    /// every frame: which views have a picture in it is [`Frame::engine_view_projection`].
    pub fn set_engine_texture(&mut self, device: &wgpu::Device, view: Option<&wgpu::TextureView>) {
        let engine = view.unwrap_or(&self.fallback);
        for (built, slot) in self.drawer.built.iter_mut().zip(&self.slots) {
            if let (Some(built), Some(slot)) = (built, slot) {
                built.bind_group = bind(device, &self.layout, &self.sampler, slot, engine);
            }
        }
    }

    /// Writes one view's draws. A shader that draws nothing this view writes nothing.
    pub fn upload(&mut self, queue: &dyn oag_gpu::writes::Writes, frame: &Frame<'_>) {
        let mut drawn = Drawn::default();
        let mut at = 0;
        for (shader, vertices) in frame.surfaces {
            let count = vertices.len().min(MAX_VERTICES - at);
            if count == 0 || !matches!(self.slots.get(*shader), Some(Some(_))) {
                continue;
            }
            queue.write_buffer(
                &self.drawer.vertices,
                (at * size_of::<GpuVertex>()) as u64,
                bytemuck::cast_slice(&vertices[..count]),
            );
            drawn.runs.push((*shader, at as u32..(at + count) as u32));
            at += count;
        }
        drawn.fill = frame.fill.map(|fill| fill.shader);
        drawn.backdrop = drawn
            .fill
            .and_then(|place| self.slots.get(place)?.as_ref())
            .is_some_and(|slot| slot.backdrop);
        let colour = frame.fill.map_or([0.0; 4], |fill| fill.colour);
        for (place, slot) in self.slots.iter().enumerate() {
            let Some(slot) = slot else { continue };
            if drawn.fill != Some(place) && !drawn.runs.iter().any(|(shader, _)| *shader == place) {
                continue;
            }
            let sent = frame.params.get(place).copied().unwrap_or_default();
            let uniforms = Uniforms {
                view_projection: frame.view_projection,
                engine_view_projection: frame.engine_view_projection.unwrap_or(IDENTITY),
                craft: frame.craft,
                fill: colour,
                params: std::array::from_fn(|i| sent[i].unwrap_or(slot.defaults[i])),
                time: frame.time,
                between: frame.draw_alpha - 1.0,
                engine_bound: f32::from(u8::from(frame.engine_view_projection.is_some())),
                pad: 0.0,
                lens: frame.lens,
                pad_lens: [0.0; 2],
            };
            queue.write_buffer(&slot.uniforms, 0, bytemuck::bytes_of(&uniforms));
        }
        self.drawn = drawn;
    }

    /// What the last upload left to draw.
    #[must_use]
    pub fn drawn(&self) -> Drawn {
        self.drawn.clone()
    }

    /// The handles the draws bind.
    #[must_use]
    pub fn drawer(&self) -> &Drawer {
        &self.drawer
    }
}

fn bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    slot: &Slot,
    engine: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("mod shader"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: slot.uniforms.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&slot.mod_texture),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(engine),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}
