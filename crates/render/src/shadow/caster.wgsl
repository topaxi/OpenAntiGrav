// Draws a caster into the shadow map: position through the light's own
// view-projection, and coverage out.
//
// There is nothing else to it, and that is a finding rather than a
// simplification. Wipeout HD's track material samples `shadowMapTex`
// projectively and uses the sample as `1 - shadow` with no compare against any
// reference, so the map holds coverage - see `mesh_render::ShadowMap`. A
// coverage map needs no depth, no normal, no uv and no bias.

struct Caster {
    // The light's view-projection times this caster's model matrix.
    mvp: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> caster: Caster;

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return caster.mvp * vec4<f32>(position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // Full coverage. How dark that draws is the receiver's business - see
    // `Scene::shadow.strength`, which is this project's number and not the
    // original's.
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
