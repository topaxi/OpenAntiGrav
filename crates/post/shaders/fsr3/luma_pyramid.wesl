// AMD FidelityFX Super Resolution 3.1 - the luma pyramid, ported to WGSL.
// Transliterated from `ffx_fsr3upscaler_luma_pyramid.h`, AMD FidelityFX SDK
// v1.1.4, MIT - see `common.wesl`'s header and
// `licences/AMD-FidelityFX-MIT.txt`.
//
// **This is one dispatch where upstream has a Single Pass Downsampler, and it
// produces one output where upstream produces three.** Both differences are
// argued below and in `docs/rendering/fsr3.md`; neither changes any arithmetic.
//
// # What upstream's pass does
//
// It builds a full mip pyramid over `(log(luma), luma, farthest depth in
// metres)` at render resolution, reducing with a plain four-tap average, and
// stores three things out of it:
//
// 1. at mip 1, the farthest depth - `farthest_depth_mip1`;
// 2. at mip 6, the pyramid itself into `spd_mips`;
// 3. at the 1x1 level, an auto-exposure estimate into `frame_info`.
//
// # Why only the first survives here
//
// **(2) is SPD's own plumbing, not a product.** `spd_mips` at mip 6 is where
// the Single Pass Downsampler hands off between the workgroup that finishes
// last and the rest of the reduction. The shading-change pyramid, which runs
// next, overwrites every level of `spd_mips` before anything reads it. With a
// conventional reduction there is no handoff, so there is nothing to store.
//
// **(3) has no reader in this port, because auto-exposure is off.** `Exposure()`
// only comes from `frame_info` when `FFX_FSR3UPSCALER_ENABLE_AUTO_EXPOSURE` is
// set; otherwise it is the app's, and an app that supplies none gets upstream's
// 1x1 default, which is a zero that `Exposure()` maps to one. See
// `common.wesl`'s own `exposure`. The pyramid's other 1x1 product,
// `SceneAverageLuma()`, is **dead in v1.1.4** - it is declared in
// `ffx_fsr3upscaler_common.h` and called from nowhere at all, which was checked
// across every pass header rather than assumed.
//
// So the whole pass reduces to (1): `farthest_depth_mip1` is the 2x2 box average
// of `farthest_depth`, which is `SpdReduce4` restricted to the one channel that
// leaves the pyramid. `luma` and `log(luma)` are computed by upstream only to
// feed (3), so they are not computed here - and `current_luma`, which the later
// passes do read, is written by `prepare_inputs` rather than by this pass.
//
// If an HDR path ever lands and wants auto-exposure, this is where the rest of
// the reduction goes: a mip chain over `(log(luma), luma)` down to 1x1 and the
// `ComputeAutoExposureFromLavg` smoothing at the end. It is left out rather than
// left half-built.
import package::fsr3::common::{
    clamp_load,
    is_on_screen,
    render_size,
};

@group(1) @binding(0) var r_farthest_depth: texture_2d<f32>;
@group(1) @binding(1) var rw_farthest_depth_mip1: texture_storage_2d<rgba16float, write>;

fn load_farthest_depth(px_pos: vec2<i32>) -> f32 {
    return textureLoad(r_farthest_depth, px_pos, 0).x;
}

fn store_farthest_depth_mip1(px_pos: vec2<i32>, depth: f32) {
    textureStore(rw_farthest_depth_mip1, px_pos, vec4<f32>(depth, 0.0, 0.0, 0.0));
}

// `SpdReduce4`, on the one channel that leaves the pyramid.
fn spd_reduce4(v0: f32, v1: f32, v2: f32, v3: f32) -> f32 {
    return (v0 + v1 + v2 + v3) * 0.25;
}

// **`ClampLoad`, not a bounds test**, and it is upstream's: a reduction whose
// source has an odd width would otherwise average a texel of whatever the clear
// left behind into the last column. Clamping repeats the edge texel instead,
// which is what a box filter over a half-covered pair should do.
@compute @workgroup_size(8, 8, 1)
fn cs_luma_pyramid(@builtin(global_invocation_id) id: vec3<u32>) {
    let px_pos = vec2<i32>(id.xy);
    let half_size = vec2<i32>(vec2<f32>(render_size()) * 0.5);
    if !is_on_screen(px_pos, half_size) {
        return;
    }

    let base = px_pos * 2;
    let size = render_size();
    let v0 = load_farthest_depth(clamp_load(base, vec2<i32>(0, 0), size));
    let v1 = load_farthest_depth(clamp_load(base, vec2<i32>(1, 0), size));
    let v2 = load_farthest_depth(clamp_load(base, vec2<i32>(0, 1), size));
    let v3 = load_farthest_depth(clamp_load(base, vec2<i32>(1, 1), size));

    store_farthest_depth_mip1(px_pos, spd_reduce4(v0, v1, v2, v3));
}
