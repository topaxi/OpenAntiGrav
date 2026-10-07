// Vineta K's tunnel glass (`slots::REFRACTION`, `mesh::rcs::refraction`):
// out = fog(C + grab * W), one opaque draw, where the grab is the
// behind-the-glass target read at the point one unit along the surface normal,
// projected through `scene.refraction`.

import package::bindings::{behind_glass_tex, emissives, scene, shadow_sampler};
import package::fog::{fogged};
import package::shadow::{shadowed};
import package::types::{VertexOutput};

// `slots::REFRACTION`.
const SLOT_REFRACTION: u32 = 1048576u;

// `slots::MATERIAL_SHIFT`.
const MATERIAL_SHIFT: u32 = 23u;

// An opaque surface's fogged colour from its lit texel: the glass's equation
// where the material is one, the ordinary `fog(shadow(C))` everywhere else.
fn opaque_colour(in: VertexOutput, shaded: vec4<f32>) -> vec3<f32> {
    let lit = shadowed(shaded.rgb, in.world);
    if (in.slots & SLOT_REFRACTION) == 0u {
        return fogged(lit, in.world, in.view_depth);
    }
    let slot = in.slots >> MATERIAL_SHIFT;
    let tint = emissives.tint_offset[slot];
    let weight = emissives.scale[slot];
    // `mt_tunnelrefraction` weights the grab by its picture's alpha (rate 1),
    // `cl_tunnelrefraction` by a constant 2 (scale) and its lit term by
    // `DiffuseColour` (offset).
    let alpha_weight = select(1.0, shaded.a, weight.y > 0.5);
    let w = tint.rgb * weight.x * alpha_weight;
    // The point the program projects: the surface pushed one unit along its
    // normal (`distortion.z` = -1 negating a negated normal, live on the
    // capture). The normal map's perturbation of that normal is not drawn.
    let n = in.normal * inverseSqrt(max(dot(in.normal, in.normal), 1e-12));
    let clip = scene.refraction * vec4<f32>(in.world + n, 1.0);
    let ndc = clip.xy / max(clip.w, 1e-6);
    let uv = vec2<f32>(0.5 + 0.5 * ndc.x, 0.5 - 0.5 * ndc.y);
    let grab = textureSampleLevel(behind_glass_tex, shadow_sampler, uv, 0.0).rgb;
    return fogged(lit * tint.w + grab * w, in.world, in.view_depth);
}
