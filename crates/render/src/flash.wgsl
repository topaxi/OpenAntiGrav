// The screen flash: one flat colour over the whole viewport, blended
// additively by the pipeline - `ScreenFlash_Draw`'s untextured quad.
// See `flash.rs`.

// 1.0 on Wipeout HD's linear float target, 0.0 on every gamma target - see
// `mesh_render::is_linear_target`. The flash is Pulse's alone, but the
// pipeline is built against whatever the race draws into.
override linear_out: f32 = 0.0;

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
    let rgb = mix(colour.rgb, pow(colour.rgb, vec3<f32>(2.2)), linear_out);
    return vec4<f32>(rgb, colour.a);
}
