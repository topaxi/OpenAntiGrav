// Rec. 709 luma weights, the correct ones for sRGB primaries (Rec. 601's are
// the older analogue-TV standard). Used on sRGB-encoded values by the edge
// detectors; `crates/post/src/screen.wgsl`'s display-era `luma` is Rec. 601 on
// purpose and does not share this.
public fn luma_rec709(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// Linear light to the standard piecewise sRGB curve, the one the console's output
// packer and texture unit use. `c` is on 0..1.
public fn srgb_encode(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3<f32>(1e-6)), vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}
