// The original's bloom, from `docs/ghidra/functions/psp-pulse-usa/bloom.md`.
//
// Three fragment entry points, one per recovered pass. Every constant here is
// read out of `BOOT.BIN`; none is fitted to a screenshot.
//
// **The bright pass is not a luminance threshold.** `Bloom_BuildBrightPassList`
// blends `GU_ADD, GU_SRC_ALPHA, GU_FIX 0`, i.e. the destination contributes
// nothing and the value written is `rgb * a`. The mask is the scene target's
// own alpha channel, which only surfaces that opt in ever write - see
// `crate::exhaust` for the first writer this project reproduces.

struct Constants {
    // One texel of the texture being sampled.
    texel: vec2<f32>,
    // (1, 0) for the horizontal blur, (0, 1) for the vertical one.
    direction: vec2<f32>,
    // The composite's `g_bloom_composite_strength`, 175/255.
    strength: f32,
    // Three scalars rather than a `vec3<f32>`: a trailing vec3 aligns to 16 in
    // WGSL's uniform address space and would push the block to 48 bytes, where
    // the Rust `Constants` is 32.
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    // Which sub-rectangle of `source_tex` was drawn, as a UV scale and a clamp
    // half a texel inside its edge. **Only `fs_bright` reads these** - it is
    // the one pass that samples the scene target, which since ADR-0037 may be
    // larger than what was drawn into it. The blurs and the composite read the
    // fixed 240x136 scratch buffers, which are always whole, and are bound a
    // block that says so. Both are exactly 1.0 until a controller moves the
    // extent; see `post::sub_rectangle`.
    uv_scale: vec2<f32>,
    uv_max: vec2<f32>,
}

@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> constants: Constants;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// The same fullscreen triangle the other passes here use: no vertex buffer,
// and no seam down a quad's diagonal.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var out: VertexOutput;
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    out.uv = uv;
    out.position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return out;
}

// `g_bloom_blur_weights` (`DAT_08ab2348`), the eleven bytes verbatim. The
// original sends each as a vertex colour under `MODULATE`.
//
// **These are deliberately not normalised.** They sum to 472/255 = 1.85, so
// each axis brightens by that and the two together by 3.43x. That gain is the
// original's and is a large part of why its glow is as strong as it is;
// dividing it out here would be inventing a different effect.
const WEIGHTS = array<u32, 11>(20u, 30u, 40u, 50u, 64u, 64u, 64u, 50u, 40u, 30u, 20u);

// A sampled texel as the 8-bit integers the GE holds it in.
fn bytes(texel: vec3<f32>) -> vec3<u32> {
    return vec3<u32>(floor(texel * 255.0 + 0.5));
}

// **Every product below is truncated to a whole byte, as the original's is.**
// The GE multiplies and adds in 8-bit integers and each of the blur's eleven
// taps is its own additive draw, so a tap's contribution is `floor(v * w /
// 255)` before it joins the sum. A float sum keeps what truncation destroys:
// the opaque mask stamp of `4` brightens a texel by at most 3, a tap of weight
// 64 turns 3 into `floor(0.75) = 0`, and the whole background term of the
// bloom is exactly zero in the original while a float chain carries it at
// 3.43 x 0.686 of the stamp. Measured against PPSSPP's software renderer on
// 2026-10-02 (`docs/ghidra/functions/psp-pulse-usa/bloom.md`, "Is ours
// stronger"): the bloom's mean-luma contribution outside the craft was `0.33`
// in the original, `4.52` here as floats and `0.37` truncated.
//
// Truncation is what that measurement supports. That every stage truncates
// rather than rounds is chosen, not measured, beyond the blur taps: they
// are the only stage whose rounding the background term depends on.

// Pass 0: `scratch = framebuffer.rgb * framebuffer.a`.
@fragment
fn fs_bright(in: VertexOutput) -> @location(0) vec4<f32> {
    // The downsample, and the one place the drawn rectangle enters: mapping
    // the extent onto the whole of the scratch buffer is what keeps the blur
    // radius a constant fraction of the picture at every render scale.
    let at = min(in.uv * constants.uv_scale, constants.uv_max);
    let texel = textureSample(source_tex, source_sampler, at);
    let alpha = u32(floor(texel.a * 255.0 + 0.5));
    return vec4<f32>(vec3<f32>(bytes(texel.rgb) * alpha / 255u) / 255.0, 1.0);
}

// Passes 1 and 2: eleven taps at -5..+5 texels along `direction`.
//
// The original emits eleven separate additive draws, each offset by one step
// and tinted by its weight, because the GE has no loop; summing the same
// eleven taps in one pass is the same arithmetic provided each tap is
// truncated before it is added, as it is there.
@fragment
fn fs_blur(in: VertexOutput) -> @location(0) vec4<f32> {
    let step = constants.texel * constants.direction;
    var sum = vec3<u32>(0u);
    for (var i = 0; i < 11; i = i + 1) {
        let offset = f32(i - 5) * step;
        let v = bytes(textureSample(source_tex, source_sampler, in.uv + offset).rgb);
        sum = min(sum + v * WEIGHTS[i] / 255u, vec3<u32>(255u));
    }
    return vec4<f32>(vec3<f32>(sum) / 255.0, 1.0);
}

// Pass 3: the composite's own colour. The additive `framebuffer +=` half is
// the pipeline's blend state, and the alpha channel is masked out of the write
// the way `Gu_PixelMask(0xff000000)` masks it in the original.
@fragment
fn fs_composite(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(source_tex, source_sampler, in.uv);
    return vec4<f32>(texel.rgb * constants.strength, 1.0);
}
