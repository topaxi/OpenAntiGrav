// Slope-based texture level of detail and anisotropic sampling.

@if(!bindless) import package::bindings::{albedo, albedo_sampler, fifth_map, lightmap, pad_mask, scene, sixth_map, wave_map};
@if(bindless) import package::bindings::{material, samplers, scene, textures};

// The GE's texture level slope and bias for a PSP `.vex` model, off by default
// (0 = the sampler's own derivative-driven selection). See
// `mesh_render::PSP_TEXLOD_SLOPE` for the recovered values and their limits.
override texlod_slope: f32 = 0.0;

override texlod_bias: f32 = 0.0;

// The race camera's part of a Pulse texture's own bias - `mesh_render::texlod::PSP_TEXLOD_FRAME`.
override texlod_frame: f32 = 0.0;

// The GE's level at view depth `w`, before TEXTURE DETAIL. `code` is the draw's
// texture's (`mesh_render::texlod::texlod_vertex_code`, the same on all three corners):
// 0 is the flat law, `log2(w * slope) + bias`. Any other is Pulse's per-texture
// one, `log2(2 w slope) + b`, with `b` the texture's own bias clamped to
// [-8, 7.5] and cut to sixteenths toward zero as the GE register holds it, or
// exactly 0 for a negative code (`docs/rendering/frame-audit.md`, "The level
// law, measured").
fn ge_level(w: f32, code: f32, frame_shift: f32) -> f32 {
    if code == 0.0 {
        return log2(max(w * texlod_slope, 1.0e-6)) + texlod_bias;
    }
    var bias = 0.0;
    if code > 0.0 {
        bias = trunc(clamp(code - 64.0 + texlod_frame + frame_shift, -8.0, 7.5) * 16.0) / 16.0;
    }
    return log2(max(2.0 * w * texlod_slope, 1.0e-6)) + bias;
}

// Wipeout HD's LOD bias on its material textures, in levels (negative reads a
// finer level): `0` is the sampler's own selection, and what a model that is not
// a HD circuit's gets, so those pipelines compile to a plain `textureSample`.
// `Model::material_lod_bias` is the value and `docs/rendering/hd-track-tone.md`
// the measurement.
override material_lod_bias: f32 = 0.0;

// The material pictures `material_sample` can read, by the name `bindings.wesl` gives them.
const MATERIAL_ALBEDO: u32 = 0u;
const MATERIAL_LIGHTMAP: u32 = 1u;
const MATERIAL_PAD_MASK: u32 = 2u;
const MATERIAL_WAVE_MAP: u32 = 3u;
const MATERIAL_FIFTH_MAP: u32 = 4u;
const MATERIAL_SIXTH_MAP: u32 = 5u;

// The player's TEXTURE DETAIL on the hardware path, in levels added to the sampler's own
// selection (`mesh_render::TextureDetail::level_shift`). A model on the GE's slope rule takes the
// preset there instead, so its pipelines answer 0 here. 0 at `original`.
fn detail_shift() -> f32 {
    if texlod_slope > 0.0 {
        return 0.0;
    }
    return scene.fog.texlod_shift;
}

// What explicit gradients are scaled by: `2^bias`, the way the sampler applies a bias. The
// title's constant factor alone at `original`.
fn grad_scale() -> f32 {
    let shift = detail_shift();
    return exp2(material_lod_bias) * select(1.0, exp2(shift), shift != 0.0);
}

// One of the material pictures (a `MATERIAL_*` constant) through the albedo sampler, `bias`
// levels from the sampler's own selection. A bias of 0 is a plain `textureSample`. The branch
// reads a pipeline constant and a uniform, so the call stays in uniform control flow.
//
// It takes the picture by name rather than as a `texture_2d` parameter: on the `bindless`
// variant a picture is an element of a binding array, and naga's SPIR-V writer passes a
// texture parameter as a pointer to a global but a binding-array element as the loaded
// value, which `spirv-val` rejects (`OpFunctionCall Argument ... type does not match`).
// `oag_shader_check::validate` refuses that shape. Every constant is a literal at its call
// site, so the switches fold away.
//
// Its derivative-uniformity diagnostic is off for `lit_colour`'s reason (`shade.wesl`): Tint judges
// this fetch against this function, and the guards of its callers are constant across a primitive.
@diagnostic(off, derivative_uniformity)
fn picture_sample(picture: u32, uv: vec2<f32>, bias: f32) -> vec4<f32> {
    // The `bindless` variant's slots - see `bindings.wesl`.
    @if(bindless) {
        let albedo = textures[material.albedo];
        let albedo_sampler = samplers[material.albedo_sampler];
        let lightmap = textures[material.lightmap];
        let pad_mask = textures[material.pad_mask];
        let wave_map = textures[material.wave_map];
        let fifth_map = textures[material.fifth_map];
        let sixth_map = textures[material.sixth_map];
    }
    if bias != 0.0 {
        // -16 is the lowest `textureSampleBias` takes, past the base of any chain.
        let at = max(bias, -16.0);
        switch picture {
            case 1u: { return textureSampleBias(lightmap, albedo_sampler, uv, at); }
            case 2u: { return textureSampleBias(pad_mask, albedo_sampler, uv, at); }
            case 3u: { return textureSampleBias(wave_map, albedo_sampler, uv, at); }
            case 4u: { return textureSampleBias(fifth_map, albedo_sampler, uv, at); }
            case 5u: { return textureSampleBias(sixth_map, albedo_sampler, uv, at); }
            default: { return textureSampleBias(albedo, albedo_sampler, uv, at); }
        }
    }
    switch picture {
        case 1u: { return textureSample(lightmap, albedo_sampler, uv); }
        case 2u: { return textureSample(pad_mask, albedo_sampler, uv); }
        case 3u: { return textureSample(wave_map, albedo_sampler, uv); }
        case 4u: { return textureSample(fifth_map, albedo_sampler, uv); }
        case 5u: { return textureSample(sixth_map, albedo_sampler, uv); }
        default: { return textureSample(albedo, albedo_sampler, uv); }
    }
}

// A material fetch at the title's [`material_lod_bias`] plus TEXTURE DETAIL.
@diagnostic(off, derivative_uniformity)
fn material_sample(picture: u32, uv: vec2<f32>) -> vec4<f32> {
    return picture_sample(picture, uv, material_lod_bias + detail_shift());
}

// A fetch of a material picture that takes TEXTURE DETAIL and no title bias: the effect
// programs, whose originals' samplers were not read for one.
@diagnostic(off, derivative_uniformity)
fn detail_sample(picture: u32, uv: vec2<f32>) -> vec4<f32> {
    return picture_sample(picture, uv, detail_shift());
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

// `material_sample` with explicit gradients: Omega's panel-map road samples its diffuse, normal and
// emissive at the marched coordinate with the *unshifted* coordinate's derivatives
// (`docs/ghidra/functions/ps4-omega-eu/panel-displacement.md`, `image_sample_cd`), so the level
// does not jump at a channel's wall. Every fetch that sits behind a test of the interpolated
// tangent (`lit_texel`'s `panel`, `omega_snow_on`, `omega_rock_on`, the hull normal map) takes this
// form too, with `uv_gradients` of its coordinate taken ahead of the test, because an implicit
// level inside those branches is not defined where the tangent's length changes across a primitive.
//
// [`material_lod_bias`] and TEXTURE DETAIL are applied the way the sampler applies a bias, as a
// shift of the level: both gradients times `2^bias` ([`grad_scale`]), 1.0 where there is neither.
fn material_sample_grad(picture: u32, uv: vec2<f32>, ddx_in: vec2<f32>, ddy_in: vec2<f32>) -> vec4<f32> {
    let level_scale = grad_scale();
    let ddx = ddx_in * level_scale;
    let ddy = ddy_in * level_scale;
    @if(bindless) {
        let albedo = textures[material.albedo];
        let albedo_sampler = samplers[material.albedo_sampler];
        let pad_mask = textures[material.pad_mask];
        let wave_map = textures[material.wave_map];
        let fifth_map = textures[material.fifth_map];
        let sixth_map = textures[material.sixth_map];
    }
    switch picture {
        case 2u: { return textureSampleGrad(pad_mask, albedo_sampler, uv, ddx, ddy); }
        case 3u: { return textureSampleGrad(wave_map, albedo_sampler, uv, ddx, ddy); }
        case 4u: { return textureSampleGrad(fifth_map, albedo_sampler, uv, ddx, ddy); }
        case 5u: { return textureSampleGrad(sixth_map, albedo_sampler, uv, ddx, ddy); }
        default: { return textureSampleGrad(albedo, albedo_sampler, uv, ddx, ddy); }
    }
}

// The screen derivatives of `uv`, `(dpdx, dpdy)`. Its derivative-uniformity diagnostic is off for
// the reason `sample_at_slope_level`'s is.
@diagnostic(off, derivative_uniformity)
fn uv_gradients(uv: vec2<f32>) -> mat2x2<f32> {
    return mat2x2<f32>(dpdx(uv), dpdy(uv));
}
