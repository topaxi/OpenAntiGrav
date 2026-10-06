// AMD FidelityFX Super Resolution 3.1 - the prepare-reactivity pass, ported to
// WGSL. Transliterated from `ffx_fsr3upscaler_prepare_reactivity.h`, AMD
// FidelityFX SDK v1.1.4, MIT - see `common.wesl`'s header and
// `licences/AMD-FidelityFX-MIT.txt`.
//
// The pass that decides, per render-resolution texel, **how much the history is
// worth**. Its output is one `Rgba8Unorm` texel of four independent answers -
// reactive, disoccluded, shading-changed, and how many frames have accumulated
// there - which `accumulate` reads to weight this frame's sample against the
// history's. It also plants *locks*: single-pixel features thin enough that
// accumulation would smear them away get marked, at presentation resolution, so
// the resolve holds on to them.
//
// # Two inputs this renderer does not have
//
// Upstream takes an application-authored **reactive mask** (surfaces whose
// appearance does not follow the motion vectors - particles, animated
// materials) and a **transparency-and-composition mask** (surfaces drawn after
// the velocity pass). An application that supplies neither gets upstream's own
// 1x1 default for both, which holds zero.
//
// This renderer supplies neither *yet*, so both are that zero, written here as
// a constant with its own name rather than as a resource nobody fills. It could
// author them - it draws its own frame and knows exactly which draws are
// particles - and if it ever does, `dilate_reactive_masks` and
// `dilate_transparency_and_composition_masks` are the two functions that grow a
// texture read. That is a deliberately small change, and it is why they exist
// as functions rather than being folded away.
import package::fsr3::common::{
    ACCUMULAION,
    DISOCCLUSION,
    FSR3UPSCALER_FP16_MAX,
    FSR3UPSCALER_FP32_MAX,
    FSR3UPSCALER_FP32_MIN,
    F_RECONSTRUCTED_DEPTH_BILINEAR_WEIGHT_THRESHOLD,
    REACTIVE,
    SHADING_CHANGE,
    accumulation_added_per_frame,
    clamp_load,
    clamp_uv,
    compute_hr_pos_from_lr_pos,
    exposure,
    get_4k_velocity,
    get_bilinear_sampling_data,
    get_view_space_depth,
    get_view_space_depth_in_meters,
    is_on_screen,
    is_uv_inside,
    jitter,
    linear_clamp,
    max_render_size,
    min_disocclusion_accumulation,
    min_divided_by_max,
    previous_frame_render_size,
    reactiveness_scale,
    reconstructed_depth_mv_px_threshold,
    render_size,
    shading_change_max_render_size,
    shading_change_render_size,
    shading_change_scale,
    velocity_factor,
    view_space_to_meters_factor,
};

@group(1) @binding(0) var r_dilated_motion_vectors: texture_2d<f32>;
@group(1) @binding(1) var r_dilated_depth: texture_2d<f32>;
// Read-only here, where `prepare_inputs` scattered into it atomically. See that
// pass for why it is a buffer.
@group(1) @binding(2) var<storage, read> r_reconstructed_previous_nearest_depth: array<u32>;
@group(1) @binding(3) var r_current_luma: texture_2d<f32>;
@group(1) @binding(4) var r_shading_change: texture_2d<f32>;
@group(1) @binding(5) var r_accumulation: texture_2d<f32>;
@group(1) @binding(6) var rw_dilated_reactive_masks: texture_storage_2d<rgba8unorm, write>;
@group(1) @binding(7) var rw_accumulation: texture_storage_2d<rgba8unorm, write>;
@group(1) @binding(8) var rw_new_locks: texture_storage_2d<rgba8unorm, write>;

fn load_dilated_motion_vector(px_pos: vec2<i32>) -> vec2<f32> {
    return textureLoad(r_dilated_motion_vectors, px_pos, 0).xy;
}

fn load_dilated_depth(px_pos: vec2<i32>) -> f32 {
    return textureLoad(r_dilated_depth, px_pos, 0).x;
}

fn load_current_luma(px_pos: vec2<i32>) -> f32 {
    return textureLoad(r_current_luma, px_pos, 0).x;
}

fn load_reconstructed_prev_depth(px_pos: vec2<i32>) -> f32 {
    let index = u32(px_pos.y) * u32(render_size().x) + u32(px_pos.x);
    return bitcast<f32>(r_reconstructed_previous_nearest_depth[index]);
}

fn sample_shading_change(uv: vec2<f32>) -> f32 {
    return textureSampleLevel(r_shading_change, linear_clamp, uv, 0.0).x * shading_change_scale();
}

fn sample_accumulation(uv: vec2<f32>) -> f32 {
    return textureSampleLevel(r_accumulation, linear_clamp, uv, 0.0).x;
}

fn store_dilated_reactive_masks(px_pos: vec2<i32>, masks: vec4<f32>) {
    textureStore(rw_dilated_reactive_masks, px_pos, masks);
}

fn store_accumulation(px_pos: vec2<i32>, accumulation: f32) {
    textureStore(rw_accumulation, px_pos, vec4<f32>(accumulation, 0.0, 0.0, 0.0));
}

fn store_new_locks(px_pos: vec2<i32>, new_lock: f32) {
    textureStore(rw_new_locks, px_pos, vec4<f32>(new_lock, 0.0, 0.0, 0.0));
}

// Upstream's `LoadReactiveMask`, at this renderer's default. See the header.
fn load_reactive_mask(px_pos: vec2<i32>) -> f32 {
    return 0.0 * reactiveness_scale();
}

// **Is this pixel newly revealed?** This frame's depth is compared against the
// depth `prepare_inputs` scattered into the previous frame's grid: if the
// surface here is *further* than whatever was standing at the reprojected
// position last frame, something that was in front has moved away and there is
// no history for it.
fn compute_disocclusions(
    uv: vec2<f32>,
    motion_vector_in: vec2<f32>,
    current_depth_view_space: f32,
) -> f32 {
    var motion_vector = motion_vector_in;
    let nearest_depth_in_meters = min(
        current_depth_view_space * view_space_to_meters_factor(),
        FSR3UPSCALER_FP16_MAX,
    );
    let reconstructed_depth_mv_threshold = reconstructed_depth_mv_px_threshold(nearest_depth_in_meters);

    motion_vector *= f32(get_4k_velocity(motion_vector) > reconstructed_depth_mv_threshold);

    let reprojected_uv = uv + motion_vector;
    let bilinear_info = get_bilinear_sampling_data(reprojected_uv, render_size());

    var disocclusion = 0.0;
    var weight_sum = 0.0;
    var potential_disocclusion = true;

    for (var sample_index = 0; sample_index < 4 && potential_disocclusion; sample_index++) {
        let offset = bilinear_info.offsets[sample_index];
        let sample_pos = clamp_load(bilinear_info.base_pos, offset, render_size());

        if is_on_screen(sample_pos, render_size()) {
            let weight = bilinear_info.weights[sample_index];
            if weight > F_RECONSTRUCTED_DEPTH_BILINEAR_WEIGHT_THRESHOLD {
                let prev_nearest_depth_view_space =
                    get_view_space_depth(load_reconstructed_prev_depth(sample_pos));
                let depth_difference = current_depth_view_space - prev_nearest_depth_view_space;

                potential_disocclusion =
                    potential_disocclusion && (depth_difference > FSR3UPSCALER_FP32_MIN);

                if potential_disocclusion {
                    let half_viewport_width = length(vec2<f32>(render_size()) * 0.5);
                    let depth_threshold = max(current_depth_view_space, prev_nearest_depth_view_space);

                    // Upstream's own separation constant, in view-space metres
                    // per pixel of half-viewport per unit depth. Unexplained
                    // there and unexplained here; it is a tuning value, not a
                    // derivation.
                    let ksep = 1.37e-05;
                    let required_depth_separation = ksep * half_viewport_width * depth_threshold;

                    disocclusion += saturate(required_depth_separation / depth_difference) * weight;
                    weight_sum += weight;
                }
            }
        }
    }

    if potential_disocclusion && weight_sum > 0.0 {
        return saturate(1.0 - disocclusion / weight_sum);
    }
    return 0.0;
}

// **Does this pixel's motion agree with the motion at the place it came from?**
// A surface moving with the camera reprojects onto a texel whose own vector is
// similar; a particle or an animated material does not, and the disagreement is
// what marks it reactive without the application having to say so.
fn compute_motion_divergence(
    uv: vec2<f32>,
    motion_vector: vec2<f32>,
    current_depth_sample: f32,
) -> f32 {
    let reprojected_pos = vec2<i32>((uv + motion_vector) * vec2<f32>(render_size()));
    let reprojected_depth = load_dilated_depth(reprojected_pos);
    let reprojected_motion_vector = load_dilated_motion_vector(reprojected_pos);

    let reprojected_velocity = get_4k_velocity(reprojected_motion_vector);
    let velocity_4k = get_4k_velocity(motion_vector);

    let nucleus_depth_in_meters = get_view_space_depth_in_meters(reprojected_depth);
    let current_depth_in_meters = get_view_space_depth_in_meters(current_depth_sample);

    let distance_factor = min_divided_by_max(nucleus_depth_in_meters, current_depth_in_meters);
    let velocity_factor = saturate(velocity_4k / 10.0);
    return (1.0 - saturate(reprojected_velocity / velocity_4k)) * distance_factor * velocity_factor;
}

fn dilate_reactive_masks(px_pos: vec2<i32>) -> f32 {
    var dilated = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let sample_coord = clamp_load(px_pos, vec2<i32>(x, y), render_size());
            dilated = max(dilated, load_reactive_mask(sample_coord));
        }
    }
    return dilated;
}

fn dilate_transparency_and_composition_masks(uv: vec2<f32>) -> f32 {
    // Upstream samples the application's mask here, clamped into its own
    // resource's coordinates. See this file's header for why it is a constant.
    return 0.0;
}

// **Is this a thin bright feature that accumulation would erase?** The 3x3
// neighbourhood is split into similar and dissimilar by luma, and the centre is
// a *ridge* if it lies outside the dissimilar ones' range entirely. A ridge
// that is also not part of any solid 2x2 quadrant is a one-pixel feature, and
// gets a lock proportional to how much contrast it carries.
fn compute_thin_feature_confidence(px_pos: vec2<i32>) -> f32 {
    //  1 2 3
    //  4 0 5
    //  6 7 8
    let nucleus_index = 0;
    let sample_offsets = array<vec2<i32>, 9>(
        vec2<i32>(0, 0),
        vec2<i32>(-1, -1),
        vec2<i32>(0, -1),
        vec2<i32>(1, -1),
        vec2<i32>(-1, 0),
        vec2<i32>(1, 0),
        vec2<i32>(-1, 1),
        vec2<i32>(0, 1),
        vec2<i32>(1, 1),
    );

    var samples: array<f32, 9>;
    var luma_min = FSR3UPSCALER_FP32_MAX;
    var luma_max = FSR3UPSCALER_FP32_MIN;

    for (var sample_index = 0; sample_index < 9; sample_index++) {
        let sample_pos = clamp_load(px_pos, sample_offsets[sample_index], render_size());
        samples[sample_index] = load_current_luma(sample_pos) * exposure();

        luma_min = min(luma_min, samples[sample_index]);
        luma_max = max(luma_max, samples[sample_index]);
    }

    let threshold = 0.9;
    var dissimilar_luma_min = FSR3UPSCALER_FP32_MAX;
    var dissimilar_luma_max = 0.0;

    // Bit `n` set means sample `n` is *similar* to the nucleus. The nucleus is
    // similar to itself by definition.
    var pattern_mask = 1u << u32(nucleus_index);

    // The four 2x2 quadrants around the nucleus. If every sample of any one of
    // them is similar, the centre is part of a solid block rather than a thin
    // feature, and no lock is planted.
    let rejection_masks = array<u32, 4>(
        (1u << 1u) | (1u << 2u) | (1u << 4u) | (1u << u32(nucleus_index)), // upper left
        (1u << 2u) | (1u << 3u) | (1u << 5u) | (1u << u32(nucleus_index)), // upper right
        (1u << 4u) | (1u << 6u) | (1u << 7u) | (1u << u32(nucleus_index)), // lower left
        (1u << 5u) | (1u << 7u) | (1u << 8u) | (1u << u32(nucleus_index)), // lower right
    );

    var bit_index = 1u;
    for (var sample_index = 1; sample_index < 9; sample_index++) {
        let difference =
            abs(samples[sample_index] - samples[nucleus_index]) / (luma_max - luma_min);

        if difference < threshold {
            pattern_mask |= 1u << bit_index;
        } else {
            dissimilar_luma_min = min(dissimilar_luma_min, samples[sample_index]);
            dissimilar_luma_max = max(dissimilar_luma_max, samples[sample_index]);
        }
        bit_index += 1u;
    }

    let is_ridge = samples[nucleus_index] > dissimilar_luma_max
        || samples[nucleus_index] < dissimilar_luma_min;

    if !is_ridge {
        return 0.0;
    }

    for (var i = 0; i < 4; i++) {
        if (pattern_mask & rejection_masks[i]) == rejection_masks[i] {
            return 0.0;
        }
    }

    return 1.0 - luma_min / luma_max;
}

// How many frames' worth of history this texel has, advanced by one and knocked
// back by whatever happened to it.
//
// Upstream's own worked example, kept because the two `lerp`s are otherwise
// hard to read: with a shading change of 1.0 at frame N and nothing after it,
// the accumulation runs 0.000, 0.333, 0.666, 0.999. With a *disocclusion* of
// 1.0 instead it starts at -0.333, one third of a frame in debt, and takes an
// extra frame to reach the same place - a newly revealed surface is trusted
// less than a merely re-lit one.
fn update_accumulation(
    px_pos: vec2<i32>,
    uv: vec2<f32>,
    motion_vector: vec2<f32>,
    disocclusion: f32,
    shading_change: f32,
) -> f32 {
    let reprojected_uv = uv + motion_vector;
    var accumulation = 0.0;

    if is_uv_inside(reprojected_uv) {
        let reprojected_uv_hw = clamp_uv(
            reprojected_uv,
            previous_frame_render_size(),
            max_render_size(),
        );
        accumulation = saturate(sample_accumulation(reprojected_uv_hw));
    }

    accumulation = mix(accumulation, 0.0, shading_change);
    accumulation = mix(
        accumulation,
        min(min_disocclusion_accumulation(), accumulation),
        disocclusion,
    );

    // Snaps a hundredth or less to zero, so that a texel a whisker above the
    // floor does not read as having history.
    accumulation *= f32(round(accumulation * 100.0) > 1.0);

    // Stored one frame ahead, and saturated because the target is unorm and
    // cannot hold the negative the disocclusion path produces. The *returned*
    // value keeps its sign; only the stored one is clamped.
    store_accumulation(px_pos, saturate(accumulation + accumulation_added_per_frame()));

    return accumulation;
}

fn compute_shading_change(uv: vec2<f32>) -> f32 {
    // NOTE: Here we re-apply jitter, will be reverted again when sampled in
    // accumulation pass.
    let shading_change_uv = clamp_uv(
        uv - jitter() / vec2<f32>(render_size()),
        shading_change_render_size(),
        shading_change_max_render_size(),
    );
    return saturate(sample_shading_change(shading_change_uv));
}

fn prepare_reactivity(px_pos: vec2<i32>) {
    let uv = (vec2<f32>(px_pos) + 0.5) / vec2<f32>(render_size());
    let motion_vector = load_dilated_motion_vector(px_pos);

    let dilated_depth = load_dilated_depth(px_pos);

    let disocclusion = compute_disocclusions(uv, motion_vector, get_view_space_depth(dilated_depth));
    let shading_change = max(dilate_reactive_masks(px_pos), compute_shading_change(uv));

    let motion_divergence = compute_motion_divergence(uv, motion_vector, dilated_depth);
    let dilated_transparency_and_composition = dilate_transparency_and_composition_masks(uv);
    let final_reactiveness = max(motion_divergence, dilated_transparency_and_composition);

    let accumulation = update_accumulation(px_pos, uv, motion_vector, disocclusion, shading_change);

    var out: vec4<f32>;
    out[REACTIVE] = final_reactiveness;
    out[DISOCCLUSION] = disocclusion;
    out[SHADING_CHANGE] = shading_change;
    out[ACCUMULAION] = accumulation;

    store_dilated_reactive_masks(px_pos, out);

    let lock_strength = compute_thin_feature_confidence(px_pos);
    if lock_strength > (1.0 / 100.0) {
        // **A scatter into the presentation grid**, which is why `new_locks` is
        // cleared before this pass rather than written everywhere: most
        // presentation texels have no render-resolution pixel landing on them
        // at all. Upstream instead relies on `accumulate` zeroing each texel as
        // it consumes it, which would need a read-write storage texture here -
        // see `post::fsr3`'s own note on the clear.
        store_new_locks(compute_hr_pos_from_lr_pos(px_pos), lock_strength);
    }
}

@compute @workgroup_size(8, 8, 1)
fn cs_prepare_reactivity(@builtin(global_invocation_id) id: vec3<u32>) {
    let px_pos = vec2<i32>(id.xy);
    if !is_on_screen(px_pos, render_size()) {
        return;
    }
    prepare_reactivity(px_pos);
}
