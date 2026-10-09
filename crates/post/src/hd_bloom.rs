//! Wipeout HD's post chain: a linear float scene target, the read
//! `FunkLayerBloom` passes over it, and the final sRGB encode the original's
//! ROP performs on write.
//!
//! Recovered from the EBOOT twice over - the fragment microcode of every
//! pass (`scripts/ps3-microcode.py`) and the PPU function that runs the
//! chain and fills the patched parameters (`FUN_003b4690`, read 2026-08-19).
//! See [renderer.md](../../../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md),
//! "The bloom chain, read pass by pass". **Every formula here is the
//! executable's; the parameters are the `.envsettings` `HDR and Bloom`
//! values, read per circuit and carried in [`Params`].** The chain the
//! original runs, in its own order:
//!
//! 1. downsample the frame to half, then to quarter;
//! 2. keep halving a copy down to a handful of pixels, read the mean colour
//!    back and update the **adapted average luminance**:
//!    `adapted += rate * (luma(mean) - adapted)`, luma weights
//!    `(0.3, 0.59, 0.11)`;
//! 3. the **gate** over the quarter-res scene - glow-mask term plus the
//!    luminance term, the latter *faded by adaptation*:
//!    `1 - min(adapted * boost * 0.25, 1)`;
//! 4. the two nine-tap blurs, ping-ponging the quarter buffers, tap spacing
//!    `authored size / buffer size` (the executable hard-codes `1/480` and
//!    `1/270` - its quarter buffers of a 1080p frame);
//! 5. the resolve, `downsamplescaleaddfeedback_fp`, every parameter name
//!    settled by crc32 preimage: the scene times the **read exposure**
//!    `scale = <Tone maximum brightness> - min(adapted * <Tone adaption
//!    boost>, <Tone darkening clamp>)`, plus the blurred bloom times
//!    `scaleAdd` = 1.0 - the bloom is deliberately *not* exposure-scaled -
//!    saturated. The exposure arithmetic is `FUN_003e3268`'s own (the
//!    settings registrar at `0x003a83d8` pins each key to its field).
//!
//! What is *not* the disc's, stated rather than hidden:
//!
//! - **The adaptation runs on the GPU here** (a 1x1 ping-pong) where the
//!   original reads the reduced buffer back and lerps on the PPU. Same
//!   arithmetic, no readback. And on the chain's **first frame** the lerp
//!   rate is forced to 1 - the state starts at zero and a single-frame
//!   capture would otherwise render the never-adapted picture no player
//!   ever sees.
//! - **The gate's event flash is not modelled.** The engine's
//!   `additiveColour` is `{0,0,0,0}` outside a whiteout flash driven by
//!   state this renderer does not carry, and `contribution.w` is hard-coded
//!   zero by the engine itself, so both drop out of the shader.
//! - **The resolve's `scaleFeedback` mix and `fullscreenTintColour`** are
//!   inert at authored defaults (no circuit authors `Bloom feedback`; the
//!   tint is an event effect) and left out.
//! - **The final gamma encode is the ROP's, not a stand-in for an exposure
//!   stage.** The exposure is the `scale` of the resolve above (1.0 on every
//!   frame measured); the original writes the result through the surface's
//!   sRGB encode (`SET_SHADER_PACKER` is 1 on the scene and ladder draws,
//!   0 on the swap buffer). `pow(1/2.2)` here is the project's single-curve
//!   approximation of that encode, nothing more.
//! - **The adaptation averages the encoded luminance.** The ladder halves the
//!   linear scene; the PPU then reads the bytes left in the last level, which
//!   the ROP wrote through the sRGB encode, so `adapted` is a luma of encoded
//!   texels (`fs_adapt` encodes each texel once, never the mean again). See
//!   renderer.md, "The adaptation is the encoded linear mean".
//!
//! # Why the chain owns the scene target
//!
//! The gate's luminance term reads the **pre-exposure scene**, so the race is
//! drawn into this chain's own target first: [`Chain::scene_view`] is what
//! the race pass attaches, and [`Chain::run`] is everything between it and
//! the caller's own surface.
//!
//! **That target is a float one here and an 8-bit one on the disc, and the
//! difference is corrected rather than kept.** This header used to say the
//! gate "only produces the reference frame's glow when the weighted luminance
//! runs over 1.0", i.e. that it reads an HDR scene. It does not: the original's
//! scene surface and all seven of `FUN_003af980`'s ladder buffers are
//! `CELL_GCM_SURFACE_A8R8G8B8`, read off `EBOOT.elf` on 2026-08-20 (`li r6, 8`
//! at eight call sites of the render-target factory `FUN_005a6ce0`; exactly one
//! `F_W16Z16Y16X16` surface exists in the executable and it is neither the
//! scene nor in the ladder). The gate's `(0.3, 0.59, 0.11) * 3` weights put its
//! knee at luma ~1/3, which is what an LDR bright pass looks like. Our target
//! stays [`SCENE_FORMAT`] because 8 bits of *linear* light bands in the darks,
//! and `hd_bloom.wesl`'s `surface()` applies the hardware's clamp at the three
//! points the chain samples the scene instead - see its own comment for the
//! measurement.

use anyhow::Result;
use oag_gpu::formats::SCENE_FORMAT;

use super::{sampler_entry, texture_entry, uniform_entry};

/// The luminance weights of the gate's `dot`, inline in
/// `FunkLayerBloomGate_fp`'s own microcode: `(0.3, 0.59, 0.11) * 3`. The
/// adaptation's own weights are the plain [`ADAPT_LUMINANCE`].
pub const GATE_LUMINANCE: [f32; 3] = [0.9, 1.77, 0.33];

/// The luminance weights of the PPU adaptation loop, the executable's own
/// constants at `0x8b74fc..0x8b7504`.
pub const ADAPT_LUMINANCE: [f32; 3] = [0.3, 0.59, 0.11];

/// The inline factor the engine applies between the adapted luminance and
/// the authored `Bloom adaption boost` in the gate fade - the constant at
/// `0x8b743c`.
pub const ADAPTION_FADE_SCALE: f32 = 0.25;

/// The blur kernel of `FunkLayerBloomBlurVertical_fp`/`Horizontal_fp`:
/// nine taps, weights `1, 0.8, 0.5, 0.2, 0.1` mirrored, divided by exactly
/// 4.2 (the microcode's final `MUL` by `0.238095`).
pub const BLUR_WEIGHTS: [f32; 9] = [0.1, 0.2, 0.5, 0.8, 1.0, 0.8, 0.5, 0.2, 0.1];

/// The kernel's divisor, the microcode's own constant.
pub const BLUR_DIVISOR: f32 = 4.2;

/// One timestamp pair, split across the chain's first and last pass.
///
/// The same shape as `oag_post::motion_blur::ChainTimestamps`, for
/// the same reason: [`Chain::run`] is a fixed sequence of render passes -
/// two downsamples, a reduction ladder, adapt, gate, two blurs and the
/// encode - and wgpu writes a timestamp per *pass*, not per encoder span. The
/// opening index rides the first downsample and the closing one rides the
/// encode, covering the whole chain with one claimed
/// [`oag_gpu::timing::PassTimer`] slot. See
/// [`oag_gpu::timing::PassTimer::half_writes`].
///
/// **Unlike motion blur's, this chain never encodes nothing.** [`Chain::run`]
/// has no early return - the downsample ladder always has at least two
/// entries and the encode always runs - so a caller that claims a slot only
/// when it is about to call `run` never needs to give it back unwritten.
#[derive(Debug)]
pub struct ChainTimestamps<'a> {
    /// Opening timestamp only, for the first downsample pass.
    pub begin: wgpu::RenderPassTimestampWrites<'a>,
    /// Closing timestamp only, for the encode pass.
    pub end: wgpu::RenderPassTimestampWrites<'a>,
}

/// One circuit's `HDR and Bloom` values - the parameters the engine patches
/// into the gate and blur programs, read from `track.envsettings`. The
/// settings-block offsets each name maps to are read out of the registrar
/// at `0x003a83d8`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// `Bloom from alpha contribution`: the glow-mask term's weight.
    pub alpha_contribution: f32,
    /// `Bloom from frame contribution`: the luminance term's weight.
    pub frame_contribution: f32,
    /// `Bloom from frame exponent`: the luminance term's power.
    pub frame_exponent: f32,
    /// `Bloom horizontal size`: the horizontal blur's tap spacing, in texels
    /// of the blur buffer.
    pub horizontal_size: f32,
    /// `Bloom vertical size`: the vertical blur's tap spacing.
    pub vertical_size: f32,
    /// `Bloom adaption rate`: the adaptation lerp's per-frame rate.
    pub adaption_rate: f32,
    /// `Bloom adaption boost`: scales the adapted luminance in the gate
    /// fade, together with [`ADAPTION_FADE_SCALE`].
    pub adaption_boost: f32,
    /// `Tone adaption boost`: scales the adapted luminance in the resolve's
    /// exposure - `scale = max_brightness - min(adapted * this, clamp)`.
    pub tone_adaption_boost: f32,
    /// `Tone darkening clamp`: the cap on that product, i.e. how far below
    /// `Tone maximum brightness` the exposure can fall.
    pub tone_darkening_clamp: f32,
    /// `Tone maximum brightness`: the exposure on a black frame.
    pub tone_maximum_brightness: f32,
    /// HD's boost and damage zoom-streak ring, when the title has one. See
    /// [`super::hd_zoom`].
    pub zoom: Option<super::hd_zoom::Tuning>,
}

/// Whether this chain's glow reaches the resolve.
///
/// The player's `[graphics] bloom` switch, which until now reached only
/// `crate::bloom` - the PSP chain - and so did nothing at all on
/// Wipeout HD, the one title whose bloom this module draws. Measured on the
/// Talon's Junction grid: the switch moved a Pulse frame's clipped-white
/// share from 2.07 % to 0.41 % and left an HD frame **byte-identical**.
///
/// [`Suppressed`](Self::Suppressed) skips the gate and the two blurs and
/// leaves the resolve's bloom input cleared. **It does not skip the chain**:
/// the downsample ladder feeds the luminance adaptation, and the exposure
/// resolve is what encodes HD's linear scene target for the surface at all -
/// switching that off would hand back an unencoded frame rather than an
/// unbloomed one. So nothing here is a magnitude: every constant the enabled
/// path reads off the disc is still read, and the disabled path reads none of
/// them differently, it just adds zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glow {
    /// The gate, the blurs and the additive resolve all run.
    Drawn,
    /// The exposure resolve runs and the bloom summand is zero.
    Suppressed,
}

/// A scratch colour buffer: sampled by the next pass, drawn into by this one.
#[derive(Debug)]
struct Target {
    #[cfg_attr(not(test), expect(dead_code, reason = "held so the view stays valid"))]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Target {
    fn new(device: &wgpu::Device, label: &str, (width, height): (u32, u32)) -> Self {
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
            format: SCENE_FORMAT,
            // A test writes its scene texels directly.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | if cfg!(test) {
                    wgpu::TextureUsages::COPY_DST
                } else {
                    wgpu::TextureUsages::empty()
                },
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view }
    }
}

/// The uniform block `hd_bloom.wesl` reads, one per pass.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Constants {
    step: [f32; 2],
    alpha_contribution: f32,
    frame_contribution: f32,
    frame_exponent: f32,
    adaption_rate: f32,
    adaption_boost: f32,
    tone_adaption_boost: f32,
    tone_darkening_clamp: f32,
    tone_maximum_brightness: f32,
    _pad: [f32; 2],
    /// Which sub-rectangle of every source in the chain was drawn.
    ///
    /// **One pair for the whole ladder**, which is what makes this tractable:
    /// each level's viewport is its own size times the same fraction, so the
    /// scale from a level's `0..1` onto its drawn rectangle is that fraction at
    /// every level. Exactly `1.0` on both until a controller moves the render
    /// extent - see `post::sub_rectangle`.
    uv_scale: [f32; 2],
    uv_max: [f32; 2],
}

impl Constants {
    fn new(params: Params, step: [f32; 2], rect: ([f32; 2], [f32; 2])) -> Self {
        Self {
            step,
            alpha_contribution: params.alpha_contribution,
            frame_contribution: params.frame_contribution,
            frame_exponent: params.frame_exponent,
            adaption_rate: params.adaption_rate,
            adaption_boost: params.adaption_boost,
            tone_adaption_boost: params.tone_adaption_boost,
            tone_darkening_clamp: params.tone_darkening_clamp,
            tone_maximum_brightness: params.tone_maximum_brightness,
            _pad: [0.0; 2],
            uv_scale: rect.0,
            uv_max: rect.1,
        }
    }
}

/// One ready-to-run pass: pipeline, its input bindings, its output.
#[derive(Debug)]
struct Pass {
    group: wgpu::BindGroup,
    target: wgpu::TextureView,
    /// `target`'s full dimensions. The rectangle actually drawn is this times
    /// the render extent's fraction of the scene target, computed at run time
    /// because the fraction is not known when the targets are built.
    level: (u32, u32),
}

/// Everything sized to the viewport, rebuilt whole on resize.
#[derive(Debug)]
struct Sized {
    scene: Target,
    /// Downsample scene -> half -> quarter, then the luminance reduction
    /// halvings, all on the copy pipeline, in order.
    downsamples: Vec<Pass>,
    /// The two adapt-pass variants, indexed by which ping-pong texture is
    /// being written this frame.
    adapt: [Pass; 2],
    /// The two gate variants, reading the adapted state written this frame.
    gate: [Pass; 2],
    blur_vertical: Pass,
    blur_horizontal: Pass,
    /// The scene target's dimensions - the allocation every level below is a
    /// fraction of.
    scene_size: (u32, u32),
    /// The quarter buffer's dimensions, which the blur tap spacing is measured
    /// against. Kept because that divisor moves with the render extent.
    quarter: (u32, u32),
    /// The three uniform buffers, `still` then the two blurs. All three carry
    /// the drawn rectangle, so all three are rewritten when it moves - which
    /// is why they are `COPY_DST` rather than mapped once at creation.
    buffers: [wgpu::Buffer; 3],
    /// The two resolve variants, reading the adapted state written this
    /// frame. `Pass::target` is unused here - the caller's view is the
    /// target.
    encode: [Pass; 2],
    #[expect(dead_code, reason = "held so the views stay valid")]
    scratch: Vec<Target>,
}

/// The whole HD post chain: the float scene target and every pass between
/// it and the caller's surface.
#[derive(Debug)]
pub struct Chain {
    params: Params,
    glow: Glow,
    copy: wgpu::RenderPipeline,
    /// The first luminance-reduction step: an exact 2x2 mean of the quarter-res
    /// scene, every texel weighing 1.
    reduce_first: wgpu::RenderPipeline,
    /// The later reduction steps: the same mean, weighted by coverage.
    reduce: wgpu::RenderPipeline,
    adapt: wgpu::RenderPipeline,
    /// The adapt pipeline with the lerp rate forced to 1, run once: the
    /// state starts at zero and a single captured frame would otherwise
    /// show the never-adapted picture. See the module header.
    adapt_jump: wgpu::RenderPipeline,
    gate: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
    encode: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    sized: Sized,
    /// Which adaptation ping-pong texture the next frame writes.
    current: std::cell::Cell<usize>,
    /// Whether the adaptation state still holds its zero-initialised value.
    fresh: std::cell::Cell<bool>,
    /// The drawn rectangle last written into the three uniform buffers, so a
    /// frame that moved nothing writes nothing.
    written: std::cell::Cell<Option<(u32, u32)>>,
    /// The zoom-streak ring, when the title has one, and the pulses it draws
    /// this frame. See `zoom.rs`.
    zoom: Option<super::hd_zoom::Zoom>,
    zoom_frame: std::cell::Cell<Option<super::hd_zoom::Frame>>,
}

impl Chain {
    /// Builds the pipelines and the targets for a viewport of `size`.
    ///
    /// `surface_format` is the caller's own target, which only the encode
    /// pass touches; everything before it runs on [`SCENE_FORMAT`].
    ///
    /// # Errors
    ///
    /// Propagates a shader or pipeline that will not build.
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        size: (u32, u32),
        params: Params,
        glow: Glow,
    ) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hd bloom"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/hd_bloom.wgsl")).into(),
            ),
        });
        // The fullscreen triple at 0..2, then the two extra source textures
        // this pass alone reads - so it composes the entries rather than
        // calling `fullscreen_layout`, which stops at binding 2.
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hd bloom"),
            entries: &[
                texture_entry(0),
                sampler_entry(1),
                uniform_entry(2),
                texture_entry(3),
                texture_entry(4),
            ],
        });
        // Bilinear and clamped. The downsample microcode is a single tap
        // whose filtering it leaves to the sampler, which over an exact 2x
        // halving makes each tap the 2x2 box mean - what the luminance
        // reduction wants - and clamping keeps the blur from wrapping glow
        // across the frame edge.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("hd bloom"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("hd bloom"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |label: &str,
                        entry: &str,
                        format: wgpu::TextureFormat,
                        blend: Option<wgpu::BlendState>,
                        constants: &[(&str, f64)]| {
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
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants,
                        ..Default::default()
                    },
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let copy = pipeline("hd bloom copy", "fs_copy", SCENE_FORMAT, None, &[]);
        let reduce_first = pipeline(
            "hd bloom reduce first",
            "fs_reduce_first",
            SCENE_FORMAT,
            None,
            &[],
        );
        let reduce = pipeline("hd bloom reduce", "fs_reduce", SCENE_FORMAT, None, &[]);
        let adapt = pipeline("hd bloom adapt", "fs_adapt", SCENE_FORMAT, None, &[]);
        let adapt_jump = pipeline(
            "hd bloom adapt jump",
            "fs_adapt",
            SCENE_FORMAT,
            None,
            &[("rate_override", 1.0)],
        );
        let gate = pipeline("hd bloom gate", "fs_gate", SCENE_FORMAT, None, &[]);
        let blur = pipeline("hd bloom blur", "fs_blur", SCENE_FORMAT, None, &[]);
        // No separate composite pass: the read resolve
        // (`downsamplescaleaddfeedback_fp`) adds the bloom itself, after
        // the exposure scale - see `fs_encode`.
        let encode = pipeline(
            "hd encode",
            "fs_encode",
            surface_format.remove_srgb_suffix(),
            None,
            &[],
        );
        let sized = Self::sized(device, &layout, &sampler, size, params);
        let zoom = Self::zoom_for(device, &params, &sized)?;
        Ok(Self {
            params,
            glow,
            copy,
            reduce_first,
            reduce,
            adapt,
            adapt_jump,
            gate,
            blur,
            encode,
            layout,
            sampler,
            sized,
            current: std::cell::Cell::new(0),
            fresh: std::cell::Cell::new(true),
            written: std::cell::Cell::new(None),
            zoom,
            zoom_frame: std::cell::Cell::new(None),
        })
    }

    /// The float scene target the race pass draws into.
    #[must_use]
    pub fn scene_view(&self) -> &wgpu::TextureView {
        &self.sized.scene.view
    }

    /// The scene target's texture, for a test that writes its texels directly.
    #[cfg(test)]
    fn scene_texture(&self) -> &wgpu::Texture {
        &self.sized.scene.texture
    }

    /// Rebuilds the targets for a new viewport size. The adaptation state
    /// restarts from zero, which the next frame's rate-1 jump re-seeds.
    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        self.sized = Self::sized(device, &self.layout, &self.sampler, size, self.params);
        self.zoom = Self::zoom_for(device, &self.params, &self.sized)
            .ok()
            .flatten();
        // The three uniform buffers are new and hold the whole rectangle, so
        // whatever was last written is no longer what is in them.
        self.written.set(None);
        self.current.set(0);
        self.fresh.set(true);
    }

    fn sized(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        (width, height): (u32, u32),
        params: Params,
    ) -> Sized {
        let (width, height) = (width.max(1), height.max(1));
        let half = ((width / 2).max(1), (height / 2).max(1));
        let quarter = ((width / 4).max(1), (height / 4).max(1));
        let scene = Target::new(device, "hd scene", (width, height));
        let half_target = Target::new(device, "hd bloom half", half);
        let quarter_a = Target::new(device, "hd bloom quarter a", quarter);
        let quarter_b = Target::new(device, "hd bloom quarter b", quarter);
        // The luminance reduction: keep halving from the quarter buffer to a
        // single pixel, the same iterated halving FUN_003b4690 runs before
        // its readback.
        let mut reductions = Vec::new();
        let (mut w, mut h) = quarter;
        while (w > 1 && h > 1) || reductions.is_empty() {
            w = w.div_ceil(2);
            h = h.div_ceil(2);
            reductions.push((Target::new(device, "hd bloom reduce", (w, h)), (w, h)));
        }
        let adapted = [
            Target::new(device, "hd bloom adapted a", (1, 1)),
            Target::new(device, "hd bloom adapted b", (1, 1)),
        ];

        let constants = |label: &str| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size_of::<Constants>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: true,
            });
            buffer
                .slice(..)
                .get_mapped_range_mut()
                .expect("a freshly mapped buffer maps")
                .copy_from_slice(bytemuck::bytes_of(&Constants::new(
                    params,
                    [0.0, 0.0],
                    ([1.0, 1.0], [1.0, 1.0]),
                )));
            buffer.unmap();
            buffer
        };
        let still = constants("hd bloom constants");
        let vertical = constants("hd bloom blur-v constants");
        let horizontal = constants("hd bloom blur-h constants");

        let group = |label: &str,
                     source: &wgpu::TextureView,
                     state: &wgpu::TextureView,
                     bloom: &wgpu::TextureView,
                     buffer: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(state),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(bloom),
                    },
                ],
            })
        };
        let pass = |label: &str,
                    source: &wgpu::TextureView,
                    state: &wgpu::TextureView,
                    bloom: &wgpu::TextureView,
                    buffer: &wgpu::Buffer,
                    target: &wgpu::TextureView,
                    level: (u32, u32)| Pass {
            group: group(label, source, state, bloom, buffer),
            target: target.clone(),
            level,
        };

        // The unused input slots of a pass bind whatever view is already
        // bound elsewhere in the pass and never its own target: a texture
        // bound for sampling in the pass that renders into it - even
        // unread - is a usage conflict.
        let mut downsamples = vec![
            pass(
                "hd bloom to-half",
                &scene.view,
                &scene.view,
                &scene.view,
                &still,
                &half_target.view,
                half,
            ),
            pass(
                "hd bloom to-quarter",
                &half_target.view,
                &half_target.view,
                &half_target.view,
                &still,
                &quarter_a.view,
                quarter,
            ),
        ];
        let mut previous = &quarter_a.view;
        for (reduction, level) in &reductions {
            downsamples.push(pass(
                "hd bloom reduce",
                previous,
                previous,
                previous,
                &still,
                &reduction.view,
                *level,
            ));
            previous = &reduction.view;
        }
        let mean = reductions.last().map_or(&quarter_a.view, |(r, _)| &r.view);
        // adapt[i] writes ping-pong texture i, reading the other as state.
        let adapt = [
            pass(
                "hd bloom adapt a",
                mean,
                &adapted[1].view,
                mean,
                &still,
                &adapted[0].view,
                (1, 1),
            ),
            pass(
                "hd bloom adapt b",
                mean,
                &adapted[0].view,
                mean,
                &still,
                &adapted[1].view,
                (1, 1),
            ),
        ];
        // gate[i] runs after adapt[i] and reads the state adapt[i] wrote.
        let gate = [
            pass(
                "hd bloom gate a",
                &quarter_a.view,
                &adapted[0].view,
                &quarter_a.view,
                &still,
                &quarter_b.view,
                quarter,
            ),
            pass(
                "hd bloom gate b",
                &quarter_a.view,
                &adapted[1].view,
                &quarter_a.view,
                &still,
                &quarter_b.view,
                quarter,
            ),
        ];
        let blur_vertical = pass(
            "hd bloom blur-v",
            &quarter_b.view,
            &quarter_b.view,
            &quarter_b.view,
            &vertical,
            &quarter_a.view,
            quarter,
        );
        let blur_horizontal = pass(
            "hd bloom blur-h",
            &quarter_a.view,
            &quarter_a.view,
            &quarter_a.view,
            &horizontal,
            &quarter_b.view,
            quarter,
        );
        // The read resolve: scene * exposure + bloom, into the caller's
        // view. encode[i] reads the adapted state written this frame.
        let encode = [
            pass(
                "hd encode a",
                &scene.view,
                &adapted[0].view,
                &quarter_b.view,
                &still,
                &scene.view,
                (width, height),
            ),
            pass(
                "hd encode b",
                &scene.view,
                &adapted[1].view,
                &quarter_b.view,
                &still,
                &scene.view,
                (width, height),
            ),
        ];
        let mut scratch = vec![half_target, quarter_a, quarter_b];
        scratch.extend(reductions.into_iter().map(|(target, _)| target));
        scratch.extend(adapted);
        Sized {
            scene,
            scene_size: (width, height),
            quarter,
            buffers: [still, vertical, horizontal],
            downsamples,
            adapt,
            gate,
            blur_vertical,
            blur_horizontal,
            encode,
            scratch,
        }
    }

    /// Runs the chain: the downsample ladder, the adaptation update, the
    /// gate, the two blurs, and the read resolve into `view`. The pass
    /// order is `FUN_003b4690`'s own, the resolve `FUN_003e3268`'s.
    ///
    /// `viewport` is the rectangle of the scene target that was drawn this
    /// frame. **Every level of the ladder follows it by the same fraction**,
    /// which is what lets one UV scale cover the chain - and what keeps the
    /// luminance reduction averaging the picture rather than the cleared
    /// region beside it. The blur's tap spacing follows it too: the
    /// executable measures `Bloom vertical size` against its quarter *frame*,
    /// and under a short extent the frame is the extent, so leaving the
    /// divisor on the resource would widen the glow as a fraction of the
    /// picture every time the scale fell.
    ///
    /// `origin` is where that rectangle sits inside `view` - `(0, 0)` unless
    /// the caller letterboxed the scene, in which case it is the fitted
    /// rectangle's own top-left. **Every internal stage ignores it**: the
    /// downsample ladder, the adaptation, the gate and the two blurs all
    /// target a dedicated scratch texture sized exactly to their own level,
    /// so `(0, 0)` is already that texture's own top-left corner. Only the
    /// last pass, "hd encode", writes into `view` itself - the same shared
    /// canvas the scene pass drew the letterboxed rectangle into - and has to
    /// start at the same corner the scene did, or it resolves a rectangle
    /// shifted from the one that was actually drawn. Found the same way
    /// `hd-frame-compare.py`'s own doc comment did: a black strip at the
    /// canvas edge under a letterboxed aspect, present with bloom either on
    /// or off, gone once `origin` is threaded here. See
    /// `crates/raceplay/src/scene/frame.rs`'s call site.
    pub fn run(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        origin: (f32, f32),
        viewport: (u32, u32),
        timestamps: Option<ChainTimestamps<'_>>,
    ) {
        let scene = self.sized.scene_size;
        let viewport = (viewport.0.clamp(1, scene.0), viewport.1.clamp(1, scene.1));
        if self.written.get() != Some(viewport) {
            let rect = super::sub_rectangle(viewport, scene);
            let quarter = level_viewport(self.sized.quarter, viewport, scene);
            let write = |buffer: &wgpu::Buffer, step: [f32; 2]| {
                queue.write_buffer(
                    buffer,
                    0,
                    bytemuck::bytes_of(&Constants::new(self.params, step, rect)),
                );
            };
            write(&self.sized.buffers[0], [0.0, 0.0]);
            write(
                &self.sized.buffers[1],
                [0.0, self.params.vertical_size / quarter.1 as f32],
            );
            write(
                &self.sized.buffers[2],
                [self.params.horizontal_size / quarter.0 as f32, 0.0],
            );
            self.written.set(Some(viewport));
        }

        // **The two halves, taken apart here rather than carried into the
        // closure.** `RenderPassTimestampWrites` borrows `self.queries` and is
        // `Clone` but not `Copy`, and `.take()` on an `Option` is the cheapest
        // way to hand the opening write to exactly one call and the closing
        // one to exactly one other, out of however many this function makes.
        let (mut opening, mut closing) = match timestamps {
            Some(pair) => (Some(pair.begin), Some(pair.end)),
            None => (None, None),
        };

        // `pipeline` is an `Option` for one caller: the suppressed glow below
        // needs the resolve's bloom input *cleared*, which is this same pass
        // with its draw left off rather than a second copy of the descriptor.
        let pass =
            |encoder: &mut wgpu::CommandEncoder,
             label: &str,
             pipeline: Option<&wgpu::RenderPipeline>,
             stage: &Pass,
             target: Option<&wgpu::TextureView>,
             load: wgpu::LoadOp<wgpu::Color>,
             // `(0.0, 0.0)` for every internal stage: each targets a
             // dedicated scratch texture sized exactly to its own level, so
             // that texture's own top-left is already the right corner. Only
             // "hd encode" passes the scene's real `origin` - see this
             // method's own doc comment.
             origin: (f32, f32),
             timestamp_writes: Option<wgpu::RenderPassTimestampWrites<'_>>| {
                let rect = level_viewport(stage.level, viewport, scene);
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(label),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target.unwrap_or(&stage.target),
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
                let Some(pipeline) = pipeline else { return };
                pass.set_pipeline(pipeline);
                pass.set_viewport(origin.0, origin.1, rect.0 as f32, rect.1 as f32, 0.0, 1.0);
                pass.set_bind_group(0, &stage.group, &[]);
                pass.draw(0..3, 0..1);
            };
        let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
        // `self.sized.downsamples` always has at least two entries - see its
        // own construction - so the opening write always lands on a real pass
        // and `opening` never survives this loop unconsumed.
        for (index, stage) in self.sized.downsamples.iter().enumerate() {
            // The two halvings are the gate's and the blur's quarter-res
            // scene; what follows is the luminance reduction.
            let pipeline = match index {
                0 | 1 => &self.copy,
                2 => &self.reduce_first,
                _ => &self.reduce,
            };
            pass(
                encoder,
                "hd bloom downsample",
                Some(pipeline),
                stage,
                None,
                clear,
                (0.0, 0.0),
                opening.take(),
            );
        }
        self.zoom_history(queue, encoder, viewport);
        let current = self.current.get();
        let adapt_pipeline = if self.fresh.replace(false) {
            &self.adapt_jump
        } else {
            &self.adapt
        };
        pass(
            encoder,
            "hd bloom adapt",
            Some(adapt_pipeline),
            &self.sized.adapt[current],
            None,
            clear,
            (0.0, 0.0),
            None,
        );
        match self.glow {
            Glow::Drawn => {
                pass(
                    encoder,
                    "hd bloom gate",
                    Some(&self.gate),
                    &self.sized.gate[current],
                    None,
                    clear,
                    (0.0, 0.0),
                    None,
                );
                pass(
                    encoder,
                    "hd bloom blur-v",
                    Some(&self.blur),
                    &self.sized.blur_vertical,
                    None,
                    clear,
                    (0.0, 0.0),
                    None,
                );
                pass(
                    encoder,
                    "hd bloom blur-h",
                    Some(&self.blur),
                    &self.sized.blur_horizontal,
                    None,
                    clear,
                    (0.0, 0.0),
                    None,
                );
            }
            // The resolve reads `blur_horizontal`'s target as its bloom
            // input, so clearing it is what makes the summand zero. A pass
            // that loads-clear and draws nothing, rather than a skipped one:
            // the texture holds the previous frame's glow otherwise, and the
            // first frame after a resize holds whatever the allocation did.
            Glow::Suppressed => pass(
                encoder,
                "hd bloom suppressed",
                None,
                &self.sized.blur_horizontal,
                None,
                clear,
                (0.0, 0.0),
                None,
            ),
        }
        self.zoom_ring(queue, encoder, viewport);
        // The closing write, on the chain's genuinely last pass - see
        // [`ChainTimestamps`]. The only stage that writes into `view` (the
        // caller's shared canvas) rather than a dedicated scratch texture, so
        // it is the only one that needs `origin`.
        pass(
            encoder,
            "hd encode",
            Some(&self.encode),
            &self.sized.encode[current],
            Some(view),
            clear,
            origin,
            closing.take(),
        );
        self.current.set(1 - current);
    }
}

/// The rectangle drawn into a target of `level`, given the scene's own drawn
/// rectangle.
///
/// The same fraction at every level, rounded and floored at one texel: the
/// ladder ends at a 1x1 mean, and a level that rounded to zero would be a pass
/// that drew nothing into the texture the next one reads.
fn level_viewport(level: (u32, u32), viewport: (u32, u32), scene: (u32, u32)) -> (u32, u32) {
    let axis = |size: u32, drawn: u32, whole: u32| {
        if drawn >= whole || whole == 0 {
            return size.max(1);
        }
        let scaled = (f64::from(size) * f64::from(drawn) / f64::from(whole)).round();
        (scaled as u32).clamp(1, size.max(1))
    };
    (
        axis(level.0, viewport.0, scene.0),
        axis(level.1, viewport.1, scene.1),
    )
}

mod zoom;

#[cfg(test)]
mod tests;
