// Wipeout HD's FunkLayerBloom passes, read out of the EBOOT's own fragment
// microcode and the PPU function that feeds it (FUN_003b4690) -
// docs/ghidra/functions/ps3-hdfury-eu/renderer.md, "The bloom chain, read
// pass by pass". The patched parameters arrive from the circuit's own
// .envsettings "HDR and Bloom" block; the inline constants are the
// executable's verbatim.

import oag_shaders::fullscreen::{VertexOutput, fullscreen_triangle};

struct Constants {
    // One blur step: the authored tap spacing already divided by the buffer
    // size - the executable's own arithmetic, whose hard-coded divisors
    // 1/480 and 1/270 are exactly its quarter-res buffer of a 1080p frame.
    // Zero for the passes that do not step.
    step: vec2<f32>,
    // "Bloom from alpha contribution": the glow-mask term's weight.
    alpha_contribution: f32,
    // "Bloom from frame contribution": the luminance term's weight.
    frame_contribution: f32,
    // "Bloom from frame exponent": the luminance term's power.
    frame_exponent: f32,
    // "Bloom adaption rate": the per-frame lerp rate of the adapted
    // average luminance.
    adaption_rate: f32,
    // "Bloom adaption boost": scales the adapted luminance in the gate fade.
    adaption_boost: f32,
    // The "Tone" family, consumed by the resolve's exposure - see fs_encode.
    tone_adaption_boost: f32,
    tone_darkening_clamp: f32,
    tone_maximum_brightness: f32,
    _pad0: f32,
    _pad1: f32,
    // Which sub-rectangle of every source in this chain was drawn, as a UV
    // scale and a clamp half a texel inside its edge. One pair for the whole
    // ladder: each level's viewport is its own size times the same fraction,
    // so the mapping from a level's 0..1 onto its drawn rectangle is that
    // fraction everywhere. Both are exactly 1.0 until a resolution controller
    // moves the render extent - see `post::sub_rectangle`.
    uv_scale: vec2<f32>,
    uv_max: vec2<f32>,
}

@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> constants: Constants;
// Second input: the 1x1 adapted-luminance state for the gate, the adapt
// pass and the resolve. Bound to a dummy view on the passes that read only
// `source_tex`.
@group(0) @binding(3) var state_tex: texture_2d<f32>;
// Third input: the blurred bloom, for the resolve. Dummy elsewhere.
@group(0) @binding(4) var bloom_tex: texture_2d<f32>;

// A source coordinate inside the drawn rectangle.
//
// Every level of this chain draws into the same fraction of its own target, so
// one scale maps a level's 0..1 onto what the level before it actually wrote.
// The clamp keeps a bilinear tap off the texels past that rectangle, which
// hold the clear rather than picture - the ladder's own downsampling is what
// would otherwise pull that inward, level by level, as a darkening edge.
fn drawn(uv: vec2<f32>) -> vec2<f32> {
    return min(uv * constants.uv_scale, constants.uv_max);
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    return fullscreen_triangle(index);
}

// FunkLayerBloomGate_fp (0x92d580): the bright pass, over the quarter-res
// scene, exactly as the original runs it. Both terms and both engine-fed
// parameters are read:
//
//   gate = frame.rgb * frame.a * <alpha contribution>
//        + frame.rgb * pow(dot(frame.rgb, (0.9, 1.77, 0.33)), <exponent>)
//          * <frame contribution> * fade
//
// The microcode's `(1 - contribution.w * frame.a)` factor is dropped because
// FUN_003b4690 hard-codes contribution.w = 0, and its `+ additiveColour`
// because the engine writes {0,0,0,0} there outside an event flash (a grey
// whiteout driven by state this renderer does not model). `fade` is the
// engine's luminance adaptation:
//
//   fade = 1 - min(adapted * <Bloom adaption boost> * 0.25, 1)
//
// with 0.25 an inline constant of the executable and `adapted` the 1x1
// state fs_adapt below maintains.
@fragment
fn fs_gate(in: VertexOutput) -> @location(0) vec4<f32> {
    let frame = surface(textureSample(source_tex, source_sampler, drawn(in.uv)));
    let adapted = textureSample(state_tex, source_sampler, vec2<f32>(0.5, 0.5)).x;
    let fade = 1.0 - min(adapted * constants.adaption_boost * 0.25, 1.0);
    let luminance = max(dot(frame.rgb, vec3<f32>(0.9, 1.77, 0.33)), 0.0);
    let gated = frame.rgb * frame.a * constants.alpha_contribution
        + frame.rgb
            * pow(luminance, constants.frame_exponent)
            * (constants.frame_contribution * fade);
    return vec4<f32>(gated, 1.0);
}

// FunkLayerBloomDownsample_fp (0x92d780): a single tap - the filtering is the
// sampler's, exactly as the microcode leaves it. Serves every halving step,
// the luminance reduction (where a bilinear tap over an exact 2x halving is
// the 2x2 box mean), and the composite's colour half; the composite's
// additive "frame +=" is the pipeline's blend state, ONE/ONE as the
// engine's own glEnable-shaped state words select.
//
// **Alpha is carried through**, not stamped 1.0: it is the glow mask, the
// microcode's own alpha scale-and-bias runs at the engine's {scale 1,
// bias 0}, and a downsample that saturated it would hand the gate's
// alpha term the whole frame - measured as exactly the wall of white it
// sounds like.
@fragment
fn fs_copy(in: VertexOutput) -> @location(0) vec4<f32> {
    return surface(textureSample(source_tex, source_sampler, drawn(in.uv)));
}

// **What an 8-bit render target does to a value, done where this chain reads
// one.** Every buffer the original's chain touches is
// `CELL_GCM_SURFACE_A8R8G8B8`: the scene target created at `0x003e17a0` and
// all seven ladder buffers `FUN_003af980` allocates - `li r6, 8` at seven
// separate call sites, read off `EBOOT.elf` on 2026-08-20 together with the
// surface-format word builder (`FUN_005c9a50`) and the render-target factory
// (`FUN_005a6ce0`), whose `+0x24` field is confirmed as the raw RSX enum by
// its own `32`/`64` depth branch. Only one `F_W16Z16Y16X16` surface exists in
// the whole executable and it is neither the scene nor in the ladder.
//
// So **a value above 1.0 never reaches the original's gate**: the ROP clamps
// when the frame is written and again at the half-res downsample. The gate's
// `dot` weights being `(0.3, 0.59, 0.11) * 3` put its knee at luma ~1/3,
// which is an LDR bright pass and not the HDR one this module's header used
// to claim.
//
// This project's scene target stays [`SCENE_FORMAT`] (`Rgba16Float`) - 8 bits
// of *linear* light bands visibly in the darks, and the shading that writes
// it is ADR-0026's - so the hardware's clamp is applied here instead, at the
// three points the chain samples the scene. Measured on the Talon's Junction
// grid: clipped-white share 16.5 % -> 14.3 %, and the gate's peak argument
// falls from ~1.6e5 to 2.43.
fn surface(texel: vec4<f32>) -> vec4<f32> {
    return clamp(texel, vec4<f32>(0.0), vec4<f32>(1.0));
}

// The luminance reduction's halving step, in two forms.
//
// **Linear, like the original's ladder.** Every ladder draw in the original
// has the ROP's sRGB write on and its units decode on fetch (gamma nibble 7;
// renderer.md, "The exposure is unity ..."), so each halving averages light
// and stores the encoded byte. Ours halves the linear scene the same way.
//
// **Each step is an exact, coverage-weighted 2x2 mean.** A bilinear tap over
// an odd-sized level reads the middle of a texel pair and drops a row or
// column, and the last step of a 3x2 level read one column alone: the ladder
// came out ~30 % under the true mean of the frame it was given. A level here
// carries, in its alpha, the share of each texel's footprint that holds
// picture, so the half-empty last texel of an odd level weighs what it covers.
// `first` is the step that reads the scene's quarter-res copy, whose alpha is
// the glow mask and not a coverage: every texel there weighs 1.
fn reduce_sum(pixel: vec2<i32>, first: bool) -> vec4<f32> {
    let dims = vec2<f32>(textureDimensions(source_tex));
    let last = vec2<i32>(max(round(dims * constants.uv_scale), vec2<f32>(1.0))) - vec2<i32>(1);
    var rgb = vec3<f32>(0.0);
    var weight = 0.0;
    for (var k = 0; k < 4; k = k + 1) {
        let at = pixel * 2 + vec2<i32>(k & 1, k >> 1);
        if (at.x <= last.x && at.y <= last.y) {
            let texel = textureLoad(source_tex, at, 0);
            var share = texel.a;
            if (first) {
                share = 1.0;
            }
            rgb = rgb + texel.rgb * share;
            weight = weight + share;
        }
    }
    return vec4<f32>(rgb / max(weight, 1e-6), weight * 0.25);
}

@fragment
fn fs_reduce_first(in: VertexOutput) -> @location(0) vec4<f32> {
    return reduce_sum(vec2<i32>(in.position.xy), true);
}

@fragment
fn fs_reduce(in: VertexOutput) -> @location(0) vec4<f32> {
    return reduce_sum(vec2<i32>(in.position.xy), false);
}

// The adaptation update FUN_003b4690 runs on the CPU after reading back the
// fully-downsampled frame:
//
//   adapted += <Bloom adaption rate> * (dot(mean, (0.3, 0.59, 0.11)) - adapted)
//
// the weights being the executable's own (plain luma - the gate's 3x set is
// the shader's alone). Run on the GPU here, into a 1x1 ping-pong, which
// spares the readback and keeps the arithmetic identical.
// 1.0 on the chain's first frame only: the ping-pong state starts at zero,
// and a single captured frame would otherwise show the never-adapted
// picture. See the module header of `hd_bloom.rs`.
override rate_override: f32 = 0.0;

@fragment
fn fs_adapt(in: VertexOutput) -> @location(0) vec4<f32> {
    // The CPU half of FUN_003b4690's readback: every texel the ladder left
    // (it stops when either side reaches 1), each read as the encoded byte the
    // original's 8-bit surface holds, summed and divided by the count. No
    // decode anywhere between those bytes and the luma.
    let dims = vec2<f32>(textureDimensions(source_tex));
    let count = vec2<i32>(max(round(dims * constants.uv_scale), vec2<f32>(1.0)));
    var rgb = vec3<f32>(0.0);
    var weight = 0.0;
    for (var y = 0; y < count.y; y = y + 1) {
        for (var x = 0; x < count.x; x = x + 1) {
            let texel = textureLoad(source_tex, vec2<i32>(x, y), 0);
            rgb = rgb
                + pow(clamp(texel.rgb, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2))
                    * texel.a;
            weight = weight + texel.a;
        }
    }
    let mean = rgb / max(weight, 1e-6);
    let previous = textureSample(state_tex, source_sampler, vec2<f32>(0.5, 0.5)).x;
    // The PPU reads the reduced buffer's own bytes, and that buffer is written
    // through the ROP's sRGB encode, so the luminance it averages is of the
    // encoded value. Inferred, not read: the encode is measured
    // (`SET_SHADER_PACKER` is 1 on the ladder's draws), that the readback takes
    // raw bytes is the reading that makes the original's exposure scale 1.0 on
    // a dim frame, which it measures.
    let encoded = pow(clamp(mean, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2));
    let luminance = dot(encoded, vec3<f32>(0.3, 0.59, 0.11));
    let rate = max(constants.adaption_rate, rate_override);
    let adapted = previous + rate * (luminance - previous);
    return vec4<f32>(adapted, 0.0, 0.0, 1.0);
}

// FunkLayerBloomBlurVertical_fp (0x92d880) / Horizontal (0x92dc80): nine taps,
// weights 1, 0.8, 0.5, 0.2, 0.1 mirrored, divided by exactly 4.2 - the
// microcode ends on a MUL by 0.238095 = 1/4.2. The tap spacing is the
// authored "Bloom vertical/horizontal size" over the buffer size, baked
// into `step`.
const BLUR_WEIGHTS = array<f32, 9>(0.1, 0.2, 0.5, 0.8, 1.0, 0.8, 0.5, 0.2, 0.1);

@fragment
fn fs_blur(in: VertexOutput) -> @location(0) vec4<f32> {
    var sum = vec3<f32>(0.0);
    for (var i = 0; i < 9; i = i + 1) {
        let offset = f32(i - 4) * constants.step;
        // The clamp has to constrain the *sampled* coordinate, not just the
        // tap's centre: `drawn(in.uv) + offset` (the previous form) added the
        // tap after `drawn()`'s own `min(..., uv_max)`, so a tap could still
        // land past the drawn sub-rectangle into another level's clear -
        // inert at native, where `uv_scale`/`uv_max` are `(1, 1)` and the
        // sampler's own edge clamp already absorbs the difference, but wrong
        // once dynamic resolution or FSR shrinks the drawn rectangle inside
        // its buffer (renderer.md, "Two defects filed and deliberately not
        // fixed"). Folding the offset into the same `min` `drawn()` uses
        // fixes that without changing anything at native.
        let coord = min(in.uv * constants.uv_scale + offset, constants.uv_max);
        sum = sum + textureSample(source_tex, source_sampler, coord).rgb * BLUR_WEIGHTS[i];
    }
    return vec4<f32>(sum / 4.2, 1.0);
}

// The read resolve, `downsamplescaleaddfeedback_fp` (0x92a500) plus the PPU
// arithmetic that fills its `scale` parameter (FUN_003e3268/FUN_003df7c0,
// all four parameter names settled by crc32 preimage):
//
//   scale = <Tone maximum brightness>
//         - min(adapted * <Tone adaption boost>, <Tone darkening clamp>)
//   out   = saturate(scene * scale + bloom * scaleAdd + tint)
//
// with `scaleAdd` fed 1.0 (the constant at 0x8b7cec) and the bloom added
// AFTER the exposure scale - the microcode's own operand order. Two terms
// are inert at authored defaults and left out: the `scaleFeedback` mix
// toward a feedback buffer (its parameter is `Bloom feedback` + a runtime
// float; no circuit file authors the key, default 0) and the persistent
// `fullscreenTintColour` (an event tint this renderer does not model).
//
// The `pow` at the end is the ROP's sRGB encode on write (`SET_SHADER_PACKER`
// is 1 on the scene and ladder draws), approximated by one `1/2.2` curve; it is
// not an exposure stage. The original's resolve itself ends on the ADD_SAT.
@fragment
fn fs_encode(in: VertexOutput) -> @location(0) vec4<f32> {
    let scene = surface(textureSample(source_tex, source_sampler, drawn(in.uv))).rgb;
    let bloom = textureSample(bloom_tex, source_sampler, drawn(in.uv)).rgb;
    let adapted = textureSample(state_tex, source_sampler, vec2<f32>(0.5, 0.5)).x;
    let scale = constants.tone_maximum_brightness
        - min(
            adapted * constants.tone_adaption_boost,
            constants.tone_darkening_clamp,
        );
    let resolved = clamp(scene * scale + bloom, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(pow(resolved, vec3<f32>(1.0 / 2.2)), 1.0);
}
