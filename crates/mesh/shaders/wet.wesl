// The enhanced weather's wet road: ours, not the original's, which draws rain as
// particles alone (`docs/ghidra/functions/psp-pulse-usa/weather.md`).
//
// A surface marked `ROAD_MARK` (`GpuVertex::texcoord2.x`, a `.vex` texture named
// for a surface) is darkened by `wet` and given a reflection of the sky that grows
// towards grazing angles, the Fresnel look of a wet road. The sky is the circuit's
// own fog colour, which Pulse authors per fogCube volume and which is the colour
// the horizon fades into. Every constant here is chosen, not measured.

import package::bindings::{scene};
import package::types::{VertexOutput};

// How dark a fully wet road gets, as the share of its dry colour removed.
const WET_DARKEN: f32 = 0.42;
// The sky's share in the reflection at a grazing angle, and its floor face-on.
const WET_SKY_GRAZING: f32 = 0.85;
const WET_SKY_FACE_ON: f32 = 0.06;

// The lit road colour `rgb` made wet. Called only where `scene.fog.wet > 0`, so
// a dry frame never enters it.
fn wet_road(rgb: vec3<f32>, in: VertexOutput) -> vec3<f32> {
    let wet = scene.fog.wet;
    let n = normalize(in.normal);
    let view = normalize(scene.fog.camera - in.world);
    let facing = clamp(abs(dot(n, view)), 0.0, 1.0);
    let fresnel = WET_SKY_FACE_ON + (WET_SKY_GRAZING - WET_SKY_FACE_ON) * pow(1.0 - facing, 3.0);
    let dark = rgb * (1.0 - WET_DARKEN * wet);
    return dark + scene.fog.colour * (fresnel * wet);
}

// Whether a fragment is a road surface under a wet scene.
fn is_wet_road(in: VertexOutput) -> bool {
    return scene.fog.wet > 0.0 && in.lightmap_texcoord.z > 0.5;
}
