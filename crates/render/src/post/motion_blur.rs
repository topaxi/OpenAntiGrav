//! Per-object motion blur: the reconstruction filter over the scene's own
//! velocity buffer, and the tier `docs/rendering/motion-blur.md` designed.
//!
//! This replaced the camera-reprojection gather that shipped first as the
//! stepping stone ([ADR-0028](../../../../docs/architecture/adr/0028-camera-motion-blur-first.md)):
//! the velocity buffer *measures* every draw's screen motion - `mesh.wgsl`'s
//! `velocity_of`, against the previous tick's premultiplied matrices - where
//! reprojection computed the camera's motion and was wrong about everything
//! that moves, craft first. The focus-mask workaround of ADR-0029 went with
//! it: a mask exempting known objects is unnecessary once motion is
//! measured. See ADR-0030 for the tier's own decisions.
//!
//! # The chain
//!
//! Five fullscreen passes, written from the published description of McGuire
//! et al., *A Reconstruction Filter for Plausible Motion Blur* (I3D 2012),
//! per [ADR-0012](../../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)'s
//! transliteration rule:
//!
//! 1. **prepare** - velocity and depth folded into one `Rgba16Float`, so
//!    every later pass reads one binding whatever the scene's sample count.
//!    The multisampled variant reads sample 0: averaging velocity across a
//!    silhouette edge produces a vector that describes neither surface.
//! 2. **tile-max** - the dominant velocity of each `K`-pixel tile, `K` being
//!    the blur's own pixel cap, which is what makes one tile of reach enough.
//! 3. **neighbour-max** - each tile takes its 3x3 neighbourhood's dominant
//!    velocity, so a sharp pixel beside a fast object still gathers along
//!    the object's path and receives its smear.
//! 4. **reconstruct** - the depth- and velocity-weighted gather itself.
//! 5. **copy** - the result back onto the scene target, which the gather
//!    cannot read and write at once.
//!
//! # An effect of this project's, not the original's
//!
//! Neither PSP build renders motion blur; this is an enhancement in the
//! class of FSR 1 and SMAA, off by default behind `[graphics] motion_blur`,
//! and it touches only how a finished frame is presented.

use anyhow::Result;

/// The cap on the gather's reach, as a fraction of the viewport height -
/// also the tile size the dominant-velocity reduction runs at.
///
/// The bound that keeps one wrong or extreme velocity - a camera cut, a
/// respawn - to a bounded smear rather than a whole-frame streak.
pub const MAX_STRETCH: f32 = 0.08;

/// The shader's uniform. `repr(C)`, 48 bytes, no implicit padding.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Constants {
    rect_offset: [f32; 2],
    rect_size: [f32; 2],
    rect_pixels: [f32; 2],
    strength: f32,
    max_px: f32,
    inv_size: [f32; 2],
    tile: u32,
    _pad: f32,
}

impl Constants {
    /// `viewport` is the scene's rectangle inside the target - `(x, y, w, h)`
    /// in pixels, the same tuple `Scene::render` takes - and `size` the whole
    /// target, both needed because a capture draws the scene into a
    /// sub-rectangle with the aspect bars outside it.
    fn new(viewport: (f32, f32, f32, f32), size: (u32, u32), strength: f32, tile: u32) -> Self {
        let (w, h) = (size.0.max(1) as f32, size.1.max(1) as f32);
        Self {
            rect_offset: [viewport.0 / w, viewport.1 / h],
            rect_size: [viewport.2.max(1.0) / w, viewport.3.max(1.0) / h],
            rect_pixels: [viewport.2.max(1.0), viewport.3.max(1.0)],
            strength,
            max_px: max_px(viewport),
            inv_size: [1.0 / w, 1.0 / h],
            tile,
            _pad: 0.0,
        }
    }
}

/// The gather's reach cap in pixels for a viewport, which is also the tile
/// edge: [`MAX_STRETCH`] of the viewport height, floored so a tiny viewport
/// cannot degenerate the tile reduction.
fn max_px(viewport: (f32, f32, f32, f32)) -> f32 {
    (MAX_STRETCH * viewport.3).max(8.0)
}

/// One frame's worth of input to [`MotionBlur::render`].
#[derive(Debug)]
pub struct Frame<'a> {
    /// The finished frame, sampled and then overwritten - legal because the
    /// gather lands in a scratch target and the copy pass carries it back,
    /// the same read-then-write shape `bloom` uses.
    pub scene: &'a wgpu::TextureView,
    /// The scene's velocity attachment - [`crate::mesh_render::VELOCITY_FORMAT`],
    /// at the scene's own sample count.
    pub velocity: &'a wgpu::TextureView,
    /// The scene's depth attachment, at the same sample count.
    pub depth: &'a wgpu::TextureView,
    /// The sample count `velocity` and `depth` carry: 1, or MSAA's 4, which
    /// routes the prepare pass through its sample-0 variant.
    pub sample_count: u32,
    /// The size of every attachment above, in pixels.
    pub size: (u32, u32),
    /// The scene's rectangle inside them: `(x, y, w, h)` in pixels, the same
    /// tuple `Scene::render` draws with.
    pub viewport: (f32, f32, f32, f32),
    /// What fraction of the tick's measured motion the gather spans - the
    /// shutter, `0.5` being the film convention. At zero or below the pass
    /// does nothing at all, which is how "off" reaches it: the caller's
    /// setting is a strength, read fresh every frame.
    pub strength: f32,
}

/// A render-and-sample scratch target.
#[derive(Debug)]
struct Target {
    #[expect(dead_code, reason = "held so the view stays valid")]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Target {
    fn new(
        device: &wgpu::Device,
        label: &str,
        format: wgpu::TextureFormat,
        (width, height): (u32, u32),
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view }
    }
}

/// Velocity and depth folded together - see the module docs' step 1.
const PREPARED_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// The tile reductions' format: one velocity per tile.
const TILE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;

/// The five pipelines, their targets, and the uniform they share.
#[derive(Debug)]
pub struct MotionBlur {
    prepare: wgpu::RenderPipeline,
    prepare_ms: wgpu::RenderPipeline,
    tile_max: wgpu::RenderPipeline,
    neighbour_max: wgpu::RenderPipeline,
    reconstruct: wgpu::RenderPipeline,
    copy: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    prepare_layout: wgpu::BindGroupLayout,
    prepare_ms_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    constants: wgpu::Buffer,
    written: Option<Constants>,
    prepared: Option<Target>,
    tile_a: Option<Target>,
    tile_b: Option<Target>,
    scratch: Option<Target>,
    format: wgpu::TextureFormat,
    size: (u32, u32),
    tile: u32,
}

/// An unfilterable-float texture at `binding` - what `textureLoad` wants,
/// and what a depth-format view may bind as.
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

impl MotionBlur {
    /// Builds the pipelines against the scene target's `format`.
    ///
    /// The targets are built at the first [`render`](Self::render), which is
    /// the first time a size is known.
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

        // Group 0, shared by every pass past prepare: the colour source, its
        // sampler, the uniform, the prepared velocity+depth, and a tile
        // texture. Passes that do not read a slot still bind something valid
        // there - one layout is simpler than four near-copies, and an unused
        // binding costs nothing.
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("motion blur"),
            entries: &[
                super::texture_entry(0),
                super::sampler_entry(1),
                super::uniform_entry(2),
                load_entry(3, false),
                load_entry(4, false),
            ],
        });
        // Group 1, the prepare pass's own: the scene's raw velocity and
        // depth, in a single-sampled and a multisampled variant.
        let prepare_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("motion blur prepare"),
            entries: &[load_entry(0, false), load_entry(1, false)],
        });
        let prepare_ms_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("motion blur prepare (msaa)"),
            entries: &[load_entry(2, true), load_entry(3, true)],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("motion blur"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let format = format.remove_srgb_suffix();
        let chain_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("motion blur"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let prepare_pipe_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("motion blur prepare"),
            bind_group_layouts: &[Some(&layout), Some(&prepare_layout)],
            immediate_size: 0,
        });
        let prepare_ms_pipe_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("motion blur prepare (msaa)"),
                bind_group_layouts: &[Some(&layout), Some(&prepare_ms_layout)],
                immediate_size: 0,
            });

        let pipeline = |label: &str,
                        entry: &str,
                        pipeline_layout: &wgpu::PipelineLayout,
                        target: wgpu::TextureFormat| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(pipeline_layout),
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
                        format: target,
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
        let prepare = pipeline(
            "motion blur prepare",
            "fs_prepare",
            &prepare_pipe_layout,
            PREPARED_FORMAT,
        );
        let prepare_ms = pipeline(
            "motion blur prepare (msaa)",
            "fs_prepare_ms",
            &prepare_ms_pipe_layout,
            PREPARED_FORMAT,
        );
        let tile_max = pipeline(
            "motion blur tile-max",
            "fs_tile_max",
            &chain_layout,
            TILE_FORMAT,
        );
        let neighbour_max = pipeline(
            "motion blur neighbour-max",
            "fs_neighbour_max",
            &chain_layout,
            TILE_FORMAT,
        );
        let reconstruct = pipeline(
            "motion blur reconstruct",
            "fs_reconstruct",
            &chain_layout,
            format,
        );
        let copy = pipeline("motion blur copy", "fs_copy", &chain_layout, format);

        let constants = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("motion blur constants"),
            size: size_of::<Constants>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            prepare,
            prepare_ms,
            tile_max,
            neighbour_max,
            reconstruct,
            copy,
            layout,
            prepare_layout,
            prepare_ms_layout,
            sampler,
            constants,
            written: None,
            prepared: None,
            tile_a: None,
            tile_b: None,
            scratch: None,
            format,
            size: (0, 0),
            tile: 0,
        })
    }

    /// Runs the chain and carries the result back onto `frame.scene`.
    ///
    /// Stateless across frames: the velocity buffer already encodes the
    /// previous tick, so unlike the camera-reprojection pass this replaced
    /// there is no camera pair to hold and nothing to reset on a cut.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame<'_>,
    ) {
        if frame.strength <= 0.0 {
            return;
        }
        let tile = max_px(frame.viewport).ceil() as u32;
        self.resize(device, frame.size, tile);
        let (Some(prepared), Some(tile_a), Some(tile_b), Some(scratch)) =
            (&self.prepared, &self.tile_a, &self.tile_b, &self.scratch)
        else {
            return;
        };

        let wanted = Constants::new(frame.viewport, frame.size, frame.strength, tile);
        if self.written != Some(wanted) {
            queue.write_buffer(&self.constants, 0, bytemuck::bytes_of(&wanted));
            self.written = Some(wanted);
        }

        // Group 0 for each pass: only the slots a pass reads matter, but a
        // bound texture must not also be that pass's render target - usage
        // scopes are validated for what is *bound*, not what the shader
        // statically reads - so each pass names all three texture slots
        // explicitly with views that are not its own target.
        let group = |label: &str,
                     colour: &wgpu::TextureView,
                     prepared_view: &wgpu::TextureView,
                     tile_view: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(colour),
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
                        resource: wgpu::BindingResource::TextureView(prepared_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(tile_view),
                    },
                ],
            })
        };

        let mut pass = |label: &str,
                        pipeline: &wgpu::RenderPipeline,
                        groups: &[&wgpu::BindGroup],
                        target: &wgpu::TextureView| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
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
            pass.set_pipeline(pipeline);
            for (index, group) in groups.iter().enumerate() {
                pass.set_bind_group(index as u32, Some(*group), &[]);
            }
            pass.draw(0..3, 0..1);
        };

        // 1. prepare: raw velocity + depth into one single-sampled texture.
        let (prepare_pipeline, prepare_group) = if frame.sample_count > 1 {
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("motion blur prepare (msaa)"),
                layout: &self.prepare_ms_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(frame.velocity),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(frame.depth),
                    },
                ],
            });
            (&self.prepare_ms, group)
        } else {
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("motion blur prepare"),
                layout: &self.prepare_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(frame.velocity),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(frame.depth),
                    },
                ],
            });
            (&self.prepare, group)
        };
        // Prepare does not read group 0, but the pipeline layout carries it
        // so the WGSL's shared declarations resolve; bind harmless views
        // that are not its own target.
        let idle = group(
            "motion blur prepare g0",
            &tile_b.view,
            &tile_b.view,
            &tile_a.view,
        );
        pass(
            "motion blur prepare",
            prepare_pipeline,
            &[&idle, &prepare_group],
            &prepared.view,
        );
        // 2 and 3: the two tile reductions.
        let tiles = group(
            "motion blur tile-max",
            &prepared.view,
            &prepared.view,
            &tile_b.view,
        );
        pass(
            "motion blur tile-max",
            &self.tile_max,
            &[&tiles],
            &tile_a.view,
        );
        let spread = group(
            "motion blur neighbour-max",
            &prepared.view,
            &prepared.view,
            &tile_a.view,
        );
        pass(
            "motion blur neighbour-max",
            &self.neighbour_max,
            &[&spread],
            &tile_b.view,
        );
        // 4: the gather, into scratch.
        let gather = group(
            "motion blur reconstruct",
            frame.scene,
            &prepared.view,
            &tile_b.view,
        );
        pass(
            "motion blur reconstruct",
            &self.reconstruct,
            &[&gather],
            &scratch.view,
        );
        // 5: home.
        let home = group(
            "motion blur copy",
            &scratch.view,
            &prepared.view,
            &tile_a.view,
        );
        pass("motion blur copy", &self.copy, &[&home], frame.scene);
    }

    fn resize(&mut self, device: &wgpu::Device, size: (u32, u32), tile: u32) {
        let size = (size.0.max(1), size.1.max(1));
        if self.size == size && self.tile == tile && self.scratch.is_some() {
            return;
        }
        self.size = size;
        self.tile = tile;
        let tiles = (size.0.div_ceil(tile.max(1)), size.1.div_ceil(tile.max(1)));
        self.prepared = Some(Target::new(
            device,
            "motion blur prepared",
            PREPARED_FORMAT,
            size,
        ));
        self.tile_a = Some(Target::new(
            device,
            "motion blur tile a",
            TILE_FORMAT,
            tiles,
        ));
        self.tile_b = Some(Target::new(
            device,
            "motion blur tile b",
            TILE_FORMAT,
            tiles,
        ));
        self.scratch = Some(Target::new(
            device,
            "motion blur scratch",
            self.format,
            size,
        ));
    }
}

#[cfg(test)]
mod tests;
