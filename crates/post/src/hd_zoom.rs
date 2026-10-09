//! Wipeout HD's boost and damage zoom-streak ring: `FunkLayerZoom`.
//!
//! **Not a motion blur.** Nothing here depends on velocity: a pulse switches the
//! pass on, the ship boosting (`E`, about 1.2 s from a speed pad or a Turbo) or
//! taking damage (`P`, decaying at 1.67 per second), and at 438 km/h with neither
//! the pass does not run. It is therefore not a value of the motion blur setting.
//! Evidence, per number, with a confidence each:
//! [funklayer-zoom.md](../../../../docs/ghidra/functions/ps3-hdfury-eu/funklayer-zoom.md).
//!
//! What the original does, in the order `FunkLayer_RunBloomChain` runs it:
//!
//! 1. **The history draw** (two draws of `FunkLayerBloomDownsample_fp`): the
//!    quarter-resolution scene is copied into one of a ping-pong pair, then the
//!    other buffer is drawn over it with `SRC_ALPHA, ONE_MINUS_SRC_ALPHA` at
//!    alpha `w = min(0.95, 0.25 E)`, its texture coordinates cropped to
//!    `uv * (1 - 2c) + c` with `c = 0.05 E`. So the history **zooms in by up to
//!    10 % a frame while the pulse runs and fades back** - the feedback that is
//!    the streak.
//! 2. **The ring** (`FunkLayerZoom_vp` and `_fp`): 24 quads between radius 0.62
//!    (alpha 0) and 1.5 (alpha `max(E, P)`) in NDC. Each outer vertex's two
//!    texture coordinates are pulled toward the inner vertex of the same angle
//!    by 0.15 and 0.95; the fragment sums the two taps of that history and
//!    blends `1 - col * (1 - (t0 + t1))` at alpha `1 - (1 - a)^2` over the
//!    **scene, before the exposure resolve and the bloom add, and before the
//!    HUD**. Draw order: after the blurs, before `FunkLayerBloom`'s resolve.
//!
//! **Chosen, not measured:**
//!
//! - The pulses step per **simulation tick**, not per rendered frame. The
//!   original steps them per update, and its decay is `0.035` per update, so a
//!   per-frame step at 144 fps would shorten the pulse and lengthen the streak.
//!   The history pass runs once per tick for the same reason.
//! - The scene, the history and the ring all stay in this chain's own scene
//!   space. The original blends 8-bit surfaces that are all in the one space;
//!   the clamp it gets from 8 bits is applied where the ring reads and writes.
//! - The one-update lag between the pulse and what a captured frame shows is the
//!   original's CPU/GPU pipelining, and is not modelled.

use anyhow::Result;
use oag_gpu::formats::SCENE_FORMAT;

use super::{sampler_entry, texture_entry, uniform_entry};

/// Alpha under which the pass does not run, the runner's own `1e-4`.
pub const OFF_BELOW: f32 = 1.0e-4;

/// The numbers `oag_title::ZoomRing` carries for a title, as this pass reads them.
///
/// A separate type because this crate reaches no title vocabulary, the way
/// [`super::hd_bloom::Params`] is separate from the `.envsettings` it is read from;
/// `oag-raceplay` converts the one into the other.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// The history draw's alpha per unit of `E`.
    pub history_weight: f32,
    /// The cap on that alpha.
    pub history_weight_cap: f32,
    /// The history sample's crop per unit of `E`.
    pub history_crop: f32,
    /// Radius, in NDC, of the ring's transparent inner edge.
    pub inner_radius: f32,
    /// Radius, in NDC, of the ring's opaque outer edge.
    pub outer_radius: f32,
    /// Quads around the ring.
    pub segments: u32,
    /// How far each outer vertex's two texture coordinates are pulled toward
    /// the inner vertex of the same angle.
    pub tap_pull: [f32; 2],
    /// `A0` as the trigger writes it.
    pub boost_start: f32,
    /// `A0` falls by `dt * boost_fall_rate`.
    pub boost_fall_rate: f32,
    /// `E = clamp(gain * ((1 - slope * A0) - offset), 0, 1)` while `A0` is up.
    pub boost_slope: f32,
    /// See [`Self::boost_slope`].
    pub boost_offset: f32,
    /// See [`Self::boost_slope`].
    pub boost_gain: f32,
    /// `E` falls by this per update once `A0` has run out.
    pub boost_decay_per_update: f32,
    /// The ring's colour is `(1 - tint[0] * P, 1 - tint[1] * P, 1)`.
    pub damage_tint: [f32; 2],
    /// `P` falls by this per second.
    pub damage_decay_per_second: f32,
    /// `size = 1 + A`, `A = retain * A + (rand % range) * step * E`.
    pub jitter_retain: f32,
    /// See [`Self::jitter_retain`].
    pub jitter_step: f32,
    /// See [`Self::jitter_retain`].
    pub jitter_range: u32,
}

/// One tick's worth of the pulses, as the pass reads them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// The boost pulse `E`, `0..=1`.
    pub boost: f32,
    /// The damage pulse `P`, `0..=1`.
    pub damage: f32,
    /// The ring's size jitter, `1 + A` on each axis.
    pub size: [f32; 2],
    /// The simulation tick this was stepped for. The history draw runs once
    /// per tick, so two frames of one tick share a history.
    pub tick: u64,
}

impl Frame {
    /// The ring's alpha at its rim, `max(E, P)`.
    #[must_use]
    pub fn alpha(&self) -> f32 {
        self.boost.max(self.damage)
    }

    /// Whether the ring is drawn at all.
    #[must_use]
    pub fn active(&self) -> bool {
        self.boost > OFF_BELOW || self.damage > OFF_BELOW
    }
}

/// The two pulses and the size jitter, stepped once per simulation tick.
#[derive(Debug, Clone, PartialEq)]
pub struct Pulse {
    tuning: Tuning,
    /// `A0`, the boost trigger's countdown.
    countdown: f32,
    boost: f32,
    damage: f32,
    jitter: [f32; 2],
    rng: u32,
    tick: u64,
}

impl Pulse {
    /// A quiet pulse: no boost, no damage, the ring off.
    #[must_use]
    pub fn new(tuning: Tuning) -> Self {
        Self {
            tuning,
            countdown: 0.0,
            boost: 0.0,
            damage: 0.0,
            jitter: [0.0; 2],
            rng: 0x2F6E_2B1D,
            tick: 0,
        }
    }

    /// The boost trigger: a speed pad or a Turbo. Writes `A0 = start`, one write,
    /// as `EngineFlare_TriggerZoomGlow` does; a second trigger during a pulse
    /// restarts the ramp from `E`'s current value upward.
    pub fn fire_boost(&mut self) {
        self.countdown = self.tuning.boost_start;
    }

    /// A hit: `P = max(P, clamp(amount, 0, 1))`, the raise `Hud_RaiseDamagePulse`
    /// does. `amount` is what that function is handed, `1.8 * damage` in the
    /// original; what a hit of ours maps to is the caller's to state.
    pub fn hit(&mut self, amount: f32) {
        self.damage = self.damage.max(amount.clamp(0.0, 1.0));
    }

    /// One update of `dt` seconds: the boost law, the damage decay, the jitter.
    pub fn advance(&mut self, dt: f32) {
        let t = &self.tuning;
        if self.countdown > 0.0 {
            self.countdown = (self.countdown - dt * t.boost_fall_rate).max(0.0);
        }
        if self.countdown > 0.0 {
            let ramp = 1.0 - t.boost_slope * self.countdown;
            if ramp > t.boost_offset {
                self.boost = (t.boost_gain * (ramp - t.boost_offset)).clamp(0.0, 1.0);
            }
        } else {
            self.boost = (self.boost - t.boost_decay_per_update).max(0.0);
        }
        self.damage = (self.damage - dt * t.damage_decay_per_second).max(0.0);
        for axis in &mut self.jitter {
            // xorshift32: a render-side random, nothing the simulation hashes.
            self.rng ^= self.rng << 13;
            self.rng ^= self.rng >> 17;
            self.rng ^= self.rng << 5;
            let draw = (self.rng % t.jitter_range.max(1)) as f32;
            *axis = t.jitter_retain * *axis + draw * t.jitter_step * self.boost;
        }
        self.tick += 1;
    }

    /// The pulses as the pass reads them for the tick last stepped.
    #[must_use]
    pub fn frame(&self) -> Frame {
        Frame {
            boost: self.boost,
            damage: self.damage,
            size: [1.0 + self.jitter[0], 1.0 + self.jitter[1]],
            tick: self.tick,
        }
    }

    /// The pulse a posed frame shows `seconds` after the boost trigger, stepped
    /// at `dt`: the trigger, then `seconds / dt` updates. For a capture that
    /// compares our frame against a paused original at a matching `E`.
    pub fn pose_boost(&mut self, seconds: f32, dt: f32) {
        self.fire_boost();
        for _ in 0..(seconds / dt).round() as u32 {
            self.advance(dt);
        }
    }
}

/// One vertex of the ring, the layout of the original's CPU mesh: position,
/// the two taps' texture coordinates, colour.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    /// NDC position, already multiplied by the size jitter.
    pub position: [f32; 2],
    /// The first tap's texture coordinate.
    pub uv0: [f32; 2],
    /// The second tap's texture coordinate.
    pub uv1: [f32; 2],
    /// Vertex colour; alpha is `0` on the inner edge and `max(E, P)` outside.
    pub colour: [f32; 4],
}

/// The ring for one frame: `segments` quads of four vertices each, in the
/// original's order (inner then outer at one angle, outer then inner at the
/// next). Angles run clockwise from straight up in 15 degree steps.
#[must_use]
pub fn ring(tuning: &Tuning, frame: &Frame) -> Vec<Vertex> {
    let segments = tuning.segments.max(1);
    let step = std::f32::consts::TAU / segments as f32;
    let rim = frame.alpha();
    let p = frame.damage;
    let outer_colour = [
        1.0 - tuning.damage_tint[0] * p,
        1.0 - tuning.damage_tint[1] * p,
        1.0,
        rim,
    ];
    let size = frame.size;
    let at = |k: u32, radius: f32| {
        let angle = step * k as f32;
        [radius * angle.sin(), radius * angle.cos()]
    };
    let screen = |p: [f32; 2]| [0.5 + p[0] * 0.5, 0.5 - p[1] * 0.5];
    let inner = |k: u32| {
        let position = at(k, tuning.inner_radius);
        let uv = screen(position);
        Vertex {
            position: [position[0] * size[0], position[1] * size[1]],
            uv0: uv,
            uv1: uv,
            colour: [1.0, 1.0, 1.0, 0.0],
        }
    };
    let outer = |k: u32| {
        let position = at(k, tuning.outer_radius);
        let own = screen(position);
        let toward = screen(at(k, tuning.inner_radius));
        let pulled = |pull: f32| {
            [
                own[0] + (toward[0] - own[0]) * pull,
                own[1] + (toward[1] - own[1]) * pull,
            ]
        };
        Vertex {
            position: [position[0] * size[0], position[1] * size[1]],
            uv0: pulled(tuning.tap_pull[0]),
            uv1: pulled(tuning.tap_pull[1]),
            colour: outer_colour,
        }
    };
    let mut vertices = Vec::with_capacity(segments as usize * 4);
    for k in 0..segments {
        let next = (k + 1) % segments;
        vertices.extend([inner(k), outer(k), outer(next), inner(next)]);
    }
    vertices
}

/// The history draw's alpha and crop for `E`.
#[must_use]
pub fn history(tuning: &Tuning, boost: f32) -> (f32, f32) {
    (
        (tuning.history_weight * boost).min(tuning.history_weight_cap),
        tuning.history_crop * boost,
    )
}

/// The uniform `hd_zoom.wesl` reads.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Constants {
    uv_scale: [f32; 2],
    uv_max: [f32; 2],
    half_texel: [f32; 2],
    weight: f32,
    crop: f32,
}

/// The pass's pipelines, history ping-pong and ring buffers.
#[derive(Debug)]
pub struct Zoom {
    tuning: Tuning,
    history_pipeline: wgpu::RenderPipeline,
    ring_pipeline: wgpu::RenderPipeline,
    /// The history draw's uniform and the ring's: two buffers, because a queue
    /// write lands before the whole submission and one buffer would hold only the
    /// later pass's values for both.
    constants: [wgpu::Buffer; 2],
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    /// `history[i]` writes ping-pong buffer `i`, reading the other.
    history_groups: [wgpu::BindGroup; 2],
    /// `ring[i]` samples ping-pong buffer `i`.
    ring_groups: [wgpu::BindGroup; 2],
    #[expect(dead_code, reason = "held so the views stay valid")]
    textures: [wgpu::Texture; 2],
    views: [wgpu::TextureView; 2],
    quarter: (u32, u32),
    /// Which buffer the last history draw wrote.
    current: std::cell::Cell<usize>,
    /// The tick the last history draw ran for.
    last_tick: std::cell::Cell<Option<u64>>,
}

impl Zoom {
    /// Builds the pass for a chain whose quarter-resolution scene is `quarter`
    /// of `quarter_size` texels.
    ///
    /// # Errors
    ///
    /// Propagates nothing today; `Result` keeps the call shape of the chain's own
    /// constructor for a pipeline that will not build.
    pub fn new(
        device: &wgpu::Device,
        tuning: Tuning,
        quarter: &wgpu::TextureView,
        quarter_size: (u32, u32),
    ) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hd zoom"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/hd_zoom.wgsl")).into(),
            ),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hd zoom"),
            entries: &[
                texture_entry(0),
                sampler_entry(1),
                uniform_entry(2),
                texture_entry(3),
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("hd zoom"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("hd zoom"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let history_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hd zoom history"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_history"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_history"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: SCENE_FORMAT,
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
        });
        // `SRC_ALPHA, ONE_MINUS_SRC_ALPHA` on colour and `ONE, ZERO` on alpha: the
        // ring's own state words, `Rsx_SetBlendFuncSeparate(0x302, 0x303, 1, 0)`.
        let blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::Zero,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let ring_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hd zoom ring"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_ring"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2, 1 => Float32x2, 2 => Float32x2, 3 => Float32x4
                    ],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_ring"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: SCENE_FORMAT,
                    blend: Some(blend),
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

        let texture = |label: &str| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: quarter_size.0.max(1),
                    height: quarter_size.1.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: SCENE_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | if cfg!(test) {
                        wgpu::TextureUsages::COPY_SRC
                    } else {
                        wgpu::TextureUsages::empty()
                    },
                view_formats: &[],
            })
        };
        let textures = [texture("hd zoom history a"), texture("hd zoom history b")];
        let views = [
            textures[0].create_view(&Default::default()),
            textures[1].create_view(&Default::default()),
        ];
        let uniform = |label: &str| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size_of::<Constants>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let constants = [
            uniform("hd zoom history constants"),
            uniform("hd zoom ring constants"),
        ];
        let group = |label: &str,
                     source: &wgpu::TextureView,
                     second: &wgpu::TextureView,
                     buffer: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(second),
                    },
                ],
            })
        };
        // history[i] writes buffer i, reading the quarter scene and buffer 1 - i.
        let history_groups = [
            group("hd zoom history a", quarter, &views[1], &constants[0]),
            group("hd zoom history b", quarter, &views[0], &constants[0]),
        ];
        // The ring samples one buffer; the second slot is never read there and
        // binds the other buffer, not the one being drawn into (the scene).
        let ring_groups = [
            group("hd zoom ring a", &views[0], &views[1], &constants[1]),
            group("hd zoom ring b", &views[1], &views[0], &constants[1]),
        ];
        let quads = tuning.segments.max(1);
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hd zoom ring vertices"),
            size: u64::from(quads) * 4 * size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_data: Vec<u16> = (0..quads)
            .flat_map(|q| {
                let b = (q * 4) as u16;
                [b, b + 1, b + 2, b, b + 2, b + 3]
            })
            .collect();
        let indices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hd zoom ring indices"),
            size: (index_data.len() * 2) as u64,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: true,
        });
        indices
            .slice(..)
            .get_mapped_range_mut()
            .expect("a freshly mapped buffer maps")
            .copy_from_slice(bytemuck::cast_slice(&index_data));
        indices.unmap();
        Ok(Self {
            tuning,
            history_pipeline,
            ring_pipeline,
            constants,
            vertices,
            indices,
            index_count: index_data.len() as u32,
            history_groups,
            ring_groups,
            textures,
            views,
            quarter: (quarter_size.0.max(1), quarter_size.1.max(1)),
            current: std::cell::Cell::new(0),
            last_tick: std::cell::Cell::new(None),
        })
    }

    /// The history draw, once per simulation tick: the quarter scene mixed with
    /// the other buffer, cropped, into the next buffer. Encoded right after the
    /// chain's quarter-resolution downsample, while the quarter scene still holds
    /// the scene (the blur passes overwrite it).
    ///
    /// `rect` is the drawn rectangle of a quarter-resolution level and `scale`
    /// and `max` its UV mapping, as `hd_bloom` computes them.
    pub fn history(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame,
        rect: (u32, u32),
        mapping: ([f32; 2], [f32; 2]),
    ) {
        if self.last_tick.get() == Some(frame.tick) {
            return;
        }
        self.last_tick.set(Some(frame.tick));
        let next = 1 - self.current.get();
        self.current.set(next);
        let (weight, crop) = history(&self.tuning, frame.boost);
        self.write_constants(queue, 0, mapping, weight, crop);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("hd zoom history"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.views[next],
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
        });
        pass.set_pipeline(&self.history_pipeline);
        pass.set_viewport(0.0, 0.0, rect.0 as f32, rect.1 as f32, 0.0, 1.0);
        pass.set_bind_group(0, &self.history_groups[next], &[]);
        pass.draw(0..3, 0..1);
    }

    /// The ring, over the scene, while a pulse runs; nothing otherwise.
    ///
    /// `rect` is the scene's drawn rectangle in pixels: the ring is in NDC of it.
    pub fn ring(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        scene: &wgpu::TextureView,
        frame: &Frame,
        rect: (u32, u32),
        mapping: ([f32; 2], [f32; 2]),
    ) {
        if !frame.active() {
            return;
        }
        let (weight, crop) = history(&self.tuning, frame.boost);
        self.write_constants(queue, 1, mapping, weight, crop);
        queue.write_buffer(
            &self.vertices,
            0,
            bytemuck::cast_slice(&ring(&self.tuning, frame)),
        );
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("hd zoom ring"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: scene,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.ring_pipeline);
        pass.set_viewport(0.0, 0.0, rect.0 as f32, rect.1 as f32, 0.0, 1.0);
        pass.set_bind_group(0, &self.ring_groups[self.current.get()], &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }

    fn write_constants(
        &self,
        queue: &wgpu::Queue,
        which: usize,
        (uv_scale, uv_max): ([f32; 2], [f32; 2]),
        weight: f32,
        crop: f32,
    ) {
        let constants = Constants {
            uv_scale,
            uv_max,
            half_texel: [0.5 / self.quarter.0 as f32, 0.5 / self.quarter.1 as f32],
            weight,
            crop,
        };
        queue.write_buffer(&self.constants[which], 0, bytemuck::bytes_of(&constants));
    }
}

#[cfg(test)]
mod tests;
