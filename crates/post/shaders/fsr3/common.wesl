// AMD FidelityFX Super Resolution 3.1 - the shared prelude, ported to WGSL.
//
// Ported from `ffx_fsr3upscaler_common.h`, its accessor callbacks in
// `ffx_fsr3upscaler_callbacks_hlsl.h`, and the `Fsr3UpscalerConstants` layout
// in `ffx_fsr3upscaler_private.h`, of AMD's FidelityFX SDK v1.1.4
// (https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK, commit
// c6efa6bf7f2027b3ec94f28578bb5965eabb9e55), which are Copyright (c) 2024
// Advanced Micro Devices, Inc. and MIT-licensed. The licence text travels with
// this project in `licences/AMD-FidelityFX-MIT.txt`.
//
// This is a transliteration, not a reinterpretation: the arithmetic, the
// thresholds and the magic constants are upstream's and are deliberately left
// exactly as they are, so that a reader can diff this against the headers line
// for line. Where upstream uses a macro (`FfxFloat32x3`, `ffxSaturate`) this
// uses the plain WGSL type or built-in; that is the only systematic change.
//
// **Every pass imports from this module** (`import package::fsr3::common::{..}`),
// and `build.rs` compiles each pass to one WGSL file with only what it reaches.
// Group 0 is identical in every pass and holds only what every pass needs; a
// pass's own resources are group 1.
//
// **Upstream works on linear light; this works on gamma**, the same space
// `fsr1.wesl` does. ADR-0020 makes gamma this renderer's authoritative colour
// space and nothing linearises, so there is none to hand it. See `post::fsr3`
// for the whole argument and what it costs.

// `Fsr3UpscalerConstants`, in upstream's declaration order. The order is
// load-bearing: HLSL's 16-byte cbuffer rows and WGSL's uniform layout rules
// agree on the offsets for this exact sequence and not for a reordered one.
struct Fsr3Constants {
    render_size: vec2<i32>,
    previous_frame_render_size: vec2<i32>,
    upscale_size: vec2<i32>,
    previous_frame_upscale_size: vec2<i32>,
    max_render_size: vec2<i32>,
    max_upscale_size: vec2<i32>,
    device_to_view_depth: vec4<f32>,
    jitter_offset: vec2<f32>,
    previous_frame_jitter_offset: vec2<f32>,
    motion_vector_scale: vec2<f32>,
    downscale_factor: vec2<f32>,
    motion_vector_jitter_cancellation: vec2<f32>,
    tan_half_fov: f32,
    jitter_phase_count: f32,
    delta_time: f32,
    delta_pre_exposure: f32,
    view_space_to_meters_factor: f32,
    frame_index: f32,
    velocity_factor: f32,
    reactiveness_scale: f32,
    shading_change_scale: f32,
    accumulation_added_per_frame: f32,
    min_disocclusion_accumulation: f32,
    // Ours, in upstream's padding - see `constants::Constants::rcas_sharpness`.
    rcas_sharpness: f32,
    _pad0: f32,
    _pad1: f32,
}

@group(0) @binding(0) var<uniform> constants: Fsr3Constants;
// Upstream's `s_PointClamp` and `s_LinearClamp`. Both clamp, because every
// gather in these passes reaches past the frame's own edge at the border and
// upstream leans on the clamp rather than testing for it.
@group(0) @binding(1) var point_clamp: sampler;
@group(0) @binding(2) var linear_clamp: sampler;

const FSR3UPSCALER_FP16_MIN: f32 = 6.10e-05;
const FSR3UPSCALER_FP16_MAX: f32 = 65504.0;
const FSR3UPSCALER_EPSILON: f32 = 6.10e-05;
const FSR3UPSCALER_TONEMAP_EPSILON: f32 = 6.10e-05;
const FSR3UPSCALER_FP32_MAX: f32 = 3.402823466e+38;
const FSR3UPSCALER_FP32_MIN: f32 = 1.175494351e-38;

// Reconstructed depth usage.
const F_RECONSTRUCTED_DEPTH_BILINEAR_WEIGHT_THRESHOLD: f32 = FSR3UPSCALER_EPSILON * 10.0;

// Accumulation.
const F_UPSAMPLE_LANCZOS_WEIGHT_SCALE: f32 = 1.0 / 16.0;
const F_AVERAGE_LANCZOS_WEIGHT_PER_FRAME: f32 = 0.74 * F_UPSAMPLE_LANCZOS_WEIGHT_SCALE;
const F_ACCUMULATION_MAX_ON_MOTION: f32 = 3.0 * F_UPSAMPLE_LANCZOS_WEIGHT_SCALE;

const SHADING_CHANGE_SET_SIZE: i32 = 5;
const I_SHADING_CHANGE_MIP_START: i32 = 0;
const F_SHADING_CHANGE_SAMPLE_POW: f32 = 1.0 / 1.0;

const F_LOCK_THRESHOLD: f32 = 1.0;
const F_LOCK_MAX: f32 = 2.0;

// Channel indices of the packed reactive-mask target.
const REACTIVE: i32 = 0;
const DISOCCLUSION: i32 = 1;
const SHADING_CHANGE: i32 = 2;
const ACCUMULAION: i32 = 3;

// Channel indices of the 1x1 frame-info target. `ACCUMULAION` above and this
// list are upstream's spellings.
const FRAME_INFO_EXPOSURE: i32 = 0;
const FRAME_INFO_LOG_LUMA: i32 = 1;
const FRAME_INFO_SCENE_AVERAGE_LUMA: i32 = 2;

fn render_size() -> vec2<i32> { return constants.render_size; }
fn previous_frame_render_size() -> vec2<i32> { return constants.previous_frame_render_size; }
fn max_render_size() -> vec2<i32> { return constants.max_render_size; }
fn upscale_size() -> vec2<i32> { return constants.upscale_size; }
fn previous_frame_upscale_size() -> vec2<i32> { return constants.previous_frame_upscale_size; }
fn max_upscale_size() -> vec2<i32> { return constants.max_upscale_size; }
fn jitter() -> vec2<f32> { return constants.jitter_offset; }
fn previous_frame_jitter() -> vec2<f32> { return constants.previous_frame_jitter_offset; }
fn downscale_factor() -> vec2<f32> { return constants.downscale_factor; }
fn tan_half_fov() -> f32 { return constants.tan_half_fov; }
fn jitter_phase_count() -> f32 { return constants.jitter_phase_count; }
fn delta_time() -> f32 { return constants.delta_time; }
fn frame_index() -> f32 { return constants.frame_index; }
fn view_space_to_meters_factor() -> f32 { return constants.view_space_to_meters_factor; }
fn device_to_view_space_transform_factors() -> vec4<f32> { return constants.device_to_view_depth; }
fn velocity_factor() -> f32 { return constants.velocity_factor; }
fn reactiveness_scale() -> f32 { return constants.reactiveness_scale; }
fn shading_change_scale() -> f32 { return constants.shading_change_scale; }
fn accumulation_added_per_frame() -> f32 { return constants.accumulation_added_per_frame; }
fn min_disocclusion_accumulation() -> f32 { return constants.min_disocclusion_accumulation; }
fn rcas_sharpness() -> f32 { return constants.rcas_sharpness; }

// `Exposure()`, and **a constant one rather than a resource**.
//
// Upstream reads a 1x1 `R32G32_FLOAT` texture here, which is either the app's
// own exposure, or - with `FFX_FSR3UPSCALER_ENABLE_AUTO_EXPOSURE` set - the
// frame-info target the luma pyramid writes. This renderer supplies neither: it
// draws into a plain low-dynamic-range target, there is no exposure anywhere in
// the pipeline, and the brightness a player controls is a grade applied after
// everything here (`upscale::Framebuffer::composite`). Adding an automatic
// global brightness adaptation that the original game does not have would be
// inventing a feature, not porting one.
//
// **One is upstream's own answer for that case, not a simplification of it.**
// `FSR3UPSCALER_DefaultExposure` is initialised to `{0.0, 0.0}` and `Exposure()`
// maps a zero to exactly one. So this returns what the SDK would have returned,
// through a constant instead of a texture read.
fn exposure() -> f32 {
    return 1.0;
}

// The ratio between this frame's exposure and the previous frame's. One
// whenever nothing pre-exposes, which is always here - kept as a constants read
// rather than folded to a literal, because unlike `exposure` above it is a
// per-frame quantity that a future HDR path would fill in rather than a
// property of there being no exposure at all.
fn delta_pre_exposure() -> f32 {
    return constants.delta_pre_exposure;
}

fn tonemap_first_frame() -> bool {
    return frame_index() == 0.0;
}

fn average_lanczos_weight_per_frame() -> f32 {
    return 0.74;
}

fn shading_change_render_size() -> vec2<i32> {
    return vec2<i32>(vec2<f32>(render_size()) * 0.5);
}

fn shading_change_max_render_size() -> vec2<i32> {
    return vec2<i32>(vec2<f32>(max_render_size()) * 0.5);
}

fn previous_frame_shading_change_render_size() -> vec2<i32> {
    return vec2<i32>(vec2<f32>(previous_frame_render_size()) * 0.5);
}

fn reconstructed_depth_mv_px_threshold(nearest_depth_in_meters: f32) -> f32 {
    return mix(0.25, 0.75, saturate(nearest_depth_in_meters / 100.0));
}

// The velocity a motion vector represents once expressed in the pixels of a 4K
// screen - upstream's own unit for "how fast is this moving", so that a
// threshold means the same thing at every render resolution.
fn get_4k_velocity(motion_vector: vec2<f32>) -> f32 {
    return length(motion_vector * vec2<f32>(3840.0, 2160.0));
}

fn ycocg_to_rgb(ycocg: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        ycocg.x + ycocg.y - ycocg.z,
        ycocg.x + ycocg.z,
        ycocg.x - ycocg.y - ycocg.z,
    );
}

fn rgb_to_ycocg(rgb: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        0.25 * rgb.r + 0.5 * rgb.g + 0.25 * rgb.b,
        0.5 * rgb.r - 0.5 * rgb.b,
        -0.25 * rgb.r + 0.5 * rgb.g - 0.25 * rgb.b,
    );
}

fn rgb_to_luma(linear_rgb: vec3<f32>) -> f32 {
    return dot(linear_rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn rgb_to_perceived_luma(linear_rgb: vec3<f32>) -> f32 {
    let luminance = rgb_to_luma(linear_rgb);

    var perceived = 0.0;
    if luminance <= 216.0 / 24389.0 {
        perceived = luminance * (24389.0 / 27.0);
    } else {
        perceived = pow(luminance, 1.0 / 3.0) * 116.0 - 16.0;
    }

    return perceived * 0.01;
}

fn tonemap(rgb: vec3<f32>) -> vec3<f32> {
    return rgb / vec3<f32>(max(max(0.0, rgb.r), max(rgb.g, rgb.b)) + 1.0);
}

fn inverse_tonemap(rgb: vec3<f32>) -> vec3<f32> {
    return rgb / vec3<f32>(max(FSR3UPSCALER_TONEMAP_EPSILON, 1.0 - max(rgb.r, max(rgb.g, rgb.b))));
}

fn safe_rcp3(v: vec3<f32>) -> vec3<f32> {
    if all(v != vec3<f32>(0.0)) {
        return vec3<f32>(1.0) / v;
    }
    return vec3<f32>(0.0);
}

fn min_divided_by_max_or(v0: f32, v1: f32, on_zero: f32) -> f32 {
    let m = max(v0, v1);
    if m != 0.0 {
        return min(v0, v1) / m;
    }
    return on_zero;
}

fn min_divided_by_max(v0: f32, v1: f32) -> f32 {
    return min_divided_by_max_or(v0, v1, 0.0);
}

fn is_uv_inside(uv: vec2<f32>) -> bool {
    return (uv.x >= 0.0 && uv.x <= 1.0) && (uv.y >= 0.0 && uv.y <= 1.0);
}

fn clamp_load(sample_pos: vec2<i32>, offset: vec2<i32>, texture_size: vec2<i32>) -> vec2<i32> {
    var result = sample_pos + offset;
    result.x = max(0, min(result.x, texture_size.x - 1));
    result.y = max(0, min(result.y, texture_size.y - 1));
    return result;
}

// Half a texel inside the *drawn rectangle*, expressed in the coordinates of
// the *resource* holding it - which is why it takes both sizes. Since
// ADR-0037 those two differ whenever a controller is varying the render
// extent, and upstream already separates them for the same reason.
fn clamp_uv(uv: vec2<f32>, texture_size: vec2<i32>, resource_size: vec2<i32>) -> vec2<f32> {
    let sample_location = uv * vec2<f32>(texture_size);
    let clamped = max(
        vec2<f32>(0.5),
        min(sample_location, vec2<f32>(texture_size) - vec2<f32>(0.5)),
    );
    return clamped / vec2<f32>(resource_size);
}

// Upstream reinterprets both as unsigned so that a negative coordinate wraps to
// a huge one and fails the upper test - one comparison instead of two. Kept,
// because a `>= 0` added here would be a different function on a negative
// input from the one the rest of the port was checked against.
fn is_on_screen(pos: vec2<i32>, size: vec2<i32>) -> bool {
    return all(vec2<u32>(pos) < vec2<u32>(size));
}

fn compute_auto_exposure_from_lavg(log_lavg: f32) -> f32 {
    let lavg = exp(log_lavg);

    let s = 100.0; // ISO arithmetic speed
    let k = 12.5;
    let exposure_iso100 = log2((lavg * s) / k);

    let q = 0.65;
    let lmax = (78.0 / (q * s)) * pow(2.0, exposure_iso100);

    return 1.0 / lmax;
}

// The presentation-resolution pixel a render-resolution pixel's *jittered*
// sample actually landed on. The jitter is subtracted because the offset moved
// the rasteriser, so the sample sits that far away from where the pixel centre
// says it does.
fn compute_hr_pos_from_lr_pos(lr_pos: vec2<i32>) -> vec2<i32> {
    let src_jittered_pos = vec2<f32>(lr_pos) + 0.5 - jitter();
    let lr_pos_in_hr = (src_jittered_pos / vec2<f32>(render_size())) * vec2<f32>(upscale_size());
    return vec2<i32>(floor(lr_pos_in_hr));
}

fn compute_ndc(px_pos: vec2<f32>, size: vec2<i32>) -> vec2<f32> {
    return px_pos / vec2<f32>(size) * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
}

fn get_view_space_depth(device_depth: f32) -> f32 {
    let d = device_to_view_space_transform_factors();
    return d[1] / (device_depth - d[0]);
}

fn get_view_space_depth_in_meters(device_depth: f32) -> f32 {
    return get_view_space_depth(device_depth) * view_space_to_meters_factor();
}

fn get_view_space_position(
    viewport_pos: vec2<i32>,
    viewport_size: vec2<i32>,
    device_depth: f32,
) -> vec3<f32> {
    let d = device_to_view_space_transform_factors();
    let z = get_view_space_depth(device_depth);
    let ndc = compute_ndc(vec2<f32>(viewport_pos), viewport_size);
    return vec3<f32>(d[2] * ndc.x * z, d[3] * ndc.y * z, z);
}

fn get_view_space_position_in_meters(
    viewport_pos: vec2<i32>,
    viewport_size: vec2<i32>,
    device_depth: f32,
) -> vec3<f32> {
    return get_view_space_position(viewport_pos, viewport_size, device_depth)
        * view_space_to_meters_factor();
}

// **Depth 1.0, not 0.0.** Upstream picks between the two on
// `FFX_FSR3UPSCALER_OPTION_INVERTED_DEPTH`; this renderer's projection is
// `glam`'s right-handed non-reversed one, so the far plane is at 1.
fn get_max_distance_in_meters() -> f32 {
    return get_view_space_depth(1.0) * view_space_to_meters_factor();
}

// The four texels a bilinear tap at `uv` would blend, and their weights -
// computed rather than sampled, because two of these passes need the
// *individual* taps and a hardware bilinear only returns the blend.
struct BilinearSamplingData {
    offsets: array<vec2<i32>, 4>,
    weights: array<f32, 4>,
    base_pos: vec2<i32>,
}

fn get_bilinear_sampling_data(uv: vec2<f32>, size: vec2<i32>) -> BilinearSamplingData {
    var data: BilinearSamplingData;

    let px_sample = (uv * vec2<f32>(size)) - vec2<f32>(0.5);
    data.base_pos = vec2<i32>(floor(px_sample));
    let px_frac = fract(px_sample);

    data.offsets[0] = vec2<i32>(0, 0);
    data.offsets[1] = vec2<i32>(1, 0);
    data.offsets[2] = vec2<i32>(0, 1);
    data.offsets[3] = vec2<i32>(1, 1);

    data.weights[0] = (1.0 - px_frac.x) * (1.0 - px_frac.y);
    data.weights[1] = px_frac.x * (1.0 - px_frac.y);
    data.weights[2] = (1.0 - px_frac.x) * px_frac.y;
    data.weights[3] = px_frac.x * px_frac.y;

    return data;
}

struct PlaneData {
    normal: vec3<f32>,
    distance_from_origin: f32,
}

fn get_plane_from_points(p0: vec3<f32>, p1: vec3<f32>, p2: vec3<f32>) -> PlaneData {
    var plane: PlaneData;
    let v0 = p0 - p1;
    let v1 = p0 - p2;
    plane.normal = normalize(cross(v0, v1));
    plane.distance_from_origin = -dot(p0, plane.normal);
    return plane;
}

fn point_to_plane_distance(plane: PlaneData, point: vec3<f32>) -> f32 {
    return abs(dot(plane.normal, point) + plane.distance_from_origin);
}

// `RectificationBox`, and the three functions that fill it. Upstream passes it
// `inout`; WGSL has `ptr<function, T>` for the same thing, which is what the
// accumulate pass uses.
struct RectificationBox {
    box_center: vec3<f32>,
    box_vec: vec3<f32>,
    aabb_min: vec3<f32>,
    aabb_max: vec3<f32>,
    box_center_weight: f32,
}

// **By value, where upstream takes `inout`.** naga's SPIR-V backend cannot
// take a `ptr<function, _>` to a struct nested inside another struct - it
// panics with "Expression is not cached" rather than rejecting the shader - and
// the rectification box lives inside `AccumulationPassData`. Returning the box
// is the same arithmetic through a different calling convention; nothing about
// what these compute changes.
fn rectification_box_add_initial_sample(
    b: RectificationBox,
    color_sample: vec3<f32>,
    sample_weight: f32,
) -> RectificationBox {
    var out = b;
    out.aabb_min = color_sample;
    out.aabb_max = color_sample;

    let weighted = color_sample * sample_weight;
    out.box_center = weighted;
    out.box_vec = color_sample * weighted;
    out.box_center_weight = sample_weight;
    return out;
}

fn rectification_box_add_sample(
    initial_sample: bool,
    b: RectificationBox,
    color_sample: vec3<f32>,
    sample_weight: f32,
) -> RectificationBox {
    if initial_sample {
        return rectification_box_add_initial_sample(b, color_sample, sample_weight);
    }

    var out = b;
    out.aabb_min = min(out.aabb_min, color_sample);
    out.aabb_max = max(out.aabb_max, color_sample);

    let weighted = color_sample * sample_weight;
    out.box_center += weighted;
    out.box_vec += color_sample * weighted;
    out.box_center_weight += sample_weight;
    return out;
}

fn rectification_box_compute_variance_box_data(b: RectificationBox) -> RectificationBox {
    var out = b;
    var weight = out.box_center_weight;
    if abs(weight) <= FSR3UPSCALER_FP32_MIN {
        weight = 1.0;
    }
    out.box_center_weight = weight;
    out.box_center /= weight;
    out.box_vec /= weight;
    let std_dev = sqrt(abs(out.box_vec - out.box_center * out.box_center));
    out.box_vec = std_dev;
    return out;
}
