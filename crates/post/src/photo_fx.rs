//! Photo mode's picture controls: exposure, saturation and depth of field on a finished frame.
//!
//! The frame is the caller's view after every other pass, motion blur included, and it is
//! overwritten in place: the effect lands in a scratch target of the same size and a copy pass
//! carries it home (the read-then-write shape [`super::motion_blur`] uses). Nothing is built or
//! allocated until the first [`PhotoFx::render`], and a caller that has nothing to apply does
//! not call it, so a frame with the mode closed costs nothing.
//!
//! # The laws
//!
//! Depth of field is HD/Fury's focus curve (`docs/ghidra/functions/ps3-hdfury-eu/photo-mode.md`,
//! "Depth of field", 84; 2048's own pad code carries the same formula): focus `f` in `0..=1`
//! gives the focus distance `d` in `3..=493`, and the picture is sharp from `d - c` to
//! `d + 2 c` with `c = d^0.55`, blurring to its widest at `d - (c + e)` and `d + 2 (c + e)`
//! with `e = c^1.7`. **Chosen, not measured:** the aperture's reach (the widest blur radius,
//! [`Dof::MAX_RADIUS`] of the picture's height at full aperture), the gather itself, and the
//! exposure: [`Grade::exposure`] is a gain of `2^ev` on linear light, HD's step
//! ([`Grade::EV_PER_STEP`]) read as stops, because the shader that applies HD's own value was
//! never read.

use anyhow::Result;
use oag_gpu::writes::Writes;

/// The wanted focus plane, from HD's focus slider.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edges {
    /// The distance the focus slider names, in world units.
    pub distance: f32,
    /// Where the blur is at its widest in front of the focus.
    pub near_edge: f32,
    /// Where the sharp band begins.
    pub near_focus: f32,
    /// Where the sharp band ends.
    pub far_focus: f32,
    /// Where the blur is at its widest behind the focus.
    pub far_edge: f32,
}

impl Edges {
    /// HD's curve for a focus `f` in `0..=1`: `d` runs `3` to `493`.
    #[must_use]
    pub fn new(focus: f32) -> Self {
        let f = focus.clamp(0.0, 1.0);
        let distance = if f <= 0.5 {
            40.0 * f + 3.0
        } else {
            let u = 2.0 * f - 1.0;
            u * (450.0 * u * u + 40.0 * f) + 40.0 * f * (1.0 - u) + 3.0
        };
        let c = distance.powf(0.55);
        let e = c.powf(1.7);
        Self {
            distance,
            near_edge: distance - (c + e),
            near_focus: distance - c,
            far_focus: distance + 2.0 * c,
            far_edge: distance + 2.0 * (c + e),
        }
    }
}

/// Depth of field: where the focus is and how wide the blur gets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dof {
    /// The focus slider, `0..=1`.
    pub focus: f32,
    /// The aperture slider, `0..=1`.
    pub aperture: f32,
}

impl Dof {
    /// The widest blur radius at full aperture, as a fraction of the picture's height.
    pub const MAX_RADIUS: f32 = 0.025;
}

/// Exposure and saturation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grade {
    /// Stops of exposure, `0` for none.
    pub exposure: f32,
    /// The saturation: `1` leaves the colour as it is, `0` is grey.
    pub saturation: f32,
}

impl Grade {
    /// HD's step of the exposure slider, read as stops.
    pub const EV_PER_STEP: f32 = 0.030_03;

    /// Whether this changes nothing.
    #[must_use]
    pub fn is_neutral(&self) -> bool {
        self.exposure == 0.0 && self.saturation == 1.0
    }
}

/// What to apply to a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// Exposure and saturation, or `None` for the colour as it is.
    pub grade: Option<Grade>,
    /// Depth of field, or `None` for a sharp picture.
    pub dof: Option<Dof>,
    /// The projection's near plane, in world units.
    pub near: f32,
    /// The projection's far plane, in world units.
    pub far: f32,
}

impl Params {
    /// Whether there is anything to apply.
    #[must_use]
    pub fn active(&self) -> bool {
        self.dof.is_some() || self.grade.is_some_and(|g| !g.is_neutral())
    }
}

/// One frame's worth of input to [`PhotoFx::render`].
#[derive(Debug)]
pub struct Frame<'a> {
    /// The finished frame, read and then overwritten.
    pub scene: &'a wgpu::TextureView,
    /// The scene's depth attachment (any single-channel float view loads the same).
    pub depth: &'a wgpu::TextureView,
    /// The sample count `depth` carries; sample 0 is read, as the motion blur's prepare does.
    pub sample_count: u32,
    /// The size of `scene`'s resource, in pixels.
    pub size: (u32, u32),
    /// The rectangle of it that was drawn: `(x, y, w, h)` in pixels.
    pub viewport: (f32, f32, f32, f32),
    /// What to apply.
    pub params: Params,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Constants {
    rect: [i32; 4],
    depth: [f32; 4],
    edges: [f32; 4],
    grade: [f32; 4],
}

impl Constants {
    fn new(viewport: (f32, f32, f32, f32), size: (u32, u32), params: &Params) -> Self {
        let x0 = viewport.0.max(0.0) as i32;
        let y0 = viewport.1.max(0.0) as i32;
        let x1 = ((viewport.0 + viewport.2) as i32 - 1).clamp(x0, size.0.max(1) as i32 - 1);
        let y1 = ((viewport.1 + viewport.3) as i32 - 1).clamp(y0, size.1.max(1) as i32 - 1);
        let (dof, edges, radius) = match params.dof {
            Some(dof) => {
                let e = Edges::new(dof.focus);
                let radius = dof.aperture.clamp(0.0, 1.0) * Dof::MAX_RADIUS * viewport.3;
                (
                    1.0,
                    [e.near_edge, e.near_focus, e.far_focus, e.far_edge],
                    radius,
                )
            }
            None => (0.0, [0.0; 4], 0.0),
        };
        let (grade, gain, saturation) = match params.grade {
            Some(g) if !g.is_neutral() => (1.0, g.exposure.exp2(), g.saturation.max(0.0)),
            _ => (0.0, 1.0, 1.0),
        };
        Self {
            rect: [x0, y0, x1, y1],
            depth: [params.near, params.far, radius, dof],
            edges,
            grade: [gain, saturation, 0.0, grade],
        }
    }
}

/// The pipelines and the scratch target.
#[derive(Debug)]
pub struct PhotoFx {
    main: wgpu::RenderPipeline,
    main_ms: wgpu::RenderPipeline,
    copy: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    layout_ms: wgpu::BindGroupLayout,
    copy_layout: wgpu::BindGroupLayout,
    constants: wgpu::Buffer,
    scratch: Option<wgpu::TextureView>,
    format: wgpu::TextureFormat,
    size: (u32, u32),
}

fn load_entry(binding: u32, multisampled: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled,
        },
        count: None,
    }
}

impl PhotoFx {
    /// Builds the pipelines against the scene view's `format`. The scratch target waits for the
    /// first [`render`](Self::render).
    ///
    /// # Errors
    ///
    /// Propagates a shader that will not compile, a build-time mistake.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Result<Self> {
        let module = |label: &str, source: &str| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
        };
        let single = module(
            "photo fx",
            include_str!(concat!(env!("OUT_DIR"), "/photo_fx.wgsl")),
        );
        let multi = module(
            "photo fx (msaa)",
            include_str!(concat!(env!("OUT_DIR"), "/photo_fx_msaa.wgsl")),
        );
        let layout_of = |label: &str, multisampled: bool| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries: &[
                    load_entry(0, false),
                    super::uniform_entry(1),
                    load_entry(2, multisampled),
                ],
            })
        };
        let layout = layout_of("photo fx", false);
        let layout_ms = layout_of("photo fx (msaa)", true);
        let copy_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("photo fx copy"),
            entries: &[load_entry(0, false)],
        });
        let target_format = format.remove_srgb_suffix();
        let pipeline = |label: &str,
                        module: &wgpu::ShaderModule,
                        entry: &str,
                        layout: &wgpu::BindGroupLayout| {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[Some(layout)],
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_format,
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
        let constants = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("photo fx constants"),
            size: size_of::<Constants>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Self {
            main: pipeline("photo fx", &single, "fs_main", &layout),
            main_ms: pipeline("photo fx (msaa)", &multi, "fs_main", &layout_ms),
            copy: pipeline("photo fx copy", &single, "fs_copy", &copy_layout),
            layout,
            layout_ms,
            copy_layout,
            constants,
            scratch: None,
            format: target_format,
            size: (0, 0),
        })
    }

    /// Applies `frame.params` to `frame.scene`. Returns whether anything was encoded: nothing is
    /// when there is nothing to apply.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &dyn Writes,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame<'_>,
    ) -> bool {
        if !frame.params.active() {
            return false;
        }
        self.resize(device, frame.size);
        let Some(scratch) = &self.scratch else {
            return false;
        };
        let constants = Constants::new(frame.viewport, frame.size, &frame.params);
        queue.write_buffer(&self.constants, 0, bytemuck::bytes_of(&constants));
        let multisampled = frame.sample_count > 1;
        let (pipeline, layout) = if multisampled {
            (&self.main_ms, &self.layout_ms)
        } else {
            (&self.main, &self.layout)
        };
        let group = oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
                label: Some("photo fx"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(frame.scene),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.constants.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(frame.depth),
                    },
                ],
            },
        );
        let home = oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
                label: Some("photo fx copy"),
                layout: &self.copy_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(scratch),
                }],
            },
        );
        let [x0, y0, x1, y1] = constants.rect;
        let rect = (
            x0 as f32,
            y0 as f32,
            (x1 - x0 + 1) as f32,
            (y1 - y0 + 1) as f32,
        );
        let mut pass = |label: &'static str,
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
                        // The caller's target keeps what lies outside the drawn rectangle.
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_viewport(rect.0, rect.1, rect.2, rect.3, 0.0, 1.0);
            pass.set_scissor_rect(rect.0 as u32, rect.1 as u32, rect.2 as u32, rect.3 as u32);
            pass.set_bind_group(0, Some(group), &[]);
            pass.draw(0..3, 0..1);
        };
        pass("photo fx", pipeline, &group, scratch);
        pass("photo fx copy", &self.copy, &home, frame.scene);
        true
    }

    fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if self.size == size && self.scratch.is_some() {
            return;
        }
        self.size = size;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("photo fx scratch"),
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
        self.scratch = Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));
    }
}

#[cfg(test)]
mod tests;
