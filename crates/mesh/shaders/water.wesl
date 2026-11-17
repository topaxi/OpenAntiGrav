// HD's scrolled-tap water (`Emissive::program` mode 17, `oag_mesh::mesh::rcs::water`).

import package::bindings::{emissive_scale, emissive_tint, scene};
import package::hull_normal::{hull_normal};
import package::texture_lod::{MATERIAL_PAD_MASK, material_sample};

// The entry's family, `program[1]`: 1 the sea, 2 Amphiseum's pool, 3 `water_test_2`,
// 4 `water_noref`, 5 `enhanced`, 6 2048's own sea (`mesh::rcs::psp2::water`); 0 where the
// entry is not a water entry.
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
    if kind < 2.5 || kind > 4.5 {
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
    if kind < 2.5 || kind > 4.5 {
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
    // Vineta K and 2048's sea decode the map's RGB (`MAD R0.xyz, tap, 2, -1`).
    let rgb = (kind > 2.5 && kind < 4.5) || kind > 5.5;
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

// **`enhanced` water** (kind 5, `mesh::vex_water`), in the gamma domain every Pulse-shaped
// asset is drawn in. The picture is the entry's colour (the surface's own texture's mean)
// times the vertex colour's baked light times a diffuse term from the wave normal; a sun glint
// and the sky's reflection are added. The taps read the world's `x` and `z`
// (`water_normal` is handed them), so the waves have one size wherever the surface's own
// texture coordinates run. The sun is the stand-in key light's direction and the numbers are
// chosen, not measured.
const ENHANCED_SUN: vec3<f32> = vec3<f32>(0.404, 0.808, 0.505);
const ENHANCED_DARK: f32 = 0.5;
const ENHANCED_LIT: f32 = 0.5;
const ENHANCED_GLINT: f32 = 0.5;

fn enhanced_water(
    tint: vec4<f32>,
    power: f32,
    specular_colour: vec3<f32>,
    vertex_colour: vec3<f32>,
    n: vec3<f32>,
    to_eye: vec3<f32>,
    sky: vec3<f32>,
) -> vec3<f32> {
    let ndl = clamp(dot(n, ENHANCED_SUN), 0.0, 1.0);
    let half_vector = normalize(to_eye + ENHANCED_SUN);
    let ndh = clamp(dot(half_vector, n), 0.0, 1.0);
    let glint = specular_colour * (pow(ndh, power) * ENHANCED_GLINT * clamp(8.0 * ndl, 0.0, 1.0));
    // The sky grazes the surface at a low angle, so it is weighted by how far the eye leans
    // over: `0.4` face on, all of `tint.w` at the horizon.
    let lean = 1.0 - clamp(dot(to_eye, n), 0.0, 1.0);
    let fresnel = 0.4 + 0.6 * lean * lean * lean;
    return tint.rgb * vertex_colour * (ENHANCED_DARK + ENHANCED_LIT * ndl)
        + glint
        + sky * (tint.w * fresnel);
}
