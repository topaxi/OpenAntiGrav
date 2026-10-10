//! The Fury menu backdrop's GPU side: the clouds as instance buffers, the
//! procedural sprite, and the four passes `backdrop.wesl` carries.
//!
//! `oag_ui::backdrop::Fury` decides what a frame looks like and hands over a
//! [`Frame`]; this draws it. Per frame, before the page's own render pass:
//!
//! 1. **Points** - the current cloud as camera-facing sprites, additively
//!    (`SRC_ALPHA, ONE`, as `PointCloud_Submit` sets it) into a cleared
//!    target the size of the viewport.
//! 2. **Blend** - the fresh particles, capped, over the decayed trail, into a
//!    temporary at half the viewport.
//! 3. **Wave** - that accumulation dimmed and pulled down, back into the
//!    half-size trail for next frame.
//!
//! Then, inside the page's pass where the draw list's
//! [`oag_ui::frontend::Draw::FuryBackdrop`] sits:
//!
//! 4. **Composite** - fresh plus trail, scaled by the screen's tint and
//!    encoded as the original's sRGB buffer stores it, added over the page.
//!
//! Every target is `Rgba8Unorm`, which clamps at one the way the original's
//! 8-bit targets do and is where the composite's `257/256` stretch comes from.
//! The sizes are `BackgroundAnimFury_AllocateTargets`' - one full, two half,
//! the half ones sampled linear - and the composite reads the full one as
//! `texture` and a half one as `waveTexture`, as `Render` binds them; the
//! blend pass's own target and quad grid are still the page's confidence-60
//! reading. See
//! [menu-backdrop.md](../../../../docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md).

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
    source_max: [f32; 4],
    wave_scale: [f32; 4],
    wave_bias: [f32; 4],
    tint: [f32; 4],
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
const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// One uploaded cloud.
struct Cloud {
    instances: wgpu::Buffer,
    count: u32,
}

/// The targets, rebuilt when the viewport changes: the particles at its
/// size, the two trail targets at half of it.
struct Targets {
    size: (u32, u32),
    particles: wgpu::TextureView,
    temporary: wgpu::TextureView,
    trail: wgpu::TextureView,
    /// `(source = particles, trail)` for the blend and the composite.
    over_trail: wgpu::BindGroup,
    /// `(source = temporary, temporary)` for the wave, which writes the trail.
    from_temporary: wgpu::BindGroup,
}

/// The backdrop's GPU resources.
pub struct FuryBackdrop {
    clouds: Vec<Cloud>,
    point_pipeline: wgpu::RenderPipeline,
    blend_pipeline: wgpu::RenderPipeline,
    wave_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    point_uniforms: wgpu::Buffer,
    post_uniforms: wgpu::Buffer,
    point_bind_group: wgpu::BindGroup,
    post_layout: wgpu::BindGroupLayout,
    post_sampler: wgpu::Sampler,
    trail_sampler: wgpu::Sampler,
    targets: Option<Targets>,
}

impl std::fmt::Debug for FuryBackdrop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuryBackdrop")
            .field("clouds", &self.clouds.len())
            .finish_non_exhaustive()
    }
}

impl FuryBackdrop {
    /// Uploads `clouds` and builds the four pipelines, the composite's for a
    /// page of `page_format`.
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
        // `AllocateTargets` gives the full-size particle target a nearest
        // view and the two half-size trail targets linear ones: the trail is
        // sampled up to the viewport bilinearly, the particles never are.
        let post_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fury post"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let trail_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("fury trail"),
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

        let post_pipeline = |label: &str, entry: &str, format, blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&post_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_post"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
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
        let blend_pipeline = post_pipeline("fury blend", "fs_blend", TARGET_FORMAT, None);
        let wave_pipeline = post_pipeline("fury wave", "fs_wave", TARGET_FORMAT, None);
        // `BlendFunc(CONSTANT_COLOR, ONE)`: the page keeps what it has and
        // the backdrop is added. The tint is applied in the shader, which has
        // to encode it - see `fs_composite`. The page's alpha is left alone.
        let tinted = wgpu::BlendState {
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
        let composite_pipeline =
            post_pipeline("fury composite", "fs_composite", page_format, Some(tinted));

        Self {
            clouds,
            point_pipeline,
            blend_pipeline,
            wave_pipeline,
            composite_pipeline,
            point_uniforms,
            post_uniforms,
            point_bind_group,
            post_layout,
            post_sampler,
            trail_sampler,
            targets: None,
        }
    }

    /// How many clouds are uploaded.
    #[must_use]
    pub fn clouds(&self) -> usize {
        self.clouds.len()
    }

    /// The three offscreen passes for `frame`, into targets sized to the
    /// viewport. Encoded before the page's own pass, which is what
    /// [`Self::composite`] then reads from.
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
        }
        let Some(cloud) = self.clouds.get(frame.cloud) else {
            return;
        };
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
        queue.write_buffer(
            &self.post_uniforms,
            0,
            bytemuck::bytes_of(&PostUniforms {
                feedback: with_w(frame.feedback, 0.0),
                source_max: with_w(frame.source_max, 0.0),
                wave_scale: with_w(backdrop::WAVE_COLOUR_SCALE, 0.0),
                wave_bias: [
                    backdrop::WAVE_COLOUR_BIAS,
                    backdrop::WAVE_BIAS_FACTOR,
                    0.0,
                    0.0,
                ],
                tint: frame.tint,
            }),
        );
        let targets = self.targets.as_ref().expect("built above");
        {
            let mut pass = full_pass(encoder, "fury points", &targets.particles, true);
            pass.set_pipeline(&self.point_pipeline);
            pass.set_bind_group(0, &self.point_bind_group, &[]);
            pass.set_vertex_buffer(0, cloud.instances.slice(..));
            pass.draw(0..6, 0..cloud.count);
        }
        {
            let mut pass = full_pass(encoder, "fury blend", &targets.temporary, false);
            pass.set_pipeline(&self.blend_pipeline);
            pass.set_bind_group(0, &targets.over_trail, &[]);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = full_pass(encoder, "fury wave", &targets.trail, false);
            pass.set_pipeline(&self.wave_pipeline);
            pass.set_bind_group(0, &targets.from_temporary, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    /// The composite, inside the page's own pass, tinted by the `tint` of the
    /// frame [`Self::prepare`] was given.
    ///
    /// Nothing is drawn before the first [`Self::prepare`], there being no
    /// targets to read.
    pub fn composite(&self, pass: &mut wgpu::RenderPass<'_>) {
        let Some(targets) = &self.targets else {
            return;
        };
        pass.set_pipeline(&self.composite_pipeline);
        pass.set_bind_group(0, &targets.over_trail, &[]);
        pass.draw(0..3, 0..1);
    }

    fn targets_for(&self, device: &wgpu::Device, size: (u32, u32)) -> Targets {
        let target = |label: &str, (width, height): (u32, u32)| {
            device
                .create_texture(&wgpu::TextureDescriptor {
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
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        // `AllocateTargets`: one target at the display's size, two at half.
        let half = ((size.0 / 2).max(1), (size.1 / 2).max(1));
        let particles = target("fury particles", size);
        let temporary = target("fury temporary", half);
        let trail = target("fury trail", half);
        let bind = |label: &str, source: &wgpu::TextureView, trail: &wgpu::TextureView| {
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
                        resource: wgpu::BindingResource::TextureView(trail),
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
        let over_trail = bind("fury over trail", &particles, &trail);
        // The wave reads the temporary alone and writes the trail, so the
        // trail must not be bound to it: the second slot is the temporary
        // again, unread.
        let from_temporary = bind("fury from temporary", &temporary, &temporary);
        Targets {
            size,
            particles,
            temporary,
            trail,
            over_trail,
            from_temporary,
        }
    }
}

fn with_w(rgb: [f32; 3], w: f32) -> [f32; 4] {
    [rgb[0], rgb[1], rgb[2], w]
}

/// A pass over the whole of `view`, cleared to black or loaded.
///
/// A fresh trail target starts black either way: a texture is zero at
/// creation, and the wave pass writes every texel of it every frame.
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
