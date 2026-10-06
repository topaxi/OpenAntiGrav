//! The intermediate targets FSR 3.1's passes write and read, and the two
//! decisions that shaped them.
//!
//! The names are upstream's - `ffx_fsr3upscaler_resources.h`'s identifier list,
//! lowercased - so that a pass's bindings can be read against the HLSL they
//! came from. What is *not* upstream's is the format each one is held in and
//! the resource kind of one of them; both are argued in
//! [fsr3.md](../../../../docs/rendering/fsr3.md) and summarised here because
//! this is the file a reader arrives at with the question.
//!
//! # Everything is baseline-storable, and nothing asks for a feature
//!
//! A WGSL compute pass writes its output through a storage texture, and WebGPU
//! guarantees storage support for a fixed list of formats that does **not**
//! include `R16Float`, `Rg16Float` or `R8Unorm` - which is most of what
//! upstream holds these in. Rather than probe for adapter-specific format
//! features and carry two layouts, each target is held in the narrowest format
//! the baseline guarantees: `R32Float` for a scalar, `Rgba16Float` for a pair.
//!
//! That costs memory - a render-resolution scalar is four times upstream's
//! `R8_UNORM` - and buys a port that runs anywhere a compute shader does. The
//! [`Sizes`] type reports the total so the cost is a number rather than a
//! feeling.
//!
//! # Three resolutions, and the one that surprises
//!
//! Most intermediates are at the **render** allocation, a few at half of it,
//! and only four at the presentation one: `new_locks`, the two
//! `internal_upscaled` halves and `upscaled_output`. In particular
//! `accumulation` and `luma_history` are **render**-sized despite their names -
//! both are properties of the sample being reprojected rather than of the pixel
//! it lands in. This port had both wrong until a readback test caught it, and
//! the failure was silent in exactly the way that matters: presentation-sized,
//! only the top-left corner is ever written and every read past it comes back
//! zero, which reads as "no history here" rather than as an error.
//!
//! # One of them is not a texture
//!
//! `reconstructed_previous_nearest_depth` is written by a *scatter* with an
//! `InterlockedMin`, so it needs an atomic, and texture atomics are behind
//! `wgpu::Features::TEXTURE_ATOMIC` while buffer atomics are core. It is a
//! storage buffer of `atomic<u32>` indexed `y * width + x`, cleared per frame
//! by a compute dispatch - `clear_buffer` writes zero, and zero is the one
//! value `atomicMin` must not start from. See `clear.wgsl`.

use wgpu::TextureFormat;

/// An `R16_FLOAT` scalar that is only ever `textureLoad`ed.
///
/// `R32Float` and not `R16Float` because `R16Float` carries no storage
/// guarantee. Twice the bytes, and no loss: widening never costs precision.
///
/// **Only for the ones nothing samples.** `R32Float` is storable in the
/// baseline but *not filterable* - `float32-filterable` is a WebGPU feature -
/// so a target any pass reads through the linear sampler has to be
/// [`FILTERABLE`] instead. The two that live here are `dilated_depth` and
/// `farthest_depth`, from an enumeration of every `Sample*` callback the passes
/// actually call - **and the enumeration has to allow digits in the name**, or
/// it silently misses `SampleFarthestDepthMip1` and puts `farthest_depth_mip1`
/// on the wrong side of the line.
const SCALAR: TextureFormat = TextureFormat::R32Float;

/// An `R16_FLOAT` scalar that some pass reads through the linear sampler.
///
/// `Rgba16Float` is both baseline-storable *and* baseline-filterable, which is
/// the pair of properties `R16Float` has neither of and `R32Float` has only one
/// of. Four times upstream's bytes for one channel, which is the price of
/// asking for no GPU feature at all.
const FILTERABLE: TextureFormat = TextureFormat::Rgba16Float;

/// An `R8_UNORM` scalar - `accumulation`, `shading_change`, `new_locks`.
///
/// **`Rgba8Unorm` is not a widening at all**, and these are the only
/// intermediates where that is true: it is storable, filterable, and quantises
/// to the same 256 levels over the same `0..1` range that upstream's
/// `R8_UNORM` does. Three channels go unwritten; the one that matters behaves
/// exactly as upstream's.
const UNORM: TextureFormat = TextureFormat::Rgba8Unorm;

/// A two-channel intermediate: `RG16_FLOAT` upstream.
///
/// `Rgba16Float` and not `Rg16Float` because `Rg16Float` carries no storage
/// guarantee, at the cost of two unwritten channels.
const PAIR: TextureFormat = TextureFormat::Rgba16Float;

/// A colour intermediate, which upstream already holds at `RGBA16_FLOAT`.
const COLOUR: TextureFormat = TextureFormat::Rgba16Float;

/// The packed reactive/composition mask set, `R8G8B8A8_UNORM` upstream and
/// baseline-storable as it stands.
const MASKS: TextureFormat = TextureFormat::Rgba8Unorm;

/// Every FSR 3.1 target is written by a compute pass and read by a later one.
const USAGE: wgpu::TextureUsages = wgpu::TextureUsages::STORAGE_BINDING
    .union(wgpu::TextureUsages::TEXTURE_BINDING)
    .union(wgpu::TextureUsages::COPY_SRC);

/// [`USAGE`] plus `RENDER_ATTACHMENT`, for the one target that has to be
/// cleared every frame.
///
/// `new_locks` is written by a *scatter* - most presentation texels have no
/// render-resolution pixel landing on them - so stale locks would survive
/// unless something wipes it. Upstream has `accumulate` zero each texel as it
/// consumes it, which needs a read-write storage texture; a clear is the same
/// result and a render-pass clear is the cheapest way to get one, because the
/// hardware has a fast path for it that a compute shader writing zeros does not.
const CLEARABLE: wgpu::TextureUsages = USAGE.union(wgpu::TextureUsages::RENDER_ATTACHMENT);

/// One intermediate.
#[derive(Debug)]
pub struct Target {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    /// Each mip level as its own view, for the pyramid passes. One entry for a
    /// single-level target.
    pub mips: Vec<wgpu::TextureView>,
}

/// How many mip levels a `size`-wide target can hold: down to 1x1 and no
/// further.
///
/// **A clamp, not a policy.** wgpu rejects a descriptor asking for more, and a
/// small render extent really does hit it - a 4x2 half-resolution target has
/// three levels, not [`PYRAMID_MIPS`]. Found by a test that ran the passes at
/// 8x4 rather than by reading the spec, which is the argument for keeping a
/// readback fixture that small.
#[must_use]
fn mip_ceiling(size: (u32, u32)) -> u32 {
    32 - size.0.max(size.1).max(1).leading_zeros()
}

impl Target {
    fn new(
        device: &wgpu::Device,
        label: &str,
        size: (u32, u32),
        format: TextureFormat,
        mips: u32,
    ) -> Self {
        Self::with_usage(device, label, size, format, mips, USAGE)
    }

    fn with_usage(
        device: &wgpu::Device,
        label: &str,
        size: (u32, u32),
        format: TextureFormat,
        mips: u32,
        usage: wgpu::TextureUsages,
    ) -> Self {
        let mips = mips.clamp(1, mip_ceiling(size));
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: mips.max(1),
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mips = (0..mips.max(1))
            .map(|level| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some(label),
                    base_mip_level: level,
                    mip_level_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        Self {
            texture,
            view,
            mips,
        }
    }

    /// Every byte this target occupies, **mip chain included**.
    ///
    /// The chain matters for exactly one target - `spd_mips`, the only
    /// multi-level one - and there it is a third again on top of mip 0. Small
    /// against the whole allocation, and [`Sizes`] is the instrument the
    /// widened-format argument is checked with, so it reports what is actually
    /// held rather than what is nearly all of it.
    fn bytes(&self) -> u64 {
        let size = self.texture.size();
        let bytes_per_texel = u64::from(
            self.texture
                .format()
                .target_pixel_byte_cost()
                .unwrap_or(4)
                .max(1),
        );
        (0..self.texture.mip_level_count())
            .map(|level| {
                let width = u64::from(size.width >> level).max(1);
                let height = u64::from(size.height >> level).max(1);
                width * height * bytes_per_texel
            })
            .sum()
    }
}

/// A pair of targets one frame writes and the next reads.
///
/// Upstream names these `_1` and `_2` and swaps which is which on the frame
/// index. Held as a pair with an explicit `current`/`previous` rather than an
/// index into an array, because every use site cares which is which and an
/// index is one place to get the parity backwards.
#[derive(Debug)]
pub struct PingPong {
    a: Target,
    b: Target,
}

impl PingPong {
    fn new(device: &wgpu::Device, label: &str, size: (u32, u32), format: TextureFormat) -> Self {
        Self::with_usage(device, label, size, format, USAGE)
    }

    fn with_usage(
        device: &wgpu::Device,
        label: &str,
        size: (u32, u32),
        format: TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        Self {
            a: Target::with_usage(device, &format!("{label} 1"), size, format, 1, usage),
            b: Target::with_usage(device, &format!("{label} 2"), size, format, 1, usage),
        }
    }

    /// What this frame writes, given an even or odd frame index.
    #[must_use]
    pub fn current(&self, frame: u64) -> &Target {
        if frame.is_multiple_of(2) {
            &self.a
        } else {
            &self.b
        }
    }

    /// What the previous frame wrote.
    #[must_use]
    pub fn previous(&self, frame: u64) -> &Target {
        if frame.is_multiple_of(2) {
            &self.b
        } else {
            &self.a
        }
    }

    fn bytes(&self) -> u64 {
        self.a.bytes() + self.b.bytes()
    }
}

/// How many mip levels the two pyramid passes reduce to.
///
/// **Six, which is upstream's** - `ffx_fsr3upscaler_callbacks_hlsl.h` declares
/// `rw_spd_mip0` through `rw_spd_mip5` and no more, and the shading-change pass
/// reads a fixed level of that pyramid. It is not a "reduce until 1x1" chain,
/// so a larger render resolution gets a shallower pyramid relative to itself,
/// exactly as upstream does.
pub const PYRAMID_MIPS: u32 = 6;

/// Every intermediate, sized for one render allocation and one presentation
/// rectangle.
///
/// Allocated at the **ceiling** on both sides - `max_render` and `max_upscale`
/// in upstream's terms - and never reallocated for a frame drawn smaller than
/// that. This is the same arrangement
/// [ADR-0037](../../../../docs/architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md)
/// made for the scene target, and FSR 3.1 asks for it directly: `renderSize`
/// and `maxRenderSize` are separate constants precisely so a temporal upscaler
/// can take a per-frame render size without anything being rebuilt.
///
/// **Upstream's `FSR3UPSCALER_FrameInfo` is deliberately absent**, and its
/// absence was checked rather than assumed. It is a 1x1 `RGBA32_FLOAT` holding
/// an exposure, a smoothed log luma and a scene average luma, written by the
/// luma pyramid's 1x1 level. Nothing in this port can read it: `Exposure()`
/// only comes from it when `FFX_FSR3UPSCALER_ENABLE_AUTO_EXPOSURE` is set and
/// this renderer has no exposure at all (see `common.wesl`'s `exposure`), and
/// `SceneAverageLuma()` is **dead in v1.1.4** - declared in
/// `ffx_fsr3upscaler_common.h` and called from no pass header. Allocating it
/// would be allocating a target with neither a reader nor a writer.
#[derive(Debug)]
pub struct Targets {
    /// The render allocation these were built for.
    pub max_render: (u32, u32),
    /// The presentation allocation these were built for.
    pub max_upscale: (u32, u32),

    /// `ReconstructPrevDepth`'s scattered `InterlockedMin` target - a buffer,
    /// not a texture. See this module's own documentation.
    pub reconstructed_previous_nearest_depth: wgpu::Buffer,
    pub dilated_motion_vectors: Target,
    pub dilated_depth: Target,
    pub farthest_depth: Target,
    pub dilated_reactive_masks: Target,
    /// `LUMA_1`/`LUMA_2`: this frame's luma and the previous frame's.
    pub luma: PingPong,
    pub luma_instability: Target,

    /// `FARTHEST_DEPTH_MIP1`, at half the render resolution.
    pub farthest_depth_mip1: Target,
    /// `SPD_MIPS`, at half the render resolution with [`PYRAMID_MIPS`] levels.
    pub spd_mips: Target,
    /// `SHADING_CHANGE`, at half the render resolution.
    pub shading_change: Target,

    /// `LUMA_HISTORY_1`/`_2`, at **render** resolution.
    pub luma_history: PingPong,
    /// `ACCUMULATION_1`/`_2`, at **render** resolution.
    ///
    /// **Render and not presentation**, which is worth stating because it is
    /// the opposite of what the name suggests and this port had it wrong until
    /// a readback test caught it. Accumulation is a property of the *sample*
    /// being reprojected, not of the pixel it lands in, so it is indexed by the
    /// render grid throughout - `UpdateAccumulation` stores at the
    /// render-resolution dispatch position. Sized presentation-wide, only the
    /// top-left corner would ever be written and every read past it would come
    /// back zero, which reads as "this pixel has no history" everywhere.
    ///
    /// **[`CLEARABLE`], because a reset has to wipe the half this frame
    /// reads.** Upstream's `resetAccumulation` clears the accumulation SRV
    /// before the frame runs, so that every texel reads as having no history
    /// and `accumulate` takes its initial-sample path; without that, a reset
    /// frame reads whatever depth of history the *previous* race left in the
    /// texture and blends its zeroed history colour in at that weight. See
    /// `Fsr3::render`.
    pub accumulation: PingPong,
    /// `INTERNAL_UPSCALED_COLOR_1`/`_2`, at presentation resolution.
    pub internal_upscaled: PingPong,
    pub new_locks: Target,
    /// What the last pass writes and [`super::Fsr3::output`] hands back.
    pub upscaled_output: Target,
}

/// What one allocation of [`Targets`] costs, in bytes, split the three ways a
/// reader would want to reduce it.
///
/// Reported rather than merely paid because the widened formats are this port's
/// one unmeasured cost and a number is what makes that argument checkable. See
/// the module documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sizes {
    /// Targets at the render allocation.
    pub render: u64,
    /// Targets at half the render allocation, the pyramid's whole mip chain
    /// included.
    pub half_render: u64,
    /// Targets at the presentation allocation.
    pub upscale: u64,
}

impl Sizes {
    /// Every byte, which is the number that matters on a handheld.
    #[must_use]
    pub fn total(self) -> u64 {
        self.render + self.half_render + self.upscale
    }
}

impl Targets {
    /// Allocates every intermediate for a `max_render` scene allocation
    /// resolved to `max_upscale`.
    #[must_use]
    pub fn new(device: &wgpu::Device, max_render: (u32, u32), max_upscale: (u32, u32)) -> Self {
        let render = (max_render.0.max(1), max_render.1.max(1));
        let upscale = (max_upscale.0.max(1), max_upscale.1.max(1));
        // Upstream's `maxRenderSizeDiv2`, and its rounding: a plain integer
        // halve, so an odd width loses its last column rather than gaining a
        // half-covered one.
        let half = ((render.0 / 2).max(1), (render.1 / 2).max(1));

        let scalar = |label: &str, size| Target::new(device, label, size, SCALAR, 1);

        Self {
            max_render: render,
            max_upscale: upscale,

            reconstructed_previous_nearest_depth: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("fsr3 reconstructed previous nearest depth"),
                // One `u32` per render texel. `STORAGE` alone, because the
                // only thing that ever writes this is a shader: `clear.wgsl`
                // fills it and `prepare_inputs` scatters into it, so there is
                // no host-side write and nothing needs `COPY_DST`. **Not
                // `clear_buffer`**, which writes zero and only zero - see
                // `clear.wgsl` for why zero is the one value that cannot be
                // used here.
                size: u64::from(render.0) * u64::from(render.1) * 4,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
            dilated_motion_vectors: Target::new(
                device,
                "fsr3 dilated motion vectors",
                render,
                PAIR,
                1,
            ),
            dilated_depth: scalar("fsr3 dilated depth", render),
            farthest_depth: scalar("fsr3 farthest depth", render),
            dilated_reactive_masks: Target::new(
                device,
                "fsr3 dilated reactive masks",
                render,
                MASKS,
                1,
            ),
            luma: PingPong::new(device, "fsr3 luma", render, FILTERABLE),
            luma_instability: Target::new(device, "fsr3 luma instability", render, FILTERABLE, 1),

            farthest_depth_mip1: Target::new(
                device,
                "fsr3 farthest depth mip1",
                half,
                FILTERABLE,
                1,
            ),
            spd_mips: Target::new(device, "fsr3 spd mips", half, PAIR, PYRAMID_MIPS),
            shading_change: Target::new(device, "fsr3 shading change", half, UNORM, 1),

            luma_history: PingPong::new(device, "fsr3 luma history", render, COLOUR),
            accumulation: PingPong::with_usage(
                device,
                "fsr3 accumulation",
                render,
                UNORM,
                CLEARABLE,
            ),

            internal_upscaled: PingPong::new(device, "fsr3 internal upscaled", upscale, COLOUR),
            new_locks: Target::with_usage(device, "fsr3 new locks", upscale, UNORM, 1, CLEARABLE),
            upscaled_output: Target::new(device, "fsr3 upscaled output", upscale, COLOUR, 1),
        }
    }

    /// Whether these were built for exactly these two allocations.
    #[must_use]
    pub fn fits(&self, max_render: (u32, u32), max_upscale: (u32, u32)) -> bool {
        self.max_render == (max_render.0.max(1), max_render.1.max(1))
            && self.max_upscale == (max_upscale.0.max(1), max_upscale.1.max(1))
    }

    /// What this allocation costs.
    #[must_use]
    pub fn sizes(&self) -> Sizes {
        Sizes {
            render: self.reconstructed_previous_nearest_depth.size()
                + self.dilated_motion_vectors.bytes()
                + self.dilated_depth.bytes()
                + self.farthest_depth.bytes()
                + self.dilated_reactive_masks.bytes()
                + self.luma.bytes()
                + self.luma_instability.bytes()
                + self.luma_history.bytes()
                + self.accumulation.bytes(),
            half_render: self.farthest_depth_mip1.bytes()
                + self.spd_mips.bytes()
                + self.shading_change.bytes(),
            upscale: self.internal_upscaled.bytes()
                + self.new_locks.bytes()
                + self.upscaled_output.bytes(),
        }
    }
}
