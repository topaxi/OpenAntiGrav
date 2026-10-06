// AMD FidelityFX Super Resolution 3.1 - the shading-change pyramid, ported to
// WGSL. Transliterated from `ffx_fsr3upscaler_shading_change_pyramid.h`, AMD
// FidelityFX SDK v1.1.4, MIT - see `common.wesl`'s header and
// `licences/AMD-FidelityFX-MIT.txt`.
//
// Asks, per render-resolution texel, "did this surface's *shading* change, or
// did it only move?" - by comparing a five-tap luma neighbourhood of this frame
// against the same neighbourhood of the previous frame, reprojected through the
// dilated motion vector. A surface that merely moved matches something in the
// previous frame; one that was re-lit, or newly disoccluded, does not. The
// answer is reduced into a small pyramid so that `shading_change` can ask the
// question at three scales at once.
//
// # The SPD substitution
//
// Upstream builds the pyramid with a Single Pass Downsampler: one dispatch,
// coordinating through a `globallycoherent` texture and a global atomic counter
// so the last workgroup to finish can reduce the rest. **WGSL has neither**, so
// this is a chain of ordinary dispatches - `cs_shading_change_pyramid_mip0`
// computes and reduces into level 0, then `cs_shading_change_pyramid_reduce`
// runs once per further level. This is the deviation
// [ADR-0012](../../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)
// predicted, and the arithmetic each level performs is upstream's `SpdReduce4`
// unchanged.
//
// **`MIP0_INDICATOR` is not carried.** Upstream's reduce averages a third
// channel that is 1.0 at every source texel and therefore 1.0 at every level,
// and `StorePyramid` writes only `.xy` - so it is never stored and never read.
// Dropping it is dropping a constant, not a signal.
import package::fsr3::common::{
    FSR3UPSCALER_EPSILON,
    FSR3UPSCALER_FP16_MAX,
    FSR3UPSCALER_FP16_MIN,
    FSR3UPSCALER_FP32_MIN,
    F_SHADING_CHANGE_SAMPLE_POW,
    clamp_load,
    delta_pre_exposure,
    exposure,
    is_on_screen,
    is_uv_inside,
    jitter,
    min_divided_by_max,
    previous_frame_jitter,
    previous_frame_render_size,
    render_size,
    shading_change_render_size,
};

@group(1) @binding(0) var r_current_luma: texture_2d<f32>;
@group(1) @binding(1) var r_previous_luma: texture_2d<f32>;
@group(1) @binding(2) var r_dilated_motion_vectors: texture_2d<f32>;
// The level being written, and - for the reduce pass - the level below it.
// Bound as a sampled texture rather than read through the storage binding,
// because a read-write storage texture carries no baseline guarantee.
@group(1) @binding(3) var r_spd_source: texture_2d<f32>;
@group(1) @binding(4) var rw_spd_target: texture_storage_2d<rgba16float, write>;
// Which mip `rw_spd_target` is. A separate uniform rather than another field of
// `Fsr3Constants` because it changes between dispatches *within* a frame, and
// an index rather than an extent so that it stays constant as a resolution
// controller moves the render extent - the sizes are derived from
// `render_size()` below, which already moves.
@group(1) @binding(5) var<uniform> level: vec4<u32>;

fn load_current_luma(px_pos: vec2<i32>) -> f32 {
    return textureLoad(r_current_luma, px_pos, 0).x;
}

fn load_previous_luma(px_pos: vec2<i32>) -> f32 {
    return textureLoad(r_previous_luma, px_pos, 0).x;
}

fn load_dilated_motion_vector(px_pos: vec2<i32>) -> vec2<f32> {
    return textureLoad(r_dilated_motion_vectors, px_pos, 0).xy;
}

const I_SAMPLE_OFFSETS: array<vec2<i32>, 5> = array<vec2<i32>, 5>(
    vec2<i32>(0, 0),
    vec2<i32>(-1, 0),
    vec2<i32>(1, 0),
    vec2<i32>(0, -1),
    vec2<i32>(0, 1),
);

// `SampleSet`, upstream's five-tap plus. A struct rather than a bare array so
// that the sort below can take a pointer to it the way upstream takes an
// `inout`.
struct SampleSet {
    samples: array<f32, 5>,
}

// `CompareSwap`, upstream's macro written out as a function.
fn compare_swap(samples: ptr<function, SampleSet>, i: i32, j: i32) {
    let tmp = min((*samples).samples[i], (*samples).samples[j]);
    (*samples).samples[j] = max((*samples).samples[i], (*samples).samples[j]);
    (*samples).samples[i] = tmp;
}

// `SortSet`: upstream's nine-comparator sorting network for five elements. The
// comparator order is upstream's and is not an arbitrary valid network - a
// different one sorts the same values but is a different sequence of `min` and
// `max` roundings.
fn sort_set(samples: ptr<function, SampleSet>) {
    compare_swap(samples, 0, 3);
    compare_swap(samples, 1, 4);
    compare_swap(samples, 0, 2);
    compare_swap(samples, 1, 3);
    compare_swap(samples, 0, 1);
    compare_swap(samples, 2, 4);
    compare_swap(samples, 1, 2);
    compare_swap(samples, 3, 4);
    compare_swap(samples, 2, 3);
}

// The smallest relative difference between any pair drawn from the two sorted
// sets, walked merge-style. Signed: negative means this frame is darker.
//
// **Two transliteration details that look like slips and are not.** The `b`
// increment reads `fSet0[a]` *after* `a` may already have been incremented on
// the line above - C's evaluation order, kept - so both indices can advance in
// one iteration. And a pair matching to within `FP16_MIN` sets `fMinDiff` to
// `FP16_MAX`, which both ends the loop and is filtered back to zero by the
// final multiply: "these neighbourhoods are identical" is reported as no
// change, not as an enormous one.
//
// The index clamps are ours. Because both indices can advance together, `a` can
// reach 5 inside five iterations, and upstream relies on HLSL's out-of-bounds
// local-array read. Clamping makes the read defined without changing any value
// the loop can actually use.
fn compute_minimum_difference(set0_in: SampleSet, set1_in: SampleSet) -> f32 {
    var min_diff = FSR3UPSCALER_FP16_MAX - 1.0;
    var a = 0;
    var b = 0;

    var set0 = set0_in;
    var set1 = set1_in;
    sort_set(&set0);
    sort_set(&set1);

    let largest = min(set0.samples[4], set1.samples[4]);

    if largest > FSR3UPSCALER_FP32_MIN {
        for (var i = 0; i < 5 && min_diff < FSR3UPSCALER_FP16_MAX; i++) {
            let v0 = set0.samples[clamp(a, 0, 4)];
            let v1 = set1.samples[clamp(b, 0, 4)];
            var diff = v0 - v1;

            if abs(diff) > FSR3UPSCALER_FP16_MIN {
                diff = sign(diff) * (1.0 - min_divided_by_max(v0, v1));

                if abs(diff) < abs(min_diff) {
                    min_diff = diff;
                }

                a += i32(v0 < v1);
                // Upstream's own read of the *updated* `a`.
                b += i32(set0.samples[clamp(a, 0, 4)] >= v1);
            } else {
                min_diff = FSR3UPSCALER_FP16_MAX;
            }
        }
    }

    return min_diff * f32(min_diff < (FSR3UPSCALER_FP16_MAX - 1.0));
}

fn get_current_luma_bilinear_samples(uv: vec2<f32>) -> SampleSet {
    let uv_jittered = uv + jitter() / vec2<f32>(render_size());
    let base_pos = vec2<i32>(floor(uv_jittered * vec2<f32>(render_size())));

    var samples: SampleSet;
    for (var i = 0; i < 5; i++) {
        let sample_pos = clamp_load(base_pos, I_SAMPLE_OFFSETS[i], render_size());
        var value = load_current_luma(sample_pos) * exposure();
        value = pow(value, F_SHADING_CHANGE_SAMPLE_POW);
        samples.samples[i] = max(value, FSR3UPSCALER_EPSILON);
    }
    return samples;
}

struct PreviousLumaBilinearSamplesData {
    samples: SampleSet,
    is_existing_sample: bool,
}

fn get_previous_luma_bilinear_samples(
    uv: vec2<f32>,
    motion_vector: vec2<f32>,
) -> PreviousLumaBilinearSamplesData {
    var data: PreviousLumaBilinearSamplesData;

    // **The previous frame's jitter and the previous frame's size**, not this
    // frame's: the neighbourhood being compared against was rasterised with
    // that offset, on a grid that may have been a different size if a
    // resolution controller moved the extent.
    let uv_jittered = uv + previous_frame_jitter() / vec2<f32>(previous_frame_render_size());
    let reprojected_uv = uv_jittered + motion_vector;

    data.is_existing_sample = is_uv_inside(reprojected_uv);

    if data.is_existing_sample {
        let base_pos = vec2<i32>(floor(reprojected_uv * vec2<f32>(previous_frame_render_size())));
        for (var i = 0; i < 5; i++) {
            let sample_pos = clamp_load(base_pos, I_SAMPLE_OFFSETS[i], previous_frame_render_size());
            var value = load_previous_luma(sample_pos) * delta_pre_exposure() * exposure();
            value = pow(value, F_SHADING_CHANGE_SAMPLE_POW);
            data.samples.samples[i] = max(value, FSR3UPSCALER_EPSILON);
        }
    }

    return data;
}

fn compute_diff(uv: vec2<f32>, motion_vector: vec2<f32>) -> f32 {
    var min_diff = 0.0;

    let current_samples = get_current_luma_bilinear_samples(uv);
    let previous_data = get_previous_luma_bilinear_samples(uv, motion_vector);

    if previous_data.is_existing_sample {
        min_diff = compute_minimum_difference(current_samples, previous_data.samples);
    }

    return min_diff;
}

// `SpdLoadSourceImage`, minus the dropped indicator channel.
fn spd_load_source_image(px_pos: vec2<i32>) -> vec2<f32> {
    let sample_pos = clamp_load(px_pos, vec2<i32>(0, 0), render_size());
    let dilated_motion_vector = load_dilated_motion_vector(sample_pos);
    let uv = (vec2<f32>(sample_pos) + 0.5) / vec2<f32>(render_size());

    let scaled_and_signed_luma_diff = compute_diff(uv, dilated_motion_vector);

    var out = vec2<f32>(0.0, 0.0);
    out.x = scaled_and_signed_luma_diff;
    out.y = select(0.0, sign(scaled_and_signed_luma_diff), scaled_and_signed_luma_diff != 0.0);
    return out;
}

fn spd_reduce4(v0: vec2<f32>, v1: vec2<f32>, v2: vec2<f32>, v3: vec2<f32>) -> vec2<f32> {
    return (v0 + v1 + v2 + v3) * 0.25;
}

// Level 0: compute the difference at four render-resolution texels and reduce
// them into one half-resolution texel.
//
// The 2x2 tiles do not overlap, so each render-resolution texel's difference is
// computed exactly once - the same total work upstream does inside SPD's own
// source load, arranged differently.
@compute @workgroup_size(8, 8, 1)
fn cs_shading_change_pyramid_mip0(@builtin(global_invocation_id) id: vec3<u32>) {
    let px_pos = vec2<i32>(id.xy);
    if !is_on_screen(px_pos, shading_change_render_size()) {
        return;
    }

    let base = px_pos * 2;
    let v0 = spd_load_source_image(base + vec2<i32>(0, 0));
    let v1 = spd_load_source_image(base + vec2<i32>(1, 0));
    let v2 = spd_load_source_image(base + vec2<i32>(0, 1));
    let v3 = spd_load_source_image(base + vec2<i32>(1, 1));

    textureStore(rw_spd_target, px_pos, vec4<f32>(spd_reduce4(v0, v1, v2, v3), 0.0, 0.0));
}

// The extent of pyramid level `n`, halving from level 0 and never reaching
// zero. Derived rather than passed so that a resolution controller moving the
// render extent moves every level with it - the alternative, an extent in the
// uniform, would be a second place for the rounding to be decided and would
// have to be rewritten every frame.
fn pyramid_level_size(n: u32) -> vec2<i32> {
    var size = shading_change_render_size();
    for (var i = 0u; i < n; i++) {
        size = max(size / 2, vec2<i32>(1, 1));
    }
    return max(size, vec2<i32>(1, 1));
}

// Every level after the first: a plain box reduction of the level below.
//
// `level.x` is the level being *written*, so the source is the one below it.
@compute @workgroup_size(8, 8, 1)
fn cs_shading_change_pyramid_reduce(@builtin(global_invocation_id) id: vec3<u32>) {
    let px_pos = vec2<i32>(id.xy);
    let source_size = pyramid_level_size(level.x - 1u);
    let target_size = pyramid_level_size(level.x);
    if !is_on_screen(px_pos, target_size) {
        return;
    }

    let base = px_pos * 2;
    let v0 = textureLoad(r_spd_source, clamp_load(base, vec2<i32>(0, 0), source_size), 0).xy;
    let v1 = textureLoad(r_spd_source, clamp_load(base, vec2<i32>(1, 0), source_size), 0).xy;
    let v2 = textureLoad(r_spd_source, clamp_load(base, vec2<i32>(0, 1), source_size), 0).xy;
    let v3 = textureLoad(r_spd_source, clamp_load(base, vec2<i32>(1, 1), source_size), 0).xy;

    textureStore(rw_spd_target, px_pos, vec4<f32>(spd_reduce4(v0, v1, v2, v3), 0.0, 0.0));
}
