// Distance fog.

import package::bindings::{scene};

fn fogged(colour: vec3<f32>, world: vec3<f32>, view_depth: f32) -> vec3<f32> {
    let distance = length(world - scene.fog.camera);
    let span = max(scene.fog.far - scene.fog.near, 1e-6);
    // 1.0 is clear, 0.0 is fully fogged, matching the GE's own sense.
    let linear = clamp((scene.fog.far - distance) / span, 0.0, 1.0);
    // Wipeout HD's curve, from its own microcode - see `Fog::curve` above.
    // Saturated exactly as the original's EX2_SAT is.
    let scaled = scene.fog.density * view_depth;
    let authored = clamp(exp(-scaled * scaled), 0.0, 1.0);
    let factor = mix(linear, authored, scene.fog.curve);
    return mix(scene.fog.colour, colour, mix(1.0, factor, scene.fog.enabled));
}
