// Pad normal-map shading.

// **The pad's tangent-space normal, in world space.** The pad programs build
// `N = nx*T + ny*B + nz*N0` from the `_ne` texel (`x*2-1` on each lane), with
// `T` an authored vertex attribute and `B = cross(N0, T) * w`
// (`docs/rendering/pads.md`, "The pad's tangent frame"). Two measurements
// shape this function:
//
// - The authored `T` points along `dP/du` of its triangle (mean dot 0.94 to
//   0.99 over 252 to 503 triangles per chunk, `hd_pad_tangent_probe`), so
//   `dP/du` from screen-space derivatives of the world position and the
//   texture coordinate stands in for it, with no new vertex attribute.
//   **The derived frame is chosen, not measured**: the authored stream is
//   per vertex and smoothed, and was not decoded into `GpuVertex`.
// - The authored `w` byte that scales `B` is `0` on 232 of 232 vertices of
//   `12_sol_2`'s `Weapon Pad` chunks, on 839 and 836 of 840 on
//   `talons_junction`'s, and on 209 to 213 of 216 on `12_sol_2`'s `Speedup
//   Pad` chunks, so `B` is the zero vector and `ny` never reaches `N`. The
//   disc's own normal is `nx*T + nz*N0`, and that is what this builds.
@diagnostic(off, derivative_uniformity)
fn pad_normal(ne: vec4<f32>, n: vec3<f32>, world: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let q1 = dpdx(world);
    let q2 = dpdy(world);
    let s1 = dpdx(uv);
    let s2 = dpdy(uv);
    let det = s1.x * s2.y - s2.x * s1.y;
    // A degenerate or unmapped pixel keeps the vertex normal.
    if abs(det) < 1.0e-20 {
        return n;
    }
    let dp_du = (q1 * s2.y - q2 * s1.y) / det;
    let t = normalize(dp_du - n * dot(n, dp_du));
    return normalize((ne.x * 2.0 - 1.0) * t + (ne.z * 2.0 - 1.0) * n);
}
