// Rec. 709 luma weights, the correct ones for sRGB primaries (Rec. 601's are
// the older analogue-TV standard). Used on sRGB-encoded values by the edge
// detectors; `crates/post/src/screen.wgsl`'s display-era `luma` is Rec. 601 on
// purpose and does not share this.
public fn luma_rec709(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}
