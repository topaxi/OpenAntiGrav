//! The Fury menu backdrop's GPU side: the clouds as instance buffers, the
//! procedural sprite, and the passes `backdrop.wesl` carries.
//!
//! `oag_ui::backdrop::Fury` decides what a frame looks like and hands over a
//! [`Frame`]; this draws it, in the order
//! [menu-backdrop.md](../../../../docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md),
//! "The frame, measured", reads off the original's command stream. Once per
//! frame, before the page's own render pass:
//!
//! 1. **Points** - the current cloud as camera-facing sprites, additively
//!    (`SRC_ALPHA, ONE`, as `PointCloud_Submit` sets it) into the cleared
//!    particle field, at the viewport's size.
//! 2. **Blend** - the field times the screen's tint, capped, over the decayed
//!    accumulation, at the same size. The original reads the target it writes;
//!    here two targets swap each frame.
//! 3. **Glow** - the accumulation added to the half-size target holding last
//!    frame's wave, through the glow gain as the constant blend colour.
//! 4. **Wave** - that glow dimmed through a pinched grid into the other half
//!    target. The two swap each frame.
//! 5. **Composite** - accumulation plus glow, over the field, blending off.
//! 6. **Band** - the smear band, which reads the field it writes, so the field
//!    is copied to a second target first.
//! 7. **Scanlines** - every fourth row multiplied by a dark blue.
//! 8. **Present** - the field read half a row low, bilinearly and in linear
//!    light, which is why a scanline reaches the screen as two half-dark rows.
//!
//! Then, inside the page's pass where the draw list's
//! [`oag_ui::frontend::Draw::FuryBackdrop`] sits, the result goes onto the page.
//!
//! The original runs all of this in linear light, decoding on read and
//! encoding on write. Here that is the targets' sRGB format, with the page
//! itself left gamma-space (ADR-0020): the last step reads the field through a
//! plain view, so the page receives the encoded bytes as they stand.
//!
//! A pass reads a target nearest but for the half-size glow and the present,
//! which are bilinear: the filter words on the command stream, not the views'.

use oag_rcs::points2::PointCloud;
use oag_ui::backdrop::{self, Frame};

use super::resources::{sampler_entry, texture_entry, uniform_entry};

/// The point pass's constants, `backdrop.wesl`'s `Points`.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PointUniforms {
    world_view: [[f32; 4]; 4],
    proj: [[f32; 4]; 4],
    colour_ramp_factors: [f32; 4],
    colour_ramp_factors2: [f32; 4],
    dof: [f32; 4],
    fog: [f32; 4],
    particle_colour: [f32; 4],
    colour_ramp: [f32; 4],
    depth_fade: [f32; 4],
}

/// The post passes' constants, `backdrop.wesl`'s `Post`.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PostUniforms {
    feedback: [f32; 4],
    wave_scale: [f32; 4],
    wave_bias: [f32; 4],
    tint: [f32; 4],
    geometry: [f32; 4],
    scanline: [f32; 4],
    band: [f32; 4],
}

/// One point as the vertex shader reads it: position, `rand.x` and normal,
/// 32 bytes so every attribute is four-aligned.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    position: [f32; 3],
    rand: f32,
    normal: [f32; 3],
    _pad: f32,
}

/// The sprite's size and mip count, `PointCloud_Construct`'s.
const SPRITE_SIZE: u32 = 32;
const SPRITE_MIPS: u32 = 6;

/// `PointCloud_MakeSprite`'s parameters for the sprite every quad mode binds:
/// `(exp0, exp1, amp0, amp1, curve)`. Dim and soft when it covers many pixels
/// (mip 0), bright and tight when it is a dot.
const SPRITE_SHAPE: (f32, f32, f32, f32, f32) = (0.6, 2.0, 0.07, 1.0, 1.0);

/// The offscreen targets' format - see the module docs.
const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// The same bytes read undecoded, for the page.
const ENCODED_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// One uploaded cloud.
struct Cloud {
    instances: wgpu::Buffer,
    count: u32,
    /// Its bounding box's centre, which the wave's pivot follows on screen.
    centre: [f32; 3],
}

/// The targets, rebuilt when the viewport changes. Each pair swaps every
/// frame; `[last]` holds what the frame before left.
struct Targets {
    size: (u32, u32),
    field: wgpu::Texture,
    field_view: wgpu::TextureView,
    finished: wgpu::Texture,
    finished_view: wgpu::TextureView,
    accumulation: [wgpu::TextureView; 2],
    half: [wgpu::TextureView; 2],
    /// By `last`: `(field, accumulation[last])`.
    blend: [wgpu::BindGroup; 2],
    /// By `last`: the accumulation that frame's blend wrote.
    glow: [wgpu::BindGroup; 2],
    /// By `last`: `half[last]`, for the wave to read.
    wave: [wgpu::BindGroup; 2],
    /// By `last`: the new accumulation and the new wave.
    composite: [wgpu::BindGroup; 2],
    /// The field, for the band to read.
    band: wgpu::BindGroup,
    /// The finished field, for the present to read.
    present: wgpu::BindGroup,
    /// What the present wrote, undecoded, for the page.
    page: wgpu::BindGroup,
}

/// The backdrop's GPU resources.
pub struct FuryBackdrop {
    clouds: Vec<Cloud>,
    point_pipeline: wgpu::RenderPipeline,
    blend_pipeline: wgpu::RenderPipeline,
    glow_pipeline: wgpu::RenderPipeline,
    wave_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    band_pipeline: wgpu::RenderPipeline,
    scanline_pipeline: wgpu::RenderPipeline,
    present_pipeline: wgpu::RenderPipeline,
    page_pipeline: wgpu::RenderPipeline,
    point_uniforms: wgpu::Buffer,
    post_uniforms: wgpu::Buffer,
    point_bind_group: wgpu::BindGroup,
    post_layout: wgpu::BindGroupLayout,
    post_sampler: wgpu::Sampler,
    trail_sampler: wgpu::Sampler,
    targets: Option<Targets>,
    /// Which of each pair of targets the frame before left its result in.
    last: usize,
    /// The frame last drawn, so a display faster than the frame rate shows it
    /// again rather than feeding it back twice.
    drawn: Option<u32>,
}

impl std::fmt::Debug for FuryBackdrop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuryBackdrop")
            .field("clouds", &self.clouds.len())
            .finish_non_exhaustive()
    }
}

impl FuryBackdrop {
    /// Uploads `clouds` and builds the pipelines, the last one's for a page
    /// of `page_format`.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        page_format: wgpu::TextureFormat,
        clouds: &[PointCloud],
    ) -> Self {
        let clouds = clouds
            .iter()
            .map(|cloud| {
                let instances: Vec<Instance> = cloud
                    .points
                    .iter()
                    .map(|p| Instance {
                        position: p.position,
                        rand: f32::from(p.rand[0]) / 255.0,
                        normal: p.normal,
                        _pad: 0.0,
                    })
                    .collect();
                Cloud {
                    instances: oag_gpu::init_buffer::init(
                        device,
                        "fury cloud",
                        wgpu::BufferUsages::VERTEX,
                        bytemuck::cast_slice(&instances),
                    ),
                    count: instances.len() as u32,
                    centre: std::array::from_fn(|axis| (cloud.min[axis] + cloud.max[axis]) * 0.5),
                }
            })
            .collect();

        let point_uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fury points"),
            size: std::mem::size_of::<PointUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let post_uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fury post"),
            size: std::mem::size_of::<PostUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sprite_view = upload_sprite(device, queue);
        let sprite_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fury sprite"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let point_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fury points"),
            entries: &[
                uniform_entry(0),
                texture_entry(1),
                sampler_entry(2, wgpu::SamplerBindingType::Filtering),
            ],
        });
        let point_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fury points"),
            layout: &point_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: point_uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&sprite_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sprite_sampler),
                },
            ],
        });
        let post_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fury post"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let trail_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fury glow"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let post_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fury post"),
            entries: &[
                uniform_entry(0),
                texture_entry(1),
                texture_entry(2),
                sampler_entry(3, wgpu::SamplerBindingType::NonFiltering),
                sampler_entry(4, wgpu::SamplerBindingType::Filtering),
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fury backdrop"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/backdrop.wgsl")).into(),
            ),
        });
        let point_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("fury points"),
                bind_group_layouts: &[Some(&point_layout)],
                immediate_size: 0,
            });
        let post_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fury post"),
            bind_group_layouts: &[Some(&post_layout)],
            immediate_size: 0,
        });

        // `BlendFunc(SRC_ALPHA, ONE)`, `FUNC_ADD`, no depth test, no cull.
        let additive = wgpu::BlendState {
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
        let point_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fury points"),
            layout: Some(&point_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_points"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Instance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32x3],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_points"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: TARGET_FORMAT,
                    blend: Some(additive),
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

        let post_pipeline = |label: &str, (vertex, fragment): (&str, &str), format, blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&post_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vertex),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fragment),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
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
        };
        let opaque = |label: &str, fragment: &str| {
            post_pipeline(label, ("vs_post", fragment), TARGET_FORMAT, None)
        };
        let blend_pipeline = opaque("fury blend", "fs_blend");
        let composite_pipeline = opaque("fury composite", "fs_composite");
        let present_pipeline = opaque("fury present", "fs_present");
        let wave_pipeline = post_pipeline("fury wave", ("vs_wave", "fs_wave"), TARGET_FORMAT, None);
        let band_pipeline = post_pipeline("fury band", ("vs_band", "fs_band"), TARGET_FORMAT, None);
        // `BlendFuncSeparate(CONSTANT_COLOR, ONE, ZERO, ONE)`.
        let glow = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Constant,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let glow_pipeline = post_pipeline(
            "fury glow",
            ("vs_post", "fs_copy"),
            TARGET_FORMAT,
            Some(glow),
        );
        // `BlendFuncSeparate(ZERO, SRC_COLOR, ONE, ZERO)`.
        let scanlines = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::Src,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::Zero,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let scanline_pipeline = post_pipeline(
            "fury scanlines",
            ("vs_post", "fs_scanlines"),
            TARGET_FORMAT,
            Some(scanlines),
        );
        // The page keeps what it has and the backdrop is added; its alpha is
        // left alone. Every Fury page is black behind the backdrop, where the
        // original's composite overwrites instead.
        let over_page = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let page_pipeline = post_pipeline(
            "fury page",
            ("vs_post", "fs_page"),
            page_format,
            Some(over_page),
        );

        Self {
            clouds,
            point_pipeline,
            blend_pipeline,
            glow_pipeline,
            wave_pipeline,
            composite_pipeline,
            band_pipeline,
            scanline_pipeline,
            present_pipeline,
            page_pipeline,
            point_uniforms,
            post_uniforms,
            point_bind_group,
            post_layout,
            post_sampler,
            trail_sampler,
            targets: None,
            last: 0,
            drawn: None,
        }
    }

    /// How many clouds are uploaded.
    #[must_use]
    pub fn clouds(&self) -> usize {
        self.clouds.len()
    }

    /// The offscreen passes for `frame`, into targets sized to the viewport.
    /// Encoded before the page's own pass, which is what [`Self::composite`]
    /// then reads from. The second call for the same frame draws nothing.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame,
        viewport: (u32, u32),
    ) {
        let size = (viewport.0.max(1), viewport.1.max(1));
        if self.targets.as_ref().is_none_or(|t| t.size != size) {
            self.targets = Some(self.targets_for(device, size));
            self.drawn = None;
        }
        if self.drawn == Some(frame.clock) {
            return;
        }
        let Some(cloud) = self.clouds.get(frame.cloud) else {
            return;
        };
        self.drawn = Some(frame.clock);
        queue.write_buffer(
            &self.point_uniforms,
            0,
            bytemuck::bytes_of(&PointUniforms {
                world_view: frame.world_view,
                proj: frame.proj,
                colour_ramp_factors: frame.colour_ramp_factors,
                colour_ramp_factors2: frame.colour_ramp_factors2,
                dof: frame.dof_factors,
                fog: frame.fog_factors,
                particle_colour: with_w(frame.particle_colour, frame.sprite_size),
                colour_ramp: with_w(frame.colour_ramp, frame.music_multiplier),
                depth_fade: [
                    frame.depth_fade_factors[0],
                    frame.depth_fade_factors[1],
                    0.0,
                    0.0,
                ],
            }),
        );
        let pivot = frame.wave_pivot(cloud.centre);
        queue.write_buffer(
            &self.post_uniforms,
            0,
            bytemuck::bytes_of(&PostUniforms {
                feedback: with_w(frame.feedback, 0.0),
                wave_scale: with_w(backdrop::WAVE_COLOUR_SCALE, 0.0),
                wave_bias: [
                    backdrop::WAVE_COLOUR_BIAS,
                    backdrop::WAVE_BIAS_FACTOR,
                    backdrop::WAVE_PINCH,
                    backdrop::WAVE_DRIFT,
                ],
                tint: frame.tint,
                geometry: [
                    pivot[0],
                    pivot[1],
                    frame.band_top,
                    backdrop::BAND_HALF_HEIGHT,
                ],
                scanline: with_w(backdrop::SCANLINE_COLOUR, backdrop::BAND_BEND),
                band: [backdrop::BAND_DIM, 0.0, 0.0, 0.0],
            }),
        );
        let targets = self.targets.as_ref().expect("built above");
        let last = self.last;
        let next = 1 - last;
        {
            let mut pass = full_pass(encoder, "fury points", &targets.field_view, true);
            pass.set_pipeline(&self.point_pipeline);
            pass.set_bind_group(0, &self.point_bind_group, &[]);
            pass.set_vertex_buffer(0, cloud.instances.slice(..));
            pass.draw(0..6, 0..cloud.count);
        }
        {
            let mut pass = full_pass(encoder, "fury blend", &targets.accumulation[next], true);
            pass.set_pipeline(&self.blend_pipeline);
            pass.set_bind_group(0, &targets.blend[last], &[]);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = full_pass(encoder, "fury glow", &targets.half[last], false);
            let gain = f64::from(frame.glow_gain);
            pass.set_pipeline(&self.glow_pipeline);
            pass.set_blend_constant(wgpu::Color {
                r: gain,
                g: gain,
                b: gain,
                a: gain,
            });
            pass.set_bind_group(0, &targets.glow[last], &[]);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = full_pass(encoder, "fury wave", &targets.half[next], true);
            pass.set_pipeline(&self.wave_pipeline);
            pass.set_bind_group(0, &targets.wave[last], &[]);
            pass.draw(0..24, 0..1);
        }
        {
            let mut pass = full_pass(encoder, "fury composite", &targets.field_view, true);
            pass.set_pipeline(&self.composite_pipeline);
            pass.set_bind_group(0, &targets.composite[last], &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_texture(
            targets.field.as_image_copy(),
            targets.finished.as_image_copy(),
            wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
        );
        {
            let mut pass = full_pass(encoder, "fury band", &targets.finished_view, false);
            pass.set_pipeline(&self.band_pipeline);
            pass.set_bind_group(0, &targets.band, &[]);
            pass.draw(0..12, 0..1);
            pass.set_pipeline(&self.scanline_pipeline);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = full_pass(encoder, "fury present", &targets.field_view, true);
            pass.set_pipeline(&self.present_pipeline);
            pass.set_bind_group(0, &targets.present, &[]);
            pass.draw(0..3, 0..1);
        }
        self.last = next;
    }

    /// The finished picture onto the page, inside the page's own pass.
    ///
    /// Nothing is drawn before the first [`Self::prepare`], there being no
    /// targets to read.
    pub fn composite(&self, pass: &mut wgpu::RenderPass<'_>) {
        let Some(targets) = &self.targets else {
            return;
        };
        pass.set_pipeline(&self.page_pipeline);
        pass.set_bind_group(0, &targets.page, &[]);
        pass.draw(0..3, 0..1);
    }

    fn targets_for(&self, device: &wgpu::Device, size: (u32, u32)) -> Targets {
        let target = |label: &str, (width, height): (u32, u32)| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TARGET_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[ENCODED_FORMAT],
            })
        };
        let view = |texture: &wgpu::Texture| texture.create_view(&Default::default());
        // `AllocateTargets`' half size; the full-size ones are the viewport's.
        let half_size = ((size.0 / 2).max(1), (size.1 / 2).max(1));
        let field = target("fury field", size);
        let finished = target("fury finished field", size);
        let accumulation = [
            view(&target("fury accumulation", size)),
            view(&target("fury accumulation", size)),
        ];
        let half = [
            view(&target("fury glow", half_size)),
            view(&target("fury glow", half_size)),
        ];
        let field_view = view(&field);
        let finished_view = view(&finished);
        let encoded_view = field.create_view(&wgpu::TextureViewDescriptor {
            format: Some(ENCODED_FORMAT),
            ..Default::default()
        });
        let bind = |label: &str, source: &wgpu::TextureView, second: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &self.post_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.post_uniforms.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(second),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.post_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&self.trail_sampler),
                    },
                ],
            })
        };
        // A pass that reads one target binds it twice: the second slot must
        // not be what the pass writes, read or not.
        let by_last = |make: &dyn Fn(usize, usize) -> wgpu::BindGroup| [make(0, 1), make(1, 0)];
        let blend = by_last(&|last, _| bind("fury blend", &field_view, &accumulation[last]));
        let glow = by_last(&|_, next| bind("fury glow", &accumulation[next], &accumulation[next]));
        let wave = by_last(&|last, _| bind("fury wave", &half[last], &half[last]));
        let composite =
            by_last(&|_, next| bind("fury composite", &accumulation[next], &half[next]));
        let band = bind("fury band", &field_view, &field_view);
        let present = bind("fury present", &finished_view, &finished_view);
        let page = bind("fury page", &encoded_view, &encoded_view);
        Targets {
            size,
            field,
            field_view,
            finished,
            finished_view,
            accumulation,
            half,
            blend,
            glow,
            wave,
            composite,
            band,
            present,
            page,
        }
    }
}

fn with_w(rgb: [f32; 3], w: f32) -> [f32; 4] {
    [rgb[0], rgb[1], rgb[2], w]
}

/// A pass over the whole of `view`, cleared to black or loaded.
///
/// A fresh glow target starts black either way: a texture is zero at creation.
fn full_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    label: &str,
    view: &wgpu::TextureView,
    clear: bool,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: if clear {
                    wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                } else {
                    wgpu::LoadOp::Load
                },
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}

/// `PointCloud_MakeSprite`'s texture: white, with an alpha that falls off
/// from the centre by a curve that tightens mip by mip.
///
/// Per mip `m` of `n`: `f = (m / (n - 1)) ^ curve`, `exp` and `amp` lerped by
/// `f`, and per texel `r = |xy - centre| / half`,
/// `alpha = amp * (1 - min(r, 1)) ^ exp`.
fn upload_sprite(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::TextureView {
    let (exp0, exp1, amp0, amp1, curve) = SPRITE_SHAPE;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("fury sprite"),
        size: wgpu::Extent3d {
            width: SPRITE_SIZE,
            height: SPRITE_SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: SPRITE_MIPS,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for mip in 0..SPRITE_MIPS {
        let size = SPRITE_SIZE >> mip;
        let f = (mip as f32 / (SPRITE_MIPS - 1) as f32).powf(curve);
        let exp = exp0 + (exp1 - exp0) * f;
        let amp = amp0 + (amp1 - amp0) * f;
        let half = size as f32 * 0.5;
        let mut texels = Vec::with_capacity((size * size * 4) as usize);
        // The texel's own index, not its centre: `MakeSprite` measures from
        // `(x, y)` to `(half, half)`, so the two smallest mips come out
        // fully transparent (every texel a whole half from the centre) and
        // the far dots fade with distance rather than sharpening.
        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - half;
                let dy = y as f32 - half;
                let r = (dx * dx + dy * dy).sqrt() / half;
                let alpha = amp * (1.0 - r.min(1.0)).powf(exp);
                texels.extend_from_slice(&[255, 255, 255, (alpha * 255.0).round() as u8]);
            }
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: mip,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &texels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size * 4),
                rows_per_image: Some(size),
            },
            wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
        );
    }
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// One of the pictures a draw list can put under its quads, in the order the
/// list puts them. The derived order is the tie-break when two sit at the same
/// quad: the Fury pass first, then the scene, then the movie.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Cut {
    Fury,
    Scene,
    Video,
}

impl super::Renderer {
    /// Uploads the Fury backdrop's clouds and builds its passes.
    ///
    /// Called once, by whoever built the menus, with the clouds the boot read
    /// off the disc; from then on a `Draw::FuryBackdrop` in a list draws.
    pub fn set_fury_backdrop(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        clouds: &[PointCloud],
    ) {
        self.fury = Some(FuryBackdrop::new(device, queue, self.target_format, clouds));
    }

    /// Runs the Fury backdrop's passes over `frames`, a still's history, so the
    /// frame drawn after them shows the trail and glow a player's would.
    /// Submitted frame by frame: each writes the constants the next replaces.
    pub fn settle_fury_backdrop(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frames: impl Iterator<Item = Frame>,
        viewport: (u32, u32),
    ) {
        let Some(fury) = &mut self.fury else {
            return;
        };
        for frame in frames {
            let mut encoder = device.create_command_encoder(&Default::default());
            fury.prepare(device, queue, &mut encoder, &frame, viewport);
            queue.submit(Some(encoder.finish()));
        }
    }

    /// Puts the HD-style backdrop's scene on the GPU; from then on a
    /// `Draw::SceneBackdrop` in a list draws. A scene that will not build is
    /// reported and leaves the list's entry drawing nothing.
    pub fn set_scene_backdrop(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: oag_mesh::mesh::Model,
    ) {
        match super::SceneBackdrop::new(device, queue, self.target_format, model) {
            Ok(scene) => self.scene = Some(scene),
            Err(error) => log::warn!("menu scene backdrop: {error:#} - it draws nothing"),
        }
    }

    /// The offscreen passes of whichever backdrops the list carries, encoded
    /// before the page's own pass.
    pub(super) fn prepare_backdrops(
        &mut self,
        (device, queue, encoder): (&wgpu::Device, &wgpu::Queue, &mut wgpu::CommandEncoder),
        (fury, scene): (Option<&Frame>, Option<&oag_ui::scene_backdrop::Frame>),
        viewport: (u32, u32),
    ) {
        if let (Some(frame), Some(fury)) = (fury, &mut self.fury) {
            fury.prepare(device, queue, encoder, frame, viewport);
        }
        if let (Some(frame), Some(scene)) = (scene, &mut self.scene) {
            scene.prepare(device, queue, encoder, frame, viewport);
        }
    }
}
