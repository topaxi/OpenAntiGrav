// Pure's sun lens flare: ten quads from one uniform, additive by the pipeline.
// The occlusion test (16 depth samples around the sun's pixel) runs in the vertex
// stage so the fraction needs no readback. See `lens_flare.rs`.

import oag_shaders::target::{linear_out, gamma_to_target, target_out};

struct Ring {
    // centre.xy, half.xy in the viewport's normalised device coordinates.
    centre_half: vec4<f32>,
    rgba: vec4<f32>,
    // The atlas cell in x.
    cell: vec4<u32>,
}

struct Frame {
    // The sun's pixel in the depth attachment (y down) in xy, the pixel scale in z.
    sun: vec4<f32>,
    rings: array<Ring, 10>,
}

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;
@group(0) @binding(3) var depth_tex: texture_2d<f32>;

// What the sky and an empty pixel read; the sky is drawn at viewport depth 1.0.
const FAR: f32 = 0.99999;

fn unoccluded() -> f32 {
    let size = vec2<i32>(textureDimensions(depth_tex));
    var open = 0.0;
    for (var j = 0; j < 4; j++) {
        for (var i = 0; i < 4; i++) {
            let offset = vec2<f32>(f32(i) * 2.0 - 3.0, f32(j) * 2.0 - 3.0) * frame.sun.z;
            let at = vec2<i32>(floor(frame.sun.xy + offset));
            if (all(at >= vec2<i32>(0)) && all(at < size)) {
                if (textureLoad(depth_tex, at, 0).x >= FAR) {
                    open += 1.0;
                }
            }
        }
    }
    return open / 16.0;
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) texcoord: vec2<f32>,
    @location(1) colour: vec4<f32>,
}

const CORNERS = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0),
    vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
);

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let ring = frame.rings[index / 6u];
    let corner = CORNERS[index % 6u];
    var out: VertexOutput;
    out.clip = vec4<f32>(ring.centre_half.xy + ring.centre_half.zw * corner, 0.0, 1.0);
    let cell = vec2<f32>(f32(ring.cell.x & 1u), f32(ring.cell.x >> 1u));
    // A cell is a half of the atlas each way; texture v runs down, the corner's y up.
    out.texcoord = (cell + vec2<f32>(corner.x * 0.5 + 0.5, 0.5 - corner.y * 0.5)) * 0.5;
    out.colour = vec4<f32>(ring.rgba.rgb, ring.rgba.a * unoccluded());
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // The GE's modulate: the vertex colour times the texel, alpha included.
    let texel = textureSample(atlas, atlas_sampler, in.texcoord);
    let rgb = gamma_to_target(texel.rgb * in.colour.rgb, linear_out);
    return vec4<f32>(target_out(rgb), texel.a * in.colour.a);
}
