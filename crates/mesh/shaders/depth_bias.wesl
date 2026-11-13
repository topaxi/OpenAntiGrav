// The original's per-batch depth bias, in this renderer's depth.

import package::bindings::{uniforms};

// **The original adds `6 * bias` to a 16-bit depth, toward the camera**: a batch's
// `sceGuDepthRange(1001 + 6 * bias, 64946 + 6 * bias)` moves its whole range by `6 * bias`
// levels (`docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`, "The per-batch depth bias is file
// data"). Its projection is near 1.2, far 2000 over a range 63,945 levels wide, so its window
// depth is `C + 76,779 / w` (`w` the view distance; 76,779 = 63,945 / 2 * 2.4014, the
// projection's `proj[14]`), larger being nearer under the `GREATER` test. Adding `6 * bias`
// levels is therefore adding `6 * bias / 76,779` to `1 / w`: a shift in the **original's** depth
// units, which is a different distance at each range (about 14 units at 250 for a bias of 3,
// 0.1 units at 20).
//
// Ours is the same hyperbola, `d = alpha + beta / w`, with its own `alpha` and `beta` (any
// near and far, standard or reversed), so the same shift in `1 / w` is `beta * shift` in
// depth, and in clip space `beta * shift * w`. `alpha` and `beta` come out of the matrix's own
// depth and `w` rows, so the shift follows whatever projection is bound (the race's near 1.0,
// the asset viewer's 0.01). An orthographic matrix has no `w` to shift and is left alone.
const LEVELS_PER_BIAS: f32 = 6.0;
const LEVELS_PER_INVERSE_DISTANCE: f32 = 76779.0;

fn biased_clip(clip: vec4<f32>, bias: f32) -> vec4<f32> {
    if bias <= 0.0 {
        return clip;
    }
    let vp = uniforms.view_projection;
    let depth_row = vec4<f32>(vp[0][2], vp[1][2], vp[2][2], vp[3][2]);
    let w_row = vec4<f32>(vp[0][3], vp[1][3], vp[2][3], vp[3][3]);
    let w_axis = dot(w_row.xyz, w_row.xyz);
    if w_axis < 1e-6 {
        return clip;
    }
    let alpha = dot(depth_row.xyz, w_row.xyz) / w_axis;
    let beta = depth_row.w - alpha * w_row.w;
    let nearer = bias * LEVELS_PER_BIAS / LEVELS_PER_INVERSE_DISTANCE;
    return vec4<f32>(clip.xy, clip.z + beta * nearer * clip.w, clip.w);
}
