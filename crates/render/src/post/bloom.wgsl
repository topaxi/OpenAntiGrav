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

// `g_bloom_blur_weights` (`DAT_08ab2348`), the eleven bytes verbatim, over
// 255 because the original sends each as a vertex colour under `MODULATE`.
//
// **These are deliberately not normalised.** They sum to 472/255 = 1.85, so
// each axis brightens by that and the two together by 3.43x. That gain is the
// original's and is a large part of why its glow is as strong as it is;
// dividing it out here would be inventing a different effect.
const WEIGHTS = array<f32, 11>(
    20.0 / 255.0, 30.0 / 255.0, 40.0 / 255.0, 50.0 / 255.0, 64.0 / 255.0,
    64.0 / 255.0,
    64.0 / 255.0, 50.0 / 255.0, 40.0 / 255.0, 30.0 / 255.0, 20.0 / 255.0,
);

// Pass 0: `scratch = framebuffer.rgb * framebuffer.a`.
@fragment
fn fs_bright(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(source_tex, source_sampler, in.uv);
    return vec4<f32>(texel.rgb * texel.a, 1.0);
}

// Passes 1 and 2: eleven taps at -5..+5 texels along `direction`.
//
// The original emits eleven separate additive draws, each offset by one step
// and tinted by its weight, because the GE has no loop; summing the same
// eleven taps in one pass is the same arithmetic and the same result.
@fragment
fn fs_blur(in: VertexOutput) -> @location(0) vec4<f32> {
    let step = constants.texel * constants.direction;
    var sum = vec3<f32>(0.0);
    for (var i = 0; i < 11; i = i + 1) {
        let offset = f32(i - 5) * step;
        sum = sum + textureSample(source_tex, source_sampler, in.uv + offset).rgb * WEIGHTS[i];
    }
    return vec4<f32>(sum, 1.0);
}

// Pass 3: the composite's own colour. The additive `framebuffer +=` half is
// the pipeline's blend state, and the alpha channel is masked out of the write
// the way `Gu_PixelMask(0xff000000)` masks it in the original.
@fragment
fn fs_composite(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(source_tex, source_sampler, in.uv);
    return vec4<f32>(texel.rgb * constants.strength, 1.0);
}
