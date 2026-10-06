// AMD FidelityFX Super Resolution 3.1 - the prepare-inputs pass, ported to
// WGSL. Transliterated from `ffx_fsr3upscaler_prepare_inputs.h` and the
// callbacks it uses in `ffx_fsr3upscaler_callbacks_hlsl.h`, AMD FidelityFX SDK
// v1.1.4, MIT - see `common.wesl`'s header and
// `licences/AMD-FidelityFX-MIT.txt`.
//
// The first pass of the chain, at render resolution. It reads the three things
// the renderer hands over - colour, depth, motion vectors - and writes the four
// derived buffers every later pass reads instead of re-deriving them: the
// nearest depth of a 3x3 neighbourhood, the motion vector belonging to that
// nearest sample, the farthest depth of the same neighbourhood in metres, and
// the frame's luma. It also scatters this frame's depth into the *previous*
// frame's grid, which is what lets the reproject step tell a disocclusion from
// a surface that merely moved.
//
// Two upstream options are compiled in rather than branched on, because this
// renderer has only one answer to each:
//
// - `FFX_FSR3UPSCALER_OPTION_INVERTED_DEPTH` is off. The projection is
//   `glam`'s right-handed non-reversed one, so nearer is *smaller* and the
//   scatter resolves collisions with a minimum.
// - `FFX_FSR3UPSCALER_OPTION_LOW_RESOLUTION_MOTION_VECTORS` is on. The
//   velocity attachment is written by the scene's own draws at render
//   resolution (ADR-0030), so a low-resolution position indexes it directly
//   and `ComputeHrPosFromLrPos` is not involved.
import package::fsr3::inputs::{r_input_motion_vectors, r_input_depth};
import package::fsr3::common::{
    FSR3UPSCALER_FP16_MAX,
    F_RECONSTRUCTED_DEPTH_BILINEAR_WEIGHT_THRESHOLD,
    constants,
    get_4k_velocity,
    get_bilinear_sampling_data,
    get_view_space_depth_in_meters,
    is_on_screen,
    reconstructed_depth_mv_px_threshold,
    render_size,
    rgb_to_luma,
};

// **Bindings 0 and 1 are declared by a prelude, not here.** The scene's
// velocity and depth attachments are multisampled whenever `[graphics]
// anti_aliasing` is at one of its MSAA levels, and a multisampled binding is a
// different WGSL type - so `post::fsr3` prepends one of two declarations and
// builds this file twice. Nothing else in the pass changes: `textureLoad(t, p,
// 0)` is spelled identically either way, the `0` being a mip level on one and a
// **sample index** on the other.
//
// Reading sample 0 rather than resolving is the same decision
// `post::motion_blur` made and for the same reason: resolving would average
// velocity across a silhouette edge, which is where the two surfaces have the
// least in common.
@group(1) @binding(2) var r_input_color_jittered: texture_2d<f32>;
@group(1) @binding(3) var rw_dilated_motion_vectors: texture_storage_2d<rgba16float, write>;
@group(1) @binding(4) var rw_dilated_depth: texture_storage_2d<r32float, write>;
// **A buffer, where upstream has an `R32_UINT` UAV.** `StoreReconstructedDepth`
// scatters with an `InterlockedMin`, and a texture atomic needs
// `wgpu::Features::TEXTURE_ATOMIC` while a buffer atomic is WebGPU core. The
// indexing is `y * render_size.x + x`, so the buffer is exactly as large as the
// texture would have been. See `post::fsr3::resources`.
@group(1) @binding(5) var<storage, read_write> rw_reconstructed_previous_nearest_depth: array<atomic<u32>>;
@group(1) @binding(6) var rw_farthest_depth: texture_storage_2d<r32float, write>;
@group(1) @binding(7) var rw_current_luma: texture_storage_2d<rgba16float, write>;

fn load_input_depth(px_pos: vec2<i32>) -> f32 {
    return textureLoad(r_input_depth, px_pos, 0).x;
}

fn load_input_color(px_pos: vec2<i32>) -> vec3<f32> {
    return textureLoad(r_input_color_jittered, px_pos, 0).rgb;
}

// Upstream multiplies by `MotionVectorScale()` to convert the engine's units
// into UV, and subtracts `MotionVectorJitterCancellation()` when the engine's
// vectors carry the jitter. Both are constants here and both are handled on the
// CPU side: the scale is a plain `-1` because this renderer already stores UV
// deltas, in the opposite direction, and the cancellation is zero because
// ADR-0039's ordering cancels the jitter before the buffer is written. Kept as
// the same two operations so the shader still reads like upstream's.
fn load_input_motion_vector(px_pos: vec2<i32>) -> vec2<f32> {
    let src = textureLoad(r_input_motion_vectors, px_pos, 0).xy;
    var uv_motion_vector = src * constants.motion_vector_scale;
    uv_motion_vector -= constants.motion_vector_jitter_cancellation;
    return uv_motion_vector;
}

fn store_dilated_motion_vector(px_pos: vec2<i32>, motion_vector: vec2<f32>) {
    textureStore(
        rw_dilated_motion_vectors,
        px_pos,
        vec4<f32>(motion_vector, 0.0, 0.0),
    );
}

fn store_dilated_depth(px_pos: vec2<i32>, depth: f32) {
    textureStore(rw_dilated_depth, px_pos, vec4<f32>(depth, 0.0, 0.0, 0.0));
}

fn store_farthest_depth(px_pos: vec2<i32>, depth: f32) {
    textureStore(rw_farthest_depth, px_pos, vec4<f32>(depth, 0.0, 0.0, 0.0));
}

fn store_current_luma(px_pos: vec2<i32>, luma: f32) {
    textureStore(rw_current_luma, px_pos, vec4<f32>(luma, 0.0, 0.0, 0.0));
}

// `InterlockedMin` on the depth's bit pattern. That works because every depth
// here is in `0..1` and non-negative IEEE floats compare the same way as their
// bit patterns do - upstream's trick, not one invented here.
fn store_reconstructed_depth(px_sample: vec2<i32>, depth: f32) {
    let index = u32(px_sample.y) * u32(render_size().x) + u32(px_sample.x);
    atomicMin(&rw_reconstructed_previous_nearest_depth[index], bitcast<u32>(depth));
}

fn reconstruct_prev_depth(px_pos: vec2<i32>, depth: f32, motion_vector_in: vec2<f32>) {
    var motion_vector = motion_vector_in;
    let nearest_depth_in_meters = min(get_view_space_depth_in_meters(depth), FSR3UPSCALER_FP16_MAX);
    let reconstructed_depth_mv_threshold = reconstructed_depth_mv_px_threshold(nearest_depth_in_meters);

    // Discard small mvs.
    motion_vector *= f32(get_4k_velocity(motion_vector) > reconstructed_depth_mv_threshold);

    let uv = (vec2<f32>(px_pos) + 0.5) / vec2<f32>(render_size());
    let reprojected_uv = uv + motion_vector;
    let bilinear_info = get_bilinear_sampling_data(reprojected_uv, render_size());

    // Project current depth into previous frame locations.
    // Push to all pixels having some contribution if reprojection is using
    // bilinear logic.
    for (var sample_index = 0; sample_index < 4; sample_index++) {
        let offset = bilinear_info.offsets[sample_index];
        let weight = bilinear_info.weights[sample_index];

        if weight > F_RECONSTRUCTED_DEPTH_BILINEAR_WEIGHT_THRESHOLD {
            let store_pos = bilinear_info.base_pos + offset;
            if is_on_screen(store_pos, render_size()) {
                store_reconstructed_depth(store_pos, depth);
            }
        }
    }
}

struct DepthExtents {
    nearest: f32,
    nearest_coord: vec2<i32>,
    farthest: f32,
}

// The nearest and farthest depth of the 3x3 neighbourhood, and where the
// nearest one was. Note that `farthest` is not the neighbourhood's maximum: it
// is only updated when a *nearer* sample is found, so it tracks the farthest
// depth seen before the eventual nearest. That is upstream's, and it is left
// as it is rather than "corrected" to a plain maximum.
fn find_depth_extents(px_pos: vec2<i32>) -> DepthExtents {
    var extents: DepthExtents;
    let sample_offsets = array<vec2<i32>, 9>(
        vec2<i32>(0, 0),
        vec2<i32>(1, 0),
        vec2<i32>(0, 1),
        vec2<i32>(0, -1),
        vec2<i32>(-1, 0),
        vec2<i32>(-1, 1),
        vec2<i32>(1, 1),
        vec2<i32>(-1, -1),
        vec2<i32>(1, -1),
    );

    // Pull out the depth loads to allow SC to batch them.
    var depth: array<f32, 9>;
    for (var sample_index = 0; sample_index < 9; sample_index++) {
        let pos = px_pos + sample_offsets[sample_index];
        depth[sample_index] = load_input_depth(pos);
    }

    // Find closest depth.
    extents.nearest_coord = px_pos;
    extents.nearest = depth[0];
    extents.farthest = depth[0];
    for (var sample_index = 1; sample_index < 9; sample_index++) {
        let pos = px_pos + sample_offsets[sample_index];
        if is_on_screen(pos, render_size()) {
            let nd_depth = depth[sample_index];
            if nd_depth < extents.nearest {
                extents.farthest = max(extents.farthest, nd_depth);
                extents.nearest_coord = pos;
                extents.nearest = nd_depth;
            }
        }
    }

    return extents;
}

fn dilate_motion_vector(px_pos: vec2<i32>, depth_extents: DepthExtents) -> vec2<f32> {
    let motion_vector_pos = depth_extents.nearest_coord;
    return load_input_motion_vector(motion_vector_pos);
}

fn get_current_frame_luma(px_pos: vec2<i32>) -> f32 {
    // We assume linear data. if non-linear input (sRGB, ...),
    // then we should convert to linear first and back to sRGB on output.
    let rgb = max(vec3<f32>(0.0), load_input_color(px_pos));
    return rgb_to_luma(rgb);
}

fn prepare_inputs(px_pos: vec2<i32>) {
    let depth_extents = find_depth_extents(px_pos);
    let dilated_motion_vector = dilate_motion_vector(px_pos, depth_extents);

    reconstruct_prev_depth(px_pos, depth_extents.nearest, dilated_motion_vector);

    store_dilated_motion_vector(px_pos, dilated_motion_vector);
    store_dilated_depth(px_pos, depth_extents.nearest);

    let farthest_depth_in_meters = min(
        get_view_space_depth_in_meters(depth_extents.farthest),
        FSR3UPSCALER_FP16_MAX,
    );
    store_farthest_depth(px_pos, farthest_depth_in_meters);

    let luma = get_current_frame_luma(px_pos);
    store_current_luma(px_pos, luma);
}

// **The bounds test is ours, and upstream does not need it.** A DirectX
// dispatch that overruns the target relies on the UAV write being discarded
// out of bounds; WGSL's `textureStore` past the edge is undefined rather than
// discarded, and the scatter above would index the buffer out of range. The
// dispatch is rounded up to whole 8x8 groups, so the last row and column of
// groups really do contain threads past the frame.
@compute @workgroup_size(8, 8, 1)
fn cs_prepare_inputs(@builtin(global_invocation_id) id: vec3<u32>) {
    let px_pos = vec2<i32>(id.xy);
    if !is_on_screen(px_pos, render_size()) {
        return;
    }
    prepare_inputs(px_pos);
}
