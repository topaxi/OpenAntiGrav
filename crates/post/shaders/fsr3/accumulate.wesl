// AMD FidelityFX Super Resolution 3.1 - the accumulate pass, ported to WGSL.
// Transliterated from `ffx_fsr3upscaler_accumulate.h` together with the two
// headers it pulls in whole, `ffx_fsr3upscaler_upsample.h` and
// `ffx_fsr3upscaler_reproject.h`, plus the Lanczos helpers of
// `ffx_fsr3upscaler_sample.h`. AMD FidelityFX SDK v1.1.4, MIT - see
// `common.wesl`'s header and `licences/AMD-FidelityFX-MIT.txt`.
//
// **The resolve itself, and the first pass here that runs at presentation
// resolution.** Everything before it was measurement; this is where a
// presentation pixel is actually decided, out of three things:
//
// 1. an **upsample** of the nine render-resolution samples around it, weighted
//    by a Lanczos-2 kernel over their true (un-jittered) positions - which is
//    what turns a sub-pixel offset into detail rather than a wobble;
// 2. the **history**, reprojected along the motion vector with a 4x4 Lanczos
//    bicubic and then *rectified* - pulled back towards the colours actually
//    present in this frame's neighbourhood, by an amount the previous passes
//    spent their whole existence deciding;
// 3. a **lock**, which protects a thin feature from being averaged away and
//    decays over one jitter sequence.
//
// # Four upstream options are compiled in rather than branched on
//
// - `FFX_FSR3UPSCALER_USE_XBOX_PAIRED_16BIT_MATH_OPTIMIZATIONS` is off. It is
//   Scarlett-only and needs `__XB_` intrinsics; upstream ships the scalar path
//   beside it and that is what this is.
// - `FFX_FSR3UPSCALER_OPTION_HDR_COLOR_INPUT` is off. This renderer's scene
//   target is low dynamic range, so the tonemap-either-side-of-accumulation
//   dance has nothing to do. `tonemap` and `inverse_tonemap` stay in
//   `common.wesl` for when it does.
// - `FFX_FSR3UPSCALER_OPTION_UPSAMPLE_USE_LANCZOS_TYPE` is **2, approximate**,
//   and `..._REPROJECT_USE_LANCZOS_TYPE` is **0, reference** - both upstream's
//   own defaults, and both analytic. That is why `LANCZOS_LUT` is not among
//   this port's resources: neither default reads it.
// - `FFX_FSR3UPSCALER_OPTION_APPLY_SHARPENING` is on, so this pass writes only
//   the internal history and `rcas` writes the output. See `post::fsr3`.
import package::fsr3::common::{
    ACCUMULAION,
    DISOCCLUSION,
    FSR3UPSCALER_EPSILON,
    FSR3UPSCALER_FP16_MIN,
    FSR3UPSCALER_FP32_MAX,
    F_AVERAGE_LANCZOS_WEIGHT_PER_FRAME,
    F_LOCK_MAX,
    F_LOCK_THRESHOLD,
    REACTIVE,
    RectificationBox,
    SHADING_CHANGE,
    clamp_load,
    clamp_uv,
    constants,
    delta_pre_exposure,
    downscale_factor,
    exposure,
    frame_index,
    get_4k_velocity,
    is_on_screen,
    is_uv_inside,
    jitter,
    jitter_phase_count,
    linear_clamp,
    max_render_size,
    previous_frame_upscale_size,
    rectification_box_add_sample,
    rectification_box_compute_variance_box_data,
    render_size,
    rgb_to_ycocg,
    upscale_size,
    velocity_factor,
    ycocg_to_rgb,
};

@group(1) @binding(0) var r_input_color_jittered: texture_2d<f32>;
@group(1) @binding(1) var r_dilated_motion_vectors: texture_2d<f32>;
@group(1) @binding(2) var r_dilated_reactive_masks: texture_2d<f32>;
@group(1) @binding(3) var r_luma_instability: texture_2d<f32>;
@group(1) @binding(4) var r_farthest_depth_mip1: texture_2d<f32>;
// The previous frame's resolve. `textureLoad`ed rather than sampled: the
// reprojection is a 4x4 Lanczos bicubic built out of individual taps, so no
// hardware filtering is involved.
@group(1) @binding(5) var r_internal_upscaled_color: texture_2d<f32>;
@group(1) @binding(6) var r_new_locks: texture_2d<f32>;
@group(1) @binding(7) var rw_internal_upscaled_color: texture_storage_2d<rgba16float, write>;

fn load_input_color(px_pos: vec2<i32>) -> vec3<f32> {
    return textureLoad(r_input_color_jittered, px_pos, 0).rgb;
}

fn load_dilated_motion_vector(px_pos: vec2<i32>) -> vec2<f32> {
    return textureLoad(r_dilated_motion_vectors, px_pos, 0).xy;
}

fn sample_dilated_reactive_masks(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(r_dilated_reactive_masks, linear_clamp, uv, 0.0);
}

fn sample_luma_instability(uv: vec2<f32>) -> f32 {
    return textureSampleLevel(r_luma_instability, linear_clamp, uv, 0.0).x;
}

fn get_farthest_depth_mip1_resource_dimensions() -> vec2<i32> {
    return vec2<i32>(textureDimensions(r_farthest_depth_mip1, 0));
}

fn sample_farthest_depth_mip1(uv: vec2<f32>) -> f32 {
    return textureSampleLevel(r_farthest_depth_mip1, linear_clamp, uv, 0.0).x;
}

fn load_history(px_history: vec2<i32>) -> vec4<f32> {
    return textureLoad(r_internal_upscaled_color, px_history, 0);
}

fn load_rw_new_locks(px_pos: vec2<i32>) -> f32 {
    return textureLoad(r_new_locks, px_pos, 0).x;
}

fn store_internal_color_and_weight(px_pos: vec2<i32>, color_and_weight: vec4<f32>) {
    textureStore(rw_internal_upscaled_color, px_pos, color_and_weight);
}

fn jitter_sequence_length() -> f32 {
    return constants.jitter_phase_count;
}

// `ClampCoord`, from `ffx_fsr3upscaler_sample.h`. **Inset by one, not clamped
// to the edge**: a 4x4 bicubic reaches two texels out, so clamping to
// `[0, size-1]` would let a base position on the border produce taps outside
// the texture. Upstream's `[1, size-2]` is what keeps every one of the sixteen
// in range.
fn clamp_coord(px_sample: vec2<i32>, px_offset: vec2<i32>, texture_size: vec2<i32>) -> vec2<i32> {
    var result = px_sample + px_offset;
    result.x = max(1, min(result.x, texture_size.x - 2));
    result.y = max(1, min(result.y, texture_size.y - 2));
    return result;
}

fn lanczos2_no_clamp(x: f32) -> f32 {
    let pi = 3.141592653589793;
    if abs(x) < FSR3UPSCALER_EPSILON {
        return 1.0;
    }
    return (sin(pi * x) / (pi * x)) * (sin(0.5 * pi * x) / (0.5 * pi * x));
}

fn lanczos2(x_in: f32) -> f32 {
    let x = min(abs(x_in), 2.0);
    return lanczos2_no_clamp(x);
}

// FSR1's Lanczos approximation. Input is x*x and must be <= 4. Upstream's
// constants, unexplained there and unexplained here.
fn lanczos2_approx_sq_no_clamp(x2: f32) -> f32 {
    let a = (2.0 / 5.0) * x2 - 1.0;
    let b = (1.0 / 4.0) * x2 - 1.0;
    return ((25.0 / 16.0) * a * a - (25.0 / 16.0 - 1.0)) * (b * b);
}

fn lanczos2_approx_sq(x2_in: f32) -> f32 {
    let x2 = min(x2_in, 4.0);
    return lanczos2_approx_sq_no_clamp(x2);
}

// The four Lanczos-2 weights for one fractional offset.
//
// **Split out of `lanczos2_row` so a row of four can share one set.** Upstream
// computes the weights inside the interpolation, which is fine where it is
// written once - but the 4x4 fetch below calls it four times with the *same*
// horizontal fraction, and every `lanczos2` here is the reference sinc, two
// `sin` each. Computed per row that is 40 `sin` for one presentation pixel;
// hoisted it is 16. The arithmetic is unchanged: same expression, same input.
fn lanczos2_weights(t: f32) -> vec4<f32> {
    return vec4<f32>(
        lanczos2(-1.0 - t),
        lanczos2(-0.0 - t),
        lanczos2(1.0 - t),
        lanczos2(2.0 - t),
    );
}

// The four-tap Lanczos-2 interpolation the history reprojection is built from.
fn lanczos2_row(c0: vec4<f32>, c1: vec4<f32>, c2: vec4<f32>, c3: vec4<f32>, w: vec4<f32>) -> vec4<f32> {
    return (w.x * c0 + w.y * c1 + w.z * c2 + w.w * c3) / (w.x + w.y + w.z + w.w);
}

// `HistorySample`: the 4x4 fetch, the separable Lanczos over it, and the
// deringing clamp against the inner 2x2. `DeclareCustomTextureSample` and
// `DeclareCustomFetchBicubicSamples` written out, because WGSL has no macros
// and there is only one instantiation of either.
fn history_sample(uv_sample: vec2<f32>, texture_size: vec2<i32>) -> vec4<f32> {
    var px_sample = (uv_sample * vec2<f32>(texture_size)) - vec2<f32>(0.5, 0.5);
    let px_frac = fract(px_sample);
    // Clamp base coords.
    px_sample.x = max(0.0, min(f32(texture_size.x - 1), px_sample.x));
    px_sample.y = max(0.0, min(f32(texture_size.y - 1), px_sample.y));

    let base = vec2<i32>(floor(px_sample));
    // One weight set for all four rows - see `lanczos2_weights`.
    let wx = lanczos2_weights(px_frac.x);

    // **Deringing, against the inner four taps only** - the ones the
    // interpolation is actually between. A Lanczos kernel has negative lobes
    // and will overshoot on an edge; this is what stops the overshoot becoming
    // a halo that then feeds back into the next frame's history.
    //
    // **Accumulated inside the fetch loop rather than re-fetched after it.**
    // Those four taps are exactly rows 1 and 2's two middle ones, which the
    // loop already has in registers; upstream's macro loads them a second time,
    // which is four extra `textureLoad`s out of twenty from the largest texture
    // this pass reads. `min` and `max` are associative and exact, so folding
    // them in this order is the same four-way reduction.
    var deringing_min = vec4<f32>(FSR3UPSCALER_FP32_MAX);
    var deringing_max = vec4<f32>(-FSR3UPSCALER_FP32_MAX);

    var rows: array<vec4<f32>, 4>;
    for (var row = 0; row < 4; row++) {
        let dy = row - 1;
        let c0 = load_history(clamp_coord(base, vec2<i32>(-1, dy), texture_size));
        let c1 = load_history(clamp_coord(base, vec2<i32>(0, dy), texture_size));
        let c2 = load_history(clamp_coord(base, vec2<i32>(1, dy), texture_size));
        let c3 = load_history(clamp_coord(base, vec2<i32>(2, dy), texture_size));
        if row == 1 || row == 2 {
            deringing_min = min(deringing_min, min(c1, c2));
            deringing_max = max(deringing_max, max(c1, c2));
        }
        rows[row] = lanczos2_row(c0, c1, c2, c3, wx);
    }
    var color = lanczos2_row(rows[0], rows[1], rows[2], rows[3], lanczos2_weights(px_frac.y));

    color = clamp(color, deringing_min, deringing_max);

    return color;
}

// `AccumulationPassCommonParams` and `AccumulationPassData`, upstream's two
// carriers. Split the same way, because every function below takes one and
// modifies the other.
struct CommonParams {
    px_hr_pos: vec2<i32>,
    hr_uv: vec2<f32>,
    lr_uv_jittered: vec2<f32>,
    lr_uv_hw_sampler: vec2<f32>,
    motion_vector: vec2<f32>,
    reprojected_hr_uv: vec2<f32>,
    velocity_4k: f32,
    disocclusion: f32,
    reactive_mask: f32,
    shading_change: f32,
    accumulation: f32,
    luma_instability_factor: f32,
    farthest_depth_in_meters: f32,
    is_existing_sample: bool,
    is_new_sample: bool,
}

struct PassData {
    clipping_box: RectificationBox,
    upsampled_color: vec3<f32>,
    upsampled_weight: f32,
    history_color: vec3<f32>,
    history_weight: f32,
    lock: f32,
    lock_contribution_this_frame: f32,
}

// `GetMotionVector` with `LOW_RESOLUTION_MOTION_VECTORS` on: the presentation
// position is mapped back onto the render grid, because that is where the
// velocity attachment lives.
fn get_motion_vector(hr_uv: vec2<f32>) -> vec2<f32> {
    return load_dilated_motion_vector(vec2<i32>(hr_uv * vec2<f32>(render_size())));
}

fn load_prepared_color(sample_pos: vec2<i32>) -> vec3<f32> {
    let rgb = max(vec3<f32>(0.0), load_input_color(sample_pos)) * exposure();
    return rgb_to_ycocg(rgb);
}

fn compute_max_kernel_weight() -> f32 {
    let kernel_size_bias = 1.0 + (1.0 / downscale_factor().x - 1.0);
    return min(1.99, kernel_size_bias);
}

fn get_upsample_lanczos_weight(src_sample_offset: vec2<f32>, kernel_weight: f32) -> f32 {
    let biased = src_sample_offset * kernel_weight;
    return lanczos2_approx_sq(dot(biased, biased));
}

// `ComputeUpsampledColorAndWeight`, upstream's scalar path.
//
// **The nine taps are around the render pixel this presentation pixel falls
// in, and their offsets are measured from the *un-jittered* sample position.**
// That is the whole mechanism: the jitter moved where the render pixel's sample
// actually sits inside itself, so the same nine texels carry different
// sub-pixel information every frame, and the Lanczos weights over those true
// positions are what let several frames add up to more than one frame's detail.
fn compute_upsampled_color_and_weight(params: CommonParams, data_in: PassData) -> PassData {
    var data = data_in;

    // We compute a sliced lanczos filter with 2 lobes (other slices are
    // accumulated temporally).
    let dst_output_pos = vec2<f32>(params.px_hr_pos) + vec2<f32>(0.5);
    let src_output_pos = dst_output_pos * downscale_factor();
    let src_input_pos = vec2<i32>(floor(src_output_pos));
    // This is the un-jittered position of the sample at offset 0,0.
    let src_unjittered_pos = (vec2<f32>(src_input_pos) + vec2<f32>(0.5, 0.5)) - jitter();
    let base_sample_offset = src_unjittered_pos - src_output_pos;

    var offset_tl: vec2<i32>;
    offset_tl.x = select(-1, -2, src_unjittered_pos.x > src_output_pos.x);
    offset_tl.y = select(-1, -2, src_unjittered_pos.y > src_output_pos.y);

    // If the un-jittered position is past the output position the top-left
    // offset is -2 and the sample offsets run [-2, 1], which would put the
    // clipping box on rows [1, 3]. Flipping the row and column indices keeps
    // the first three of the sampled array usable for the box in either case -
    // upstream's own note, and the reason two loops below index the same way.
    let flip_row = src_unjittered_pos.y > src_output_pos.y;
    let flip_col = src_unjittered_pos.x > src_output_pos.x;
    let offset_tl_f = vec2<f32>(offset_tl);

    let is_initial_sample = params.accumulation == 0.0;

    var samples: array<vec3<f32>, 9>;
    var sample_index = 0;
    for (var row = 0; row < 3; row++) {
        for (var col = 0; col < 3; col++) {
            let sample_col_row = vec2<i32>(
                select(col, 3 - col, flip_col),
                select(row, 3 - row, flip_row),
            );
            let src_sample_pos = src_input_pos + offset_tl + sample_col_row;
            let sample_coord = clamp_load(src_sample_pos, vec2<i32>(0, 0), render_size());
            samples[sample_index] = load_prepared_color(sample_coord);
            sample_index += 1;
        }
    }

    // Identify how much of each upsampled color to be used for this frame.
    let kernel_bias_max = compute_max_kernel_weight();
    let kernel_bias_min = max(1.0, (1.0 + kernel_bias_max) * 0.3);

    let kernel_bias_weight = min(
        1.0 - params.disocclusion * 0.5,
        min(1.0 - params.shading_change, saturate(data.history_weight * 5.0)),
    );

    let kernel_bias = mix(kernel_bias_min, kernel_bias_max, kernel_bias_weight);

    sample_index = 0;
    for (var row = 0; row < 3; row++) {
        for (var col = 0; col < 3; col++) {
            let sample_col_row = vec2<i32>(
                select(col, 3 - col, flip_col),
                select(row, 3 - row, flip_row),
            );
            let offset = offset_tl_f + vec2<f32>(sample_col_row);
            let src_sample_offset = base_sample_offset + offset;

            let src_sample_pos = src_input_pos + offset_tl + sample_col_row;
            let on_screen_factor = f32(is_on_screen(src_sample_pos, render_size()));

            if !is_initial_sample {
                let sample_weight =
                    on_screen_factor * get_upsample_lanczos_weight(src_sample_offset, kernel_bias);

                data.upsampled_color += samples[sample_index] * sample_weight;
                data.upsampled_weight += sample_weight;
            }

            // Update rectification box. **A different, much wider kernel than
            // the upsample's** - a plain Gaussian over the same nine taps,
            // whose job is to describe the neighbourhood's colour distribution
            // rather than to resolve it.
            {
                let rectification_curve_bias = -2.3;
                let src_sample_offset_sq = dot(src_sample_offset, src_sample_offset);
                let box_sample_weight =
                    exp(rectification_curve_bias * src_sample_offset_sq) * on_screen_factor;

                let initial_sample = (row == 0) && (col == 0);
                data.clipping_box = rectification_box_add_sample(
                    initial_sample,
                    data.clipping_box,
                    samples[sample_index],
                    box_sample_weight,
                );
            }
            sample_index += 1;
        }
    }

    data.clipping_box = rectification_box_compute_variance_box_data(data.clipping_box);

    data.upsampled_weight *= f32(data.upsampled_weight > FSR3UPSCALER_EPSILON);

    if data.upsampled_weight > FSR3UPSCALER_EPSILON {
        // Normalize for deringing (we need to compare colors).
        data.upsampled_color = data.upsampled_color / data.upsampled_weight;
        data.upsampled_weight *= F_AVERAGE_LANCZOS_WEIGHT_PER_FRAME;

        // `Deringing`: clamp into the neighbourhood's own range.
        data.upsampled_color = clamp(
            data.upsampled_color,
            data.clipping_box.aabb_min,
            data.clipping_box.aabb_max,
        );
    }

    // Initial samples using tonemapped upsampling.
    if is_initial_sample {
        data.upsampled_color = data.clipping_box.box_center;
        data.upsampled_weight = 1.0;
        data.history_weight = 0.0;
    }

    return data;
}

fn reproject_history_color(params: CommonParams, data_in: PassData) -> PassData {
    var data = data_in;
    let reprojected_history =
        history_sample(params.reprojected_hr_uv, previous_frame_upscale_size());

    data.history_color = reprojected_history.rgb;
    data.history_color *= delta_pre_exposure();
    data.history_color *= exposure();

    data.history_color = rgb_to_ycocg(data.history_color);

    data.lock = reprojected_history.w;

    return data;
}

// **The history is pulled towards this frame's neighbourhood, not clamped into
// it.** The clipping box is an ellipsoid around the neighbourhood's mean, sized
// by its standard deviation, and a history colour outside it is pushed back to
// its surface along the line to the centre. Then - and this is the part that
// makes it a *soft* rectification - the result is blended back towards the
// unrectified history by however much this pixel has earned: its lock, its luma
// stability, and how much history it has accumulated.
fn rectify_history(params: CommonParams, data_in: PassData) -> PassData {
    var data = data_in;

    let velocity_factor_4k = saturate(params.velocity_4k / 20.0);
    let distance_factor = saturate(0.75 - params.farthest_depth_in_meters / 20.0);
    let accumulation_factor = 1.0 - params.accumulation;
    let reactive_factor = pow(params.reactive_mask, 1.0 / 2.0);
    let shading_change_factor = params.shading_change;
    let box_scale_t = max(
        velocity_factor_4k,
        max(
            distance_factor,
            max(accumulation_factor, max(reactive_factor, shading_change_factor)),
        ),
    );

    // Three standard deviations where nothing is happening, one where anything
    // is: a settled pixel is allowed to keep a history far from this frame's
    // mean, a moving or freshly revealed one is not.
    let box_scale = mix(3.0, 1.0, box_scale_t);
    // **1.7 on luma, 1.0 on the two chroma axes**, which is why the box is
    // built in YCoCg at all: brightness is allowed to vary more than colour.
    let scaled_box_vec = data.clipping_box.box_vec * vec3<f32>(1.7, 1.0, 1.0) * box_scale;

    let clamped_scaled_box_vec = max(scaled_box_vec, vec3<f32>(1.193e-7));
    let transformed_history_color =
        (data.history_color - data.clipping_box.box_center) / clamped_scaled_box_vec;

    if length(transformed_history_color) > 1.0 {
        let clamped_history_color = normalize(transformed_history_color);
        let final_clamped_history_color =
            (clamped_history_color * scaled_box_vec) + data.clipping_box.box_center;

        // Scale history color using rectification info, also using accumulation
        // mask to avoid potential invalid color protection.
        let history_contribution = max(params.luma_instability_factor, data.lock_contribution_this_frame)
            * params.accumulation
            * (1.0 - params.disocclusion);
        data.history_color = mix(
            final_clamped_history_color,
            data.history_color,
            saturate(history_contribution),
        );
    }

    return data;
}

fn update_lock_status(params: CommonParams, data_in: PassData) -> PassData {
    var data = data_in;

    data.lock *= f32(!params.is_new_sample);

    let lifetime_decrease_factor = max(
        saturate(params.shading_change),
        max(params.reactive_mask, params.disocclusion),
    );
    data.lock = max(0.0, data.lock - lifetime_decrease_factor * F_LOCK_MAX);

    // Compute this frame lock contribution.
    data.lock_contribution_this_frame =
        saturate(saturate(data.lock - F_LOCK_THRESHOLD) * (F_LOCK_MAX - F_LOCK_THRESHOLD));

    // **`fShadingChange * 0` is upstream's, and it is not a typo to tidy.** The
    // term is deliberately disabled in place rather than deleted, which says
    // shading change was tried here and taken back out; deleting it would erase
    // that and make a future reader think it was never considered.
    let new_lock_intensity = load_rw_new_locks(params.px_hr_pos)
        * (1.0 - max(params.shading_change * 0.0, params.reactive_mask));
    data.lock = max(0.0, min(data.lock + new_lock_intensity, F_LOCK_MAX));

    // Preparing for next frame. **A lock lives for one jitter sequence**, which
    // is what ties its lifetime to the phase count rather than to a frame
    // count: at a lower render scale there are more phases, and a thin feature
    // needs protecting for longer.
    let lifetime_decrease = (0.1 / jitter_sequence_length()) * (1.0 - lifetime_decrease_factor);
    data.lock = max(0.0, data.lock - lifetime_decrease);

    // We expect similar motion for next frame; kill the lock if that location
    // is off screen, to avoid locks being clamped to the screen borders.
    let estimated_uv_next_frame = params.hr_uv - params.motion_vector;
    data.lock *= f32(is_uv_inside(estimated_uv_next_frame));

    return data;
}

fn compute_base_accumulation_weight(params: CommonParams, data_in: PassData) -> PassData {
    var data = data_in;

    var base_accumulation = params.accumulation;

    // Fast motion caps the history at 0.15 frames however deep it actually is:
    // a surface crossing the screen has reprojection error the accumulation
    // count knows nothing about.
    base_accumulation = min(
        base_accumulation,
        mix(
            base_accumulation,
            0.15,
            saturate(max(0.0, (params.velocity_4k * velocity_factor()) / 0.5)),
        ),
    );

    data.history_weight = base_accumulation;

    return data;
}

fn accumulate_colors(data_in: PassData) -> PassData {
    var data = data_in;

    // Avoid invalid values when accumulation and upsampled weight is 0.
    data.history_weight *= f32(data.history_weight > FSR3UPSCALER_FP16_MIN);
    data.history_weight = max(FSR3UPSCALER_EPSILON, data.history_weight + data.upsampled_weight);

    let alpha = saturate(data.upsampled_weight / data.history_weight);
    data.history_color = mix(data.history_color, data.upsampled_color, alpha);
    data.history_color = ycocg_to_rgb(data.history_color);

    return data;
}

fn init_pass_data(px_hr_pos: vec2<i32>) -> CommonParams {
    var params: CommonParams;

    params.px_hr_pos = px_hr_pos;
    let hr_uv = (vec2<f32>(px_hr_pos) + 0.5) / vec2<f32>(upscale_size());
    params.hr_uv = hr_uv;
    params.lr_uv_jittered = hr_uv + jitter() / vec2<f32>(render_size());
    params.lr_uv_hw_sampler = clamp_uv(params.lr_uv_jittered, render_size(), max_render_size());

    params.motion_vector = get_motion_vector(hr_uv);
    params.velocity_4k = get_4k_velocity(params.motion_vector);

    // `ComputeReprojectedUVs`.
    params.reprojected_hr_uv = params.hr_uv + params.motion_vector;
    params.is_existing_sample = is_uv_inside(params.reprojected_hr_uv);

    let luma_instability_uv_hw = clamp_uv(hr_uv, render_size(), max_render_size());
    params.luma_instability_factor = sample_luma_instability(luma_instability_uv_hw);

    let farthest_depth_uv = clamp_uv(
        params.lr_uv_jittered,
        render_size() / 2,
        get_farthest_depth_mip1_resource_dimensions(),
    );
    params.farthest_depth_in_meters = sample_farthest_depth_mip1(farthest_depth_uv);
    params.is_new_sample = !params.is_existing_sample || frame_index() == 0.0;

    let dilated_reactive_masks = sample_dilated_reactive_masks(params.lr_uv_hw_sampler);
    params.reactive_mask = saturate(dilated_reactive_masks[REACTIVE]);
    params.disocclusion = saturate(dilated_reactive_masks[DISOCCLUSION]);
    params.shading_change = saturate(dilated_reactive_masks[SHADING_CHANGE]);
    params.accumulation = saturate(dilated_reactive_masks[ACCUMULAION]);
    params.accumulation *= f32(round(params.accumulation * 100.0) > 1.0);

    return params;
}

fn accumulate(px_hr_pos: vec2<i32>) {
    let params = init_pass_data(px_hr_pos);

    var data: PassData;
    data.upsampled_color = vec3<f32>(0.0);
    data.history_color = vec3<f32>(0.0);
    data.history_weight = 1.0;
    data.upsampled_weight = 0.0;
    data.lock = 0.0;
    data.lock_contribution_this_frame = 0.0;

    if params.is_existing_sample && !params.is_new_sample {
        data = reproject_history_color(params, data);
    }

    data = update_lock_status(params, data);
    data = compute_base_accumulation_weight(params, data);
    data = compute_upsampled_color_and_weight(params, data);
    data = rectify_history(params, data);
    data = accumulate_colors(data);

    data.history_color /= exposure();
    data.history_color = max(data.history_color, vec3<f32>(0.0));

    // The lock rides in the alpha channel, which is what makes the history one
    // texture rather than two.
    store_internal_color_and_weight(px_hr_pos, vec4<f32>(data.history_color, data.lock));

    // Upstream ends with `StoreNewLocks(iPxHrPos, 0)`, consuming the lock as it
    // reads it. This port clears `new_locks` at the start of the frame instead
    // - the same result without a read-write storage texture. See
    // `post::fsr3::resources::CLEARABLE`.
}

@compute @workgroup_size(8, 8, 1)
fn cs_accumulate(@builtin(global_invocation_id) id: vec3<u32>) {
    let px_pos = vec2<i32>(id.xy);
    if !is_on_screen(px_pos, upscale_size()) {
        return;
    }
    accumulate(px_pos);
}
