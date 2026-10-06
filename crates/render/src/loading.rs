//! The loading screen's procedural wave.
//!
//! Reproduces `Loading_DrawWave` (`0x0890a8e4`), documented at
//! `docs/ghidra/functions/psp-pulse-usa/loading-screen.md`. There is no loading
//! *movie* in either build - the band is generated every frame, and the only
//! asset it touches is one 32x32 cyan glow strip
//! (`data/defaults/loading/LoadingPulseOverlay.mip`). That is why this can be
//! drawn while the thing being loaded does not exist yet, which is the whole
//! reason it is worth having.
//!
//! # The randomness is spatial, not temporal
//!
//! The original zeroes its six-float state array at the top of every call and
//! walks it along **screen X**, not along time. One frame's wave is therefore a
//! single random walk read left to right: pinned flat at the left edge, fully
//! developed at the right. Re-running it with fresh draws each frame is what
//! makes it wriggle. Getting this backwards - integrating over time instead -
//! produces a wave that looks superficially similar and behaves nothing like
//! the original, so it is the first thing the tests below pin.
//!
//! Only three things survive a frame: the envelope phase, the finished flag,
//! and the caller's [`Rng`].
//!
//! # Resolution independence is a deliberate divergence
//!
//! The original is written in PSP pixels: 240 columns of 2x32 quads, baseline
//! at y=220 on a 272-line display, and two X ramps with bounds `(10, 350)` and
//! `(30, 286)`. **The PS2 port kept every one of those numbers on a 640-wide
//! screen**, so its wave spans 480 of 640 pixels and its ramps sit in the wrong
//! place - a real bug in the original, tabulated at
//! `docs/ghidra/functions/ps2-pulse-eu/loading-screen.md`. We do not reproduce
//! it. Everything here is computed in the PSP's 480x272 reference space and
//! mapped to the target framebuffer at the end, in the same category of
//! deliberate divergence as [ADR-0007](../../../docs/architecture/adr/0007-fixed-timestep-vs-original.md)'s
//! fixed timestep.
//!
//! **The column count is not a resolution parameter.** [`COLUMNS`] is the random
//! walk's step count, so it is part of the waveform's identity: adding columns
//! on a wider screen makes the walk finer and visibly changes the wave's
//! character. The quad *width* scales; the count does not.
//!
//! # The simulation half and the drawing half
//!
//! [`Wave`] and [`Wave::quads`] are the recovered algorithm and need no GPU;
//! [`Pipeline`] draws what they produce. The split is the one
//! [`oag_fx::sparks`] uses, for the same reason: the motion is testable, and
//! is tested, without an adapter.

use anyhow::{Context, Result};
use oag_core::rng::Rng;

/// Reference display width, in the pixels every constant here is written in.
pub const REF_WIDTH: f32 = 480.0;

/// Reference display height.
pub const REF_HEIGHT: f32 = 272.0;

/// The wave's own numbers, all of them Pulse's: `oag_pulse::loading`.
///
/// [`COLUMNS`], [`BANDS`], [`BASELINE_Y`], [`STRIP_SIZE`], [`ENVELOPE`] and the
/// rest moved to the title package under [ADR-0022]. They came out of
/// `Loading_DrawWave` and the `.rodata` beside it, so they are facts about what
/// this release ships; the algorithm below is the mechanism and stays here.
///
/// [`REF_WIDTH`] and [`REF_HEIGHT`] deliberately did **not** move: 480x272 is the
/// PSP's screen, a console fact under [ADR-0004], and this module's own reference
/// space rather than anything a second title would restate.
///
/// [ADR-0004]: ../../../docs/architecture/adr/0004-asset-pipeline.md
/// [ADR-0022]: ../../../docs/architecture/adr/0022-title-packages.md
pub use oag_pulse::loading::{
    ALPHA_RAMP, AMPLITUDE_RAMP, BANDS, BASELINE_Y, BLEND_WEIGHTS, COLUMNS, ENERGY_DAMPING,
    ENERGY_IMPULSE, ENVELOPE, ENVELOPE_FLOOR, ENVELOPE_PEAK, LAYER_RATES, SLEW_DEADBAND, SLEW_STEP,
    STRIP_SIZE,
};

/// A clamped integer ramp, `((t - lo) * 255) / (hi - lo)`.
///
/// `Loading_Ramp255` (`0x0890a280`). Integer throughout, including the divide,
/// because the original is - a float version drifts by up to one step and there
/// is no reason to introduce that.
#[must_use]
pub fn ramp255(lo: i32, hi: i32, t: i32) -> i32 {
    if hi <= lo {
        return 255;
    }
    (((t - lo) * 255) / (hi - lo)).clamp(0, 255)
}

/// Moves `current` toward `target`, but only once the gap is worth moving for.
///
/// `Loading_SlewToward` (`0x0890a3e8`). The deadband is what makes the third
/// band lag visibly rather than tracking: it does not move at all until it is
/// more than `deadband` behind, and then by at most `step`.
#[must_use]
pub fn slew_toward(current: f32, target: f32, step: f32, deadband: f32) -> f32 {
    let gap = target - current;
    if gap.abs() <= deadband {
        return current;
    }
    current + if gap > 0.0 { step } else { -step }
}

/// The texture column this screen column samples.
///
/// `u = x & 0x3f; if u > 0x1f { u = 0x3f - u }` - a triangle wave, so the strip
/// is mirror-tiled with period 64 and its scanlines never show a seam.
///
/// **How wide a slice each column takes is not pinned.** The original sets up a
/// 32x32 source against a 2x32 destination and then passes `u` per column,
/// which is over-determined as written; the recovered geometry carries
/// confidence 88 for this reason. One texel per column is taken here because it
/// is the only reading that stays inside the strip - `u` reaches 31 at `x = 32`,
/// so a two-texel slice would sample past the right edge. Worth settling with a
/// runtime capture, which that page already lists as its missing step.
#[must_use]
pub fn texture_column(x: i32) -> i32 {
    let u = x & 0x3f;
    if u > 0x1f { 0x3f - u } else { u }
}

/// One column's three band offsets, in reference pixels from [`BASELINE_Y`].
pub type Column = [f32; BANDS];

/// One quad, in normalised screen space: origin top left, `0..1` on both axes.
///
/// `u0`/`u1` are normalised texture coordinates into the 32x32 glow strip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quad {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
    /// Left texture coordinate.
    pub u0: f32,
    /// Right texture coordinate.
    pub u1: f32,
    /// Alpha from the across-screen ramp, `0..1`.
    pub alpha: f32,
}

/// Everything about the wave that outlives a frame.
///
/// Which is very little, deliberately - see the module doc comment. The state
/// the wave *looks* like it should carry is rebuilt from scratch every frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Wave {
    phase: u32,
    finished: bool,
}

impl Wave {
    /// A wave at the start of its first beat.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances the heartbeat by one frame.
    ///
    /// Does nothing once [`Wave::finish`] has been called: the original gates
    /// both the phase and the motion on `g_loading_finished`, so the wave
    /// freezes in place while the screen fades out rather than continuing to
    /// animate under the fade.
    pub fn advance(&mut self) {
        if !self.finished {
            self.phase = (self.phase + 1) % ENVELOPE.len() as u32;
        }
    }

    /// Freezes the wave, for when whatever was loading has finished.
    pub fn finish(&mut self) {
        self.finished = true;
    }

    /// Whether the wave is frozen.
    #[must_use]
    pub fn is_finished(self) -> bool {
        self.finished
    }

    /// Index into [`ENVELOPE`].
    #[must_use]
    pub fn phase(self) -> u32 {
        self.phase
    }

    /// This frame's envelope value, before the floor is applied.
    #[must_use]
    pub fn pulse(self) -> f32 {
        ENVELOPE[self.phase as usize % ENVELOPE.len()]
    }

    /// Runs the walk across X and returns every column's band offsets, in
    /// reference pixels.
    ///
    /// Draws from `rng` twice per column, which is the only reason two calls
    /// with the same [`Wave`] differ. A frozen wave still walks - the original
    /// keeps drawing the band it froze - but the caller stops advancing it.
    #[must_use]
    pub fn columns(&self, rng: &mut Rng) -> Vec<Column> {
        // Zeroed here, once per call, exactly as the original zeroes its stack
        // array. These are carried along X within this frame and thrown away
        // at the end of it.
        let mut raw = [0.0f32; 2];
        let mut display = [0.0f32; 2];
        let mut energy = [0.0f32; 2];

        let pulse = self.pulse() / ENVELOPE_PEAK + ENVELOPE_FLOOR;
        let mut out = Vec::with_capacity(COLUMNS);

        for column in 0..COLUMNS {
            let x = column as i32 * 2;
            let envelope = ramp255(AMPLITUDE_RAMP.0, AMPLITUDE_RAMP.1, x) as f32 / 255.0;

            for layer in 0..2 {
                // The target is read from *this* column's energy, before the
                // impulse below updates it. That ordering is what leaves the
                // first column flat, and it is visible in the original as the
                // wave being pinned at the left edge.
                let target = energy[layer] * envelope * pulse;
                raw[layer] += (target - raw[layer]) * LAYER_RATES[layer];
                display[layer] = slew_toward(display[layer], raw[layer], SLEW_STEP, SLEW_DEADBAND);
                let impulse = (rng.next_f32() - 0.5) * ENERGY_IMPULSE;
                energy[layer] = (energy[layer] + impulse) * ENERGY_DAMPING;
            }

            // The third band is a separate, calmer one built from the
            // slew-limited pair, not a filter applied to the other two.
            let blended = display[0] * BLEND_WEIGHTS[0] + display[1] * BLEND_WEIGHTS[1];
            out.push([raw[0], raw[1], blended]);
        }

        out
    }

    /// Lays the wave out as quads in normalised screen space.
    ///
    /// Takes no framebuffer size, which *is* the divergence: every figure is a
    /// fraction of the reference display, so the band spans the full width and
    /// sits at the same fraction of the height at any resolution, and there is
    /// no size for a caller to get wrong. The original works in PSP pixels, and
    /// its PS2 port shipping those same pixels on a 640-wide screen is the bug
    /// this avoids by construction.
    #[must_use]
    pub fn quads(&self, columns: &[Column]) -> Vec<Quad> {
        let column_w = 1.0 / COLUMNS as f32;
        let strip_h = STRIP_SIZE / REF_HEIGHT;
        let mut out = Vec::with_capacity(columns.len() * BANDS);

        for (index, bands) in columns.iter().enumerate() {
            let x = index as i32 * 2;
            let u = texture_column(x) as f32;
            let alpha = ramp255(ALPHA_RAMP.0, ALPHA_RAMP.1, x) as f32 / 255.0;

            for offset in bands {
                // The quad is anchored by its centre line, the way a 32-tall
                // strip drawn at a baseline is.
                let y = (BASELINE_Y + offset) / REF_HEIGHT - strip_h * 0.5;
                out.push(Quad {
                    x: index as f32 * column_w,
                    y,
                    w: column_w,
                    h: strip_h,
                    u0: u / STRIP_SIZE,
                    u1: (u + 1.0) / STRIP_SIZE,
                    alpha,
                });
            }
        }

        out
    }
}

/// One vertex of the overlay.
///
/// Two floats of position rather than three, and no normal, colour or `lit`
/// flag: this is a screen-space overlay, so there is nothing for a light rig or
/// a camera to do. That is why it does not reuse
/// [`oag_mesh::mesh::GpuVertex`] the way [`oag_fx::exhaust`] and [`oag_fx::sparks`]
/// do - they draw in the world and this does not.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    /// Normalised screen space, origin top left, `0..1` on both axes.
    pub position: [f32; 2],
    /// Into the 32x32 glow strip.
    pub texcoord: [f32; 2],
    /// The across-screen ramp, `0..1`.
    pub alpha: f32,
}

/// Six vertices - two triangles - for one [`Quad`].
///
/// `v` runs 0 at the quad's top edge to 1 at its bottom, because
/// [`oag_texture::texture::Texture::to_rgba`] is row-major from the top left
/// and the strip is uploaded in that order. Wound as an explicit triangle list
/// rather than a strip, matching the rest of the crate.
fn quad_vertices(quad: &Quad) -> [GpuVertex; 6] {
    let corner = |x: f32, y: f32, u: f32, v: f32| GpuVertex {
        position: [x, y],
        texcoord: [u, v],
        alpha: quad.alpha,
    };
    let tl = corner(quad.x, quad.y, quad.u0, 0.0);
    let tr = corner(quad.x + quad.w, quad.y, quad.u1, 0.0);
    let bl = corner(quad.x, quad.y + quad.h, quad.u0, 1.0);
    let br = corner(quad.x + quad.w, quad.y + quad.h, quad.u1, 1.0);
    [tl, bl, tr, bl, br, tr]
}

/// Expands [`Wave::quads`] into the vertex buffer [`Pipeline`] draws.
///
/// Kept separate from `quads` so the geometry stays testable without any of
/// this module's GPU half existing.
#[must_use]
pub fn vertices(quads: &[Quad]) -> Vec<GpuVertex> {
    let mut out = Vec::with_capacity(quads.len() * 6);
    for quad in quads {
        out.extend_from_slice(&quad_vertices(quad));
    }
    out
}

/// Most vertices [`Pipeline`]'s buffer holds: one quad per band per column.
pub const MAX_VERTICES: usize = COLUMNS * BANDS * 6;

/// The overlay's blend: `dst + src.rgb * src.a`, the same shape
/// [`oag_fx::exhaust::BLEND`] and [`oag_fx::sparks::BLEND`] use.
///
/// Additive rather than alpha-over for two reasons, and the second is the one
/// that decides it:
///
/// 1. It is a glow on black. Three bands overlap in every column and overlap
///    should read as *brighter*, not as one band occluding another.
/// 2. **The original's across-screen ramp is a colour tint, not an alpha.**
///    `Loading_DrawWave` ramps the vertex colour from `0xff000000` to
///    `0xff808080` - full alpha throughout, rgb rising. Under this blend the
///    two are the same operation, since `src.rgb * src.a` is exactly
///    "scale the texel's brightness by the ramp". Under alpha-over they are
///    not, and [`Quad::alpha`] would then mean something the original never
///    computed.
pub const BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// The 32x32 glow strip, as RGBA8.
///
/// Normally `data/defaults/loading/LoadingPulseOverlay.mip` - `Data.wad` entry
/// 68, name hash `d857f34b`, 2,064 bytes on the PSP, and the same declared name
/// under `.pct` as `WADS2.WAD` entry 138 (`312beffb`, 2,317 bytes) on the PS2.
/// See `oag_pulse::ps2_texture_name`. Unlike
/// [`oag_fx::exhaust::FlareTexture`], which takes pixels the caller decoded, this
/// carries its own [`GlowStrip::decode`]: `oag-render` already depends on
/// `oag-formats`, there is exactly one blob this pipeline ever wants, and
/// pushing the parse onto the caller only spreads that one fact around. The
/// caller still owns the archive - this takes bytes, never a path or a disc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlowStrip {
    /// Width in texels.
    pub width: u32,
    /// Height in texels.
    pub height: u32,
    /// `width * height * 4` bytes, RGBA8.
    pub rgba: Vec<u8>,
}

impl GlowStrip {
    /// Decodes a `.mip` blob, or the `.pct` the PS2 keeps the same strip in.
    ///
    /// The PSP format is tried first and **its** error is the one reported, for
    /// the reason `oag_hud::sprite::Image::decode` gives: a `.mip` that will
    /// not parse is the commoner failure, and the PS2 parser's complaint about
    /// a PSP blob points at the wrong decoder.
    ///
    /// # Errors
    ///
    /// Propagates whatever [`oag_texture::texture::Texture::parse`] rejects,
    /// when [`oag_texture::ps2_texture::parse`] will not take the blob either.
    /// The size arithmetic in both is exact, so a failure means the blob is not
    /// a texture rather than that it is a damaged one.
    pub fn decode(blob: &[u8]) -> Result<Self> {
        let psp = match oag_texture::texture::Texture::parse(blob) {
            Ok(texture) => {
                return Ok(Self {
                    width: u32::from(texture.width),
                    height: u32::from(texture.height),
                    rgba: texture.to_rgba(),
                });
            }
            Err(e) => e,
        };
        if let Ok(texture) = oag_texture::ps2_texture::parse(blob) {
            return Ok(Self {
                width: u32::from(texture.width),
                height: u32::from(texture.height),
                rgba: texture.to_rgba(),
            });
        }
        Err(anyhow::anyhow!("{psp}"))
            .with_context(|| format!("decoding the glow strip, {} bytes", blob.len()))
    }

    /// An authored stand-in, for when the disc's own strip is unavailable.
    ///
    /// Used by the tests below, which must run without a disc image. **Not** a
    /// silent substitute in the game: a caller that cannot read entry 68 should
    /// say so, for the reason [`oag_fx::exhaust::FlareTexture::placeholder`]
    /// gives - a plausible-looking stand-in is how a decode failure survives
    /// review.
    ///
    /// Authored to the *character* the RE page describes rather than copied
    /// from the asset: dark at the top and bottom rows, brighter towards the
    /// middle, cut by a few bright scanlines, on a black-to-cyan ramp ending at
    /// the documented `(222, 255, 255)`. Nothing here is measured off the
    /// original's pixels, which would be reproducing game content.
    #[must_use]
    pub fn placeholder(size: u32) -> Self {
        let size = size.max(2);
        let mut rgba = Vec::with_capacity((size * size * 4) as usize);
        for y in 0..size {
            // Exactly -1 and +1 on the outermost rows, so the strip fades to
            // black at both edges instead of clipping.
            let t = (y as f32 / (size - 1) as f32) * 2.0 - 1.0;
            let bell = 1.0 - t * t;
            let scanline = if y % 5 == 1 { 1.0 } else { 0.45 };
            let level = bell * scanline;
            for _ in 0..size {
                let channel = |peak: f32| (peak * level) as u8;
                rgba.extend_from_slice(&[
                    channel(222.0),
                    channel(255.0),
                    channel(255.0),
                    // Constant, matching the real palette: every one of its 256
                    // entries is opaque, so the alpha channel carries no shape.
                    255,
                ]);
            }
        }
        Self {
            width: size,
            height: size,
            rgba,
        }
    }

    fn bind(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        let size = wgpu::Extent3d {
            width: self.width.max(1),
            height: self.height.max(1),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("loading glow strip"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Raw, not sRGB, exactly as `FlareTexture::bind` uploads: the
            // texels feed an additive blend, so an encode on the sample would
            // change the arithmetic the blend is doing.
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.width.max(1) * 4),
                rows_per_image: Some(self.height.max(1)),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // Clamped on both axes. A column samples one texel of `u` and the full
        // height of `v`, so neither coordinate ever leaves `[0, 1]` - and
        // clamping means a `u` at the very edge cannot wrap round and pull in
        // the opposite side of the strip under bilinear filtering.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("loading glow strip"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("loading glow strip"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        })
    }
}

/// Size of the tint uniform block.
const TINT_SIZE: u64 = std::mem::size_of::<[f32; 4]>() as u64;

/// The wave's draw pipeline.
///
/// Modelled on [`oag_fx::sparks::Pipeline`] - `new`/`upload`/`draw`, every entry
/// point taking the device and queue the caller already has - with two
/// differences, both because this is a 2D overlay:
///
/// - **No depth.** `depth_stencil` is `None` here, where the world-space
///   effects share [`oag_mesh::mesh_render::DEPTH_FORMAT`]. The overlay is drawn
///   over a finished frame and there is nothing for it to be occluded by. A
///   pass drawing it must therefore not attach a depth target either.
/// - **No view-projection.** The vertices arrive in normalised screen space, so
///   the uniform block holds a tint rather than a camera.
#[derive(Debug)]
pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    texture: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    /// Vertices actually uploaded by the last [`Pipeline::upload`].
    count: u32,
}

impl Pipeline {
    /// Builds the pipeline and uploads the glow strip.
    ///
    /// `format` must be the target the caller's render pass writes, and
    /// `sample_count` must match its multisample state - see
    /// `mesh_render::build`.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        strip: &GlowStrip,
        sample_count: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("loading wave"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/loading.wgsl")).into(),
            ),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("loading wave tint"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("loading glow strip"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("loading wave"),
            bind_group_layouts: &[Some(&layout), Some(&texture_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("loading wave"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2, 1 => Float32x2, 2 => Float32
                    ],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(BLEND),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                // Screen-space quads are emitted in one winding and never seen
                // from behind, but culling them buys nothing and a flipped
                // winding would then show as nothing drawing at all.
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("loading wave tint"),
            size: TINT_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("loading wave tint"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });

        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("loading wave vertices"),
            size: (MAX_VERTICES * std::mem::size_of::<GpuVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let texture = strip.bind(device, queue, &texture_layout);

        Self {
            pipeline,
            uniforms,
            bind_group,
            texture,
            vertices,
            count: 0,
        }
    }

    /// Uploads this frame's tint and geometry, as [`vertices`] returns them.
    ///
    /// `tint` is rgb and a fade in `a`, and `[1.0; 4]` is the neutral value.
    /// **Neither is a recovered term**: the only fade the RE page attributes to
    /// this screen (`Loading_Ramp255(30, 60, t)`) tints the full-screen backdrop
    /// blit, not the wave. It is here because a caller fading the loading screen
    /// out needs the overlay to go with it, and doing that by rebuilding every
    /// quad's alpha would corrupt the across-screen ramp that *is* recovered.
    ///
    /// Takes `&mut self` only for the vertex count; both writes go through
    /// `queue`, the same split [`oag_fx::sparks::Pipeline::upload`] uses.
    pub fn upload(&mut self, queue: &wgpu::Queue, tint: [f32; 4], vertices: &[GpuVertex]) {
        queue.write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&tint));
        let n = vertices.len().min(MAX_VERTICES);
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&vertices[..n]));
        self.count = n as u32;
    }

    /// Draws into a pass the caller already opened.
    ///
    /// An overlay, so it belongs last in the frame - after the scene and after
    /// any post-processing, since it is UI and must not be blurred or sharpened
    /// by an upscaler.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        // `Buffer::slice` on a zero-length range is the crash `mesh_render`'s
        // empty-model test guards; nothing to draw has to mean no commands.
        if self.count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_bind_group(1, &self.texture, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..self.count, 0..1);
    }
}

#[cfg(test)]
mod tests;
