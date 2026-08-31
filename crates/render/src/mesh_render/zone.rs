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

/// The five layout entries the Zone half of bind group 2 adds: the stage
/// texture and its sampler at bindings 1-2, a nearest-filtered clone of the
/// same stage texture at binding 3 (the visualiser's own band index is read
/// from a texel's alpha, and interpolating between two bands would smear
/// them together), and the visualiser lookup itself at bindings 4-5.
///
/// A sampler of its own for the stage texture rather than the albedo's,
/// because the coordinate is one the shader builds - `zoneColourTint.xy * (1
/// - meshUV)` - and runs outside `[0, 1]` wherever the file's own scale does.
pub(super) fn layout_entries() -> [wgpu::BindGroupLayoutEntry; 5] {
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
    ]
}

/// Everything [`resources`] builds: the stage texture's own view and its two
/// samplers, and the visualiser lookup's texture, view and sampler.
///
/// [`Self::vis_texture`] is kept, unlike the stage texture's own
/// `wgpu::Texture` - it is written every frame by [`write_vis`], where the
/// stage texture is loaded once and never touched again.
pub(super) struct Resources {
    pub sampler: wgpu::Sampler,
    pub view: wgpu::TextureView,
    /// The same texels as [`Self::view`], point-filtered - see
    /// [`layout_entries`].
    pub nearest_sampler: wgpu::Sampler,
    pub vis_texture: wgpu::Texture,
    pub vis_view: wgpu::TextureView,
    pub vis_sampler: wgpu::Sampler,
}

/// The sampler and the view to bind, for a model that has a Zone stage
/// texture and for one that does not - plus the visualiser lookup, which
/// every model gets regardless, silent until [`write_vis`] is called.
///
/// **Black, not white, where there is no stage texture.**
/// [`super::Scene::off`] leaves `zone.enabled` at zero so the sample is
/// multiplied out anyway, and a black placeholder means even a caller that
/// writes a Zone uniform without a texture adds nothing rather than adding a
/// white sheet - the failure this project wants from a missing asset is an
/// absence. [`Resources::vis_texture`] starts the same way: all-black, so a
/// model drawn before the first [`write_vis`] call - or on a build with no
/// audio device at all - shows no glow rather than an invented one.
pub(super) fn resources(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    anisotropy: super::Anisotropy,
    stage: Option<&Arc<ModelTexture>>,
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
    let view = match stage {
        Some(texture) => texture::upload(
            device,
            queue,
            texture,
            device
                .features()
                .contains(wgpu::Features::TEXTURE_COMPRESSION_BC),
        ),
        None => texture::upload_rgba(device, queue, 1, 1, &[0, 0, 0, 255], "no zone stage"),
    };

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
        nearest_sampler,
        vis_texture,
        vis_view,
        vis_sampler,
    }
}

/// Rewrites the visualiser lookup from `bands` levels, each `0.0..=1.0`,
/// tinted by `tint` - or blanks it when `tint` is `None`, which is what a
/// stage that authors no `EQ colour tint` gets: a missing input draws
/// nothing, not an invented white.
///
/// **Not the original's own mechanism** - see `mesh.wgsl`'s `zone_glow` and
/// `docs/formats/effectsettings.md`'s "EQ keys are an audio spectrum" finding
/// for why: HD/Fury's own `zoneTexVis` is zero-filled at load and confirmed
/// rewritten every frame by `Environment_UpdateStageBlend`
/// (`docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`'s
/// twenty-sixth pass) - but the value it writes is untraced past a float
/// compared against the recovered stage ladder, so whether it is
/// audio-reactive is still open, and transcribing it would be game content
/// this project may not carry regardless. This is a genuine spectrum of the
/// audio this project's own mixer is producing, computed in
/// `oag_audio::spectrum` and handed in here as plain numbers - the same seam
/// `Fog`/`Light` cross from `oag-game` into this crate already.
///
/// # Why every texel is interpolated, not block-repeated
///
/// **A shipped stage texture samples only a narrow, arbitrary window of the
/// 256 indices, and different stages use different windows.**
/// `zone-shader.md`'s own alpha histogram measures `zonemodetrack9`/`10` at
/// exactly `{31..40}`, `track14` at a wider spread, `track6`/`7` at
/// `{1..162}` - and the disc's own live per-frame writer
/// (`zone-effectsettings-loader.md`'s twenty-sixth pass) touches texels
/// `1`-`10` and `161`, the same shape: a handful of *consecutive* low
/// indices. Stretching `bands.len()` values across 256 texels in fixed-size
/// blocks (`VIS_WIDTH / bands.len()` texels per band) would put several
/// consecutive shipped indices inside the *same* block, so a ten-wide window
/// like `track9`'s reads one repeated colour - a flash, not a spectrum. This
/// linearly interpolates `bands` across the *whole* 256-wide strip instead,
/// so every texel differs a little from its neighbour and any narrow,
/// disc-chosen window still shows genuine variation. `bands.len()` need not
/// be [`VIS_WIDTH`] either way.
///
/// **This does not weaken the point-sampling the shader depends on.** The
/// interpolation happens once, here, when the 256 texel *values* are
/// authored; `zone_glow` still reads exactly one of them, through a
/// nearest-filtered sampler, with no runtime blending between texels. See
/// `zone-shader.md`'s own reasoning for why the lookup wants a discrete
/// answer per index - that is about how a coordinate is *sampled*, not about
/// whether the values behind it may vary smoothly.
pub fn write_vis(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    bands: &[f32],
    tint: Option<[u8; 3]>,
) {
    let mut pixels = vec![0u8; (VIS_WIDTH * 4) as usize];
    if let Some(tint) = tint.filter(|_| !bands.is_empty()) {
        for (pixel, level) in pixels.as_chunks_mut::<4>().0.iter_mut().zip(levels(bands)) {
            pixel[0] = (f32::from(tint[0]) * level).round() as u8;
            pixel[1] = (f32::from(tint[1]) * level).round() as u8;
            pixel[2] = (f32::from(tint[2]) * level).round() as u8;
            pixel[3] = 255;
        }
    }
    // Alpha stays `0` in the blanked case too - `pixels` is zero-initialised
    // and nothing above touches it when `tint` is `None` or `bands` is
    // empty, which is the explicit "draw nothing" this function promises
    // rather than leaving whatever the texture held from a previous frame.
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &pixels,
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

/// [`VIS_WIDTH`] levels, `bands` linearly interpolated across the whole
/// strip - the pure arithmetic half of [`write_vis`], split out so it is
/// testable without a GPU, the same reason
/// `mesh_render::uniforms::view_projection` is split from
/// `mesh_render::uniforms::write_uniforms`.
///
/// `bands` must be non-empty; [`write_vis`] is what handles the empty case.
fn levels(bands: &[f32]) -> [f32; VIS_WIDTH as usize] {
    let last = bands.len() - 1;
    std::array::from_fn(|texel| {
        #[expect(
            clippy::cast_precision_loss,
            reason = "VIS_WIDTH and bands.len() are both far under f32's exact-integer range"
        )]
        let position = texel as f32 / (VIS_WIDTH - 1) as f32 * last as f32;
        let low = position.floor() as usize;
        let high = (low + 1).min(last);
        let frac = position - position.floor();
        (bands[low] * (1.0 - frac) + bands[high] * frac).clamp(0.0, 1.0)
    })
}

#[cfg(test)]
mod tests;
