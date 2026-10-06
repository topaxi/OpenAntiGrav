// AMD FidelityFX Super Resolution 3.1 - the luma-instability pass, ported to
// WGSL. Transliterated from `ffx_fsr3upscaler_luma_instability.h`, AMD
// FidelityFX SDK v1.1.4, MIT - see `common.wesl`'s header and
// `licences/AMD-FidelityFX-MIT.txt`.
//
// **Catches a surface that is oscillating rather than settling.** Each texel
// keeps the last four frames' luma, reprojected along with it; if this frame's
// luma is *further* from the immediately previous frame than it is from an
// older one - and the two differences point the same way - the surface has
// wobbled back towards where it already was. That is a flicker, not a change,
// and the accumulate pass leans on the history less where this says so.
//
// The four-deep history is why `luma_history` is an `RGBA16_FLOAT`: the four
// channels are frames N-1 through N-4, not a colour.
//
// # Two conditions guard it
//
// The instability factor is only computed at all where the accumulation is
// **over 0.9 frames deep** - a texel with almost no history has nothing to
// oscillate against - and it is then scaled down by velocity, disocclusion,
// reactivity and shading change. Every one of those is a reason a luma
// difference is *legitimate* rather than a wobble.
import package::fsr3::common::{
    ACCUMULAION,
    DISOCCLUSION,
    REACTIVE,
    SHADING_CHANGE,
    clamp_uv,
    delta_pre_exposure,
    exposure,
    get_4k_velocity,
    is_on_screen,
    is_uv_inside,
    jitter,
    linear_clamp,
    max_render_size,
    min_divided_by_max,
    min_divided_by_max_or,
    previous_frame_jitter,
    previous_frame_render_size,
    render_size,
};

@group(1) @binding(0) var r_dilated_motion_vectors: texture_2d<f32>;
@group(1) @binding(1) var r_dilated_reactive_masks: texture_2d<f32>;
@group(1) @binding(2) var r_current_luma: texture_2d<f32>;
@group(1) @binding(3) var r_luma_history: texture_2d<f32>;
@group(1) @binding(4) var r_farthest_depth_mip1: texture_2d<f32>;
@group(1) @binding(5) var rw_luma_history: texture_storage_2d<rgba16float, write>;
@group(1) @binding(6) var rw_luma_instability: texture_storage_2d<rgba16float, write>;

fn load_dilated_motion_vector(px_pos: vec2<i32>) -> vec2<f32> {
    return textureLoad(r_dilated_motion_vectors, px_pos, 0).xy;
}

fn sample_dilated_reactive_masks(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(r_dilated_reactive_masks, linear_clamp, uv, 0.0);
}

fn sample_current_luma(uv: vec2<f32>) -> f32 {
    return textureSampleLevel(r_current_luma, linear_clamp, uv, 0.0).x;
}

fn sample_luma_history(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(r_luma_history, linear_clamp, uv, 0.0);
}

fn get_farthest_depth_mip1_resource_dimensions() -> vec2<i32> {
    return vec2<i32>(textureDimensions(r_farthest_depth_mip1, 0));
}

fn sample_farthest_depth_mip1(uv: vec2<f32>) -> f32 {
    return textureSampleLevel(r_farthest_depth_mip1, linear_clamp, uv, 0.0).x;
}

fn store_luma_history(px_pos: vec2<i32>, luma_history: vec4<f32>) {
    textureStore(rw_luma_history, px_pos, luma_history);
}

fn store_luma_instability(px_pos: vec2<i32>, luma_instability: f32) {
    textureStore(
        rw_luma_instability,
        px_pos,
        vec4<f32>(luma_instability, 0.0, 0.0, 0.0),
    );
}

struct LumaInstabilityFactorData {
    luma_history: vec4<f32>,
    luma_instability_factor: f32,
}

const N_MINUS_1: i32 = 0;
const N_MINUS_2: i32 = 1;
const N_MINUS_3: i32 = 2;
const N_MINUS_4: i32 = 3;

// **`fFarthestDepthInMeters` is taken and never used**, upstream included. The
// parameter is threaded all the way from a sampled mip that exists only to feed
// it, which suggests a heuristic that was tried and removed without the
// plumbing being taken out. Kept as upstream has it, because dropping it would
// also drop the only thing that reads `farthest_depth_mip1` and turn two other
// passes into dead code - a bigger deviation than carrying an unused argument.
fn compute_luma_instability_factor(
    data_in: LumaInstabilityFactorData,
    current_frame_luma: f32,
    farthest_depth_in_meters: f32,
) -> LumaInstabilityFactorData {
    var data = data_in;

    var luma_instability = 0.0;
    let diffs0 = current_frame_luma - data.luma_history[N_MINUS_1];
    let similarity0 = min_divided_by_max_or(current_frame_luma, data.luma_history[N_MINUS_1], 1.0);

    var max_similarity = similarity0;

    if similarity0 < 1.0 {
        for (var i = N_MINUS_2; i <= N_MINUS_4; i++) {
            let diffs1 = current_frame_luma - data.luma_history[i];
            let similarity1 = min_divided_by_max(current_frame_luma, data.luma_history[i]);

            // Same sign: this frame moved the *same* way from an older frame as
            // it did from the last one, so an older frame being a closer match
            // means the value came back rather than went on.
            if sign(diffs0) == sign(diffs1) {
                max_similarity = max(max_similarity, similarity1);
            }
        }

        luma_instability = f32(max_similarity > similarity0);
    }

    // Shift history.
    data.luma_history[N_MINUS_4] = data.luma_history[N_MINUS_3];
    data.luma_history[N_MINUS_3] = data.luma_history[N_MINUS_2];
    data.luma_history[N_MINUS_2] = data.luma_history[N_MINUS_1];
    data.luma_history[N_MINUS_1] = current_frame_luma;

    data.luma_history /= exposure();

    // **The oldest slot gates the answer.** A texel whose history is not yet
    // four frames deep has a zero in `N_MINUS_4`, and reports no instability
    // whatever the other three say - so a freshly disoccluded surface is never
    // called unstable on the strength of two samples.
    data.luma_instability_factor = luma_instability * f32(data.luma_history[N_MINUS_4] != 0.0);

    return data;
}

fn luma_instability(px_pos: vec2<i32>) {
    var data: LumaInstabilityFactorData;
    data.luma_instability_factor = 0.0;
    data.luma_history = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    let dilated_motion_vector = load_dilated_motion_vector(px_pos);
    let uv = (vec2<f32>(px_pos) + 0.5) / vec2<f32>(render_size());
    let uv_curr_frame_jittered = uv + jitter() / vec2<f32>(render_size());
    let uv_prev_frame_jittered =
        uv + previous_frame_jitter() / vec2<f32>(previous_frame_render_size());
    let reprojected_uv = uv_prev_frame_jittered + dilated_motion_vector;

    if is_uv_inside(reprojected_uv) {
        let uv_reactive_hw = clamp_uv(uv_curr_frame_jittered, render_size(), max_render_size());

        let dilated_reactive_masks = sample_dilated_reactive_masks(uv_reactive_hw);
        let reactive_mask = saturate(dilated_reactive_masks[REACTIVE]);
        let disocclusion = saturate(dilated_reactive_masks[DISOCCLUSION]);
        let shading_change = saturate(dilated_reactive_masks[SHADING_CHANGE]);
        let accumulation = saturate(dilated_reactive_masks[ACCUMULAION]);

        if accumulation > 0.9 {
            let uv_hw = clamp_uv(uv_curr_frame_jittered, render_size(), max_render_size());
            let current_frame_luma = sample_current_luma(uv_hw) * exposure();

            let reprojected_uv_hw = clamp_uv(
                reprojected_uv,
                previous_frame_render_size(),
                max_render_size(),
            );
            data.luma_history =
                sample_luma_history(reprojected_uv_hw) * delta_pre_exposure() * exposure();

            let farthest_depth_uv_hw = clamp_uv(
                uv_curr_frame_jittered,
                render_size() / 2,
                get_farthest_depth_mip1_resource_dimensions(),
            );
            let farthest_depth_in_meters = sample_farthest_depth_mip1(farthest_depth_uv_hw);

            data = compute_luma_instability_factor(
                data,
                current_frame_luma,
                farthest_depth_in_meters,
            );

            let velocity_weight = 1.0 - saturate(get_4k_velocity(dilated_motion_vector) / 20.0);
            data.luma_instability_factor *= velocity_weight
                * (1.0 - disocclusion)
                * (1.0 - reactive_mask)
                * (1.0 - shading_change);
        }
    }

    // **Stored unconditionally, including the zeros.** A texel that failed
    // either guard writes a cleared history rather than leaving the previous
    // frame's in place - which is what makes the four-frame window a window
    // rather than a smear, and is why this is outside the `if`.
    store_luma_history(px_pos, data.luma_history);
    store_luma_instability(px_pos, data.luma_instability_factor);
}

@compute @workgroup_size(8, 8, 1)
fn cs_luma_instability(@builtin(global_invocation_id) id: vec3<u32>) {
    let px_pos = vec2<i32>(id.xy);
    if !is_on_screen(px_pos, render_size()) {
        return;
    }
    luma_instability(px_pos);
}
