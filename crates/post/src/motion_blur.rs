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
//! Six fullscreen passes, written from the published description of McGuire
//! et al., *A Reconstruction Filter for Plausible Motion Blur* (I3D 2012),
//! per [ADR-0012](../../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)'s
//! transliteration rule:
//!
//! 1. **prepare** - velocity and depth folded into one `Rgba16Float`, so
//!    every later pass reads one binding whatever the scene's sample count.
//!    The multisampled variant reads sample 0: averaging velocity across a
//!    silhouette edge produces a vector that describes neither surface.
//! 2. **tile-max, horizontally** - each run of `K` pixels reduced to its
//!    dominant velocity, `K` being the blur's own pixel cap, which is what
//!    makes one tile of reach enough.
//! 3. **tile-max, vertically** - the same down the columns, landing on the
//!    tile grid. Two passes rather than one square reduction because the
//!    square form leaves the GPU almost idle - see `fs_tile_max_x` in the
//!    shader for the measurement.
//! 4. **neighbour-max** - each tile takes its 3x3 neighbourhood's dominant
//!    velocity, so a sharp pixel beside a fast object still gathers along
//!    the object's path and receives its smear.
//! 5. **reconstruct** - the depth- and velocity-weighted gather itself.
//! 6. **copy** - the result back onto the scene target, which the gather
//!    cannot read and write at once.
//!
//! **All six rasterise the render extent, not the allocation.** The scene
//! target is allocated at the render-scale ceiling and dynamic resolution
//! draws into a sub-rectangle of it, so a fullscreen triangle over the whole
//! target pays the ceiling's price at every extent - invisibly, because the
//! picture is identical either way. [`MotionBlur::render`] sets a viewport
//! per pass; the load operations are the part that cannot follow, and the
//! comment there is the one to read before changing either.
//!
//! [ADR-0030](../../../../docs/architecture/adr/0030-velocity-buffer-motion-blur.md)
//! counts the chain as five passes and four scratch targets, which is what
//! landed with it; the separable tile-max made it six and five. ADRs are
//! immutable, so the current count lives here and in
//! `docs/rendering/motion-blur.md`.
//!
//! # An effect of this project's, not the original's
//!
//! Neither PSP build renders motion blur; this is an enhancement in the
//! class of FSR 1 and SMAA, off by default behind `[graphics] motion_blur`,
//! and it touches only how a finished frame is presented.

use anyhow::Result;
use oag_core::math::Mat4;

/// The cap on the gather's reach, as a fraction of the viewport height -
/// also the tile size the dominant-velocity reduction runs at.
///
/// The bound that keeps one wrong or extreme velocity - a camera cut, a
/// respawn - to a bounded smear rather than a whole-frame streak.
pub const MAX_STRETCH: f32 = 0.08;

/// The shader's uniform. `repr(C)`, 112 bytes, no implicit padding.
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
    /// [`Frame::camera_shake`], column-major.
    shake: [[f32; 4]; 4],
}

impl Constants {
    /// `viewport` is the scene's rectangle inside the target - `(x, y, w, h)`
    /// in pixels, the same tuple `Scene::render` takes - and `size` the whole
    /// target, both needed because a capture draws the scene into a
    /// sub-rectangle with the aspect bars outside it.
    fn new(
        viewport: (f32, f32, f32, f32),
        size: (u32, u32),
        strength: f32,
        tile: u32,
        shake: Mat4,
    ) -> Self {
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
            shake: shake.to_cols_array_2d(),
        }
    }
}

/// The gather's reach cap in pixels for a viewport, which is also the tile
/// edge: [`MAX_STRETCH`] of the viewport height, floored so a tiny viewport
/// cannot degenerate the tile reduction.
fn max_px(viewport: (f32, f32, f32, f32)) -> f32 {
    (MAX_STRETCH * viewport.3).max(8.0)
}

/// `viewport` in the space of a target whose texels each cover `by` texels of
/// the full-size ones: the offset floors and the far edge ceils, so the
/// rectangle covers every reduced texel the original touches and the
/// reduction can never come up short at its own boundary.
fn reduced(viewport: (f32, f32, f32, f32), by: (f32, f32)) -> (f32, f32, f32, f32) {
    let x = (viewport.0 / by.0).floor();
    let y = (viewport.1 / by.1).floor();
    (
        x,
        y,
        ((viewport.0 + viewport.2) / by.0).ceil() - x,
        ((viewport.1 + viewport.3) / by.1).ceil() - y,
    )
}

/// `viewport` clamped inside `target`, because `set_viewport` validates that
/// the rectangle lies within the attachment and [`reduced`]'s ceiling can
/// otherwise land one texel past a target whose size did not divide evenly.
fn clamped(viewport: (f32, f32, f32, f32), target: (u32, u32)) -> (f32, f32, f32, f32) {
    let (w, h) = (target.0.max(1) as f32, target.1.max(1) as f32);
    let x = viewport.0.clamp(0.0, w - 1.0);
    let y = viewport.1.clamp(0.0, h - 1.0);
    (
        x,
        y,
        viewport.2.clamp(1.0, w - x),
        viewport.3.clamp(1.0, h - y),
    )
}

/// One timestamp pair, split across the first and last pass of a chain.
///
/// Two descriptors rather than one because a pair spans six render passes here
/// and wgpu writes a timestamp per *pass*: `beginning_of_pass_write_index` and
/// `end_of_pass_write_index` are independently optional, so the opening index
/// rides the first pass and the closing one the last. See
/// [`oag_gpu::timing::PassTimer::half_writes`], which builds both halves from one
/// claimed slot.
#[derive(Debug)]
pub struct ChainTimestamps<'a> {
    /// Opening timestamp only, for the chain's first pass.
    pub begin: wgpu::RenderPassTimestampWrites<'a>,
    /// Closing timestamp only, for the chain's last pass.
    pub end: wgpu::RenderPassTimestampWrites<'a>,
}

/// One frame's worth of input to [`MotionBlur::render`].
#[derive(Debug)]
pub struct Frame<'a> {
    /// The finished frame, sampled and then overwritten - legal because the
    /// gather lands in a scratch target and the copy pass carries it back,
    /// the same read-then-write shape `bloom` uses.
    pub scene: &'a wgpu::TextureView,
    /// The scene's velocity attachment - [`oag_gpu::formats::VELOCITY_FORMAT`],
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
    /// How far the camera's impact shake moved the picture since the previous
    /// tick, as a clip-space map: a pixel's clip position goes to where the
    /// same direction sat a tick ago under the shake alone. The prepare pass
    /// subtracts that motion from the velocity it reads, so the blur smears
    /// what really moved and not the whole frame the shake turned - the
    /// velocity buffer itself stays true screen motion, which the temporal
    /// upscaler needs. [`Mat4::IDENTITY`] for none, and then exactly nothing
    /// is subtracted.
    ///
    /// A rotation about the eye moves a pixel by an amount that depends on its
    /// screen position and not its depth, which is why one matrix will do.
    pub camera_shake: Mat4,
}

/// Velocity and depth folded together - see the module docs' step 1.
const PREPARED_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// The tile reductions' format: one velocity per tile.
const TILE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;

/// The half-size gather's format: a premultiplied blend over the scene, kept
/// float so the smear's colour keeps its range on HD's linear target - see
/// `fs_reconstruct`.
const GATHERED_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// The gather target's size for a full-size `size`: half each way, rounded up
/// so it covers every full-size pixel.
fn half(size: (u32, u32)) -> (u32, u32) {
    (size.0.div_ceil(2).max(1), size.1.div_ceil(2).max(1))
}

/// The seven bind groups the chain binds, and what they were built against.
///
/// Rebuilt when - and only when - one of the things inside them moves: the
/// caller's three attachment views (`attachments`), this pass's own scratch
/// targets (`size`, `tile`) or which prepare variant is in use
/// (`sample_count`). Everything else the groups name is owned here and
/// outlives any frame.
///
/// A bind group is a driver object, and the chain needs seven of them;
/// building them every frame was 194 heap allocations a frame on its own,
/// measured on Talon's Junction - about half of everything
/// `race::Scene::render` allocated.
///
/// **The three caller views are held by value, and that is what makes the
/// comparison sound rather than merely cheap.** `wgpu::TextureView` compares
/// by the identity of its inner handle, so two *different* views could in
/// principle land on the same address once the first is freed - but a clone
/// kept here keeps the first alive, so an address this cache still holds can
/// never be handed to anything else. Nothing outside this struct has to
/// promise anything about when it rebuilds its attachments.
#[derive(Debug)]
struct Groups {
    key: Key,
    prepare: wgpu::BindGroup,
    idle: wgpu::BindGroup,
    rows: wgpu::BindGroup,
    columns: wgpu::BindGroup,
    spread: wgpu::BindGroup,
    gather: wgpu::BindGroup,
    merge: wgpu::BindGroup,
    home: wgpu::BindGroup,
    /// Which prepare pipeline `prepare` was built for; the MSAA variant has
    /// its own layout.
    multisampled: bool,
}

/// Everything a [`Groups`] was built against: the caller's three views, this
/// pass's own target geometry, and the sample count that picks the prepare
/// variant.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Key {
    scene: wgpu::TextureView,
    velocity: wgpu::TextureView,
    depth: wgpu::TextureView,
    size: (u32, u32),
    tile: u32,
    sample_count: u32,
}

/// The six pipelines, their targets, and the uniform they share.
#[derive(Debug)]
pub struct MotionBlur {
    prepare: wgpu::RenderPipeline,
    prepare_ms: wgpu::RenderPipeline,
    tile_max_x: wgpu::RenderPipeline,
    tile_max_y: wgpu::RenderPipeline,
    neighbour_max: wgpu::RenderPipeline,
    reconstruct: wgpu::RenderPipeline,
    reconstruct_half: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    /// Whether the gather runs at half resolution - see
    /// [`Self::set_half_resolution`].
    half: bool,
    copy: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    prepare_layout: wgpu::BindGroupLayout,
    prepare_ms_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    constants: wgpu::Buffer,
    written: Option<Constants>,
    prepared: Option<Target>,
    /// The horizontal tile reduction's output: `ceil(w / tile)` by the full
    /// height, the one target here that is neither full-size nor fully
    /// tiled.
    tile_rows: Option<Target>,
    tile_a: Option<Target>,
    tile_b: Option<Target>,
    /// The half-size gather's output - see [`GATHERED_FORMAT`].
    gathered: Option<Target>,
    scratch: Option<Target>,
    /// The chain's bind groups, kept across frames - see [`Groups`].
    groups: Option<Groups>,
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
        let tile_max_x = pipeline(
            "motion blur tile-max (x)",
            "fs_tile_max_x",
            &chain_layout,
            TILE_FORMAT,
        );
        let tile_max_y = pipeline(
            "motion blur tile-max (y)",
            "fs_tile_max_y",
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
        let reconstruct_half = pipeline(
            "motion blur reconstruct (half)",
            "fs_reconstruct_half",
            &chain_layout,
            GATHERED_FORMAT,
        );
        let composite = pipeline(
            "motion blur composite",
            "fs_composite",
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
            tile_max_x,
            tile_max_y,
            neighbour_max,
            reconstruct,
            reconstruct_half,
            composite,
            half: false,
            copy,
            layout,
            prepare_layout,
            prepare_ms_layout,
            sampler,
            constants,
            written: None,
            prepared: None,
            tile_rows: None,
            tile_a: None,
            tile_b: None,
            gathered: None,
            scratch: None,
            groups: None,
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
    /// Gathers at half resolution from the next [`Self::render`] on, or at
    /// full resolution again. Full is the default.
    ///
    /// Half gathers a quarter of the pixels and blends the smear back over
    /// the full-size frame by depth - `fs_reconstruct_half` and
    /// `fs_composite`. Measured on HD on an integrated GPU, the chain 15.4 ->
    /// 4.4 ms at 1600x900 and 59.3 -> 19.4 ms at 3200x1800; the smear reads
    /// grainier, which is why it is a choice rather than the default.
    pub fn set_half_resolution(&mut self, half: bool) {
        self.half = half;
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame<'_>,
        timestamps: Option<ChainTimestamps<'_>>,
    ) -> bool {
        if frame.strength <= 0.0 {
            return false;
        }
        let tile = max_px(frame.viewport).ceil() as u32;
        self.resize(device, frame.size, tile);
        let (
            Some(prepared),
            Some(tile_rows),
            Some(tile_a),
            Some(tile_b),
            Some(gathered),
            Some(scratch),
        ) = (
            &self.prepared,
            &self.tile_rows,
            &self.tile_a,
            &self.tile_b,
            &self.gathered,
            &self.scratch,
        )
        else {
            return false;
        };

        let wanted = Constants::new(
            frame.viewport,
            frame.size,
            frame.strength,
            tile,
            frame.camera_shake,
        );
        if self.written != Some(wanted) {
            queue.write_buffer(&self.constants, 0, bytemuck::bytes_of(&wanted));
            self.written = Some(wanted);
        }

        let key = Key {
            scene: frame.scene.clone(),
            velocity: frame.velocity.clone(),
            depth: frame.depth.clone(),
            size: frame.size,
            tile,
            sample_count: frame.sample_count,
        };
        if self.groups.as_ref().is_none_or(|groups| groups.key != key) {
            self.groups = Some(self.build_groups(device, frame, key));
        }
        let Some(groups) = &self.groups else {
            return false;
        };

        // **The two halves are split across the first and last pass**, which is
        // what makes one reading cover the whole six-pass chain without
        // `TIMESTAMP_QUERY_INSIDE_ENCODERS` - see `timing::PassTimer::half_writes`.
        // Destructured here rather than carried into the closure so the
        // borrow ends before `encoder` is used again.
        let (mut opening, mut closing) = match timestamps {
            Some(pair) => (Some(pair.begin), Some(pair.end)),
            None => (None, None),
        };

        // **Every pass rasterises the render extent, not the allocation.**
        // Since [ADR-0037] the scene target is allocated at the render-scale
        // ceiling and dynamic resolution draws into a sub-rectangle of it, so
        // a full-screen triangle over the whole target pays the ceiling's
        // price at every extent - six times over, and invisibly, because the
        // picture inside the rectangle is identical either way. `drs::Cost`
        // counts this chain as *scalable* - "what falls when the render
        // extent does" - and that is only true once the raster falls with it.
        //
        // [ADR-0037]: ../../../../docs/architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md
        let tiles = (
            frame.size.0.max(1).div_ceil(tile.max(1)),
            frame.size.1.max(1).div_ceil(tile.max(1)),
        );
        let t = tile.max(1) as f32;
        let full_rect = clamped(frame.viewport, frame.size);
        let rows_rect = clamped(reduced(frame.viewport, (t, 1.0)), (tiles.0, frame.size.1));
        let grid_rect = clamped(reduced(frame.viewport, (t, t)), tiles);
        let half_rect = clamped(reduced(frame.viewport, (2.0, 2.0)), half(frame.size));

        let mut pass =
            |label: &'static str,
             pipeline: &wgpu::RenderPipeline,
             groups: &[&wgpu::BindGroup],
             target: &wgpu::TextureView,
             rect: (f32, f32, f32, f32),
             load: wgpu::LoadOp<wgpu::Color>,
             timestamp_writes: Option<wgpu::RenderPassTimestampWrites<'_>>| {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(label),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                oag_gpu::perfprobe::marks::mark(&mut pass, label);
                pass.set_pipeline(pipeline);
                // **The scissor as well as the viewport**, which is belt and
                // braces rather than duplication: a viewport is a transform
                // and it is the triangle's own bounds that keep the fragments
                // inside it, where a scissor is the clip that says so. Both
                // rectangles are the same and integral by construction - see
                // `reduced`, which floors and ceils.
                pass.set_viewport(rect.0, rect.1, rect.2, rect.3, 0.0, 1.0);
                pass.set_scissor_rect(rect.0 as u32, rect.1 as u32, rect.2 as u32, rect.3 as u32);
                for (index, group) in groups.iter().enumerate() {
                    pass.set_bind_group(index as u32, Some(*group), &[]);
                }
                pass.draw(0..3, 0..1);
                if label == "motion blur copy" {
                    oag_gpu::perfprobe::marks::mark(&mut pass, "end");
                } else {
                    oag_gpu::perfprobe::marks::close(&mut pass);
                }
            };

        let prepare_pipeline = if groups.multisampled {
            &self.prepare_ms
        } else {
            &self.prepare
        };
        // **A load operation has no sub-rectangle**, so the four targets a
        // later pass reads outside the rectangle are still *cleared* whole:
        // that is what keeps the texels the raster no longer covers zero
        // rather than stale, and a tile reduction overlapping the boundary
        // then reduces over a velocity of nothing. The two that are not:
        // `scratch`, which only the viewport-restricted copy reads, and
        // `frame.scene`, which belongs to the caller - clearing that one
        // wiped everything outside the rectangle the moment the raster
        // stopped covering it, which is a whole frame on a stage that draws
        // pillarboxed.
        //
        // 1. prepare: raw velocity + depth into one single-sampled texture.
        pass(
            "motion blur prepare",
            prepare_pipeline,
            &[&groups.idle, &groups.prepare],
            &prepared.view,
            full_rect,
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            opening.take(),
        );
        // 2 and 3: the separable tile reduction, horizontal then vertical.
        pass(
            "motion blur tile-max (x)",
            &self.tile_max_x,
            &[&groups.rows],
            &tile_rows.view,
            rows_rect,
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            None,
        );
        pass(
            "motion blur tile-max (y)",
            &self.tile_max_y,
            &[&groups.columns],
            &tile_a.view,
            grid_rect,
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            None,
        );
        // 4: neighbour-max.
        pass(
            "motion blur neighbour-max",
            &self.neighbour_max,
            &[&groups.spread],
            &tile_b.view,
            grid_rect,
            wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            None,
        );
        // 5: the gather, into scratch.
        if self.half {
            pass(
                "motion blur reconstruct (half)",
                &self.reconstruct_half,
                &[&groups.gather],
                &gathered.view,
                half_rect,
                // Cleared: a texel the half rectangle does not reach is still
                // within the composite's footprint at the rectangle's edge,
                // and zero is "no smear".
                wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                None,
            );
            pass(
                "motion blur composite",
                &self.composite,
                &[&groups.merge],
                &scratch.view,
                full_rect,
                wgpu::LoadOp::Load,
                None,
            );
        } else {
            pass(
                "motion blur reconstruct",
                &self.reconstruct,
                &[&groups.gather],
                &scratch.view,
                full_rect,
                wgpu::LoadOp::Load,
                None,
            );
        }
        // 6: home.
        pass(
            "motion blur copy",
            &self.copy,
            &[&groups.home],
            frame.scene,
            full_rect,
            wgpu::LoadOp::Load,
            closing.take(),
        );
        true
    }

    /// Builds all seven bind groups for one `key`. See [`Groups`].
    ///
    /// Group 0 for each pass: only the slots a pass reads matter, but a bound
    /// texture must not also be that pass's render target - usage scopes are
    /// validated for what is *bound*, not what the shader statically reads -
    /// so each pass names all three texture slots explicitly with views that
    /// are not its own target. Five targets and three slots, so the whole
    /// table in one place:
    ///
    /// | pass          | reads          | slot it reads through | target    |
    /// | ---           | ---            | ---                   | ---       |
    /// | prepare       | group 1 only   | -                     | prepared  |
    /// | tile-max x    | prepared       | prepared (3)          | tile_rows |
    /// | tile-max y    | tile_rows      | tile (4)              | tile_a    |
    /// | neighbour-max | tile_a         | tile (4)              | tile_b    |
    /// | reconstruct   | scene, prepared, tile_b | 0, 3, 4      | scratch   |
    /// | copy          | scratch        | colour (0)            | scene     |
    fn build_groups(&self, device: &wgpu::Device, frame: &Frame<'_>, key: Key) -> Groups {
        // Every caller has already checked these, and a `render` that reached
        // here without them would have returned; `expect` rather than a second
        // `let else` so the shape of this function stays one build.
        let (prepared, tile_rows, tile_a, tile_b, gathered, scratch) = (
            self.prepared.as_ref().expect("resized"),
            self.tile_rows.as_ref().expect("resized"),
            self.tile_a.as_ref().expect("resized"),
            self.tile_b.as_ref().expect("resized"),
            self.gathered.as_ref().expect("resized"),
            self.scratch.as_ref().expect("resized"),
        );
        let group = |label: &str,
                     colour: &wgpu::TextureView,
                     prepared_view: &wgpu::TextureView,
                     tile_view: &wgpu::TextureView| {
            oag_gpu::perfprobe::bind_group(
                device,
                &wgpu::BindGroupDescriptor {
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
                },
            )
        };
        let multisampled = frame.sample_count > 1;
        let prepare = if multisampled {
            oag_gpu::perfprobe::bind_group(
                device,
                &wgpu::BindGroupDescriptor {
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
                },
            )
        } else {
            oag_gpu::perfprobe::bind_group(
                device,
                &wgpu::BindGroupDescriptor {
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
                },
            )
        };
        Groups {
            key,
            prepare,
            // Prepare does not read group 0, but the pipeline layout carries
            // it so the WGSL's shared declarations resolve; bind harmless
            // views that are not its own target.
            idle: group(
                "motion blur prepare g0",
                &tile_b.view,
                &tile_b.view,
                &tile_a.view,
            ),
            rows: group(
                "motion blur tile-max (x)",
                &prepared.view,
                &prepared.view,
                &tile_b.view,
            ),
            columns: group(
                "motion blur tile-max (y)",
                &prepared.view,
                &prepared.view,
                &tile_rows.view,
            ),
            spread: group(
                "motion blur neighbour-max",
                &prepared.view,
                &prepared.view,
                &tile_a.view,
            ),
            gather: group(
                "motion blur reconstruct",
                frame.scene,
                &prepared.view,
                &tile_b.view,
            ),
            // The blend, the full-size depths and the scene through the
            // three slots - see `fs_composite`.
            merge: group(
                "motion blur composite",
                &gathered.view,
                &prepared.view,
                frame.scene,
            ),
            home: group(
                "motion blur copy",
                &scratch.view,
                &prepared.view,
                &tile_a.view,
            ),
            multisampled,
        }
    }

    fn resize(&mut self, device: &wgpu::Device, size: (u32, u32), tile: u32) {
        let size = (size.0.max(1), size.1.max(1));
        if self.size == size && self.tile == tile && self.scratch.is_some() {
            return;
        }
        // **The two full-size targets are rebuilt on a size change alone.**
        // The tile edge is `MAX_STRETCH` of the *extent*, so under dynamic
        // resolution it moves every time the controller steps - and rebuilding
        // `prepared` and `scratch` for that would drop and recreate two
        // allocation-sized textures for a reason that has nothing to do with
        // either of them. The three tile targets below genuinely depend on it.
        let full = self.size != size || self.scratch.is_none();
        self.size = size;
        self.tile = tile;
        let tiles = (size.0.div_ceil(tile.max(1)), size.1.div_ceil(tile.max(1)));
        if full {
            self.prepared = Some(Target::new(
                device,
                "motion blur prepared",
                PREPARED_FORMAT,
                size,
            ));
            self.scratch = Some(Target::new(
                device,
                "motion blur scratch",
                self.format,
                size,
            ));
            self.gathered = Some(Target::new(
                device,
                "motion blur gathered",
                GATHERED_FORMAT,
                half(size),
            ));
        }
        // Reduced across x but not yet across y - the intermediate the
        // separable tile-max needs, and the only target here whose two
        // dimensions come from different places.
        self.tile_rows = Some(Target::new(
            device,
            "motion blur tile rows",
            TILE_FORMAT,
            (tiles.0, size.1),
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
    }
}

mod target;
use target::Target;

#[cfg(test)]
mod extent_tests;
#[cfg(test)]
mod ghost_tests;
#[cfg(test)]
mod lattice_tests;
#[cfg(test)]
mod shake_tests;
#[cfg(test)]
mod tests;
