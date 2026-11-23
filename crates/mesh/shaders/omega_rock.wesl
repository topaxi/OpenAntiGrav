// Omega's `DC_Rocks` pixel program (`mesh::rcs::psp2::snow`, `Emissive::program` mode 22).
//
// Read off the lightmapped, shadow-map-free permutation (pixel `@0xca64`, vertex `@0xc424` of
// `DC_Rocks1.rcsmaterial`; `docs/formats/omega-status.md`, "The rock program"):
//
//     n1 = RockColourNormal(uv)          n2 = RockDetailNormal(uv * DetailScale)
//     z  = 2 n1.z - 1                    h  = 1 + DetailNormalStrength (2 n2.z - 2)
//     L  = (h (2 n1.xy - 1) + DetailNormalStrength z (2 n2.xy - 1), z h)      N = T L.x + B L.y + N0 L.z
//     w(x) = sat((x + Wrap) / (1 + Wrap))
//     Fsl  = 0.75 + 0.25 w(N.SL) + SLSD.y w(R.SL)^SLSD.x
//     D    = w(N.L) liveLighting0diffuse LM.a + nova Fsl           nova = scale pow(LM.rgb, power) + bias
//     out  = fog(rock * D + rock * SpecStrength (2 n2.z - 1) liveLighting0specular LM.a
//                           w(R.L)^e 0.06981317 / max(0.06981317, 0.17453294 k))
//
// `k` is `reflectiveBlurCoeffs.z + ReflectivityBlur * reflectiveBlurCoeffs.w`, the engine's constant
// `(1.72285, 2, 0, 1)` read live (`panel-displacement.md`, "Frame constants read live"), so
// `k = ReflectivityBlur`, and `e = SpecPower (1 - 0.75 k)`. The program also adds a cube reflection
// of weight `ReflectivityStrength (ReflectivityMin + ReflectivityScale (1 - N.V)^ReflectivityPower)`
// whose cube is bound by the engine and was not read: it is not drawn.
//
// The entry carries `DetailNormalStrength`, `DetailScale`, `SpecStrength` (tint), `SpecPower`
// (tint `w`), `k` (scale `x`) and `Wrap` (scale `w`). The colour normal is the third binding and the
// detail normal the fourth. `nova * Fsl` and the wrapped sun are summed in `shade.wesl` beside the
// lightmap's own decode.

import package::bindings::{emissive_scale, emissive_tint, scene};
import package::omega_snow::snow_wrap;
import package::texture_lod::{MATERIAL_PAD_MASK, MATERIAL_WAVE_MAP, material_sample_grad};
import package::types::{VertexOutput};

struct OmegaRock {
    normal: vec3<f32>,
    // `w(N.L)`, the sun's wrapped facing term.
    wrapped: f32,
    // `Fsl`, the shadow light's factor on the lightmap's Nova term.
    shadow_factor: f32,
    // The sun lobe times the picture's `SpecStrength` gloss, before the picture and the lightmap's mask.
    specular: vec3<f32>,
}

// `grad` is `uv_gradients(in.texcoord)`, for the reason `omega_snow_normal` gives.
fn omega_rock(entry: u32, in: VertexOutput, n: vec3<f32>, grad: mat2x2<f32>) -> OmegaRock {
    let c = emissive_tint(entry);
    let row = emissive_scale(entry);
    let uv = in.texcoord;
    let n1 = material_sample_grad(MATERIAL_PAD_MASK, uv, grad[0], grad[1]).rgb;
    let n2 = material_sample_grad(MATERIAL_WAVE_MAP, uv * c.y, grad[0] * c.y, grad[1] * c.y).rgb;
    let z = 2.0 * n1.z - 1.0;
    let h = 1.0 + c.x * (2.0 * n2.z - 2.0);
    let local = vec3<f32>(
        h * (2.0 * n1.x - 1.0) + c.x * z * (2.0 * n2.x - 1.0),
        h * (2.0 * n1.y - 1.0) + c.x * z * (2.0 * n2.y - 1.0),
        z * h,
    );
    let t = in.tangent.xyz;
    var scale = in.tangent.w * 2.0 - 1.0;
    if abs(scale) < 0.01 {
        scale = 0.0;
    }
    let b = cross(n, t) * scale;
    let mapped = local.x * t + local.y * b + local.z * n;
    var lit = n;
    if dot(mapped, mapped) >= 1.0e-12 {
        lit = normalize(mapped);
    }
    let wrap = row.w;
    let seen = normalize(scene.fog.camera - in.world);
    let reflected = reflect(-seen, lit);
    let light = scene.light.direction;
    var out: OmegaRock;
    out.normal = lit;
    out.wrapped = snow_wrap(dot(lit, light), wrap);
    out.shadow_factor = 1.0;
    if scene.light.shadow_light.w > 0.5 {
        let sl = scene.light.shadow_light.xyz;
        out.shadow_factor = 0.75 + 0.25 * snow_wrap(dot(lit, sl), wrap)
            + scene.light.shadow_light_specular.y
                * pow(snow_wrap(dot(reflected, sl), wrap), scene.light.shadow_light_specular.x);
    }
    let k = row.x;
    let lobe = pow(snow_wrap(dot(reflected, light), wrap), c.w * (1.0 - 0.75 * k))
        * (0.06981317 / max(0.06981317, 0.17453294 * k));
    out.specular = scene.light.sun_specular.rgb * (lobe * c.z * (2.0 * n2.z - 1.0));
    return out;
}
