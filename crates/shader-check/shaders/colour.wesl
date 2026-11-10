// Rec. 709 luma weights, the correct ones for sRGB primaries (Rec. 601's are
// the older analogue-TV standard). Used on sRGB-encoded values by the edge
// detectors; `crates/post/src/screen.wgsl`'s display-era `luma` is Rec. 601 on
// purpose and does not share this.
public fn luma_rec709(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// The sRGB EOTF of one channel, exactly - the piecewise curve, not a `pow(c, 2.2)`
// standing in for it. A value decoded here was encoded by the hardware with the
// exact curve, so anything else would not round-trip and would tint the darks,
// which is where the two curves differ most and where a racing game spends much
// of its frame. A negative input takes the linear arm and is not clamped; a
// caller that owes the `0..1` range clamps first.
public fn srgb_decode_channel(v: f32) -> f32 {
    if v <= 0.04045 {
        return v / 12.92;
    }
    return pow((v + 0.055) / 1.055, 2.4);
}

// [`srgb_decode_channel`] on all three channels. Built from the scalar form and
// not from a vector `pow`: the driver compiles the two to results that differ by
// an ulp, which moved one pixel of the HD leach ribbon by one level.
public fn srgb_decode(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(srgb_decode_channel(c.x), srgb_decode_channel(c.y), srgb_decode_channel(c.z));
}

// Linear light to the standard piecewise sRGB curve, the one the console's output
// packer and texture unit use. `c` is on 0..1.
public fn srgb_encode(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3<f32>(1e-6)), vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}
