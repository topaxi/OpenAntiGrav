//! Camera motion blur: the first consumer of the temporal infrastructure TAA
//! needs, and deliberately the smallest one.
//!
//! [ADR-0013](../../../../docs/architecture/adr/0013-anti-aliasing-architecture.md)
//! names three things temporal reconstruction is missing: motion vectors,
//! camera jitter and a history buffer. This pass builds the first - a
//! per-pixel screen-space velocity, recovered by unprojecting each pixel's
//! depth through the inverse of the frame's view-projection and reprojecting
//! it through the previous frame's - and spends it on the simplest effect
//! that makes it visible and testable: a straight-line gather along that
//! velocity. TAA will replace the gather with a history accumulation and keep
//! the reprojection; see
//! [ADR-0028](../../../../docs/architecture/adr/0028-camera-motion-blur-first.md).
//!
//! # What "camera" motion blur means
//!
//! The velocity is exact for every static point in the world and blind to
//! anything moving through it: an opponent's craft blurs by however the
//! camera swept across it, not by its own speed. Per-object velocity needs
//! the scene pass to write a velocity attachment, which is a later increment
//! and recorded in the ADR rather than approximated here.
//!
//! # An effect of this project's, not the original's
//!
//! Neither PSP build renders motion blur; this is an enhancement in the class
//! of FSR 1 and SMAA, off by default and behind `[graphics] motion_blur`. It
//! invents no asset and stands in for nothing - the rule about playing the
//! disc's own data is about the game's content, and this touches only how a
//! finished frame is presented.

use anyhow::Result;

use oag_core::math::Mat4;

/// The cap on the gather's reach, as a fraction of the viewport height.
///
/// The bound that keeps a camera cut - a view switch, a respawn - from
/// smearing across the whole frame: one frame of arbitrarily large velocity
/// becomes one frame of at most this much stretch.
///
/// Raised from `0.05` on the first real-race feedback: at racing speed the
/// near ground and walls - the streaks that carry the sense of speed - were
/// hitting the cap and coming out uniformly short, which read as "the track
/// barely blurs". Eight per cent still bounds a cut to a fraction of the
/// frame while leaving the legitimate near-field motion visible.
pub const MAX_STRETCH: f32 = 0.08;

/// How many focus spheres [`Frame::focus`] can carry - one per grid slot.
pub const MAX_FOCUS: usize = 8;

/// The shader's uniform. `repr(C)`; two matrices, four rows of packed
/// vec2/scalars, and the focus spheres - 304 bytes with no implicit padding.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Constants {
    inv_view_proj: [[f32; 4]; 4],
    prev_view_proj: [[f32; 4]; 4],
    rect_offset: [f32; 2],
    rect_size: [f32; 2],
    rect_pixels: [f32; 2],
    strength: f32,
    max_stretch: f32,
    focus_count: u32,
    _pad: [f32; 3],
    focus: [[f32; 4]; MAX_FOCUS],
}

impl Constants {
    /// `viewport` is the scene's rectangle inside the target - `(x, y, w, h)`
    /// in pixels, the same tuple `Scene::render` takes - and `size` the whole
    /// target, both needed because a capture draws the scene into a
    /// sub-rectangle with the aspect bars outside it.
    fn new(
        view_projection: Mat4,
        previous: Mat4,
        viewport: (f32, f32, f32, f32),
        size: (u32, u32),
        strength: f32,
        focus: &[[f32; 4]],
    ) -> Self {
        let (w, h) = (size.0.max(1) as f32, size.1.max(1) as f32);
        let count = focus.len().min(MAX_FOCUS);
        let mut spheres = [[0.0; 4]; MAX_FOCUS];
        spheres[..count].copy_from_slice(&focus[..count]);
        Self {
            inv_view_proj: view_projection.inverse().to_cols_array_2d(),
            prev_view_proj: previous.to_cols_array_2d(),
            rect_offset: [viewport.0 / w, viewport.1 / h],
            rect_size: [viewport.2.max(1.0) / w, viewport.3.max(1.0) / h],
            rect_pixels: [viewport.2.max(1.0), viewport.3.max(1.0)],
            strength,
            max_stretch: MAX_STRETCH,
            focus_count: count as u32,
            _pad: [0.0; 3],
            focus: spheres,
        }
    }
}

/// The two cameras the reprojection runs between.
///
/// `previous` is the last view-projection that **differed** from the current
/// one - not simply the one handed to the last render call - but only while
/// the difference is one the simulation has not yet had a tick to settle.
/// Two failure modes force that shape, one on each side of a naive rule:
///
/// - The camera moves per simulation tick and frames can outrun ticks, so at
///   a display rate above 60 Hz every other frame re-renders an unmoved
///   camera. Taking each render call's camera as "the previous" would read
///   those repeats as zero velocity and flicker the blur off on exactly
///   alternate frames.
/// - Holding the last distinct camera with no expiry gets the opposite
///   wrong: a camera that genuinely stops - a pause, a craft parked on the
///   grid - would keep blurring by its last movement forever.
///
/// `stamp` is what tells the two apart: it advances when the simulation
/// does. Same camera, same stamp is a frame outrunning the tick and keeps
/// the held `previous`; same camera, new stamp is a camera that really held
/// still for a tick, and `previous` collapses onto it.
#[derive(Debug, Clone, Copy)]
struct Camera {
    current: Mat4,
    previous: Mat4,
    stamp: u64,
}

impl Camera {
    /// Folds this frame's camera in, per the rule above. The first
    /// observation seeds both matrices, so a scene's first frame has zero
    /// velocity rather than a smear from an uninitialised matrix.
    fn observe(camera: &mut Option<Self>, view_projection: Mat4, stamp: u64) {
        match camera {
            None => {
                *camera = Some(Self {
                    current: view_projection,
                    previous: view_projection,
                    stamp,
                });
            }
            Some(camera) if camera.current != view_projection => {
                camera.previous = camera.current;
                camera.current = view_projection;
                camera.stamp = stamp;
            }
            Some(camera) => {
                if camera.stamp != stamp {
                    camera.previous = camera.current;
                    camera.stamp = stamp;
                }
            }
        }
    }
}

/// One frame's worth of input to [`MotionBlur::render`].
#[derive(Debug)]
pub struct Frame<'a> {
    /// The finished frame, sampled and then overwritten - legal because the
    /// gather lands in a scratch target and a second pass carries it back,
    /// the same read-then-write shape `bloom` uses.
    pub scene: &'a wgpu::TextureView,
    /// A **single-sampled** view of the scene's depth attachment. The caller
    /// owns the MSAA decision, and a multisampled depth buffer cannot bind as
    /// `texture_depth_2d` - see `race::Scene::new`, which skips building this
    /// pass at all under MSAA rather than binding what it cannot.
    pub depth: &'a wgpu::TextureView,
    /// The size of `scene` and `depth` in pixels.
    pub size: (u32, u32),
    /// The scene's rectangle inside them: `(x, y, w, h)` in pixels, the same
    /// tuple `Scene::render` draws with. The whole target in a window,
    /// a sub-rectangle with bars outside it in a capture.
    pub viewport: (f32, f32, f32, f32),
    /// The view-projection this frame was drawn with.
    pub view_projection: Mat4,
    /// A counter that advances when the simulation the camera rides does -
    /// the race tick. What separates a frame outrunning the tick from a
    /// camera that genuinely held still; see [`Camera`].
    pub stamp: u64,
    /// What fraction of the frame-to-frame camera travel the gather spans -
    /// the shutter, `0.5` being the film convention. At zero or below the
    /// pass does nothing at all, which is how "off" reaches it: the caller's
    /// setting is a strength, read fresh every frame.
    pub strength: f32,
    /// Screen-space spheres the blur keeps sharp - the craft. Each is
    /// `[centre u, centre v, radius in pixels, NDC depth of the far side]`,
    /// centre in viewport-relative uv; entries past [`MAX_FOCUS`] are
    /// dropped. Camera reprojection computes the *world's* velocity at every
    /// pixel, which is exactly wrong for a craft holding station near the
    /// camera - the reference object a player's eye rests on - so the caller
    /// hands over the bounds it already knows and those pixels gather
    /// nothing. The depth component keeps the road right behind a craft
    /// blurring up to its silhouette.
    pub focus: &'a [[f32; 4]],
}

/// The gather and copy pipelines, the scratch target, and the camera pair.
#[derive(Debug)]
pub struct MotionBlur {
    blur: wgpu::RenderPipeline,
    copy: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    constants: wgpu::Buffer,
    written: Option<Constants>,
    scratch: Option<Scratch>,
    format: wgpu::TextureFormat,
    size: (u32, u32),
    camera: Option<Camera>,
}

#[derive(Debug)]
struct Scratch {
    #[expect(dead_code, reason = "held so the view stays valid")]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl MotionBlur {
    /// Builds both pipelines against the scene target's `format`.
    ///
    /// The scratch target is built at the first [`render`](Self::render),
    /// which is the first time a size is known.
    ///
    /// # Errors
    ///
    /// Propagates a shader that will not compile, which is a build-time
    /// mistake rather than anything a player can cause.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("motion blur"),
            source: wgpu::ShaderSource::Wgsl(include_str!("motion_blur.wgsl").into()),
        });

        // The fullscreen trio plus the depth texture. Depth rather than float
        // so `textureLoad` returns the attachment's own values; no sampler
        // touches it, so it needs no `Filtering` pairing.
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("motion blur"),
            entries: &[
                super::texture_entry(0),
                super::sampler_entry(1),
                super::uniform_entry(2),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("motion blur"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("motion blur"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let format = format.remove_srgb_suffix();
        let pipeline = |label: &str, entry: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
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
        };
        let blur = pipeline("motion blur gather", "fs_blur");
        let copy = pipeline("motion blur copy", "fs_copy");

        let constants = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("motion blur constants"),
            size: size_of::<Constants>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            blur,
            copy,
            layout,
            sampler,
            constants,
            written: None,
            scratch: None,
            format,
            size: (0, 0),
            camera: None,
        })
    }

    /// Forgets the previous camera, so the next frame renders unblurred.
    ///
    /// For a deliberate cut the caller knows about - a view switch, a respawn.
    /// A cut this is not called for still cannot smear more than
    /// [`MAX_STRETCH`], which is the shader's own bound.
    pub fn reset(&mut self) {
        self.camera = None;
    }

    /// Runs the gather and carries it back onto `frame.scene`.
    ///
    /// Cheap on a still camera: when this frame's view-projection matches the
    /// last distinct one there is no travel to reconstruct, and both passes
    /// are skipped outright rather than run to produce the identity.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame<'_>,
    ) {
        if frame.strength <= 0.0 {
            // Off. The camera pair is dropped rather than merely left alone,
            // so turning the setting back on mid-race does not smear the
            // first frame by however far the camera travelled while it was
            // off.
            self.camera = None;
            return;
        }
        Camera::observe(&mut self.camera, frame.view_projection, frame.stamp);
        let Some(camera) = self.camera else { return };
        if camera.previous == camera.current {
            return;
        }

        self.resize(device, frame.size);
        let Some(scratch) = &self.scratch else { return };

        let wanted = Constants::new(
            frame.view_projection,
            camera.previous,
            frame.viewport,
            frame.size,
            frame.strength,
            frame.focus,
        );
        if self.written != Some(wanted) {
            queue.write_buffer(&self.constants, 0, bytemuck::bytes_of(&wanted));
            self.written = Some(wanted);
        }

        let bind = |source: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("motion blur"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.constants.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(frame.depth),
                    },
                ],
            })
        };
        let gather_group = bind(frame.scene);
        let copy_group = bind(&scratch.view);

        let mut pass = |label: &str,
                        pipeline: &wgpu::RenderPipeline,
                        group: &wgpu::BindGroup,
                        target: &wgpu::TextureView| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Every texel is written by the fullscreen triangle,
                        // so there is nothing for a load to preserve.
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, Some(group), &[]);
            pass.draw(0..3, 0..1);
        };
        pass(
            "motion blur gather",
            &self.blur,
            &gather_group,
            &scratch.view,
        );
        pass("motion blur copy", &self.copy, &copy_group, frame.scene);
    }

    fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if self.size == size && self.scratch.is_some() {
            return;
        }
        self.size = size;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("motion blur scratch"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.scratch = Some(Scratch { texture, view });
    }
}

#[cfg(test)]
mod tests;
