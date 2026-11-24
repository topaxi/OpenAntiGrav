// Distance fog.

import oag_shaders::target::{decode, encode, linear_out};
import package::bindings::{scene};

// The GE's fog is a linear ramp between `near` and `far` - `Gu_Fog`
// (`0x08811748`) sends `far` and `1/(far - near)` and nothing else, so there is
// no curve to reproduce here. What the original varies is the *parameters*,
// re-sampled every frame from the camera's position inside the fog volume;
// that happens on the CPU before this uniform is written. See
// `docs/ghidra/functions/psp-pulse-usa/fog.md`.
//
// **The linear ramp measures radial distance from the eye, where the hardware
// uses view-space depth.**
// The two differ towards the screen edges, by up to `1 / cos(fov / 2)` - about
// 15 % at the corners of Pulse's authored field of view. Reproducing view-space
// z needs the view matrix separately, which this uniform block does not carry;
// recorded as a known divergence rather than silently accepted.
fn fogged(colour: vec3<f32>, world: vec3<f32>, view_depth: f32) -> vec3<f32> {
    // `curve` is 0.0 or 1.0 on every title, so a uniform branch runs one of the two factors and
    // not both; the `mix` stays for any other value.
    var factor: f32;
    if scene.fog.curve == 0.0 {
        factor = linear_fog(world);
    } else if scene.fog.curve == 1.0 {
        factor = authored_fog(view_depth);
    } else {
        factor = mix(linear_fog(world), authored_fog(view_depth), scene.fog.curve);
    }
    // The fog lerp sits inside the program's final instruction, so its output
    // scale halves the fog colour with the lit colour (`output_scale` is 1.0
    // where no title scales).
    return mix(scene.fog.colour * scene.light.output_scale, colour, mix(1.0, factor, scene.fog.enabled));
}

// The GE's ramp: 1.0 is clear, 0.0 is fully fogged, matching the GE's own sense.
fn linear_fog(world: vec3<f32>) -> f32 {
    let distance = length(world - scene.fog.camera);
    let span = max(scene.fog.far - scene.fog.near, 1e-6);
    return clamp((scene.fog.far - distance) / span, 0.0, 1.0);
}

// Wipeout HD's curve, from its own microcode - see `Fog::curve` in `types.wesl`. Saturated
// exactly as the original's EX2_SAT is.
fn authored_fog(view_depth: f32) -> f32 {
    let scaled = scene.fog.density * view_depth;
    return clamp(exp(-scaled * scaled), 0.0, 1.0);
}

// A light the program adds after its fog lerp (`Lit::after_fog`), in linear light: added
// straight to a linear scene's colour, and through the decode and encode on a gamma one. The
// colour is returned untouched where there is nothing to add.
fn plus_after_fog(colour: vec3<f32>, term: vec3<f32>) -> vec3<f32> {
    if all(term == vec3<f32>(0.0)) {
        return colour;
    }
    if linear_out > 0.5 {
        return colour + term;
    }
    return encode(decode(max(colour, vec3<f32>(0.0))) + term);
}
