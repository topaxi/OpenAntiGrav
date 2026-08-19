// Wipeout HD's FunkLayerBloom passes, read out of the EBOOT's own fragment
// microcode and the PPU function that feeds it (FUN_003b4690) -
// docs/ghidra/functions/ps3-hdfury-eu/renderer.md, "The bloom chain, read
// pass by pass". The patched parameters arrive from the circuit's own
// .envsettings "HDR and Bloom" block; the inline constants are the
// executable's verbatim.

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

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// The same fullscreen triangle every post pass in this crate uses.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var out: VertexOutput;
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    out.uv = uv;
    out.position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return out;
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
    let frame = textureSample(source_tex, source_sampler, in.uv);
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
    return textureSample(source_tex, source_sampler, in.uv);
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
    let mean = textureSample(source_tex, source_sampler, vec2<f32>(0.5, 0.5)).rgb;
    let previous = textureSample(state_tex, source_sampler, vec2<f32>(0.5, 0.5)).x;
    let luminance = dot(mean, vec3<f32>(0.3, 0.59, 0.11));
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
        sum = sum + textureSample(source_tex, source_sampler, in.uv + offset).rgb * BLUR_WEIGHTS[i];
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
// The `pow` at the end is not the original's - its resolve ends on the
// ADD_SAT - and remains ADR-0026's display-encode stand-in.
@fragment
fn fs_encode(in: VertexOutput) -> @location(0) vec4<f32> {
    let scene = textureSample(source_tex, source_sampler, in.uv).rgb;
    let bloom = textureSample(bloom_tex, source_sampler, in.uv).rgb;
    let adapted = textureSample(state_tex, source_sampler, vec2<f32>(0.5, 0.5)).x;
    let scale = constants.tone_maximum_brightness
        - min(
            adapted * constants.tone_adaption_boost,
            constants.tone_darkening_clamp,
        );
    let resolved = clamp(scene * scale + bloom, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(pow(resolved, vec3<f32>(1.0 / 2.2)), 1.0);
}
