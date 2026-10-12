// The Quake's travelling displacement of the road.

import package::bindings::{scene};

// **Wipeout HD's `StaticQuake` vertex program, step for step.** The class is
// `Static` with this in front of the transform (`track_surface.rcsmaterial`'s
// vertex block, decoded with `scripts/ps3-microcode.py vp-file`;
// `docs/ghidra/functions/ps3-hdfury-eu/quake.md`, confidence 85 for the program,
// registers read live on RPCS3 on a fired Quake). With `A`, `B` the two
// track-edge points at the wave, `M` their midpoint and `radius` `A.w`:
//
//     lift = B.w                                  amplitude
//          * cos(min(|(A - P) x (B - A)| / |B - A| / radius, pi / 2))
//                                                  nearest-the-line bump
//          * sqrt(saturate(1 - |M - P| / radius)) the falloff round the centre
//     P'   = P + up * lift
//
// `|(A - P) x (B - A)| / |B - A|` is `P`'s distance from the line through the
// two edge points, which runs across the road, so the bump is a quarter cosine
// along the track and the falloff clips it to a disc of `radius` around the
// midpoint. The normal is not touched, and neither is anything with
// `amplitude == 0`, which is how a race with no wave in flight draws exactly
// what it drew before this existed.
fn quake_lifted(world: vec3<f32>) -> vec3<f32> {
    let wave = scene.quake;
    let amplitude = wave.point_b.w;
    let radius = wave.point_a.w;
    let width = wave.offset.w;
    if amplitude <= 0.0 || radius <= 0.0 || width <= 0.0 {
        return world;
    }
    let a = wave.point_a.xyz;
    let midpoint = (a + wave.point_b.xyz) * 0.5;
    let falloff = sqrt(saturate(1.0 - length(midpoint - world) / radius));
    let along = length(cross(a - world, wave.offset.xyz)) / width / radius;
    let bump = cos(clamp(along, 0.0, 1.5707964));
    return world + wave.up.xyz * (amplitude * bump * falloff);
}
