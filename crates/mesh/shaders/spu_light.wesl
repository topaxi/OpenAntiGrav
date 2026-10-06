// The PSP SPU light sum.

import package::bindings::{scene};

// **Wipeout HD's `EdgeGeom` light loop, per vertex.** Read off the SPU job's
// own binary (`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "`EdgeGeom`'s
// light path is read", confidence 80): for every light within range,
//
//     max(0, 1 - |light.pos - P| / D) ^ w  *  max(0, N . L)  *  colour
//
// summed, with `L` the unit vector from the vertex to the light and `w` the
// record's own exponent - `1.0` on every record ever captured, which is the
// job's fast path, so the `pow` is only reached where a record says
// otherwise. The `256/255` is the RGBE round trip the `SVC1` decode's own
// `255`/`128` literals introduce (same page, "The `SVC1` combine is read",
// confidence 88) - the packer's quantisation itself is not reproduced.
//
// **Chosen, not measured**: no per-chunk sphere cull. The original's
// `LightCulling` job drops a light from a chunk's list when it is more than
// `D` from the chunk's bounding sphere - a light whose falloff would be zero
// on every vertex in it anyway - so the sum is the same either way.
fn spu_light_sum(world: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    var sum = vec3<f32>(0.0);
    let count = min(scene.spu_lights.count.x, 128u);
    let n = select(vec3<f32>(0.0), normalize(normal), dot(normal, normal) > 0.0);
    for (var i = 0u; i < count; i++) {
        let light = scene.spu_lights.lights[i];
        let range = light.colour.w;
        let d = light.position.xyz - world;
        let dist = length(d);
        if range <= 0.0 || dist >= range {
            continue;
        }
        let linear = 1.0 - dist / range;
        let w = light.position.w;
        let att = select(pow(linear, w), linear, w == 1.0);
        let ndl = max(dot(n, d / max(dist, 1e-6)), 0.0);
        sum += light.colour.rgb * (att * ndl);
    }
    return sum * (256.0 / 255.0);
}
