// HD's scrolled-tap water (`Emissive::program` mode 17, `oag_mesh::mesh::rcs::water`).

import package::bindings::{emissive_scale, emissive_tint, scene};
import package::hull_normal::{hull_normal};
import package::texture_lod::{MATERIAL_PAD_MASK, material_sample};

// The entry's family, `program[1]`: 1 the sea, 2 Amphiseum's pool, 3 `water_test_2`,
// 4 `water_noref`; 0 where the entry is not a water entry.
fn water_kind(entry: u32, row_scale: vec4<f32>) -> f32 {
    if entry != 0u && row_scale.z > 16.5 && row_scale.z < 17.5 {
        return row_scale.w;
    }
    return 0.0;
}

// Where tap `i` of the entry fetches: `uv * scale + time * scroll`, the numbers laid by
// `mesh::rcs::water::entries`, `time` the scene's clock in seconds. The sea and the pool
// (`kind` 1, 2) carry one entry per tap after the header; Vineta K (3, 4) the three scales
// in the next entry and the three scrolls in the one after, both axes alike.
fn water_tap_uv(entry: u32, kind: f32, i: u32, uv: vec2<f32>) -> vec2<f32> {
    if kind < 2.5 {
        let tap = emissive_tint(entry + 1u + min(i, 1u));
        return uv * tap.xy + scene.time.x * tap.zw;
    }
    let scale = emissive_tint(entry + 1u)[min(i, 2u)];
    let scroll = emissive_tint(entry + 2u)[min(i, 2u)];
    return uv * scale + vec2<f32>(scene.time.x * scroll);
}

// The weight of tap `i`: the tap entry's `scale` (sea, pool; two taps), the header's tint
// (Vineta K; three taps).
fn water_tap_weight(entry: u32, kind: f32, i: u32) -> f32 {
    if kind < 2.5 {
        return select(0.0, emissive_scale(entry + 1u + min(i, 1u)).x, i < 2u);
    }
    return emissive_tint(entry)[min(i, 2u)];
}

// The sea's `SpecularColour`, in the two tap entries' `program` lanes.
fn water_specular_colour(entry: u32) -> vec3<f32> {
    return vec3<f32>(emissive_scale(entry + 1u).zw, emissive_scale(entry + 2u).z);
}

// The lighting normal: every tap of the normal map (third binding) decoded and summed by
// its weight, put through the chunk's tangent frame (`hull_normal`'s, the programs'
// `N = x T + y B + z N0`, `B = cross(N0, T) * w`). The sea and the pool decode `.ag` as
// `2x - 1` and rebuild `z = sqrt(1 - x^2 - y^2)` (the sea's `@0x00`..`@0x1f`); Vineta K
// decodes RGB (`MAD R0.xyz, tap, 2, -1`). Three taps, unrolled; the sea's third weighs zero.
fn water_normal(entry: u32, kind: f32, uv: vec2<f32>, n: vec3<f32>, tangent: vec4<f32>) -> vec3<f32> {
    let rgb = kind > 2.5;
    var sum = vec3<f32>(0.0);
    for (var i = 0u; i < 3u; i = i + 1u) {
        let texel = material_sample(MATERIAL_PAD_MASK, water_tap_uv(entry, kind, i, uv));
        let xy = vec2<f32>(texel.a, texel.g) * 2.0 - 1.0;
        let ag = vec3<f32>(xy, sqrt(max(1.0 - dot(xy, xy), 0.0)));
        let decoded = select(ag, texel.rgb * 2.0 - 1.0, rgb);
        sum = sum + decoded * water_tap_weight(entry, kind, i);
    }
    if dot(sum, sum) < 1.0e-12 || dot(tangent.xyz, tangent.xyz) < 0.25 {
        return n;
    }
    return hull_normal(vec4<f32>(normalize(sum) * 0.5 + 0.5, 0.0), n, tangent);
}
