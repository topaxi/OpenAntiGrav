// The screen flash: one flat colour over the whole viewport, blended
// additively by the pipeline - `ScreenFlash_Draw`'s untextured quad.
// See `flash.rs`.

import oag_shaders::target::{linear_out, gamma_to_target};

@group(0) @binding(0) var<uniform> colour: vec4<f32>;

// One triangle that covers the viewport; nothing to upload.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32((index << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(index & 2u) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    let rgb = gamma_to_target(colour.rgb, linear_out);
    return vec4<f32>(rgb, colour.a);
}
