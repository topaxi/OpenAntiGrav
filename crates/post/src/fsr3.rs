//! AMD FidelityFX Super Resolution 3.1: the temporal upscaler.
//!
//! Eight compute dispatches - upstream's own eight passes - transliterated
//! from AMD's MIT-licensed FidelityFX SDK `v1.1.4`. They are encoded as *two*
//! `wgpu` compute passes, split where the resolution changes, so each half
//! carries its own timestamp pair; see [`ChainTimestamps`]. The route - port to WGSL, drive no native SDK - is
//! [ADR-0012](../../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md);
//! what is ported, what deviates and why is
//! [fsr3.md](../../../../docs/rendering/fsr3.md), which is the page to read
//! before changing anything here.
//!
//! **All eight passes are built, and the UPSCALER row selects them.**
//! `oag_game::upscale::Framebuffer::resolve_scene` reads [`Fsr3::output`] on a
//! race frame whose adapter has compute shaders, and falls one rung to
//! [`super::fsr1`] otherwise. It is off by default; what has *not* happened is
//! anybody judging a moving frame it produced - see the handover thread.
//! [`Fsr3::PASSES`] is the spine either way.
//!
//! # Why this and not FSR 1
//!
//! [`super::fsr1`] is spatial: one frame in, one frame out, and its ceiling is
//! set by how much a single frame's pixels can be argued to imply. FSR 3.1
//! reconstructs from a *history* - several previous frames, each rasterised at
//! a different sub-pixel offset, reprojected onto this one through the motion
//! vectors. That buys detail a spatial filter cannot invent, and costs
//! everything on the input list below plus the history's own failure modes
//! (ghosting, disocclusion) that the middle five passes exist to manage.
//!
//! # Colour space: gamma, and upstream would want otherwise
//!
//! Upstream accumulates **linear light**, and for good reason: averaging
//! several frames of one surface in an encoded space weights a dark sample as
//! brighter than it is. [`super`]'s own table anticipated that and listed FSR
//! 3.1 as wanting "a fourth thing".
//!
//! **It cannot have it here.**
//! [ADR-0020](../../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md)
//! makes gamma this renderer's authoritative colour space and *nothing*
//! linearises - the blend equations the art was authored against are defined on
//! stored bytes, and the scene target holds gamma values in a deliberately
//! non-sRGB format. There is no linear light anywhere in the pipeline to hand
//! this, and manufacturing some for one pass would be exactly the
//! inconsistency that ADR removed.
//!
//! So [`Frame::colour`] takes the same view [`super::fsr1`] takes, and
//! accumulation runs on encoded values. That is a real divergence from
//! upstream, in the same family as the FP16 path this port does not take, and
//! it is the *consistent* choice rather than the accurate one - which is the
//! trade ADR-0020 already made for every other pass in this renderer.
//!
//! # Jitter is an input, not a setting
//!
//! Without a sub-pixel offset per frame, every frame samples the same point in
//! each pixel and there is nothing for accumulation to reconstruct *from*: the
//! result degrades to a blurry reprojection. [`Frame::jitter`] must therefore
//! be the offset the scene was actually drawn with, and
//! [`crate::jitter::phases`] must have chosen the sequence length from the same
//! two sizes passed here. ADR-0039 records why the offset is applied where it
//! is; the short version is that the culling frustum and the velocity buffer
//! must not see it.

use anyhow::Result;

mod bindings;
mod constants;
mod groups;
mod resources;

use bindings::{
    atomic_entry, level_entry, load_entry, load_multisampled_entry, read_buffer_entry,
    sample_entry, shared_layout, store_entry,
};

pub use constants::{Camera, Constants, Dispatch, camera_from_projection};
pub use resources::{PYRAMID_MIPS, Sizes, Targets};

/// Whether `adapter` can run this at all.
///
/// **One question, where [ADR-0012] expected several.** The ADR predicted an
/// adapter probe over storage-texture formats and access modes, because FSR
/// 3.1's intermediates are held in formats outside the WebGPU baseline. This
/// port widens them instead - see [`resources`] - so no `wgpu::Features` bit is
/// requested and the only thing left to ask is whether the adapter has compute
/// shaders at all. A downlevel GL adapter does not; everything else does.
///
/// The caller's job is what the ADR actually requires: degrade, never fail to
/// boot. `false` here means fall to [`super::fsr1`], which is fragment-only and
/// runs anywhere a triangle does.
///
/// [ADR-0012]: ../../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md
#[must_use]
pub fn supported(adapter: &wgpu::Adapter) -> bool {
    adapter
        .get_downlevel_capabilities()
        .flags
        .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
}

/// One timestamp pair for each half of the chain.
///
/// **Two readings rather than one, because the chain is two costs.** Six of
/// the eight dispatches run at the render extent or half of it and shrink when
/// a resolution controller lowers it; `accumulate` and `rcas` run at
/// presentation resolution and do not move at all. One pair over the lot
/// answers "what did FSR 3.1 cost" and nothing else, and a dynamic-resolution
/// budget that has to divide by the part that moves cannot use it - which is
/// why [ADR-0042](../../../../docs/architecture/adr/0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md)
/// counted the whole reading as fixed and
/// [ADR-0045](../../../../docs/architecture/adr/0045-fsr3-splits-into-a-scaled-and-a-presented-reading.md)
/// is where that changed.
///
/// **Both or neither.** The two halves come from two rings, and a caller that
/// claims one slot without the other hands `Session::feed_drs` a frame whose
/// readings do not match - so this is one struct with two non-optional fields
/// rather than two independent arguments, and [`Fsr3::render`]'s `bool` is
/// what says whether both were written. The same shape, and the same reason,
/// as `super::motion_blur::ChainTimestamps`.
#[derive(Debug)]
pub struct ChainTimestamps<'a> {
    /// Around the six dispatches that scale with the render extent: the
    /// input clear and prepare, the luma pyramid, the shading-change pyramid
    /// and its resolve, reactivity, and luma instability.
    pub scaled: wgpu::ComputePassTimestampWrites<'a>,
    /// Around `accumulate` and `rcas`, both at presentation resolution.
    pub presented: wgpu::ComputePassTimestampWrites<'a>,
}

/// One frame's worth of input.
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    /// The scene target, in whatever space this renderer draws in - which is
    /// gamma, and is the same view [`super::fsr1`] takes. See the module docs
    /// for why upstream would want linear light and why there is none here.
    pub colour: &'a wgpu::TextureView,
    /// The scene's depth attachment, stored rather than discarded since
    /// [ADR-0028](../../../../docs/architecture/adr/0028-camera-motion-blur-first.md).
    pub depth: &'a wgpu::TextureView,
    /// The scene's velocity attachment - screen motion in UV units, written by
    /// every race draw since
    /// [ADR-0030](../../../../docs/architecture/adr/0030-velocity-buffer-motion-blur.md).
    /// The sign is the opposite of what FSR 3.1 wants and the constants carry
    /// the correction; see `constants::MOTION_VECTOR_SCALE`.
    pub velocity: &'a wgpu::TextureView,
    /// Everything that is a number rather than a resource.
    pub dispatch: Dispatch,
}

/// Which pass a name refers to, in upstream's dispatch order.
///
/// Ported and unported alike, because the list *is* the port's remaining work
/// and a table with holes in it says more than a table of what happens to exist.
/// [`Pass::ported`] is the one that moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pass {
    /// Nearest and farthest depth of each 3x3 neighbourhood, the motion vector
    /// belonging to the nearest, this frame's luma, and the scattered
    /// projection of this frame's depth into the previous frame's grid.
    PrepareInputs,
    /// The luma pyramid, upstream's first SPD dispatch - which here reduces to
    /// a single 2x2 average producing `farthest_depth_mip1`, because its other
    /// two products are SPD's own plumbing and an auto-exposure this renderer
    /// has no use for. `luma_pyramid.wgsl` argues both.
    LumaPyramid,
    /// The shading-change pyramid, upstream's second SPD dispatch.
    ShadingChangePyramid,
    /// How much the shading of each half-resolution texel changed.
    ShadingChange,
    /// The packed reactive/disocclusion/shading-change/accumulation mask.
    PrepareReactivity,
    /// How unstable each pixel's luma has been across the history.
    LumaInstability,
    /// The temporal resolve itself, at presentation resolution.
    Accumulate,
    /// RCAS, the same sharpen [`super::fsr1`] ends with.
    Rcas,
}

impl Pass {
    /// Upstream's dispatch order, which is also the order they must be ported
    /// in: a pass reads what the ones before it wrote.
    pub const ALL: [Self; 8] = [
        Self::PrepareInputs,
        Self::LumaPyramid,
        Self::ShadingChangePyramid,
        Self::ShadingChange,
        Self::PrepareReactivity,
        Self::LumaInstability,
        Self::Accumulate,
        Self::Rcas,
    ];

    /// Upstream's own name for the pass, so a reader can find the header.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::PrepareInputs => "prepare_inputs",
            Self::LumaPyramid => "luma_pyramid",
            Self::ShadingChangePyramid => "shading_change_pyramid",
            Self::ShadingChange => "shading_change",
            Self::PrepareReactivity => "prepare_reactivity",
            Self::LumaInstability => "luma_instability",
            Self::Accumulate => "accumulate",
            Self::Rcas => "rcas",
        }
    }

    /// Whether this pass is built yet.
    ///
    /// A method rather than a comment because the fallback in
    /// [`Fsr3::output`] keys on it: an incomplete chain must produce nothing
    /// and say so, not a half-resolved frame that reads as a rendering bug.
    #[must_use]
    pub fn ported(self) -> bool {
        matches!(
            self,
            Self::PrepareInputs
                | Self::LumaPyramid
                | Self::ShadingChangePyramid
                | Self::ShadingChange
                | Self::PrepareReactivity
                | Self::LumaInstability
                | Self::Accumulate
                | Self::Rcas
        )
    }
}

/// One pass's compiled WGSL: `build.rs` resolves its `import`s of
/// `shaders/fsr3/common.wesl` and writes the result to `$OUT_DIR`.
macro_rules! pass_wgsl {
    ($name:literal) => {
        include_str!(concat!(env!("OUT_DIR"), "/fsr3_", $name, ".wgsl"))
    };
}

/// How many threads each of upstream's render-resolution passes covers, in each
/// axis - `FFX_FSR3UPSCALER_THREAD_GROUP_WIDTH` and `..._HEIGHT`.
const GROUP: u32 = 8;

/// How many workgroups cover `size` at `group` threads each.
fn groups(size: u32, group: u32) -> u32 {
    size.div_ceil(group.max(1))
}

/// The pipelines, the intermediates, and the history's own bookkeeping.
#[derive(Debug)]
pub struct Fsr3 {
    /// Group 0 of every pass, built once. The samplers and the layout it was
    /// built from are not kept beside it: a `wgpu::BindGroup` holds its own
    /// resources alive, and the layout is only needed while pipelines are being
    /// created.
    shared: wgpu::BindGroup,
    constants: wgpu::Buffer,
    written: Option<Constants>,
    /// The previous frame's constants, which is where the four `previousFrame*`
    /// fields and the frame index come from. `None` before the first frame and
    /// after a reset.
    previous: Option<Constants>,

    clear_layout: wgpu::BindGroupLayout,
    clear: wgpu::ComputePipeline,
    /// The two builds of `prepare_inputs`: single-sampled and multisampled.
    /// Which one runs is [`Dispatch::sample_count`]'s only consequence.
    prepare_inputs_layout: [wgpu::BindGroupLayout; 2],
    prepare_inputs: [wgpu::ComputePipeline; 2],
    luma_pyramid_layout: wgpu::BindGroupLayout,
    luma_pyramid: wgpu::ComputePipeline,
    shading_change_pyramid_layout: wgpu::BindGroupLayout,
    shading_change_pyramid_mip0: wgpu::ComputePipeline,
    shading_change_pyramid_reduce: wgpu::ComputePipeline,
    shading_change_layout: wgpu::BindGroupLayout,
    shading_change: wgpu::ComputePipeline,
    prepare_reactivity_layout: wgpu::BindGroupLayout,
    prepare_reactivity: wgpu::ComputePipeline,
    luma_instability_layout: wgpu::BindGroupLayout,
    luma_instability: wgpu::ComputePipeline,
    accumulate_layout: wgpu::BindGroupLayout,
    accumulate: wgpu::ComputePipeline,
    rcas_layout: wgpu::BindGroupLayout,
    rcas: wgpu::ComputePipeline,
    /// One uniform per pyramid level, holding that level's source extent. See
    /// [`level_entry`]; written once, because the extents are a function of the
    /// allocation rather than of the frame.
    levels: Vec<wgpu::Buffer>,

    targets: Option<Targets>,
    /// Both ping-pong parities' bind groups, or `None` before the first frame
    /// and after a resize.
    ///
    /// **Fifteen `create_bind_group` calls and a `Vec` allocation that used to
    /// happen every frame**, for a set of bindings in which nothing is
    /// per-frame: see `groups.rs`'s own header. Held here rather than beside
    /// [`Targets`] because it is invalidated by one more thing than an
    /// allocation is - the scene's own views, which the caller owns.
    groups: Option<groups::Cache>,
    /// How many times [`Self::groups`] has been built.
    ///
    /// **The only externally visible evidence that the cache works.** A bind
    /// group that is rebuilt every frame and one that is reused produce
    /// identical pictures, so nothing but a count can tell them apart - see
    /// [`Self::group_rebuilds`].
    group_rebuilds: u64,
}

impl Fsr3 {
    /// Every pass, ported or not - [`Pass::ALL`], re-exported here so a caller
    /// holding a `Fsr3` does not have to reach for the enum.
    pub const PASSES: [Pass; 8] = Pass::ALL;

    /// Builds every pass this port has.
    ///
    /// **Nothing is allocated here.** The intermediates want a render
    /// allocation and a presentation rectangle, and neither is known until the
    /// first [`render`](Self::render) - the same arrangement
    /// [`super::fsr1`] uses.
    ///
    /// # Errors
    ///
    /// Propagates a shader that will not compile, which is a build-time mistake
    /// rather than anything a player can cause. The caller keeps the `Result`
    /// rather than unwrapping it, so a broken shader reports itself once and
    /// leaves the game running on the next rung of the ladder.
    pub fn new(device: &wgpu::Device) -> Result<Self> {
        let shared_layout = shared_layout(device);

        // **Both clamp**, which is what upstream's `s_PointClamp` and
        // `s_LinearClamp` are; every gather in these passes reaches past the
        // frame's edge at the border and upstream leans on the clamp rather
        // than testing for it.
        let sampler = |label, filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        let point = sampler("fsr3 point clamp", wgpu::FilterMode::Nearest);
        let linear = sampler("fsr3 linear clamp", wgpu::FilterMode::Linear);

        let constants = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fsr3 constants"),
            size: Constants::SIZE as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shared = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fsr3 shared"),
            layout: &shared_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: constants.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&point),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&linear),
                },
            ],
        });

        let clear_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fsr3 clear"),
            entries: &[atomic_entry(0)],
        });
        let prepare_inputs_entries = |multisampled: bool| {
            [
                load_multisampled_entry(0, multisampled),
                load_multisampled_entry(1, multisampled),
                load_entry(2),
                store_entry(3, wgpu::TextureFormat::Rgba16Float),
                store_entry(4, wgpu::TextureFormat::R32Float),
                atomic_entry(5),
                store_entry(6, wgpu::TextureFormat::R32Float),
                store_entry(7, wgpu::TextureFormat::Rgba16Float),
            ]
        };
        let prepare_inputs_layout = [false, true].map(|multisampled| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("fsr3 prepare inputs"),
                entries: &prepare_inputs_entries(multisampled),
            })
        });
        let luma_pyramid_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("fsr3 luma pyramid"),
                entries: &[
                    load_entry(0),
                    store_entry(1, wgpu::TextureFormat::Rgba16Float),
                ],
            });
        // One layout for both pyramid entry points, so that a level-0 dispatch
        // and a reduce dispatch bind the same shape. Each reads a different
        // subset of it and an unused binding costs nothing - the argument group
        // 0 already makes.
        let shading_change_pyramid_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("fsr3 shading change pyramid"),
                entries: &[
                    load_entry(0),
                    load_entry(1),
                    load_entry(2),
                    load_entry(3),
                    store_entry(4, wgpu::TextureFormat::Rgba16Float),
                    level_entry(5),
                ],
            });

        let shading_change_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("fsr3 shading change"),
                entries: &[
                    sample_entry(0),
                    store_entry(1, wgpu::TextureFormat::Rgba8Unorm),
                ],
            });
        let prepare_reactivity_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("fsr3 prepare reactivity"),
                entries: &[
                    load_entry(0),
                    load_entry(1),
                    read_buffer_entry(2),
                    load_entry(3),
                    sample_entry(4),
                    sample_entry(5),
                    store_entry(6, wgpu::TextureFormat::Rgba8Unorm),
                    store_entry(7, wgpu::TextureFormat::Rgba8Unorm),
                    store_entry(8, wgpu::TextureFormat::Rgba8Unorm),
                ],
            });
        let luma_instability_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("fsr3 luma instability"),
                entries: &[
                    load_entry(0),
                    sample_entry(1),
                    sample_entry(2),
                    sample_entry(3),
                    sample_entry(4),
                    store_entry(5, wgpu::TextureFormat::Rgba16Float),
                    store_entry(6, wgpu::TextureFormat::Rgba16Float),
                ],
            });
        let accumulate_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fsr3 accumulate"),
            entries: &[
                load_entry(0),
                load_entry(1),
                sample_entry(2),
                sample_entry(3),
                sample_entry(4),
                load_entry(5),
                load_entry(6),
                store_entry(7, wgpu::TextureFormat::Rgba16Float),
            ],
        });
        let rcas_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fsr3 rcas"),
            entries: &[
                load_entry(0),
                store_entry(1, wgpu::TextureFormat::Rgba16Float),
            ],
        });

        // `0..PYRAMID_MIPS`, each in its own uniform. An index rather than an
        // extent, so these never have to be rewritten - see the shader's
        // `pyramid_level_size`.
        // `mapped_at_creation` rather than a queue write, because `new` has no
        // queue - and wants none: these are build-time constants, not per-frame
        // data.
        let levels = (0..PYRAMID_MIPS)
            .map(|level| {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("fsr3 pyramid level"),
                    size: 16,
                    usage: wgpu::BufferUsages::UNIFORM,
                    mapped_at_creation: true,
                });
                buffer
                    .slice(..)
                    .get_mapped_range_mut()
                    .expect("a freshly mapped buffer")
                    .copy_from_slice(bytemuck::cast_slice(&[level, 0u32, 0, 0]));
                buffer.unmap();
                buffer
            })
            .collect();

        let pipeline = |label: &str, wgsl: &str, entry: &str, own: &wgpu::BindGroupLayout| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(wgsl.into()),
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[Some(&shared_layout), Some(own)],
                immediate_size: 0,
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                module: &module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };

        let clear = pipeline(
            "fsr3 clear",
            pass_wgsl!("clear"),
            "cs_clear_reconstructed_depth",
            &clear_layout,
        );
        let prepare_inputs = [
            pipeline(
                "fsr3 prepare inputs",
                pass_wgsl!("prepare_inputs"),
                "cs_prepare_inputs",
                &prepare_inputs_layout[0],
            ),
            pipeline(
                "fsr3 prepare inputs (msaa)",
                pass_wgsl!("prepare_inputs_msaa"),
                "cs_prepare_inputs",
                &prepare_inputs_layout[1],
            ),
        ];
        let luma_pyramid = pipeline(
            "fsr3 luma pyramid",
            pass_wgsl!("luma_pyramid"),
            "cs_luma_pyramid",
            &luma_pyramid_layout,
        );
        let shading_change_pyramid_mip0 = pipeline(
            "fsr3 shading change pyramid mip0",
            pass_wgsl!("shading_change_pyramid"),
            "cs_shading_change_pyramid_mip0",
            &shading_change_pyramid_layout,
        );
        let shading_change_pyramid_reduce = pipeline(
            "fsr3 shading change pyramid reduce",
            pass_wgsl!("shading_change_pyramid"),
            "cs_shading_change_pyramid_reduce",
            &shading_change_pyramid_layout,
        );
        let shading_change = pipeline(
            "fsr3 shading change",
            pass_wgsl!("shading_change"),
            "cs_shading_change",
            &shading_change_layout,
        );
        let prepare_reactivity = pipeline(
            "fsr3 prepare reactivity",
            pass_wgsl!("prepare_reactivity"),
            "cs_prepare_reactivity",
            &prepare_reactivity_layout,
        );
        let luma_instability = pipeline(
            "fsr3 luma instability",
            pass_wgsl!("luma_instability"),
            "cs_luma_instability",
            &luma_instability_layout,
        );
        let accumulate = pipeline(
            "fsr3 accumulate",
            pass_wgsl!("accumulate"),
            "cs_accumulate",
            &accumulate_layout,
        );
        let rcas = pipeline("fsr3 rcas", pass_wgsl!("rcas"), "cs_rcas", &rcas_layout);

        Ok(Self {
            shared,
            constants,
            written: None,
            previous: None,
            clear_layout,
            clear,
            prepare_inputs_layout,
            prepare_inputs,
            luma_pyramid_layout,
            luma_pyramid,
            shading_change_pyramid_layout,
            shading_change_pyramid_mip0,
            shading_change_pyramid_reduce,
            shading_change_layout,
            shading_change,
            prepare_reactivity_layout,
            prepare_reactivity,
            luma_instability_layout,
            luma_instability,
            accumulate_layout,
            accumulate,
            rcas_layout,
            rcas,
            levels,
            targets: None,
            groups: None,
            group_rebuilds: 0,
        })
    }

    /// The resolved frame, in the same **gamma** space it was handed - see
    /// this module's own header, and ADR-0020 behind it. `None` until the
    /// chain is complete enough to produce one.
    ///
    /// **Keyed off [`Pass::ported`] rather than off whether a texture happens
    /// to exist.** Every pass is built now, so this is `Some` from the first
    /// frame - but the gate stays, because it is what made an incomplete chain
    /// hand back nothing rather than a half-resolved picture, and a
    /// half-resolved picture reads as a rendering bug rather than as unfinished
    /// work. A pass added or removed moves both this and the test that asserts
    /// on it.
    #[must_use]
    pub fn output(&self) -> Option<&wgpu::TextureView> {
        if !Pass::ALL.iter().all(|pass| pass.ported()) {
            return None;
        }
        self.targets
            .as_ref()
            .map(|targets| &targets.upscaled_output.view)
    }

    /// What the intermediates currently cost, or `None` before the first frame.
    ///
    /// Exposed because the widened formats are this port's one unmeasured cost
    /// (see [`resources`]), and a caller that can print a number is what turns
    /// that from an argument into a measurement.
    #[must_use]
    pub fn sizes(&self) -> Option<Sizes> {
        self.targets.as_ref().map(Targets::sizes)
    }

    /// The constants this frame was dispatched with, for a test.
    #[must_use]
    pub fn constants(&self) -> Option<&Constants> {
        self.written.as_ref()
    }

    /// The texture behind [`output`](Self::output), for a readback or a
    /// capture. The same accessor [`super::fsr1`] carries, for the same reason.
    #[must_use]
    pub fn output_texture(&self) -> Option<&wgpu::Texture> {
        if !Pass::ALL.iter().all(|pass| pass.ported()) {
            return None;
        }
        self.targets
            .as_ref()
            .map(|targets| &targets.upscaled_output.texture)
    }

    /// How many times the bind groups have been rebuilt, for a test.
    ///
    /// Exposed because the cache is otherwise invisible: it changes what the
    /// frame path *costs* and nothing about what it produces, so a regression
    /// that put `Cache::build` back in the per-frame path would pass every
    /// other test in this module. One per allocation and one per scene-view
    /// change is the contract; a count that tracks the frame index is the bug.
    #[must_use]
    pub fn group_rebuilds(&self) -> u64 {
        self.group_rebuilds
    }

    /// The intermediates, for a test that reads one back.
    ///
    /// Exposed because until the chain reaches [`Pass::Accumulate`] there is no
    /// *output* to check, and a port with nothing checkable in it for six more
    /// passes is a port nobody can trust. Each pass's product is a texture, and
    /// a readback against arithmetic worked out on the CPU is the only
    /// instrument available this early.
    #[must_use]
    pub fn targets(&self) -> Option<&Targets> {
        self.targets.as_ref()
    }

    /// Runs every ported pass over `frame`, reporting whether it encoded them.
    ///
    /// `timestamps` times the chain in **two halves**, which is the shape
    /// [`ChainTimestamps`] exists to explain: the six dispatches that scale
    /// with the render extent, and the two that do not.
    /// `TIMESTAMP_QUERY_INSIDE_PASSES` would be needed to bracket a *single*
    /// dispatch and is not WebGPU-portable, so two compute passes is as fine
    /// a breakdown as this can have and it is exactly the one both readers
    /// want. `None` leaves both untimed, which is what a device without
    /// [`wgpu::Features::TIMESTAMP_QUERY`] gets.
    ///
    /// **The return value is what a caller must gate `abandon` on.** A pair
    /// claimed and never written does not read back as zero - its value is
    /// unspecified and the query set is not cleared between frames - so a
    /// caller that claimed two slots and got `false` here has to give both
    /// back. `false` means neither pass was encoded, and the only way to it is
    /// the missing-`targets` return below, which sits above both.
    ///
    /// **The clear of `new_locks` is deliberately outside both.** That is a
    /// render pass, and a timestamp pair cannot span two passes; it is one
    /// hardware fast clear against eight dispatches, so the reading is the
    /// chain either way.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: Frame<'_>,
        timestamps: Option<ChainTimestamps<'_>>,
    ) -> bool {
        let dispatch = frame.dispatch;
        self.resize(device, dispatch.max_render, dispatch.upscale);

        // A reset throws the history away, which here means forgetting the
        // previous frame's constants: every `previousFrame*` field then
        // describes this frame and the frame index restarts at zero, which is
        // upstream's `resetAccumulation`.
        let previous = if dispatch.reset {
            None
        } else {
            self.previous.as_ref()
        };
        // **Unconditionally, because `frame_index` moves every frame.** This
        // once compared against the last block written and skipped a matching
        // write; no two consecutive frames can match, so the comparison never
        // saved a write and only read as though it might.
        let wanted = Constants::new(dispatch, previous);
        oag_gpu::perfprobe::write_buffer(queue, &self.constants, 0, bytemuck::bytes_of(&wanted));
        self.written = Some(wanted);
        self.previous = Some(wanted);

        if self.targets.is_none() {
            return false;
        }
        // Both parities, rebuilt only when the scene views or the sample count
        // move - `resize` above has already dropped them if an allocation did.
        // See `groups.rs`: nothing in a bind group here is per-frame.
        if !self.groups.as_ref().is_some_and(|cache| cache.fits(frame)) {
            let cache = {
                let targets = self.targets.as_ref().expect("checked just above");
                groups::Cache::build(self, device, targets, frame)
            };
            self.groups = Some(cache);
            self.group_rebuilds += 1;
        }
        let targets = self.targets.as_ref().expect("checked just above");
        let frame_index = wanted.frame_index as u64;
        let bind = self
            .groups
            .as_ref()
            .expect("built just above")
            .groups(frame_index);

        // An empty render pass is the cheapest wipe available: no pipeline,
        // no draw, just the hardware's fast clear.
        let clear = |encoder: &mut wgpu::CommandEncoder, label, view| {
            encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(label),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
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
                })
                .forget_lifetime();
        };

        // **The lock target, wiped before anything writes it.** `new_locks` is
        // written by a scatter, so most presentation texels are never touched
        // and last frame's locks would otherwise survive into this one.
        // Upstream avoids the clear by having `accumulate` zero each texel as
        // it reads it, which would need a read-write storage texture - see
        // `resources::CLEARABLE`.
        clear(encoder, "fsr3 clear new locks", &targets.new_locks.view);

        // **A reset also wipes the accumulation this frame reads**, which is
        // upstream's `resetAccumulation` clearing the accumulation SRV, and it
        // is the half of a reset that forgetting `previous` above cannot do.
        // `frame_index == 0` already makes every presentation pixel a new
        // sample, so no history *colour* is reprojected - but the history
        // *weight* comes from `prepare_reactivity`, which reads the previous
        // frame's accumulation texture, and after a race restart into the
        // same targets that texture still says "fully accumulated" at every
        // texel. `accumulate` would then blend its zeroed history in at that
        // weight against a few percent of the fresh upsample, and the first
        // frames of the new race come out darkened rather than being the
        // plain initial-sample upsample they are on a fresh allocation.
        //
        // Only the *previous* half: the current half is written before
        // anything reads it. Upstream's reset also clears `spd_mips`, which
        // this port does not need to: every level of the pyramid is rewritten
        // by the dispatch chain each frame, where upstream's single-pass
        // downsampler writes only the levels it reaches.
        if wanted.frame_index == 0.0 {
            clear(
                encoder,
                "fsr3 clear accumulation on reset",
                &targets.accumulation.previous(frame_index).view,
            );
        }

        let render = (dispatch.render.0.max(1), dispatch.render.1.max(1));

        // **Two compute passes, split where the resolution changes.** Every
        // dispatch in this one runs at the render extent or half of it, so its
        // cost falls when a resolution controller lowers the extent; the two
        // in the second do not. `drs::Cost` needs those apart to divide a
        // budget by the half that moves - see
        // [ADR-0045](../../../../docs/architecture/adr/0045-fsr3-splits-into-a-scaled-and-a-presented-reading.md).
        let (scaled, presented) = match timestamps {
            Some(pair) => (Some(pair.scaled), Some(pair.presented)),
            None => (None, None),
        };
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("fsr3 scaled"),
            timestamp_writes: scaled,
        });
        pass.set_bind_group(0, Some(&self.shared), &[]);

        pass.set_pipeline(&self.clear);
        pass.set_bind_group(1, Some(&bind.clear), &[]);
        pass.dispatch_workgroups(groups(render.0, GROUP), groups(render.1, GROUP), 1);

        pass.set_pipeline(&self.prepare_inputs[usize::from(dispatch.sample_count > 1)]);
        pass.set_bind_group(1, Some(&bind.prepare_inputs), &[]);
        pass.dispatch_workgroups(groups(render.0, GROUP), groups(render.1, GROUP), 1);

        // **Half the render extent**, which is upstream's `maxRenderSizeDiv2`
        // rounding: an integer halve, so an odd width loses its last column
        // rather than gaining a half-covered one. The shader clamps its taps
        // to the render extent for the same reason.
        let half = ((render.0 / 2).max(1), (render.1 / 2).max(1));
        pass.set_pipeline(&self.luma_pyramid);
        pass.set_bind_group(1, Some(&bind.luma_pyramid), &[]);
        pass.dispatch_workgroups(groups(half.0, GROUP), groups(half.1, GROUP), 1);

        // The pyramid, level by level. Upstream is one dispatch; this is
        // `pyramid_levels` of them, which is ADR-0012's predicted SPD
        // substitution and the port's one structural deviation.
        //
        // **Each level is its own `dispatch_workgroups` inside one compute
        // pass, and that is sufficient synchronisation**: wgpu inserts a
        // barrier between dispatches that write and then read the same
        // resource, which is exactly the dependency SPD's global atomic exists
        // to establish inside a single dispatch.
        let mut level_size = half;
        pass.set_pipeline(&self.shading_change_pyramid_mip0);
        pass.set_bind_group(1, Some(&bind.pyramid[0]), &[]);
        pass.dispatch_workgroups(groups(level_size.0, GROUP), groups(level_size.1, GROUP), 1);

        pass.set_pipeline(&self.shading_change_pyramid_reduce);
        for group in &bind.pyramid[1..] {
            level_size = ((level_size.0 / 2).max(1), (level_size.1 / 2).max(1));
            pass.set_bind_group(1, Some(group), &[]);
            pass.dispatch_workgroups(groups(level_size.0, GROUP), groups(level_size.1, GROUP), 1);
        }

        pass.set_pipeline(&self.shading_change);
        pass.set_bind_group(1, Some(&bind.shading_change), &[]);
        pass.dispatch_workgroups(groups(half.0, GROUP), groups(half.1, GROUP), 1);

        pass.set_pipeline(&self.prepare_reactivity);
        pass.set_bind_group(1, Some(&bind.prepare_reactivity), &[]);
        pass.dispatch_workgroups(groups(render.0, GROUP), groups(render.1, GROUP), 1);

        pass.set_pipeline(&self.luma_instability);
        pass.set_bind_group(1, Some(&bind.luma_instability), &[]);
        pass.dispatch_workgroups(groups(render.0, GROUP), groups(render.1, GROUP), 1);

        // **Presentation resolution, where every pass above is at render
        // resolution or half of it.** This is the one that decides a pixel,
        // and the boundary the two readings are taken either side of.
        drop(pass);
        let upscale = (dispatch.upscale.0.max(1), dispatch.upscale.1.max(1));
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("fsr3 presented"),
            timestamp_writes: presented,
        });
        pass.set_bind_group(0, Some(&self.shared), &[]);
        pass.set_pipeline(&self.accumulate);
        pass.set_bind_group(1, Some(&bind.accumulate), &[]);
        pass.dispatch_workgroups(groups(upscale.0, GROUP), groups(upscale.1, GROUP), 1);

        pass.set_pipeline(&self.rcas);
        pass.set_bind_group(1, Some(&bind.rcas), &[]);
        pass.dispatch_workgroups(groups(upscale.0, GROUP), groups(upscale.1, GROUP), 1);
        true
    }

    /// Rebuilds every intermediate when either allocation moves.
    ///
    /// **Both allocations are ceilings and neither moves per frame.** A render
    /// scale row or a window resize moves them; a resolution controller varying
    /// the extent inside the ceiling does not, which is the whole point of
    /// ADR-0037's arrangement and is what makes a per-frame render size cost a
    /// uniform write here rather than a dozen reallocations.
    fn resize(&mut self, device: &wgpu::Device, max_render: (u32, u32), upscale: (u32, u32)) {
        if self
            .targets
            .as_ref()
            .is_some_and(|targets| targets.fits(max_render, upscale))
        {
            return;
        }
        self.targets = Some(Targets::new(device, max_render, upscale));
        // Every bind group points into the targets just dropped.
        self.groups = None;
        // A new history is no history. Without this, the first frame after a
        // resize would reproject through constants describing the old one.
        self.previous = None;
    }
}

#[cfg(test)]
mod readback;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod timing_tests;
// A second test file rather than more of `tests`, which is already past 900
// lines against `just check-size`'s 1,000-line ratchet.
#[cfg(test)]
mod reset_tests;
