// AMD FidelityFX Super Resolution 3.1 - the RCAS pass, ported to WGSL.
// Transliterated from `ffx_fsr3upscaler_rcas.h` and the `FsrRcasF` it includes
// from `fsr1/ffx_fsr1.h`, AMD FidelityFX SDK v1.1.4, MIT - see `common.wesl`'s
// header and `licences/AMD-FidelityFX-MIT.txt`.
//
// **This is FSR 1's RCAS, and this project already had it** - see
// `post::fsr1`'s `fs_rcas`, ported from the same routine. FSR 3.1's pass is a
// thin wrapper that reads the accumulate pass's history instead of an EASU
// output and sets one define. The duplication here is deliberate rather than a
// missed refactor: the two live in different shading languages' worth of
// surrounding scaffolding (a fragment pass over a full-screen triangle against
// a compute dispatch over a storage texture), and folding them together would
// mean neither could be diffed against its own upstream file.
//
// # The one difference, and it is the whole reason this is not shared code
//
// `ffx_fsr3upscaler_rcas.h` sets **`FSR_RCAS_DENOISE 1`**, which `post::fsr1`
// does not. That gates a five-tap luma noise-detection term, `nz`, which scales
// the sharpening lobe down where the neighbourhood looks like noise rather than
// like an edge. Upstream's own comment on the define recommends applying film
// grain after RCAS instead of enabling it - but FSR 3.1 turns it on, because a
// temporally accumulated frame carries reconstruction noise that a
// single-frame sharpen never sees, and sharpening that is exactly the wrong
// thing to do.
//
// Both the exposure multiply on the way in and the divide on the way out are
// upstream's; with this port's constant exposure of one, they cancel. Kept so
// the shader still reads like `CurrFilter`.
import package::fsr3::common::{
    exposure,
    is_on_screen,
    rcas_sharpness,
    upscale_size,
};

@group(1) @binding(0) var r_rcas_input: texture_2d<f32>;
@group(1) @binding(1) var rw_upscaled_output: texture_storage_2d<rgba16float, write>;

fn load_rcas_input(px_pos: vec2<i32>) -> vec4<f32> {
    let limit = vec2<i32>(textureDimensions(r_rcas_input, 0)) - vec2<i32>(1);
    // Upstream leaves edge addressing to the resource's own clamp;
    // `textureLoad` has no address mode, so the clamp is explicit here - the
    // same note `fsr1.wesl` carries.
    return textureLoad(r_rcas_input, clamp(px_pos, vec2<i32>(0), limit), 0);
}

fn fsr_rcas_load_f(px_pos: vec2<i32>) -> vec3<f32> {
    return load_rcas_input(px_pos).rgb * exposure();
}

fn store_upscaled_output(px_pos: vec2<i32>, color: vec3<f32>) {
    textureStore(rw_upscaled_output, px_pos, vec4<f32>(color, 1.0));
}

// This is set at the limit of providing unnatural results for sharpening.
const FSR_RCAS_LIMIT: f32 = 0.25 - (1.0 / 16.0);

fn max3(x: f32, y: f32, z: f32) -> f32 {
    return max(x, max(y, z));
}

fn min3(x: f32, y: f32, z: f32) -> f32 {
    return min(x, min(y, z));
}

// `ffxApproximateReciprocalMedium`: the medium-precision reciprocal
// approximation upstream insists on here, because an exact divide produces
// visible tonality changes across the sharpen. The bit pattern is upstream's.
fn approximate_reciprocal_medium(value: f32) -> f32 {
    let b = bitcast<f32>(0x7ef19fffu - bitcast<u32>(value));
    return b * (-b * value + 2.0);
}

fn fsr_rcas_f(px_pos: vec2<i32>, sharpness: f32) -> vec3<f32> {
    //    b
    //  d e f
    //    h
    let b = fsr_rcas_load_f(px_pos + vec2<i32>(0, -1));
    let d = fsr_rcas_load_f(px_pos + vec2<i32>(-1, 0));
    let e = fsr_rcas_load_f(px_pos);
    let f = fsr_rcas_load_f(px_pos + vec2<i32>(1, 0));
    let h = fsr_rcas_load_f(px_pos + vec2<i32>(0, 1));

    // Luma times 2.
    let b_l = b.b * 0.5 + (b.r * 0.5 + b.g);
    let d_l = d.b * 0.5 + (d.r * 0.5 + d.g);
    let e_l = e.b * 0.5 + (e.r * 0.5 + e.g);
    let f_l = f.b * 0.5 + (f.r * 0.5 + f.g);
    let h_l = h.b * 0.5 + (h.r * 0.5 + h.g);

    // Noise detection. **`FSR_RCAS_DENOISE`, which FSR 3.1 sets and FSR 1 does
    // not.** How far the centre sits from its four neighbours' mean, normalised
    // by their range: one at a clean edge, towards zero where the ring is
    // noisy. It ends up in `0.5..1.0`, so at worst it halves the lobe.
    var nz = 0.25 * b_l + 0.25 * d_l + 0.25 * f_l + 0.25 * h_l - e_l;
    nz = saturate(
        abs(nz) * approximate_reciprocal_medium(
            max3(max3(b_l, d_l, e_l), f_l, h_l) - min3(min3(b_l, d_l, e_l), f_l, h_l),
        ),
    );
    nz = -0.5 * nz + 1.0;

    // Min and max of the ring.
    let mn4 = min(min(min(b, d), f), h);
    let mx4 = max(max(max(b, d), f), h);
    // Immediate constants for peak range.
    let peak_c = vec2<f32>(1.0, -1.0 * 4.0);
    // Limiters, these need to be high precision RCPs.
    let hit_min = min(mn4, e) / (4.0 * mx4);
    let hit_max = (vec3<f32>(peak_c.x) - max(mx4, e)) / (4.0 * mn4 + vec3<f32>(peak_c.y));
    let lobe_rgb = max(-hit_min, hit_max);
    var lobe = max(-FSR_RCAS_LIMIT, min(max3(lobe_rgb.r, lobe_rgb.g, lobe_rgb.b), 0.0)) * sharpness;

    // Apply noise removal.
    lobe *= nz;

    // Resolve, which needs the medium precision rcp approximation to avoid
    // visible tonality changes.
    let rcp_l = approximate_reciprocal_medium(4.0 * lobe + 1.0);
    return (lobe * b + lobe * d + lobe * h + lobe * f + e) * rcp_l;
}

// **One thread per output pixel, where upstream's does four.** `RCAS()` remaps
// the local index for a quad-like swizzle and filters four positions, which is
// a memory-locality optimisation for the shape of a wave rather than part of
// the filter. A plain grid produces the same pixels; if this ever shows up in a
// profile, upstream's `ffxRemapForQuad` is where to look.
@compute @workgroup_size(8, 8, 1)
fn cs_rcas(@builtin(global_invocation_id) id: vec3<u32>) {
    let px_pos = vec2<i32>(id.xy);
    if !is_on_screen(px_pos, upscale_size()) {
        return;
    }
    // `constants.jitter_phase_count` and the rest are the shared block; the
    // sharpness rides in its own field because upstream keeps a separate RCAS
    // constant buffer for it.
    let color = fsr_rcas_f(px_pos, rcas_sharpness()) / exposure();
    store_upscaled_output(px_pos, color);
}
