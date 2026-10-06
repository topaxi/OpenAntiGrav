// Per-fragment screen-space velocity for the motion-vector target.

import package::types::{VertexOutput};

// How far this surface point moved on screen since the previous tick, in uv
// units: the NDC delta halved and y-flipped, which is the encoding the
// reconstruction filter expects. A point that was behind the previous
// camera's eye plane has no meaningful previous position and reports zero.
fn velocity_of(in: VertexOutput) -> vec2<f32> {
    if in.prev_clip.w <= 0.0 {
        return vec2<f32>(0.0);
    }
    let cur = in.cur_clip.xy / in.cur_clip.w;
    let prev = in.prev_clip.xy / in.prev_clip.w;
    return (cur - prev) * vec2<f32>(0.5, -0.5);
}
