//! Bind group 2's Zone half: the stage texture the Zone material variant
//! samples, the sampler it reads it through, and the visualiser lookup the
//! glow term indexes.
//!
//! Split out of `mesh_render` rather than written inline for one reason worth
//! stating: the scene group is now two unrelated things in one binding set -
//! the fog and light *buffer*, which every title writes, and a set of
//! *textures* only a Zone race on Wipeout HD/Fury ever has. Keeping the
//! second one here means the first reads as it always did.
//!
//! See [`super::Zone`] for what the shader does with the stage texture, and
//! `mesh.wgsl`'s own `zone_glow` for the visualiser term this module now also
//! feeds. [`write_vis`] is the seam a caller updates every frame; everything
//! else here is built once, at model load.

use std::sync::Arc;

use super::texture;
use crate::mesh::ModelTexture;

/// Texels in [`Resources::vis_texture`]. Matches the original's own
/// `zoneTexVis` - a 256-entry lookup keyed on a texel's alpha - not a choice
/// this project made; see `mesh.wgsl`'s `zone_glow` and
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-shader.md`.
pub const VIS_WIDTH: u32 = 256;

/// How many texels one band's bar meter occupies, and therefore the stride
/// from one band's bar to the next.
///
/// **Recovered**, from `Environment_UpdateStageBlend`'s own pointer set: it
/// holds ten pointers into the pixel buffer, at texels `1` through `10`, and
/// advances every one of them by `0x28` bytes - ten texels - per band. See
/// [`write_vis`] and
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-visualiser.md`.
pub const SEGMENTS: usize = 10;

/// Where the smooth half of the lookup begins: one texel per band, after the
/// bars.
///
/// **Recovered**: the eleventh pointer the same function holds is `r9 +
/// 0x284`, texel `161`, and it advances by one texel per band rather than
/// ten. `161 = 1 + 10 * 16`, immediately past the sixteenth bar.
pub const SMOOTH_BASE: usize = 161;

/// The showing stage's two textures, as [`resources`] binds them.
///
/// **Two, because the original publishes two.** HD's Zone parameters go out
/// in two paired blocks: `zoneModeTrack<n>.gtf` beside the `Track.*` colours
/// for a chunk whose render-block flags carry the track bit
/// (`oag_rcs::rcsmodel::RENDER_TRACK`), and `zoneMode<n>.gtf` beside the
/// `Scene.*` colours for every other chunk. `mesh.wgsl` selects per fragment
/// on `slots::ZONE_TRACK`; see [`super::Zone`].
///
/// Either slot `None` binds a 1x1 black, on the terms [`resources`] states.
#[derive(Debug, Clone, Default)]
pub struct StageArt {
    /// `zoneModeTrack<n>.gtf` - the set with the art in it.
    pub track: Option<Arc<ModelTexture>>,
    /// `zoneMode<n>.gtf` - a flat white on every stage HD/Fury ships, and
    /// bound rather than replaced by a constant because it is the file's
    /// own statement of what a Scene chunk samples.
    pub scene: Option<Arc<ModelTexture>>,
}

impl StageArt {
    /// Neither texture: what every model outside an HD Zone race binds.
    pub const NONE: Self = Self {
        track: None,
        scene: None,
    };
}

/// The six layout entries the Zone half of bind group 2 adds: the track
/// stage texture and its sampler at bindings 1-2, a nearest-filtered clone
/// of the same stage texture at binding 3 (the visualiser's own band index
/// is read from a texel's alpha, and interpolating between two bands would
/// smear them together), the visualiser lookup itself at bindings 4-5, and
/// the scene stage texture at binding 10 - past the shadow map's 6-9, which
/// were numbered before the second texture set was bound.
///
/// A sampler of its own for the stage texture rather than the albedo's,
/// because the coordinate is one the shader builds, `zoneColourTint.xy *
/// (1 - meshUV)`, and runs outside `[0, 1]` wherever the file's own scale
/// does. The scene texture shares both samplers: same coordinate, same
/// filter.
pub(super) fn layout_entries() -> [wgpu::BindGroupLayoutEntry; 6] {
    let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    };
    let sampler_entry = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    };
    [
        texture_entry(1),
        sampler_entry(2),
        sampler_entry(3),
        texture_entry(4),
        sampler_entry(5),
        texture_entry(10),
    ]
}

/// Everything [`resources`] builds: the two stage textures' views and the
/// two samplers they share, and the visualiser lookup's texture, view and
/// sampler.
///
/// [`Self::vis_texture`] is kept, unlike the stage textures' own
/// `wgpu::Texture`s - it is written every frame by [`write_vis`], where a
/// stage texture is loaded once and never touched again.
pub(super) struct Resources {
    pub sampler: wgpu::Sampler,
    /// [`StageArt::track`].
    pub view: wgpu::TextureView,
    /// [`StageArt::scene`].
    pub scene_view: wgpu::TextureView,
    /// The same texels as [`Self::view`], point-filtered - see
    /// [`layout_entries`].
    pub nearest_sampler: wgpu::Sampler,
    pub vis_texture: wgpu::Texture,
    pub vis_view: wgpu::TextureView,
    pub vis_sampler: wgpu::Sampler,
}

/// The samplers and the views to bind, for a model that has the Zone stage
/// textures and for one that does not - plus the visualiser lookup, which
/// every model gets regardless, silent until [`write_vis`] is called.
///
/// **Black, not white, where there is no stage texture.**
/// [`super::Scene::off`] leaves `zone.enabled` at zero so the sample is
/// multiplied out anyway, and a black placeholder means even a caller that
/// writes a Zone uniform without a texture adds nothing rather than adding a
/// white sheet - the failure this project wants from a missing asset is an
/// absence. That holds for each of [`StageArt`]'s two slots on its own: a
/// stage whose scene texture failed to decode draws its scene chunks black,
/// not through the track set. [`Resources::vis_texture`] starts the same
/// way: all-black, so a model drawn before the first [`write_vis`] call - or
/// on a build with no audio device at all - shows no glow rather than an
/// invented one.
pub(super) fn resources(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    anisotropy: super::Anisotropy,
    stage: &StageArt,
) -> Resources {
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("zone"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        anisotropy_clamp: anisotropy.clamp(),
        ..Default::default()
    });
    // Repeat, on the same terms as the linear sampler above: it addresses the
    // same texels with the same shader-built coordinate, and only the filter
    // differs.
    let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("zone nearest"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let upload = |texture: &Option<Arc<ModelTexture>>, label| match texture {
        Some(texture) => texture::upload(
            device,
            queue,
            texture,
            device
                .features()
                .contains(wgpu::Features::TEXTURE_COMPRESSION_BC),
        ),
        None => texture::upload_rgba(device, queue, 1, 1, &[0, 0, 0, 255], label, None),
    };
    let view = upload(&stage.track, "no zone track stage");
    let scene_view = upload(&stage.scene, "no zone scene stage");

    let vis_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("zone vis"),
        size: wgpu::Extent3d {
            width: VIS_WIDTH,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: texture::FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &vis_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[0u8; (VIS_WIDTH * 4) as usize],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(VIS_WIDTH * 4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: VIS_WIDTH,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let vis_view = vis_texture.create_view(&wgpu::TextureViewDescriptor::default());
    // Clamp, not repeat: the shader indexes this by a texel alpha in
    // `0.0..=1.0` (see `mesh.wgsl`'s `zone_glow`), which never leaves that
    // range, and nearest-filtered for the same reason the stage texture's own
    // clone is - a band lookup wants a discrete answer, not a blend between
    // two neighbours.
    let vis_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("zone vis"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    Resources {
        sampler,
        view,
        scene_view,
        nearest_sampler,
        vis_texture,
        vis_view,
        vis_sampler,
    }
}

/// The visualiser's per-band peak-hold, which the original keeps in its own
/// environment struct and this keeps beside the texture it feeds.
///
/// **Recovered ballistics**, read off `Environment_UpdateStageBlend` at
/// instruction level: a band's held level takes a rising sample immediately
/// and falls by [`Self::DECAY`] per *frame* otherwise, clamped at zero -
///
/// ```text
/// if new >= held { held = new } else { held = max(held - 0.1, 0.0) }
/// ```
///
/// The original stores `held` as a sixteen-float array at `+0x32f8` of the
/// same struct that holds the `zoneTexVis` wrapper, and the raw sample beside
/// it at `+0x32b8`. See
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-visualiser.md`.
///
/// **Per frame, not per second.** The original's caller is
/// `Scene_PrepareFrame`, so the fall rate is frame-rate dependent in the
/// original too; converting it to a per-second rate would be a change, not a
/// port, and is deliberately not done here.
#[derive(Debug, Default)]
pub struct Hold {
    held: Vec<f32>,
}

impl Hold {
    /// How much a band's held level falls in one frame it is not rising.
    ///
    /// `0x3dcccccd`, the float this function loads from its own TOC at
    /// `-0x5a7c(r2)`.
    pub const DECAY: f32 = 0.1;

    /// Folds one frame's `bands` in and returns the held levels.
    pub fn advance(&mut self, bands: &[f32]) -> &[f32] {
        self.held.resize(bands.len(), 0.0);
        for (held, &new) in self.held.iter_mut().zip(bands) {
            *held = if new >= *held {
                new
            } else {
                (*held - Self::DECAY).max(0.0)
            };
        }
        &self.held
    }
}

/// Rewrites the visualiser lookup from `bands` levels, each `0.0..=1.0`,
/// tinted by `tint` - or blanks it when `tint` is `None`, which is what a
/// stage that authors no `EQ colour tint` gets: a missing input draws
/// nothing, not an invented white.
///
/// # This is the original's own layout, recovered
///
/// It used to be an invention: `bands` spread linearly across all 256
/// texels, on the grounds that HD/Fury's per-frame write was untraced past
/// the float it converts. It is traced now
/// (`docs/ghidra/functions/ps3-hdfury-eu/zone-visualiser.md`), the invention
/// was wrong in a way that made the effect invisible, and what replaces it is
/// read off the executable:
///
/// ```text
/// texel 0                     never written
/// texels 1 + 10b ..= 10 + 10b band b's bar: segment s is lit when s <= level
/// texel  161 + b              band b's smooth level, the tint scaled by it
/// ```
///
/// with `level = min(10, trunc(held * 11.0))` and a lit segment carrying the
/// **whole** tint rather than a fraction of it - the original stores one
/// packed colour word into each lit segment and zero into each unlit one, so
/// a bar is a bar and not a gradient. `11.0` and `255.0` are the two floats
/// the function loads from its own TOC (`0x41300000`, `0x437f0000`).
///
/// **The shipped art is the independent confirmation, and it is exact.**
/// `zone-shader.md`'s alpha histogram of the stage textures measures
/// `zonemodetrack9`/`10` at exactly `{31..40}` - which is band 3's ten
/// segments, `1 + 10*3` through `10 + 10*3` - `track14` at "groups of 4 on a
/// stride of 10", the band stride itself, and `track6`/`7` at `{1..162}`,
/// every bar plus the first two smooth slots. Three histograms measured
/// before this layout was read, all three landing on it.
///
/// `bands.len()` need not be sixteen; a shorter slice simply leaves the
/// higher bars dark, and anything past texel [`VIS_WIDTH`] is dropped.
pub fn write_vis(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    bands: &[f32],
    tint: Option<[u8; 3]>,
) {
    let pixels = vis_pixels(bands, tint);
    let pixels: &[u8] = pixels.as_flattened();
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(VIS_WIDTH * 4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: VIS_WIDTH,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
}

/// The 256 RGBA texels [`write_vis`] uploads - its whole arithmetic, split
/// out so it is testable without a GPU, the same reason
/// `mesh_render::uniforms::view_projection` is split from
/// `mesh_render::uniforms::write_uniforms`.
///
/// Every texel this frame does not light stays `0`, alpha included: the
/// original writes a literal zero word into each unlit segment (`r15`, set
/// by `li r15,0`), and that is also the explicit "draw nothing" a `None`
/// tint or an empty `bands` gets, rather than leaving whatever the texture
/// held from a previous frame.
fn vis_pixels(bands: &[f32], tint: Option<[u8; 3]>) -> [[u8; 4]; VIS_WIDTH as usize] {
    let mut texels = [[0u8; 4]; VIS_WIDTH as usize];
    let Some(tint) = tint.filter(|_| !bands.is_empty()) else {
        return texels;
    };
    let lit = [tint[0], tint[1], tint[2], 255];
    let scaled = |channel: u8, by: f32| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "`by` is clamped to 0.0..=1.0 and the channel is a u8"
        )]
        let out = (f32::from(channel) * by).round() as u8;
        out
    };
    for (band, &level) in bands.iter().enumerate() {
        for step in 0..segments(level) {
            if let Some(texel) = texels.get_mut(1 + SEGMENTS * band + step) {
                *texel = lit;
            }
        }
        // The smooth half: the same tint scaled by the level rather than
        // quantised to a segment. The original builds it by multiplying each
        // 0-255 colour component by the 0-255 level and keeping the high
        // byte, which is this to within a rounding step.
        if let Some(texel) = texels.get_mut(SMOOTH_BASE + band) {
            let by = level.clamp(0.0, 1.0);
            *texel = [
                scaled(tint[0], by),
                scaled(tint[1], by),
                scaled(tint[2], by),
                255,
            ];
        }
    }
    texels
}

/// How many of a bar's [`SEGMENTS`] a level lights: `min(10, trunc(level *
/// 11.0))`.
///
/// **`11.0`, not `10.0`, and the truncation is the original's.** It converts
/// with `fctiwz` - round toward zero - and then clamps at ten, so a full
/// level lands on ten lit segments while every value below `10/11` shares the
/// remaining nine evenly. Scaling by `10.0` instead would light the tenth
/// segment only at exactly `1.0`.
fn segments(level: f32) -> usize {
    // `matches!` on the ordering rather than `!(level > 0.0)`: a NaN level
    // must light nothing, and clippy will not take the negated comparison.
    if !matches!(level.partial_cmp(&0.0), Some(std::cmp::Ordering::Greater)) {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "guarded above zero and clamped to SEGMENTS immediately below"
    )]
    let steps = (level * 11.0) as usize;
    steps.min(SEGMENTS)
}

#[cfg(test)]
mod tests;
