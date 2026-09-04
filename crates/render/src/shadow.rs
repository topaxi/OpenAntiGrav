//! The `blob` shadow tier: one ground-aligned quad per craft.
//!
//! Step 3 of [`docs/rendering/shadows.md`](../../../docs/rendering/shadows.md)'s
//! plan, and only that tier. `original` - Pulse's `Dynamic Shadow Occluder`
//! hulls, HD's shadow-map jobs - is step 5 and nothing here is it.
//!
//! # What is the disc's and what is ours
//!
//! **The shape is the disc's on Wipeout HD.** Nine `ambient_shadow.gtf`, one
//! per team, 128x64, single-channel, decoded already
//! ([`gtf.md`](../../../docs/formats/gtf.md)); read in Morton order each is
//! that team's craft in soft silhouette. This module samples that texture and
//! nothing else on a title that ships one.
//!
//! **The falloff is ours, and only where a title ships no such asset**, which
//! is every title but HD. That is the narrow case `CLAUDE.md`'s never-invent
//! rule allows - a substitute for the missing asset alone. It is never an
//! override: a title *with* the asset never sees it, and the whole tier is
//! off by default.
//!
//! **Also ours, and each says so where it is written**: the height fade
//! ([`fade`]), the lift off the surface ([`LIFT`]), and where in the frame the
//! quad is drawn. No title in the lineage draws a blob at all - `blob` `0x3e0`
//! and `textureBlob` `0x3df` are authored zero times across all 415 `.vex`
//! files on the Pulse disc - so there is no original behaviour to be faithful
//! to here, and none is claimed.
//!
//! # Two halves, deliberately
//!
//! [`quad`] and [`fade`] are pure arithmetic with tests, the same split
//! [`crate::exhaust`] and [`crate::shield`] use: the placement can be wrong in
//! a way a screenshot does not show, so the part that decides where a shadow
//! goes is testable on a machine with no graphics driver.

use oag_core::math::Vec3;

use crate::mesh::GpuVertex;

/// How far off the surface the quad is lifted, in world units.
///
/// **Ours, and a fudge rather than a reading**: a quad laid exactly on the
/// track z-fights the ribbon it is drawn over. Small against a craft (the
/// hulls run 4-6 units long) and large against the depth buffer's resolution
/// at race distances, which is the whole window it has to sit in.
///
/// Applied along the *surface normal*, not world up, so it keeps its meaning
/// on a banked corner and inside a loop.
pub const LIFT: f32 = 0.05;

/// The largest number of quads [`Pipeline`] holds: one per grid slot.
///
/// Eight, matching `oag_gameplay::MAX_SHIPS`. Stated as a literal rather than
/// imported because this crate depends on no gameplay crate and must not - see
/// `just check-deps`.
pub const MAX_QUADS: usize = 8;

/// Six vertices per quad, two triangles, as [`quad`] builds them.
pub const MAX_VERTICES: usize = MAX_QUADS * 6;

/// How far above its own ride height a craft keeps any shadow at all, as a
/// multiple of that height.
///
/// **Ours.** A craft in normal flight hovers at its ride height and its shadow
/// has to be at full strength there, so the fade cannot start at the ground -
/// measured on Talon's Junction, a resting craft sits `4.00` units over the
/// floor against a `4.125` spring target, which under a fade that started at
/// zero left it at strength `0.030`: drawn, and invisible. Three ride heights
/// is where a craft is unambiguously airborne rather than hovering.
pub const FADE_REACH: f32 = 3.0;

/// How dark a craft's shadow is at `height` above the surface, given the
/// `ride` height its own hover spring holds it at.
///
/// **Ours, both halves.** Full strength anywhere up to `ride` - which is where
/// a craft in normal flight lives, so this is the case a player sees almost
/// always - then linear to nothing at `ride * FADE_REACH`, so a craft thrown
/// off a jump loses its shadow smoothly instead of it vanishing on the tick
/// the cast misses. Nothing in any title's binary says what a blob should do
/// here, because no title draws one.
#[must_use]
pub fn fade(height: f32, ride: f32) -> f32 {
    if ride <= 0.0 {
        return 0.0;
    }
    let airborne = (height - ride).max(0.0);
    (1.0 - airborne / (ride * (FADE_REACH - 1.0))).clamp(0.0, 1.0)
}

/// Where one craft's shadow goes and how dark it is.
///
/// Built by the caller, which is the side that owns the collision world and
/// can cast the ray - see `oag_game::race::shadow`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// The point on the surface the craft's downward cast hit, **before**
    /// [`LIFT`] is applied.
    pub contact: Vec3,
    /// The hit surface's normal, normalised. The quad lies in its plane.
    pub normal: Vec3,
    /// The craft's own forward axis. Projected into the surface plane to give
    /// the quad's long axis, so a shadow turns with the craft rather than with
    /// the camera.
    pub forward: Vec3,
    /// Half the hull's length, along the projected [`Self::forward`].
    pub half_length: f32,
    /// Half the hull's width, across it.
    pub half_width: f32,
    /// [`fade`]'s result: how dark this shadow is, `0.0..=1.0`.
    pub strength: f32,
    /// Which of [`Pipeline`]'s silhouettes this craft wears, as an index into
    /// the slice the pipeline was built with.
    pub silhouette: usize,
}

/// Two triangles laid on the surface under one craft.
///
/// The quad's axes come from the *surface*, not the camera: `forward` is
/// projected into the hit plane and the cross product completes the basis, so
/// a craft on a banked corner casts a shadow that lies in the road rather than
/// standing up out of it.
///
/// **The texture is mapped to the hull's own footprint**, whatever aspect that
/// is. On Wipeout HD that stretches a 128x64 image over a rectangle whose
/// length/width ratio is the craft's `<Misc>` dimensions and not 2:1 - a
/// deliberate choice rather than an oversight: the silhouette was authored for
/// the hull it belongs to, so the hull's own numbers are what it should cover.
/// Fitting the quad to the texture's aspect instead would size a craft's
/// shadow from an image dimension, which is not a fact about the craft.
///
/// A degenerate basis - `forward` parallel to `normal`, which a craft standing
/// exactly on its nose would give - falls back to any perpendicular axis
/// rather than producing NaNs.
#[must_use]
pub fn quad(placement: &Placement) -> [GpuVertex; 6] {
    let normal = placement.normal.normalize_or_zero();
    let projected = placement.forward - normal * placement.forward.dot(normal);
    let forward = if projected.length_squared() > 1e-8 {
        projected.normalize()
    } else {
        // Any axis in the plane will do: with the craft pointing straight into
        // the ground there is no meaningful heading to align to, and a NaN
        // basis would put the whole quad at the origin.
        normal.any_orthonormal_vector()
    };
    let right = forward.cross(normal).normalize_or_zero();
    let centre = placement.contact + normal * LIFT;
    let long = forward * placement.half_length;
    let across = right * placement.half_width;

    let corner = |sx: f32, sy: f32, u: f32, v: f32| GpuVertex {
        position: (centre + across * sx + long * sy).to_array(),
        normal: normal.to_array(),
        // Black, with the fade in the alpha: the shader returns black and the
        // alpha-over blend turns it into `dst * (1 - a)`. Only the alpha is
        // read - see `shadow.wgsl`.
        colour: [0.0, 0.0, 0.0, placement.strength],
        texcoord: [u, v],
        // Not lit by the mesh rig: a shadow is not a surface.
        lit: 0.0,
        ..bytemuck::Zeroable::zeroed()
    };
    // `v` runs with the craft's forward axis, so the silhouette's nose points
    // where the craft does - the textures are authored nose-up.
    let bl = corner(-1.0, -1.0, 0.0, 1.0);
    let br = corner(1.0, -1.0, 1.0, 1.0);
    let tl = corner(-1.0, 1.0, 0.0, 0.0);
    let tr = corner(1.0, 1.0, 1.0, 0.0);
    [bl, br, tl, br, tr, tl]
}

/// The alpha-over blend a shadow darkens with.
///
/// [`crate::psys::BLEND_ALPHA_OVER`] by another name, and deliberately that
/// one rather than a second copy of the same equation: `src.a * src.rgb +
/// (1 - src.a) * dst.rgb` with `src.rgb` black is exactly `dst * (1 - a)`.
pub const BLEND: wgpu::BlendState = crate::psys::BLEND_ALPHA_OVER;

/// One craft's silhouette, as pixels.
///
/// Coverage is in the **red** channel - high is shadow - which is the shape
/// Wipeout HD's own `ambient_shadow.gtf` decodes to: a single-channel `B8`
/// texture whose `remap` broadcasts the stored byte across rgb and forces
/// alpha opaque. [`Silhouette::falloff`] matches that convention so both paths
/// sample the same channel.
#[derive(Debug, Clone)]
pub struct Silhouette {
    /// Width in texels.
    pub width: u32,
    /// Height in texels.
    pub height: u32,
    /// `width * height * 4` bytes, RGBA8.
    pub rgba: Vec<u8>,
}

impl Silhouette {
    /// A soft elliptical falloff, for a title that ships no silhouette of its
    /// own.
    ///
    /// **Ours, and a substitute for a missing asset only.** Every title but
    /// Wipeout HD is in this case: `blob` `0x3e0` and `textureBlob` `0x3df`
    /// are authored zero times on the Pulse disc, so there is nothing to play
    /// instead of this. It is never drawn on a title that ships the real
    /// thing, and the whole tier is off by default.
    ///
    /// Square, because the quad's own rectangle carries the hull's shape - see
    /// [`quad`] - and a squared falloff so the edge reaches zero rather than
    /// clipping, the same curve `exhaust::FlareTexture::placeholder` uses.
    #[must_use]
    pub fn falloff(size: u32) -> Self {
        let size = size.max(1);
        let mut rgba = Vec::with_capacity((size * size * 4) as usize);
        let centre = (size as f32 - 1.0) / 2.0;
        for y in 0..size {
            for x in 0..size {
                let dx = (x as f32 - centre) / centre.max(1.0);
                let dy = (y as f32 - centre) / centre.max(1.0);
                let d = (dx * dx + dy * dy).sqrt().min(1.0);
                let coverage = ((1.0 - d) * (1.0 - d) * 255.0) as u8;
                // Broadcast across rgb with opaque alpha, which is what HD's
                // own remap produces - so `shadow.wgsl` reads one channel for
                // both paths rather than branching on where the pixels came
                // from.
                rgba.extend_from_slice(&[coverage, coverage, coverage, 0xff]);
            }
        }
        Self {
            width: size,
            height: size,
            rgba,
        }
    }

    /// Uploads this image as its own bind group.
    fn bind(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        label: &str,
    ) -> wgpu::BindGroup {
        let size = wgpu::Extent3d {
            width: self.width.max(1),
            height: self.height.max(1),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
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
            &self.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.width.max(1) * 4),
                rows_per_image: Some(self.height.max(1)),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // Clamped, not repeated: the quad is exactly the silhouette's extent,
        // and a sample that ran off the edge under repeat would put the nose
        // of one shadow at the tail of the next.
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
}

/// The blob pass: one pipeline, one vertex buffer, one bind group per
/// silhouette.
///
/// Depth-tested and **not** depth-writing, the same state the exhaust and the
/// particles draw with and for the same reason: a shadow is blended, so a
/// depth write would let it occlude the very geometry it is meant to sit on.
#[derive(Debug)]
pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// One per distinct silhouette, indexed by [`Placement::silhouette`].
    silhouettes: Vec<wgpu::BindGroup>,
    vertices: wgpu::Buffer,
    /// `(silhouette, first vertex, count)` per run, as the last
    /// [`Pipeline::upload`] grouped them.
    runs: Vec<(usize, u32, u32)>,
}

impl Pipeline {
    /// Builds the pipeline and uploads every silhouette.
    ///
    /// `format` must be the target the caller's render pass writes and
    /// `sample_count` must match its multisample state - see
    /// `mesh_render::build`. An empty `silhouettes` builds a pipeline that
    /// draws nothing, which is what a title with no shadow texture and no
    /// fallback gets.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        silhouettes: &[Silhouette],
        sample_count: u32,
        velocity: crate::mesh_render::Velocity,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shadow.wgsl").into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow uniforms"),
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
            label: Some("shadow silhouette"),
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
            label: Some("shadow"),
            bind_group_layouts: &[Some(&layout), Some(&texture_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow"),
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
                // The velocity target, where the race has one, rides along
                // write-masked empty: the quad is rebuilt from scratch every
                // frame with no vertex correspondence to reproject, and it
                // writes no depth - so the velocity at its pixels stays the
                // road's behind it. The same reasoning `exhaust::Pipeline`
                // gives for its own.
                targets: &{
                    let mut targets = vec![Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(BLEND),
                        // Colour only: the bloom's glow mask lives in the
                        // alpha channel, and a shadow has no business
                        // brightening anything. See `mesh_render::GlowMask`.
                        write_mask: wgpu::ColorWrites::COLOR,
                    })];
                    targets.extend(velocity.target(true));
                    targets
                },
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                // Two-sided: the quad's winding follows a basis built from the
                // surface normal and the craft's heading, which flips as a
                // craft rolls through a loop. The alternative is a shadow that
                // disappears upside down.
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::mesh_render::DEPTH_FORMAT,
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
            label: Some("shadow uniforms"),
            size: crate::mesh_render::UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow uniforms"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });

        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow vertices"),
            size: (MAX_VERTICES * std::mem::size_of::<GpuVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let silhouettes = silhouettes
            .iter()
            .enumerate()
            .map(|(index, image)| {
                image.bind(
                    device,
                    queue,
                    &texture_layout,
                    &format!("shadow silhouette {index}"),
                )
            })
            .collect();

        Self {
            pipeline,
            uniforms,
            bind_group,
            silhouettes,
            vertices,
            runs: Vec::new(),
        }
    }

    /// Uploads this frame's camera matrix and quads.
    ///
    /// Placements are grouped by [`Placement::silhouette`] so each texture is
    /// one draw call - eight craft on a grid of eight teams is eight draws of
    /// six vertices, and a field sharing one livery is one draw. A placement
    /// naming a silhouette this pipeline was not built with, or one faded to
    /// nothing, is **dropped rather than substituted**: a shadow wearing
    /// another team's outline is worse than no shadow.
    pub fn upload(
        &mut self,
        queue: &wgpu::Queue,
        view_projection: &[[f32; 4]; 4],
        placements: &[Placement],
    ) {
        let mut block = [[0.0f32; 4]; 8];
        block[..4].copy_from_slice(view_projection);
        block[4] = [1.0, 0.0, 0.0, 0.0];
        block[5] = [0.0, 1.0, 0.0, 0.0];
        block[6] = [0.0, 0.0, 1.0, 0.0];
        block[7] = [0.0, 0.0, 0.0, 1.0];
        queue.write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&block));

        self.runs.clear();
        let mut vertices: Vec<GpuVertex> = Vec::with_capacity(MAX_VERTICES);
        for index in 0..self.silhouettes.len() {
            let first = vertices.len() as u32;
            for placement in placements
                .iter()
                .filter(|p| p.silhouette == index && p.strength > 0.0)
                .take(MAX_QUADS.saturating_sub(vertices.len() / 6))
            {
                vertices.extend(quad(placement));
            }
            let count = vertices.len() as u32 - first;
            if count > 0 {
                self.runs.push((index, first, count));
            }
        }
        if !vertices.is_empty() {
            queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&vertices));
        }
    }

    /// Draws into a pass the caller already opened.
    ///
    /// **After the track and before the hulls.** After, because the road's
    /// depth has to be in the buffer for the quad's depth test to keep it off
    /// the geometry behind; before, because the hulls are opaque and write
    /// depth, so a craft drawn afterwards covers its own shadow where it
    /// overlaps it rather than the shadow drawing over the craft.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.runs.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        for (silhouette, first, count) in &self.runs {
            pass.set_bind_group(1, &self.silhouettes[*silhouette], &[]);
            pass.draw(*first..*first + *count, 0..1);
        }
    }

    /// How many quads the last [`Pipeline::upload`] kept.
    ///
    /// An observable for the tests and the loader report: a pass that uploads
    /// nothing and a pass that draws nothing produce the same picture, so the
    /// difference needs something to read.
    #[must_use]
    pub fn quads(&self) -> usize {
        self.runs
            .iter()
            .map(|(_, _, count)| *count as usize / 6)
            .sum()
    }
}

#[cfg(test)]
mod tests;
