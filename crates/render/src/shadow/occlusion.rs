//! The second map a Wipeout HD craft carries: the track around it, drawn from
//! the sun as its own baked sun-occlusion mask, which the hull's sun term is
//! gated by.
//!
//! # What the original does
//!
//! `Job RenderShips` opens with `Shadow_RenderShipSunOcclusionMaps`: for each
//! ship it clears a small per-ship colour map to black, projects with the
//! **same** sun-view box `Shadow_RenderModelShadowMaps` fits to that ship,
//! and compiles every track chunk whose bounding sphere is within 10 units
//! of the ship through its `SunOcclusionLightmap` or `SunOcclusionVertex`
//! technique - a fragment program that writes nothing but the chunk's
//! `lightmap.a` or its colour set's fourth byte. No depth test, blue channel
//! only. The hull's `ShadowMap` variant then samples it projectively and
//! multiplies the sun's `N.L` by it. Read on
//! [`ship-sun-occlusion.md`](../../../../docs/ghidra/functions/ps3-hdfury-eu/ship-sun-occlusion.md);
//! the design is on [`shadows.md`](../../../../docs/rendering/shadows.md).
//!
//! # What is the original's and what is ours
//!
//! **The original's**: the mechanism - the road's own mask re-sampled along
//! the sun through the craft - the 10-unit cull radius ([`RADIUS`]), the
//! black clear, the two carriers, and that the sun term alone is gated.
//!
//! **Ours**: the map's size (the original's slot-0 map is 256 texels and the
//! rest smaller, by a slot assignment not read - every layer here is
//! [`SIZE`]), the fit (the original's box is fitted to the ship's bbox
//! from 70 units up the sun; this one is [`super::map::Fit`], centred on the
//! craft), and that the clear stays black on the six tracks and in Zone,
//! where the original clears white - what those names map to on the disc
//! was not resolved.
//!
//! The other factor of the original's gate - the craft's own depth map, its
//! self-shadow - is [`super::self_shadow`], owned by [`Maps`] so the two are
//! rendered through one matrix per layer.
//!
//! # Shape
//!
//! One `R8Unorm` array texture of [`OCCLUSION_LAYERS`] layers rather than eight
//! textures, because the hull pipeline reads it through the shared scene
//! bind group (group 2) with a per-drawable layer index - a fifth bind group
//! is not available on the downlevel limit `mesh_render` already sits at.

use oag_core::math::{Mat4, Vec3};

use super::map::{Caster, Fit};
use super::self_shadow;
use oag_mesh::mesh::{DrawCall, GpuVertex};
use oag_mesh::mesh_render::{OCCLUSION_FORMAT, OCCLUSION_LAYERS};

/// Each layer's resolution, square.
///
/// **The original's largest**: `g_ShipMapSizes` gives slot 0 a 256-texel
/// occlusion map and the seven others 128 down to 32. Which craft gets which
/// slot is unread, so every layer here gets the largest - 512 KiB for the
/// whole array.
pub const SIZE: u32 = 256;

/// How far from the **sun's line through the craft** a track chunk's
/// bounding sphere may sit and still be drawn into its map, in world units.
///
/// **The original's**: `Shadow_CompileSunOcclusionTrackRedraw` is called with
/// `10.0` (`fRam008b7e98`), squares it, and tests each chunk's
/// `|cross(centre - craft, sun)|^2` - the squared distance of the chunk's
/// centre from the sun ray through the craft - against
/// `chunk_radius^2 + 100`. A cylinder along the sun, not a sphere around the
/// craft: what lands near the craft's texels from the sun's view is drawn,
/// however far along the ray it sits, and the box's own depth
/// ([`REACH`]) is the only bound that way.
pub const RADIUS: f32 = 10.0;

/// How far the map's box reaches along the sun either side of the craft, in
/// world units.
///
/// **The original's**: `Shadow_BuildShipShadowMatrices` looks from 70 units
/// up the sun with near 1 and far 140
/// ([`shadow-model-maps.md`](../../../../docs/ghidra/functions/ps3-hdfury-eu/shadow-model-maps.md)),
/// and this map is projected through that same matrix. Deep enough that an
/// overhead deck is in the box - and, with no depth test, whichever of the
/// deck and the road under it draws later is what the hull reads. The
/// original's chunk order there is unread; this pass draws in the model's
/// own draw order.
pub const REACH: f32 = 70.0;

/// Bytes per layer uniform: one `mat4x4`, padded to the dynamic-offset
/// alignment every backend accepts.
const UNIFORM_STRIDE: u64 = 256;

/// The track as this pass draws it: its buffers, and every opaque draw call
/// with the material bind group it draws through.
///
/// Borrowed rather than owned, for the reason [`super::map::Caster`] is: the
/// geometry and the bind groups already exist for the main pass.
#[derive(Debug, Clone, Copy)]
pub struct Track<'a> {
    /// The track's vertex buffer, in [`GpuVertex`] layout.
    pub vertices: &'a wgpu::Buffer,
    /// Its index buffer, `u32`.
    pub indices: &'a wgpu::Buffer,
    /// The opaque draw calls, and separately the cutout and the transparent
    /// ones - together every surface that carries a baked mask.
    pub draws: &'a [DrawCall],
    /// The cutout draw calls, drawn exactly like [`Self::draws`] - the mask
    /// is per texel of the lightmap, not of the coverage, so a fence's holes
    /// carry the fence's shadow, as the original's technique has it.
    pub cutouts: &'a [DrawCall],
    /// The transparent draw calls, drawn the same way. **Not left out**: the
    /// original's compiler keys on the chunk's lighting family (lightmapped
    /// or vertex-lit), never on its blend, and Talon's Junction's glass floor
    /// is a lightmapped transparent surface - leaving it out left the map
    /// black under a craft on a lit glass road, which reads as no sun.
    pub transparent: &'a [DrawCall],
    /// The material bind groups, in `mesh_render`'s "albedo" layout, indexed
    /// by `DrawCall::texture + 1` exactly as the main pass indexes them.
    pub materials: &'a [wgpu::BindGroup],
    /// Model to world. The identity for a circuit.
    pub model: Mat4,
}

/// The maps, their pipeline, and the per-layer uniforms.
#[derive(Debug)]
pub struct Maps {
    texture: wgpu::Texture,
    array_view: wgpu::TextureView,
    layer_views: Vec<wgpu::TextureView>,
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    binds: Vec<wgpu::BindGroup>,
    /// The projection each layer was last rendered with, for the receiver.
    matrices: [Mat4; OCCLUSION_LAYERS as usize],
    /// How many draw calls each layer's last pass drew - a cleared layer and
    /// a never-rendered one look identical otherwise.
    drawn: [usize; OCCLUSION_LAYERS as usize],
    /// The craft's own depth map per layer, rendered through the same
    /// matrix as its occlusion layer - see [`Self::render_self_shadow`].
    self_shadow: self_shadow::Maps,
}

impl Maps {
    /// Builds the array and the pass's pipeline.
    ///
    /// `material_layout` is `mesh_render`'s own "albedo" bind group layout,
    /// so the track drawable's material bind groups can be set on this
    /// pipeline directly - see [`oag_mesh::mesh_render::material_bind_group_layout`].
    #[must_use]
    pub fn new(device: &wgpu::Device, material_layout: &wgpu::BindGroupLayout) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sun occlusion maps"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: OCCLUSION_LAYERS,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OCCLUSION_FORMAT,
            // `COPY_SRC` so a test can read a layer back: the map's only
            // other observable is a sampler inside the hull's shader.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let array_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("sun occlusion maps"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let layer_views = (0..OCCLUSION_LAYERS)
            .map(|layer| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("sun occlusion map layer"),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sun occlusion"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/occlusion.wgsl")).into(),
            ),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sun occlusion"),
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sun occlusion"),
            bind_group_layouts: &[Some(&layout), Some(material_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sun occlusion"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    // The mesh pipeline's own attribute list, so the track's
                    // buffer binds unchanged; the shader reads four of them.
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2,
                        4 => Float32, 5 => Uint32, 6 => Float32x2, 7 => Uint32, 8 => Float32,
                        9 => Uint32, 10 => Float32
                    ],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: OCCLUSION_FORMAT,
                    // No blend: the original writes each chunk's mask over
                    // whatever was there, in chunk order, with no depth test.
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                // The original's own pass sets `SET_CULL_FACE` to back while
                // this map draws; a road seen from the sun is front-facing.
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            // **No depth attachment**, as the original: `Rsx_SetDepthMask(0)`
            // and depth test off. What is nearest the sun is not the question;
            // what the road under the craft says is.
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sun occlusion uniforms"),
            size: UNIFORM_STRIDE * u64::from(OCCLUSION_LAYERS),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let binds = (0..OCCLUSION_LAYERS)
            .map(|layer| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("sun occlusion"),
                    layout: &layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &uniforms,
                            offset: UNIFORM_STRIDE * u64::from(layer),
                            size: std::num::NonZeroU64::new(64),
                        }),
                    }],
                })
            })
            .collect();

        Self {
            texture,
            array_view,
            layer_views,
            pipeline,
            uniforms,
            binds,
            matrices: [Mat4::IDENTITY; OCCLUSION_LAYERS as usize],
            drawn: [0; OCCLUSION_LAYERS as usize],
            self_shadow: self_shadow::Maps::new(device),
        }
    }

    /// Clears `layer` to black and draws into it every draw call of `track`
    /// whose bounds come within [`RADIUS`] of `fit.centre`, projected by
    /// `fit`.
    ///
    /// Returns how many draw calls it drew. Zero is a real answer - a craft
    /// off the track over nothing - and it leaves the layer black, which the
    /// hull reads as no sun, exactly as the original's black clear does.
    ///
    /// **Black, every frame.** The original clears per ship per frame; a
    /// stale layer would drag last frame's road under a craft that has moved.
    pub fn render(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        layer: usize,
        fit: &Fit,
        track: &Track<'_>,
    ) -> usize {
        let Some(view) = self.layer_views.get(layer) else {
            return 0;
        };
        let matrix = fit.matrix_reaching(REACH);
        self.matrices[layer] = matrix;
        let mvp = matrix * track.model;
        queue.write_buffer(
            &self.uniforms,
            UNIFORM_STRIDE * layer as u64,
            bytemuck::cast_slice(&mvp.to_cols_array()),
        );
        let sun = fit.towards_light.normalize_or_zero();
        let near: Vec<&DrawCall> = track
            .draws
            .iter()
            .chain(track.cutouts)
            .chain(track.transparent)
            .filter(|draw| within(draw, fit.centre, sun))
            .collect();
        self.drawn[layer] = near.len();

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sun occlusion map"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
        if near.is_empty() {
            // The pass still ran, so the clear landed.
            return 0;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.binds[layer], &[]);
        pass.set_vertex_buffer(0, track.vertices.slice(..));
        pass.set_index_buffer(track.indices.slice(..), wgpu::IndexFormat::Uint32);
        for draw in &near {
            let slot = draw.texture.map_or(0, |index| index + 1);
            let Some(material) = track.materials.get(slot) else {
                continue;
            };
            pass.set_bind_group(1, material, &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
        near.len()
    }

    /// Renders `caster` - the craft itself, its opaque ranges - into
    /// `layer`'s self-shadow depth map through the matrix the last
    /// [`Self::render`] of that layer projected with, so the hull's one
    /// projective coordinate lands on the same texel of both maps. Call it
    /// after `render`; a layer since cleared projects through the identity
    /// and holds nothing useful, which is harmless because no hull names a
    /// cleared layer.
    ///
    /// Returns how many index ranges it drew.
    pub fn render_self_shadow(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        layer: usize,
        caster: &Caster<'_>,
    ) -> usize {
        let matrix = self.matrix(layer);
        self.self_shadow
            .render(queue, encoder, layer, matrix, caster)
    }

    /// Clears `layer` to black without drawing, so a craft that is not
    /// active this frame reads no sun rather than last frame's road. Its
    /// self-shadow layer clears to far with it.
    ///
    /// A caller that skips this while [`Self::drawn`] is zero (as the frame
    /// does) may leave a self-shadow layer stale - which is unobservable
    /// only as long as a hull names its layer by the *occlusion* count, the
    /// invariant `oag-game`'s `sun_occlusion_layer` keeps. Gate a layer on
    /// this count, never on the self-shadow's own.
    pub fn clear(&mut self, encoder: &mut wgpu::CommandEncoder, layer: usize) {
        let Some(view) = self.layer_views.get(layer) else {
            return;
        };
        self.drawn[layer] = 0;
        self.matrices[layer] = Mat4::IDENTITY;
        self.self_shadow.clear(encoder, layer);
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sun occlusion map clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
    }

    /// The array view the hull pipelines sample, through the scene group.
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.array_view
    }

    /// The self-shadow depth maps [`Self::render_self_shadow`] fills: their
    /// array view for the scene group, and their readback.
    #[must_use]
    pub fn self_shadow(&self) -> &self_shadow::Maps {
        &self.self_shadow
    }

    /// The projection `layer` was last rendered with, for the receiver's own
    /// uniform. The identity for a layer never rendered or since cleared.
    #[must_use]
    pub fn matrix(&self, layer: usize) -> Mat4 {
        self.matrices.get(layer).copied().unwrap_or(Mat4::IDENTITY)
    }

    /// How many draw calls `layer`'s last pass drew.
    #[must_use]
    pub fn drawn(&self, layer: usize) -> usize {
        self.drawn.get(layer).copied().unwrap_or(0)
    }

    /// The texture behind [`Self::view`], for a reader that wants a layer
    /// back - the headless checks do.
    #[must_use]
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// Reads `layer` back as one byte per texel, row-major from the top -
    /// [`SIZE`] squared of them - blocking on the GPU. For a headless check
    /// or a dump; the frame never calls this.
    #[must_use]
    pub fn read_back(&self, device: &wgpu::Device, queue: &wgpu::Queue, layer: u32) -> Vec<u8> {
        let row = SIZE.div_ceil(256) * 256;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sun occlusion readback"),
            size: u64::from(row * SIZE),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer.min(OCCLUSION_LAYERS - 1),
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(SIZE),
                },
            },
            wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("draining the queue");
        let view = slice.get_mapped_range().expect("mapping the readback");
        let mut out = vec![0u8; (SIZE * SIZE) as usize];
        for y in 0..SIZE as usize {
            let start = y * row as usize;
            out[y * SIZE as usize..(y + 1) * SIZE as usize]
                .copy_from_slice(&view[start..start + SIZE as usize]);
        }
        drop(view);
        buffer.unmap();
        out
    }
}

/// Whether `draw`'s bounding sphere comes within [`RADIUS`] of the line
/// through `centre` along `sun` - the original's own chunk test:
/// `|cross(chunk - craft, sun)|^2 <= chunk_radius^2 + RADIUS^2`, `sun` a
/// unit vector.
///
/// A draw whose bounds cannot be trusted (`moving`) is left out rather than
/// drawn everywhere: the main pass draws it unculled because the error there
/// has to point towards drawing too much, but here drawing too much means a
/// moving object's rest-pose mask painted under every craft on the circuit.
#[must_use]
pub fn within(draw: &DrawCall, centre: Vec3, sun: Vec3) -> bool {
    if draw.moving {
        return false;
    }
    let offset = Vec3::from_array(draw.bounds.centre) - centre;
    let radius = draw.bounds.radius;
    offset.cross(sun).length_squared() <= radius * radius + RADIUS * RADIUS
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_mesh::mesh::Bounds;

    fn draw(centre: [f32; 3], radius: f32, moving: bool) -> DrawCall {
        DrawCall {
            range: 0..3,
            texture: None,
            bounds: Bounds { centre, radius },
            moving,
            culled: false,
            blend: None,
            blend_state: None,
            layer: 0,
            alpha_test_ref: None,
            node: None,
            chunk: None,
        }
    }

    #[test]
    fn the_cull_is_a_cylinder_along_the_sun_not_a_sphere_around_the_craft() {
        let craft = Vec3::new(0.0, 4.0, 0.0);
        let sun = Vec3::Y;
        // The road under the craft.
        assert!(within(&draw([0.0, 0.0, 0.0], 5.0, false), craft, sun));
        // A deck sixty units overhead, straight up the sun: in the cylinder,
        // however far along the ray it sits.
        assert!(within(&draw([0.0, 64.0, 0.0], 2.0, false), craft, sun));
        // A chunk twelve units to the side with a two-unit sphere:
        // 144 > 4 + 100, out.
        assert!(!within(&draw([12.0, 4.0, 0.0], 2.0, false), craft, sun));
        // The same centre with a sphere big enough to reach: 144 <= 49 + 100.
        assert!(within(&draw([12.0, 4.0, 0.0], 7.0, false), craft, sun));
    }

    #[test]
    fn a_moving_chunk_is_never_drawn() {
        let craft = Vec3::ZERO;
        assert!(!within(&draw([0.0, 0.0, 0.0], 5.0, true), craft, Vec3::Y));
    }
}
