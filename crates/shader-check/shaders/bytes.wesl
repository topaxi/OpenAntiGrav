// A sampled texel as the 8-bit integers the PSP's GE and the PS2's GS hold it
// in; the bloom passes truncate each product to a whole byte as those do.
public fn bytes(texel: vec3<f32>) -> vec3<u32> {
    return vec3<u32>(floor(texel * 255.0 + 0.5));
}
