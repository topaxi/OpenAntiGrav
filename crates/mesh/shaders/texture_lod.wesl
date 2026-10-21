// Slope-based texture level of detail and anisotropic sampling.

@if(!bindless) import package::bindings::{albedo, albedo_sampler};
@if(bindless) import package::bindings::{material, samplers, textures};

// The GE's texture level slope and bias for a PSP `.vex` model, off by default
// (0 = the sampler's own derivative-driven selection). See
// `mesh_render::PSP_TEXLOD_SLOPE` for the recovered values and their limits.
override texlod_slope: f32 = 0.0;

override texlod_bias: f32 = 0.0;

// Wipeout HD's LOD bias on its material textures, in levels (negative reads a
// finer level): `0` is the sampler's own selection, and what a model that is not
// a HD circuit's gets, so those pipelines compile to a plain `textureSample`.
// `Model::material_lod_bias` is the value and `docs/rendering/hd-track-tone.md`
// the measurement.
override material_lod_bias: f32 = 0.0;

// `textureSample` with [`material_lod_bias`] added to the level. A pipeline
// constant, so the branch is resolved at pipeline creation and the call stays in
// uniform control flow.
@diagnostic(off, derivative_uniformity)
fn material_sample(t: texture_2d<f32>, s: sampler, uv: vec2<f32>) -> vec4<f32> {
    if material_lod_bias != 0.0 {
        return textureSampleBias(t, s, uv, material_lod_bias);
    }
    return textureSample(t, s, uv);
}

// The sampler's `anisotropy_clamp`, for `sample_at_slope_level`. 1 is off.
override aniso_max: f32 = 1.0;

// Samples `albedo` at mip level `lod` and still takes an anisotropic footprint.
// `textureSampleLevel` is an explicit level and the hardware then filters
// isotropically, so the sampler's `anisotropy_clamp` does nothing. This uses
// `textureSampleGrad` with the screen-space derivatives of `uv` reshaped so
// the level the hardware picks is `lod` and the probe count is only what the
// surface needs: with `w = 2^lod` the width of one texel of that level, the
// footprint's long axis is `pmax` base texels, so `probes = ceil(pmax / w)`
// (at most the sampler's clamp). The long gradient is set to `probes * w` and
// the short one to `w`, each along its own direction: the hardware then reads
// a ratio of `probes` and a level of `log2(probes * w / probes) = lod`.
//
// Where the footprint is no wider than one texel of the level (a near surface,
// or anisotropy off) that is one probe, and the plain explicit level. The
// level is the slope law's either way; anisotropy only antialiases along the
// long axis, and never picks a finer level.
//
// Its derivative-uniformity diagnostic is off for the reason `zone_glow`'s is:
// `lit_texel` calls it after an early return on a flat varying, which Chrome's
// Tint judges against this function's own `dpdx`, `dpdy` and gradient fetch.
@diagnostic(off, derivative_uniformity)
fn sample_at_slope_level(uv: vec2<f32>, lod: f32) -> vec4<f32> {
    // The `bindless` variant's slot - see `bindings.wesl`.
    @if(bindless) {
        let albedo = textures[material.albedo];
        let albedo_sampler = samplers[material.albedo_sampler];
    }
    let size = vec2<f32>(textureDimensions(albedo, 0));
    let tx = dpdx(uv) * size;
    let ty = dpdy(uv) * size;
    let lx = length(tx);
    let ly = length(ty);
    let pmax = max(lx, ly);
    let width = exp2(lod);
    let probes = clamp(ceil(pmax / width), 1.0, aniso_max);
    if probes < 2.0 {
        return textureSampleLevel(albedo, albedo_sampler, uv, lod);
    }
    // The short gradient is a hair over `width` so the hardware's own
    // `ceil(pmax / pmin)` cannot round up to `probes + 1`.
    let short_length = width * 1.01;
    let x_is_long = lx >= ly;
    let long_tx = select(ty, tx, x_is_long);
    let short_tx = select(tx, ty, x_is_long);
    let long_axis = long_tx * (probes * width / pmax);
    let short_len = length(short_tx);
    // A short gradient with no direction of its own takes the long one's normal.
    let normal = vec2<f32>(-long_tx.y, long_tx.x) / pmax;
    let short_axis = select(normal * short_length, short_tx * (short_length / max(short_len, 1.0e-6)), short_len > 1.0e-6);
    let new_x = select(short_axis, long_axis, x_is_long) / size;
    let new_y = select(long_axis, short_axis, x_is_long) / size;
    return textureSampleGrad(albedo, albedo_sampler, uv, new_x, new_y);
}
