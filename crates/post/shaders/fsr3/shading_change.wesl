// AMD FidelityFX Super Resolution 3.1 - the shading-change pass, ported to
// WGSL. Transliterated from `ffx_fsr3upscaler_shading_change.h`, AMD FidelityFX
// SDK v1.1.4, MIT - see `common.wesl`'s header and
// `licences/AMD-FidelityFX-MIT.txt`.
//
// Reads the pyramid the previous pass built and collapses three of its levels
// into one half-resolution answer per texel: *how much* did this
// neighbourhood's shading change. Three levels rather than one because a
// re-light is a low-frequency event and a moving edge is a high-frequency one,
// and taking the maximum across scales catches both without letting either
// dominate.
//
// **The two channels are multiplied, not added**, and that is the whole idea:
// `x` is the mean signed difference over the level's footprint and `y` is the
// mean of its signs. Where a footprint's differences agree, `y` is near +/-1
// and the product is the full difference; where they cancel - noise, or an
// edge that moved - `y` collapses towards zero and takes the product with it.
// A shading change is a *coherent* difference, and that is how the pass says so.
//
// This is also the first pass here that *samples* rather than loads: it reads
// the pyramid through a linear clamp sampler at an explicit mip level, so the
// half-resolution grid it walks is interpolated across each level's coarser
// one rather than snapped to it.
import package::fsr3::common::{
    I_SHADING_CHANGE_MIP_START,
    clamp_uv,
    is_on_screen,
    jitter,
    linear_clamp,
    render_size,
    shading_change_render_size,
};

@group(1) @binding(0) var r_spd_mips: texture_2d<f32>;
@group(1) @binding(1) var rw_shading_change: texture_storage_2d<rgba8unorm, write>;

fn get_spd_mip_dimensions(mip_level: u32) -> vec2<i32> {
    return vec2<i32>(textureDimensions(r_spd_mips, mip_level));
}

fn sample_spd_mip_level(uv: vec2<f32>, mip_level: i32) -> vec2<f32> {
    return textureSampleLevel(r_spd_mips, linear_clamp, uv, f32(mip_level)).xy;
}

fn store_shading_change(px_pos: vec2<i32>, shading_change: f32) {
    textureStore(rw_shading_change, px_pos, vec4<f32>(shading_change, 0.0, 0.0, 0.0));
}

// How many levels of the pyramid contribute. Upstream's `s_MipLevelsToUse`,
// declared in the pass rather than in the common header.
const S_MIP_LEVELS_TO_USE: i32 = 3;

struct ShadingChangeLumaInfo {
    samples: array<f32, 3>,
}

fn compute_shading_change_luma(uv: vec2<f32>) -> ShadingChangeLumaInfo {
    var info: ShadingChangeLumaInfo;

    // **Clamped against the drawn rectangle, expressed in the resource's own
    // coordinates.** Since ADR-0037 the pyramid is allocated at the render
    // *ceiling* while only part of it is written, so a tap half a texel past
    // the drawn edge would read whatever the last frame left there.
    let mip_uv = clamp_uv(uv, shading_change_render_size(), get_spd_mip_dimensions(0u));

    for (var mip_level = I_SHADING_CHANGE_MIP_START; mip_level < S_MIP_LEVELS_TO_USE; mip_level++) {
        let sample = sample_spd_mip_level(mip_uv, mip_level);
        info.samples[mip_level] = abs(sample.x * sample.y);
    }

    return info;
}

fn shading_change(px_pos: vec2<i32>) {
    if is_on_screen(px_pos, shading_change_render_size()) {
        let uv = (vec2<f32>(px_pos) + 0.5) / vec2<f32>(shading_change_render_size());
        // **This frame's jitter over this frame's *render* size**, not over the
        // half-resolution grid the pass is walking: the offset is a fraction of
        // a render-resolution pixel wherever it is applied, and dividing it by
        // the half-size would double it.
        let uv_jittered = uv + jitter() / vec2<f32>(render_size());

        let info = compute_shading_change_luma(uv_jittered);

        // Upstream's own scale, and note that with `iShadingChangeMipStart` at
        // zero it is exactly 1.0 - the term exists so that starting the sweep
        // at a coarser level compensates for the levels skipped. Kept as
        // written rather than folded, because folding it would delete the only
        // place that says what the start index is for.
        let scale = 1.0 + f32(I_SHADING_CHANGE_MIP_START) / f32(S_MIP_LEVELS_TO_USE);
        var change = 0.0;
        for (var mip_level = I_SHADING_CHANGE_MIP_START; mip_level < S_MIP_LEVELS_TO_USE; mip_level++) {
            if info.samples[mip_level] > 0.0 {
                change = max(change, info.samples[mip_level]) * scale;
            }
        }

        store_shading_change(px_pos, saturate(change));
    }
}

@compute @workgroup_size(8, 8, 1)
fn cs_shading_change(@builtin(global_invocation_id) id: vec3<u32>) {
    shading_change(vec2<i32>(id.xy));
}
