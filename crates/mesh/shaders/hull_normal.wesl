// A hull's normal map.

// **The hull programs' lighting normal**, `N = nx T + ny B + nz N0` from the
// map's RGB as `2x - 1` (plain RGB, not DXT5nm), normalised
// (`docs/rendering/hd-ship-materials.md`, "Hull normal maps"). `T` is the
// authored vertex tangent in world space and `B = cross(N0, T) * w`, the
// vertex program's own formula; `w` is the tangent's fourth authored byte,
// `0` or `1`, so a `w = 0` vertex has no `B` and its `ny` never reaches `N`.
//
// **Chosen, not measured: `B` is built here from the interpolated `N0` and
// `T`**, where the original's vertex program builds it per vertex and
// interpolates it. They agree at every vertex and differ inside a triangle by
// the second-order change of the frame, which a hull's smooth normals make
// small. It saves a varying (the vertex stage uses locations 0 to 14 with the
// tangent, one more than `lightmap_texcoord`'s packing was avoiding; the GL
// backend, best effort, has the least room).
fn hull_normal(texel: vec4<f32>, n: vec3<f32>, tangent: vec4<f32>) -> vec3<f32> {
    let t = tangent.xyz;
    let b = cross(n, t) * tangent.w;
    let mapped = (texel.x * 2.0 - 1.0) * t + (texel.y * 2.0 - 1.0) * b + (texel.z * 2.0 - 1.0) * n;
    // A degenerate sum keeps the vertex normal rather than normalising zero.
    if dot(mapped, mapped) < 1.0e-12 {
        return n;
    }
    return normalize(mapped);
}
