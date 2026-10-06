//! The Cannon's two hand-built textured quads: the bolt streak, drawn every
//! tick a round is alive, and the muzzle flash, drawn only for its first
//! [`geometry::FLASH_WINDOW_SECONDS`].
//!
//! **Not a particle effect.** `Cannon_DrawRound` builds these from two raw
//! GU display lists bound to `Cannon_bolt.mip`/`Cannon_muzzle_flash.mip`, no
//! `Data\Psys\*.POB` involved - see
//! `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, the
//! 2026-09-17 section, for the full read this module implements.
//!
//! Split the way [`crate::exhaust`] is: [`geometry`] is the wgpu-free vertex
//! maths, [`random`] the render-local roll the flash needs, and [`Pipeline`]
//! here is the device-side half neither depends on.
//!
//! **Depth write is on, unlike [`crate::exhaust::Pipeline`]'s flare.**
//! `Cannon_BuildBoltList`/`Cannon_BuildMuzzleFlashList` both call
//! `Gu_DepthMask(1)` - read directly, not inferred - so this pipeline's
//! [`wgpu::DepthStencilState::depth_write_enabled`] is `true` where the
//! exhaust's is `false`. An additive quad that writes depth is unusual and
//! is kept exactly as measured rather than quietly matched to the exhaust's
//! own state.

pub mod geometry;
pub mod random;
#[cfg(test)]
mod tests;

use oag_mesh::mesh::GpuVertex;

/// How many live rounds this pipeline's buffers are sized for.
///
/// Duplicated from `oag_weapons::projectile::MAX_PROJECTILES` rather than
/// imported, the same reason [`crate::exhaust::MAX_TRAILS`] is: this crate
/// must not depend on the simulation. A `const _: () = assert!(...)` on the
/// game crate's side is where the two are compared - see
/// `crates/raceplay/src/weapons/visuals.rs`.
pub const MAX_ROUNDS: usize = 128;

/// Vertices per round's bolt (two six-vertex quads) and per flash (one).
const BOLT_VERTICES_PER_ROUND: usize = 12;
const FLASH_VERTICES_PER_ROUND: usize = 6;

/// An RGBA8 image for one of the two textures, with its dimensions.
///
/// Re-exported from [`crate::exhaust`] rather than duplicated: both quads
/// take an `oag_texture`-decoded `.mip` the same way the exhaust flare does.
pub use crate::exhaust::FlareTexture;

/// Binds one [`FlareTexture`] as a plain single-texture bind group - unlike
/// [`FlareTexture`]'s own `bind`, which is `pub(super)` to `exhaust` and
/// always wires a second, HD-only texture slot this pipeline has no use for.
fn bind_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    label: &str,
    image: &FlareTexture,
) -> wgpu::BindGroup {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: image.width.max(1),
            height: image.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &image.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(image.width.max(1) * 4),
            rows_per_image: Some(image.height.max(1)),
        },
        wgpu::Extent3d {
            width: image.width.max(1),
            height: image.height.max(1),
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(label),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    })
}

/// Draws the bolt and the flash.
///
/// One render pipeline covers both: `Cannon_BuildBoltList` and
/// `Cannon_BuildMuzzleFlashList` set identical GU state (fog off, depth
/// test `func 6` with write on, the same additive blend), so the only
/// per-draw difference is which texture and vertex buffer is bound.
#[derive(Debug)]
pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bolt_texture: wgpu::BindGroup,
    flash_texture: wgpu::BindGroup,
    bolt_vertices: wgpu::Buffer,
    flash_vertices: wgpu::Buffer,
    bolt_count: u32,
    flash_count: u32,
}

impl Pipeline {
    /// Builds the pipeline and uploads both textures.
    ///
    /// `bolt` is normally `Data\Weapons\Textures\Cannon_bolt.mip` and
    /// `flash` `Data\Weapons\Textures\Cannon_muzzle_flash.mip`, both decoded
    /// by `oag_texture`. `format` must match the caller's render pass and
    /// `sample_count` its multisample state - see `mesh_render::build`.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        bolt: &FlareTexture,
        flash: &FlareTexture,
        sample_count: u32,
        velocity: oag_mesh::mesh_render::Velocity,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("weapon quads"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/weapon_quads.wgsl")).into(),
            ),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("weapon quads uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("weapon quads texture"),
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
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("weapon quads"),
            bind_group_layouts: &[Some(&layout), Some(&texture_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("weapon quads"),
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
                targets: &{
                    let mut targets = vec![Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(crate::exhaust::BLEND),
                        write_mask: wgpu::ColorWrites::COLOR,
                    })];
                    // Both quads are rebuilt from scratch every draw with no
                    // frame-to-frame vertex correspondence - the same reason
                    // `exhaust::Pipeline`'s flare masks its own velocity
                    // write - so even though this pipeline writes depth
                    // (see the module doc comment), it has no meaningful
                    // motion vector to write either; masked keeps the
                    // velocity buffer describing the surface behind it.
                    targets.extend(velocity.target(true));
                    targets
                },
                compilation_options: oag_mesh::mesh_render::fragment_options(format),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: oag_mesh::mesh_render::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
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
        });

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("weapon quads uniforms"),
            size: oag_mesh::mesh_render::UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("weapon quads uniforms"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });

        let vertex_buffer = |label: &str, vertices: usize| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (vertices * std::mem::size_of::<GpuVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let bolt_vertices =
            vertex_buffer("cannon bolt vertices", MAX_ROUNDS * BOLT_VERTICES_PER_ROUND);
        let flash_vertices = vertex_buffer(
            "cannon flash vertices",
            MAX_ROUNDS * FLASH_VERTICES_PER_ROUND,
        );

        let bolt_texture = bind_texture(device, queue, &texture_layout, "cannon bolt", bolt);
        let flash_texture = bind_texture(device, queue, &texture_layout, "cannon flash", flash);

        Self {
            pipeline,
            uniforms,
            bind_group,
            bolt_texture,
            flash_texture,
            bolt_vertices,
            flash_vertices,
            bolt_count: 0,
            flash_count: 0,
        }
    }

    /// Uploads this frame's camera matrix and both quads' geometry, as
    /// [`geometry::bolt_vertices`]/[`geometry::flash_vertices`] build it.
    pub fn upload(
        &mut self,
        queue: &wgpu::Queue,
        view_projection: &[[f32; 4]; 4],
        bolt: &[GpuVertex],
        flash: &[GpuVertex],
    ) {
        let mut block = [[0.0f32; 4]; 8];
        block[..4].copy_from_slice(view_projection);
        block[4] = [1.0, 0.0, 0.0, 0.0];
        block[5] = [0.0, 1.0, 0.0, 0.0];
        block[6] = [0.0, 0.0, 1.0, 0.0];
        block[7] = [0.0, 0.0, 0.0, 1.0];
        queue.write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&block));

        let cap = MAX_ROUNDS * BOLT_VERTICES_PER_ROUND;
        debug_assert!(
            bolt.len() <= cap,
            "cannon bolt vertices past the buffer: {}, cap {cap}",
            bolt.len(),
        );
        let n = bolt.len().min(cap);
        queue.write_buffer(&self.bolt_vertices, 0, bytemuck::cast_slice(&bolt[..n]));
        self.bolt_count = n as u32;

        let cap = MAX_ROUNDS * FLASH_VERTICES_PER_ROUND;
        debug_assert!(
            flash.len() <= cap,
            "cannon flash vertices past the buffer: {}, cap {cap}",
            flash.len(),
        );
        let n = flash.len().min(cap);
        queue.write_buffer(&self.flash_vertices, 0, bytemuck::cast_slice(&flash[..n]));
        self.flash_count = n as u32;
    }

    /// Draws into a pass the caller already opened, after the opaque
    /// geometry (this pipeline blends).
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        if self.bolt_count > 0 {
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_bind_group(1, &self.bolt_texture, &[]);
            pass.set_vertex_buffer(0, self.bolt_vertices.slice(..));
            pass.draw(0..self.bolt_count, 0..1);
        }
        if self.flash_count > 0 {
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_bind_group(1, &self.flash_texture, &[]);
            pass.set_vertex_buffer(0, self.flash_vertices.slice(..));
            pass.draw(0..self.flash_count, 0..1);
        }
    }
}
