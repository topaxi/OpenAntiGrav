//! AMD FidelityFX Super Resolution 3.1: the temporal upscaler.
//!
//! Eight compute passes, transliterated from AMD's MIT-licensed FidelityFX SDK
//! `v1.1.4`. The route - port to WGSL, drive no native SDK - is
//! [ADR-0012](../../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md);
//! what is ported, what deviates and why is
//! [fsr3.md](../../../../docs/rendering/fsr3.md), which is the page to read
//! before changing anything here.
//!
//! **This is in flight.** [`Fsr3::PASSES`] is the spine and says which passes
//! exist; the rest of the chain is not built, so [`Fsr3::output`] is `None` and
//! nothing selects this yet. The thread is
//! `handover/fsr-3-1-is-a-seven-pass-port-and.md`.
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
//! # Colour space
//!
//! **Linear light**, which is the opposite of [`super::fsr1`] and is the
//! "fourth thing" [`super`]'s own table always listed. Accumulation averages
//! several frames of one surface; an average of sRGB-encoded values weights a
//! dark sample as brighter than it is. So the caller hands [`Frame::colour`] an
//! **sRGB view** - the one that decodes on read - where FSR 1 is handed the
//! non-sRGB one, and must not decode [`Fsr3::output`] again.
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

mod constants;
mod resources;

pub use constants::{Camera, Constants, Dispatch, camera_from_projection};
pub use resources::{Sizes, Targets};

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

/// One frame's worth of input.
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    /// An **sRGB** view of the scene target - the one that decodes to linear
    /// light on read. See the module docs; FSR 1 wants the opposite view of the
    /// same texture.
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
        matches!(self, Self::PrepareInputs | Self::LumaPyramid)
    }
}

/// The `Fsr3Constants` uniform, the two samplers, and nothing else - group 0 of
/// every pass.
///
/// One shared layout rather than one per pass, for the reason
/// [`super::motion_blur`] gives for its own: a binding a pass does not read
/// costs nothing, and four near-identical layouts is four places for a binding
/// number to drift from the WGSL.
fn shared_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let sampler = |binding, ty| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Sampler(ty),
        count: None,
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("fsr3 shared"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            sampler(1, wgpu::SamplerBindingType::NonFiltering),
            sampler(2, wgpu::SamplerBindingType::Filtering),
        ],
    })
}

/// A texture read with `textureLoad`, which is what every pass here does.
///
/// `filterable: false` because the depth attachment binds through this too and
/// a depth format is unfilterable - the same entry
/// [`super::motion_blur`] needed for the same reason.
fn load_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

/// A write-only storage texture at `binding`.
///
/// **Write-only, never `ReadWrite`.** Read-write access carries no baseline
/// guarantee, and a pass that needs to read what an earlier one wrote binds it
/// through [`load_entry`] instead - every target here carries
/// `TEXTURE_BINDING` as well as `STORAGE_BINDING` for exactly that.
fn store_entry(binding: u32, format: wgpu::TextureFormat) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format,
            view_dimension: wgpu::TextureViewDimension::D2,
        },
        count: None,
    }
}

/// A read-write storage buffer at `binding` - the atomic scatter target, and
/// the one resource in this port that is not a texture.
fn atomic_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// The prelude every pass is compiled with. See `common.wgsl`'s own header for
/// why concatenation rather than an include.
const COMMON: &str = include_str!("fsr3/common.wgsl");

/// `common.wgsl` followed by one pass's source.
fn source(pass: &str) -> String {
    format!("{COMMON}\n{pass}")
}

/// How many threads each of upstream's render-resolution passes covers, in each
/// axis - `FFX_FSR3UPSCALER_THREAD_GROUP_WIDTH` and `..._HEIGHT`.
const GROUP: u32 = 8;

/// The one-dimensional group size of the clear, which is not upstream's pass
/// and has no upstream number to match.
const CLEAR_GROUP: u32 = 64;

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
    prepare_inputs_layout: wgpu::BindGroupLayout,
    prepare_inputs: wgpu::ComputePipeline,
    luma_pyramid_layout: wgpu::BindGroupLayout,
    luma_pyramid: wgpu::ComputePipeline,

    targets: Option<Targets>,
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
        let prepare_inputs_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("fsr3 prepare inputs"),
                entries: &[
                    load_entry(0),
                    load_entry(1),
                    load_entry(2),
                    store_entry(3, wgpu::TextureFormat::Rgba16Float),
                    store_entry(4, wgpu::TextureFormat::R32Float),
                    atomic_entry(5),
                    store_entry(6, wgpu::TextureFormat::R32Float),
                    store_entry(7, wgpu::TextureFormat::R32Float),
                ],
            });
        let luma_pyramid_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("fsr3 luma pyramid"),
                entries: &[load_entry(0), store_entry(1, wgpu::TextureFormat::R32Float)],
            });

        let pipeline = |label: &str, wgsl: &str, entry: &str, own: &wgpu::BindGroupLayout| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source(wgsl).into()),
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
            include_str!("fsr3/clear.wgsl"),
            "cs_clear_reconstructed_depth",
            &clear_layout,
        );
        let prepare_inputs = pipeline(
            "fsr3 prepare inputs",
            include_str!("fsr3/prepare_inputs.wgsl"),
            "cs_prepare_inputs",
            &prepare_inputs_layout,
        );
        let luma_pyramid = pipeline(
            "fsr3 luma pyramid",
            include_str!("fsr3/luma_pyramid.wgsl"),
            "cs_luma_pyramid",
            &luma_pyramid_layout,
        );

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
            targets: None,
        })
    }

    /// The resolved frame, in **linear light**. `None` until the chain is
    /// complete enough to produce one.
    ///
    /// **Deliberately `None` while the port is in flight**, keyed off
    /// [`Pass::ported`] rather than off whether a texture happens to exist:
    /// handing back the last written intermediate would put a half-resolved
    /// picture on screen, and a half-resolved picture reads as a rendering bug
    /// rather than as unfinished work. Nothing is the honest answer, and it is
    /// the one the caller's fallback ladder is built to handle.
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

    /// Runs every ported pass over `frame`.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: Frame<'_>,
    ) {
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
        let wanted = Constants::new(dispatch, previous);
        if self.written != Some(wanted) {
            queue.write_buffer(&self.constants, 0, bytemuck::bytes_of(&wanted));
            self.written = Some(wanted);
        }
        self.previous = Some(wanted);

        let Some(targets) = &self.targets else {
            return;
        };

        let clear_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fsr3 clear"),
            layout: &self.clear_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: targets
                    .reconstructed_previous_nearest_depth
                    .as_entire_binding(),
            }],
        });
        let prepare_inputs_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fsr3 prepare inputs"),
            layout: &self.prepare_inputs_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(frame.velocity),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(frame.depth),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(frame.colour),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(
                        &targets.dilated_motion_vectors.view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&targets.dilated_depth.view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: targets
                        .reconstructed_previous_nearest_depth
                        .as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&targets.farthest_depth.view),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(
                        &targets.luma.current(wanted.frame_index as u64).view,
                    ),
                },
            ],
        });
        let luma_pyramid_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fsr3 luma pyramid"),
            layout: &self.luma_pyramid_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&targets.farthest_depth.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&targets.farthest_depth_mip1.view),
                },
            ],
        });

        let render = (dispatch.render.0.max(1), dispatch.render.1.max(1));
        let texels = render.0 * render.1;

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("fsr3"),
            timestamp_writes: None,
        });
        pass.set_bind_group(0, Some(&self.shared), &[]);

        pass.set_pipeline(&self.clear);
        pass.set_bind_group(1, Some(&clear_group), &[]);
        pass.dispatch_workgroups(groups(texels, CLEAR_GROUP), 1, 1);

        pass.set_pipeline(&self.prepare_inputs);
        pass.set_bind_group(1, Some(&prepare_inputs_group), &[]);
        pass.dispatch_workgroups(groups(render.0, GROUP), groups(render.1, GROUP), 1);

        // **Half the render extent**, which is upstream's `maxRenderSizeDiv2`
        // rounding: an integer halve, so an odd width loses its last column
        // rather than gaining a half-covered one. The shader clamps its taps
        // to the render extent for the same reason.
        let half = ((render.0 / 2).max(1), (render.1 / 2).max(1));
        pass.set_pipeline(&self.luma_pyramid);
        pass.set_bind_group(1, Some(&luma_pyramid_group), &[]);
        pass.dispatch_workgroups(groups(half.0, GROUP), groups(half.1, GROUP), 1);
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
        // A new history is no history. Without this, the first frame after a
        // resize would reproject through constants describing the old one.
        self.previous = None;
    }
}

#[cfg(test)]
mod readback;
#[cfg(test)]
mod tests;
