// A hull's normal map.

// **The hull programs' lighting normal**, `N = nx T + ny B + nz N0` from the
// map's RGB as `2x - 1` (plain RGB, not DXT5nm), normalised
// (`docs/rendering/hd-ship-materials.md`, "Hull normal maps"). `T` is the
// authored vertex tangent in world space and `B = cross(N0, T) * s`, the
// vertex program's own formula. `s` arrives baked in `tangent.w` as `(s + 1) / 2`
// (`mesh::rcs::baked_tangent`): the compile that reads `w` raw gives `s = w` (a `w = 0` vertex
// has no `B`), the compile that reads it as `2w - 1` gives `s = -1` there, and which compile
// a material has is read off its vertex program (`docs/rendering/hd-track-tone.md`, "Tech de
// Ra's near-left wall"). A raw `w = 0` bakes to byte 128 and decodes to +0.004, which is
// snapped to 0.
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
    var scale = tangent.w * 2.0 - 1.0;
    if abs(scale) < 0.01 {
        scale = 0.0;
    }
    let b = cross(n, t) * scale;
    let mapped = (texel.x * 2.0 - 1.0) * t + (texel.y * 2.0 - 1.0) * b + (texel.z * 2.0 - 1.0) * n;
    // A degenerate sum keeps the vertex normal rather than normalising zero.
    if dot(mapped, mapped) < 1.0e-12 {
        return n;
    }
    return normalize(mapped);
}
