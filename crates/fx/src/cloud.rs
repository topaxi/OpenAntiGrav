//! `05_Track`'s clouds: every `cloudGroup`'s field of sprites, built the way
//! the original builds it, and drawn.
//!
//! Recovered at instruction level from `CloudGroup_Init` (`0x08933048`), its
//! build `FUN_08933c1c`, `CloudGroup_Draw` (`0x0893280c`) and their siblings.
//! The evidence, the addresses and a confidence score per claim are in
//! `docs/ghidra/functions/psp-pulse-usa/clouds.md`; this module implements what
//! that page describes and cites it rather than restating it.
//! `oag_vex::cloud::cloud_groups` is the loader the groups come from.
//!
//! # The field
//!
//! A `cloudCube` is not a sprite. It is a box: its world matrix's row lengths
//! are its size, its volume times `0.1` is how many sprite records it authors,
//! and each record lands at a random point of the box, `+/- 5` units along
//! each of the cube's own axes before its scale, with half-size
//! `(SpriteRadius +/- SpriteRadius * SpriteRadiusVar) * 10 * scale`. Each
//! `cloudGroup` builds a field from every cube in its subtree (a nested
//! group's cubes too), culls it with `Overlap`, colours each survivor from a
//! ramp over the field's height and draws it from one cell of the cloud
//! atlas. [`field`] is that build, step for step. On `05_Track` it gives three
//! fields: 46 records culled to 14, twice over one cube, and 277 culled to 74
//! over four cubes, half-sizes 32 to 48.
//!
//! # The seed: chosen, not measured
//!
//! The build draws everything from the particle generator, reseeded from the
//! group's `Seed` ([`crate::ranrot`]). Every shipped group leaves `Seed`
//! unset, and the original then rolls one from the clock-seeded libc `rand()`
//! at load, so each boot of the original draws a different field. With the
//! seed it kept, the port lands on the original's field exactly: checked
//! against all 369 records of one boot's three groups in a RAM dump
//! (`clouds.md`). Where a group's `Seed` is unset this module uses
//! [`CHOSEN_SEEDS`], **chosen, not measured**: they are that boot's three
//! draws, kept so a capture of ours is one field the original really drew.
//! An authored non-zero `Seed` is used as authored.
//!
//! # The roll term
//!
//! `CloudGroup_Draw` builds each quad in **view space**: it transforms the
//! sprite's centre by the view matrix, adds corner offsets rotated by
//! `g_camera_roll - phase`, and draws with identity view and model matrices.
//! `g_camera_roll` is the angle a world-level line makes on screen, so the
//! quad's spin is measured from the world's horizon, not the screen's: as the
//! camera banks, the clouds keep their orientation in the world.
//! [`Layer::extend_vertices`] reads the roll with [`crate::mist::camera_roll`],
//! the same law the mist uses.
//!
//! # What this module does not reproduce
//!
//! The original skips a group's draw when the camera is far from it (live: the
//! phases froze at the start line). The test is not read, so every group draws
//! every frame here, and every phase advances every tick.
//!
//! # Two halves, deliberately
//!
//! [`Sprite`] and [`Layer`] are the state and the maths, with no `wgpu` in
//! them, so the recovered constants are testable on a machine with no
//! graphics driver - the same split `exhaust` and `psys` already use.
//! [`Pipeline`] is the GPU side, in `psys::Pipeline`'s shape (one texture, one
//! blend class) rather than `exhaust::Pipeline`'s (no ribbon, no HD variant).

pub mod field;

use oag_core::math::Vec3;

use crate::exhaust::FlareTexture;
use crate::psys::field::Frame;
use oag_mesh::mesh::GpuVertex;

/// Bound of the per-sprite rotation rate, `Psys_RandFloatRange(-0.002, 0.002)`
/// read directly off `CloudGroup_BuildDisplayList` (`0xbb03126f`/`0x3b03126f`).
///
/// Added to the sprite's phase **once per draw call, with no `dt` scaling** -
/// `CloudGroup_Draw` does exactly `phase += rate`, and this project draws once
/// per simulation tick, so [`Sprite::advance`] reproduces that literally.
pub const ROTATION_RATE_MAX: f32 = 0.002;

/// The seeds a group with no authored `Seed` builds from, by group order on
/// the track, wrapping. **Chosen, not measured** - see the module doc: one
/// boot's runtime draws (2079, 6378, 9169), read out of the original's RAM.
pub const CHOSEN_SEEDS: [u32; 3] = [2079, 6378, 9169];

/// One baked cloud billboard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sprite {
    /// World position.
    pub position: Vec3,
    /// Half-extent of the billboard quad, in world units.
    pub half_size: f32,
    /// The baked colour, each channel a byte over 255, as the GE reads the
    /// packed vertex colour.
    pub colour: [f32; 4],
    /// The atlas cell it draws, see [`field::cell_uv`].
    pub cell: u8,
    phase: f32,
    rate: f32,
}

impl Sprite {
    /// The sprite [`field::build`] baked.
    #[must_use]
    pub fn from_baked(baked: &field::Baked) -> Self {
        let byte = |shift: u32| f32::from(((baked.colour >> shift) & 0xff) as u8) / 255.0;
        Self {
            position: Vec3::from(baked.position),
            half_size: baked.half_size,
            colour: [byte(0), byte(8), byte(16), byte(24)],
            cell: baked.cell,
            phase: baked.phase,
            rate: baked.rate,
        }
    }

    /// Advances one simulation tick. See [`ROTATION_RATE_MAX`] for why this
    /// takes no `dt`.
    pub fn advance(&mut self) {
        self.phase += self.rate;
    }

    /// This sprite's six vertices - two triangles - camera-facing and
    /// rotated by `roll - phase` from the camera's right towards its up.
    ///
    /// `right`/`up` are the camera's own basis vectors, the same convention
    /// `exhaust::Exhaust::vertices` uses: a view-space billboard whose
    /// extents are world-sized. `roll` is `g_camera_roll`, see the module
    /// doc's "roll term".
    #[must_use]
    pub fn vertices(&self, right: Vec3, up: Vec3, roll: f32) -> [GpuVertex; 6] {
        let (sin, cos) = (roll - self.phase).sin_cos();
        let r = right * cos + up * sin;
        let u = up * cos - right * sin;
        quad(
            self.position,
            r * self.half_size,
            u * self.half_size,
            self.colour,
            field::cell_uv(self.cell),
        )
    }
}

/// Six vertices for one camera-facing quad, coloured flat, textured with one
/// atlas cell.
///
/// The live vertex buffer pairs the cell's bottom-left texel `(u0, v1)` with
/// the `(-h, -h)` corner, its top-left `(u0, v0)` with `(-h, h)`, and so on
/// round (`clouds.md`).
fn quad(
    centre: Vec3,
    right: Vec3,
    up: Vec3,
    colour: [f32; 4],
    (u0, v0, u1, v1): (f32, f32, f32, f32),
) -> [GpuVertex; 6] {
    let corner = |sx: f32, sy: f32, u: f32, v: f32| GpuVertex {
        position: (centre + right * sx + up * sy).to_array(),
        normal: [0.0, 0.0, 1.0],
        colour,
        texcoord: [u, v],
        // Emissive-shaped like the flare: lighting is measured off
        // (`Gu_Disable(GU_LIGHTING)` in `CloudGroup_ApplyDrawState`), so this
        // keeps the mesh light rig off the sprite too.
        lit: 0.0,
        ..bytemuck::Zeroable::zeroed()
    };
    let bl = corner(-1.0, -1.0, u0, v1);
    let br = corner(1.0, -1.0, u1, v1);
    let tl = corner(-1.0, 1.0, u0, v0);
    let tr = corner(1.0, 1.0, u1, v0);
    [bl, br, tl, br, tr, tl]
}

/// Every cloud sprite on one track, built once at load and advanced per tick.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Layer {
    sprites: Vec<Sprite>,
}

impl Layer {
    /// Builds every `cloudGroup`'s field on a track, in the order the original
    /// constructs and draws them.
    ///
    /// Group `k` with no authored `Seed` builds from `seeds[k % seeds.len()]`;
    /// pass [`CHOSEN_SEEDS`] unless matching a particular boot of the
    /// original.
    #[must_use]
    pub fn from_groups(data: &[u8], nodes: &[oag_vex::vex::Node], seeds: &[u32]) -> Self {
        let sprites = oag_vex::cloud::cloud_groups(data, nodes)
            .iter()
            .enumerate()
            .flat_map(|(k, group)| {
                let seed = authored_seed(group.attributes.seed)
                    .or_else(|| seeds.get(k % seeds.len().max(1)).copied())
                    .unwrap_or(CHOSEN_SEEDS[k % CHOSEN_SEEDS.len()]);
                field::build(group, seed)
            })
            .map(|baked| Sprite::from_baked(&baked))
            .collect();
        Self { sprites }
    }

    /// Whether this layer has anything to draw - a track that authors no
    /// cloud node at all, which is every Pulse circuit but `05_Track`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sprites.is_empty()
    }

    /// How many sprites this layer draws, for the load report.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sprites.len()
    }

    /// The sprites, in draw order.
    #[must_use]
    pub fn sprites(&self) -> &[Sprite] {
        &self.sprites
    }

    /// Advances every sprite's rotation by one tick.
    pub fn advance(&mut self) {
        for sprite in &mut self.sprites {
            sprite.advance();
        }
    }

    /// Appends this frame's vertices to `out`, facing `camera` and
    /// counter-rotated by its roll per [`Sprite::vertices`]. The original
    /// reads the roll every frame, not every tick.
    pub fn extend_vertices(&self, out: &mut Vec<GpuVertex>, camera: &Frame) {
        let roll = crate::mist::camera_roll(camera);
        out.reserve(self.sprites.len() * 6);
        for sprite in &self.sprites {
            out.extend_from_slice(&sprite.vertices(camera.right, camera.up, roll));
        }
    }
}

/// `CloudGroup_Init` reads `Seed` as a float truncated to an integer, and
/// takes `0` as unset.
fn authored_seed(seed: f32) -> Option<u32> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let s = seed as i32 as u32;
    (s != 0).then_some(s)
}

/// How many sprites [`Pipeline`]'s buffer holds.
///
/// `05_Track` draws 102 (14 + 14 + 74 per `clouds.md`); the original's own
/// list holds at most 32 groups, so this is headroom rather than a fit.
pub const MAX_SPRITES: usize = 1024;

/// The maximum vertices [`Pipeline`]'s buffer holds, six per [`MAX_SPRITES`].
pub const MAX_VERTICES: usize = MAX_SPRITES * 6;

/// The blend `CloudGroup_ApplyDrawState` sets, as `wgpu` spells it.
///
/// `Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA, GU_ONE_MINUS_SRC_ALPHA, 0, 0)` - a
/// standard alpha lerp, **not** `exhaust::BLEND`'s additive one. See
/// `docs/ghidra/functions/psp-pulse-usa/clouds.md`.
pub const BLEND: wgpu::BlendState = wgpu::BlendState {
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

/// The cloud draw pipeline: one texture, one blend class, in `psys::Pipeline`'s
/// shape rather than `exhaust::Pipeline`'s - there is no ribbon and no HD
/// variant here.
#[derive(Debug)]
pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    texture: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    /// Vertices actually uploaded by the last [`Pipeline::upload`].
    count: u32,
}

impl Pipeline {
    /// Builds the pipeline and uploads the cloud texture.
    ///
    /// `texture` is RGBA8, normally
    /// `Data\Tex\Cloud\Wipeout_Clouds_D_128x64x4.mip` decoded by
    /// `oag_texture::texture`. `format` must match the caller's render pass
    /// and `sample_count` its multisample state - see `mesh_render::build`.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        texture: &FlareTexture,
        sample_count: u32,
        velocity: oag_mesh::mesh_render::Velocity,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cloud"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/cloud.wgsl")).into(),
            ),
        });

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cloud uniforms"),
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
            label: Some("cloud texture"),
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
            label: Some("cloud"),
            bind_group_layouts: &[Some(&uniform_layout), Some(&texture_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cloud"),
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
                        blend: Some(BLEND),
                        // Colour only - `Gu_PixelMask(0xff000000)` masks the
                        // alpha channel out of every write; see clouds.md.
                        write_mask: wgpu::ColorWrites::COLOR,
                    })];
                    targets.extend(velocity.target(true));
                    targets
                },
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: oag_mesh::mesh_render::linear_constants(format),
                    ..Default::default()
                },
            }),
            primitive: wgpu::PrimitiveState {
                // A camera-facing quad has no meaningful winding - measured
                // off (`Gu_Disable(GU_CULL_FACE)`) as well as structurally
                // necessary.
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
        });

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cloud uniforms"),
            size: oag_mesh::mesh_render::UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cloud uniforms"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });

        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cloud vertices"),
            size: (MAX_VERTICES * std::mem::size_of::<GpuVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let texture = bind_texture(device, queue, &texture_layout, texture);

        Self {
            pipeline,
            uniforms,
            bind_group,
            texture,
            vertices,
            count: 0,
        }
    }

    /// Uploads this frame's camera matrix and geometry.
    pub fn upload(
        &mut self,
        queue: &wgpu::Queue,
        view_projection: &[[f32; 4]; 4],
        vertices: &[GpuVertex],
    ) {
        let mut block = [[0.0f32; 4]; 8];
        block[..4].copy_from_slice(view_projection);
        block[4] = [1.0, 0.0, 0.0, 0.0];
        block[5] = [0.0, 1.0, 0.0, 0.0];
        block[6] = [0.0, 0.0, 1.0, 0.0];
        block[7] = [0.0, 0.0, 0.0, 1.0];
        queue.write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&block));

        let n = vertices.len().min(MAX_VERTICES);
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&vertices[..n]));
        self.count = n as u32;
    }

    /// Draws into a pass the caller already opened.
    ///
    /// Must be issued after the opaque geometry, for the same depth-write-off
    /// reason `exhaust::Pipeline::draw` documents. Drawn after the sky and
    /// before the track's own transparent surfaces is closest to the
    /// original's own distance-sorted submission, though this project does
    /// not reproduce that sort - see `race::Scene`.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_bind_group(1, &self.texture, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..self.count, 0..1);
    }
}

/// Uploads `texture` and builds the two-entry bind group `Pipeline::new`
/// wants.
///
/// Not `exhaust::FlareTexture::bind`: that method's bind group always has a
/// **third** entry (the ribbon's second texture, HD-only), which this
/// pipeline's layout has no matching slot for. `FlareTexture`'s fields are
/// public for exactly this - a second consumer with a different bind-group
/// shape uploads them itself rather than fighting the first consumer's
/// layout.
fn bind_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    texture: &FlareTexture,
) -> wgpu::BindGroup {
    let gpu_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("cloud texture"),
        size: wgpu::Extent3d {
            width: texture.width.max(1),
            height: texture.height.max(1),
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
            texture: &gpu_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &texture.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(texture.width.max(1) * 4),
            rows_per_image: Some(texture.height.max(1)),
        },
        wgpu::Extent3d {
            width: texture.width.max(1),
            height: texture.height.max(1),
            depth_or_array_layers: 1,
        },
    );
    let view = gpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("cloud texture"),
        // The shared texture is sampled `0..1` per sprite corner - see
        // `quad` - so clamping rather than repeating matches how the flare's
        // own single-texture sampler is built.
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("cloud texture"),
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

#[cfg(test)]
mod tests;
