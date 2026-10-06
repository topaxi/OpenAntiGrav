// Pulse PS2's bloom, from `docs/rendering/ps2-bloom.md`: the five GS passes of a
// race field, read off PCSX2 GS dumps. Every constant below is a register or a
// vertex colour of those dumps; the arithmetic is the GS's own 8-bit integer
// blend, which is why each product is truncated to a whole byte.

import oag_shaders::bytes::bytes;
import oag_shaders::fullscreen::{VertexOutput, fullscreen_triangle};

struct Constants {
    // The scene's drawn sub-rectangle as a UV scale and a clamp half a texel
    // inside its edge (`post::sub_rectangle`); only `fs_down` reads them.
    uv_scale: vec2<f32>,
    uv_max: vec2<f32>,
    // (1, 0) for the horizontal blur, (0, 1) for the vertical one.
    direction: vec2<f32>,
    // The composite's destination offset in UV, 5 px of the 512 x 512 frame.
    shift: vec2<f32>,
}

@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> constants: Constants;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    return fullscreen_triangle(index);
}

// The seven tap weights, over 128: the vertex colour of each strip of the two
// blur passes (`16, 32, 32, 64, 32, 32, 16`), under `MODULATE`.
const WEIGHTS = array<u32, 7>(16u, 32u, 32u, 64u, 32u, 32u, 16u);

// Pass 1: `ALPHA 0x88`, `Cs * As >> 7`, with `TFX` decal so the source colour
// is the frame's texel and the source alpha is the frame's alpha channel. The
// scene's alpha holds `1.0` where the PS2's holds `0x80`, so the alpha byte
// is `a * 128`.
@fragment
fn fs_down(in: VertexOutput) -> @location(0) vec4<f32> {
    let at = min(in.uv * constants.uv_scale, constants.uv_max);
    let texel = textureSample(source_tex, source_sampler, at);
    let alpha = u32(floor(min(texel.a, 1.0) * 128.0 + 0.5));
    return vec4<f32>(vec3<f32>((bytes(texel.rgb) * alpha) >> vec3<u32>(7u)) / 255.0, 1.0);
}

// Passes 2 and 3: seven additive draws offset -3..+3 texels along `direction`,
// each `texel * weight >> 7` before it joins the sum. **A tap that falls off
// the buffer contributes nothing**: the strips are drawn shifted over a clear
// buffer, so what lies past the edge is not clamped but absent.
@fragment
fn fs_blur(in: VertexOutput) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(source_tex));
    let here = vec2<i32>(in.position.xy);
    let step = vec2<i32>(constants.direction);
    var sum = vec3<u32>(0u);
    for (var i = 0; i < 7; i = i + 1) {
        let at = here + step * (i - 3);
        if any(at < vec2<i32>(0)) || any(at >= size) {
            continue;
        }
        let v = bytes(textureLoad(source_tex, at, 0).rgb);
        sum = min(sum + ((v * WEIGHTS[i]) >> vec3<u32>(7u)), vec3<u32>(255u));
    }
    return vec4<f32>(vec3<f32>(sum) / 255.0, 1.0);
}

// The composite quad's vertex colour and `FIX`, set from `ps2_bloom.rs`'s
// `COMPOSITE_COLOUR` and `COMPOSITE_FIX`.
override composite_colour: f32 = 127.0;
override composite_fix: f32 = 64.0;

// Pass 4: `ALPHA 0x4000000068`, `Cs * 0x40 >> 7 + Cd` with `Cs = Ct * 127 >> 7`
// (the quad's vertex colour is 127, `MODULATE`), the buffer stretched bilinearly
// and drawn five pixels up and left. Where that leaves the buffer short of the
// bottom and right edge, nothing is drawn. The additive blend and the saturation
// are the pipeline's; the alpha is masked out of the write (`FBMSK 0xff000000`).
@fragment
fn fs_composite(in: VertexOutput) -> @location(0) vec4<f32> {
    let at = in.uv + constants.shift;
    if any(at >= vec2<f32>(1.0)) {
        discard;
    }
    let ct = floor(textureSample(source_tex, source_sampler, at).rgb * 255.0);
    let cs = floor(ct * composite_colour / 128.0);
    return vec4<f32>(floor(cs * composite_fix / 128.0) / 255.0, 1.0);
}
