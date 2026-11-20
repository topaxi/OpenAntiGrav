// Omega's `RT_snow1` pixel program (`mesh::rcs::psp2::snow`, `Emissive::program` mode 21).
//
// Read off the vertex-lit permutation (`NOVAColor`, no lightmap, no shadow map, no vertex light:
// pixel `@0xaa24`, vertex `@0xa1e4` of `RT_snow1.rcsmaterial`; `docs/formats/omega-status.md`,
// "The snow program"):
//
//     N    = nx T + ny B + nz N0,  n = drift (+) detail (+) large detail, normalised
//     F    = FresnelMin + FresnelScale (1 - N.V)^FresnelPower
//     w(x) = sat((x + DiffuseWrap) / (1 + DiffuseWrap))
//     Fsl  = 0.75 + 0.25 w(N.SL) + SLSD.y w(R.SL)^SLSD.x
//     D    = w(N.L) liveLighting0diffuse a + nova Fsl           nova = scale pow(NOVAColor.rgb, power) + bias
//     out  = fog(F FresnelStrength FresnelColour VertexColour2 tex1 D
//                + SpecStrength SpecColour liveLighting0specular tex2.a a w(R.L)^SpecPower)
//
// The entry the vertex names carries `FresnelStrength * FresnelColour` (tint), `FresnelMin` (offset)
// and `FresnelScale` (scale) with `DiffuseWrap` in `program[1]`; the next entry `SpecStrength *
// SpecColour` (tint), `SpecPower` (offset), `SpecTwinkleScale` (scale) and `FresnelPower` (rate);
// the third the normal blend's `snownormDetailStrength`, `snownormDetailScale` and
// `snownormDetailLargeScale` (tint).
//
// The vertex carries `NOVAColor.rgb` in `colour`, its alpha `a` in `sun_mask`, `VertexColour2.rgb`
// in `lightmap_texcoord.xyz` and the drift mask `VertexColour1.x` in `lightmap_texcoord.w`. The
// drift normal is the third binding, the small detail the fourth, the large detail the fifth; the
// picture's alpha is `Texture2` as well, the same file.
//
// `reflectiveBlurCoeffs.z` is engine-bound and read as 0 (it reads 0 live on the sea,
// `panel-displacement.md`): the specular exponent is the plain `SpecPower` and its scale 1.

import package::bindings::{emissive_scale, emissive_tint, scene};
import package::texture_lod::{MATERIAL_ALBEDO, MATERIAL_FIFTH_MAP, MATERIAL_PAD_MASK, MATERIAL_WAVE_MAP, material_sample};
import package::types::{VertexOutput};

// What the draw's mode needs from the three normal maps and the program's constants: the lit
// normal, and the two light terms (`diffuse` times the decoded picture, `specular` added).
struct OmegaSnow {
    normal: vec3<f32>,
    diffuse: vec3<f32>,
    sun: vec3<f32>,
    specular: vec3<f32>,
}

fn snow_wrap(x: f32, wrap: f32) -> f32 {
    return clamp((x + wrap) / (1.0 + wrap), 0.0, 1.0);
}

// The bumped normal, in the world frame the interpolated tangent and normal give.
fn omega_snow_normal(entry: u32, in: VertexOutput, n: vec3<f32>) -> vec3<f32> {
    let uv = in.texcoord;
    let blend = emissive_tint(entry + 2u);
    let drift = material_sample(MATERIAL_PAD_MASK, uv).rgb;
    let small = material_sample(MATERIAL_WAVE_MAP, uv * blend.y).rgb;
    let large = material_sample(MATERIAL_FIFTH_MAP, uv * blend.z).rgb;
    let mask = in.lightmap_texcoord.w;
    // The drift normal fades to `(0.21952, 0.21952, 1)` where the mask is 0 (`@0x174`-`@0x1b4`).
    let dx = mask * (2.0 * drift.x - 1.0) + 0.21952 * (1.0 - mask);
    let dy = mask * (2.0 * drift.y - 1.0) + 0.21952 * (1.0 - mask);
    let dz = 1.0 + mask * (2.0 * drift.z - 2.0);
    let detail_xy = 2.0 * (small.xy + large.xy) - vec2<f32>(2.0);
    let height = 1.0 + blend.x * (2.0 * (small.z + large.z) - 3.0);
    let lift = blend.x * dz;
    let local = vec3<f32>(lift * detail_xy.x + dx * height, lift * detail_xy.y + dy * height, dz * height);
    let t = in.tangent.xyz;
    var scale = in.tangent.w * 2.0 - 1.0;
    if abs(scale) < 0.01 {
        scale = 0.0;
    }
    let b = cross(n, t) * scale;
    let mapped = local.x * t + local.y * b + local.z * n;
    if dot(mapped, mapped) < 1.0e-12 {
        return n;
    }
    return normalize(mapped);
}

fn omega_snow(entry: u32, in: VertexOutput, lit_normal: vec3<f32>) -> OmegaSnow {
    let program = emissive_scale(entry);
    let fresnel = emissive_tint(entry);
    let spec_entry = emissive_tint(entry + 1u);
    let spec_row = emissive_scale(entry + 1u);
    let wrap = program.w;
    let seen = normalize(scene.fog.camera - in.world);
    let reflected = reflect(-seen, lit_normal);
    let light = scene.light.direction;
    let sun_mask = in.sun_mask;
    // The Fresnel factor on the whole diffuse.
    let rim = pow(max(1.0 - dot(lit_normal, seen), 0.0), spec_row.y);
    let f = fresnel.w + program.x * rim;
    // The authored Nova decode of the baked colour, which the vertex program does per vertex
    // (`vs_main` leaves `scale * pow(colour, power)` in `lit_light.yzw`), then the bias; scaled
    // by the shadow light's factor below.
    let nova = in.lit_light.yzw + vec3<f32>(scene.light.prelit_bias);
    var shadow_factor = 1.0;
    if scene.light.shadow_light.w > 0.5 {
        let sl = scene.light.shadow_light.xyz;
        shadow_factor = 0.75 + 0.25 * snow_wrap(dot(lit_normal, sl), wrap)
            + scene.light.shadow_light_specular.y
                * pow(snow_wrap(dot(reflected, sl), wrap), scene.light.shadow_light_specular.x);
    }
    let sun = scene.light.sun * (snow_wrap(dot(lit_normal, light), wrap) * sun_mask);
    var out: OmegaSnow;
    out.normal = lit_normal;
    out.sun = f * fresnel.rgb * in.lightmap_texcoord.xyz * sun;
    out.diffuse = f * fresnel.rgb * in.lightmap_texcoord.xyz * (sun + nova * shadow_factor);
    let gloss = material_sample(MATERIAL_ALBEDO, in.texcoord * spec_row.x).a;
    out.specular = scene.light.sun_specular.rgb * spec_entry.rgb * (gloss * sun_mask
        * pow(snow_wrap(dot(reflected, light), wrap), spec_entry.w));
    return out;
}
