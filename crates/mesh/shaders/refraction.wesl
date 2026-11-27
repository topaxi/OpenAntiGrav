// Vineta K's tunnel glass (`slots::REFRACTION`, `mesh::rcs::refraction`):
// out = fog(C + grab * W), one opaque draw, where the grab is the
// behind-the-glass target read at the point one unit along the surface normal,
// projected through `scene.refraction`.

import oag_shaders::target::{target_in};
import package::bindings::{behind_glass_tex, emissive_scale, emissive_tint, material_row, program_in, scene, shadow_sampler};
import package::fog::{fogged, plus_after_fog};
import package::shadow::{shadowed};
import package::types::{Lit, VertexOutput};
import package::wet::{is_wet_road, wet_road};

// `slots::REFRACTION`.
const SLOT_REFRACTION: u32 = 1048576u;

// An opaque surface's fogged colour from its lit texel: the glass's equation
// where the material is one, the ordinary `fog(shadow(C))` everywhere else.
fn opaque_colour(in: VertexOutput, lit_pixel: Lit) -> vec3<f32> {
    let shaded = lit_pixel.colour;
    let shade_out = shadowed(shaded.rgb, lit_pixel.sun, in.world);
    if (in.slots & SLOT_REFRACTION) == 0u {
        if is_wet_road(in) {
            return plus_after_fog(fogged(wet_road(shade_out.colour, in), in.world, in.view_depth, in.slots), shade_out.after_fog);
        }
        return plus_after_fog(fogged(shade_out.colour, in.world, in.view_depth, in.slots), shade_out.after_fog);
    }
    let lit = shade_out.colour;
    let slot = material_row(in.slots);
    let tint = emissive_tint(slot);
    let weight = emissive_scale(slot);
    // `mt_tunnelrefraction` weights the grab by its picture's alpha (rate 1),
    // `cl_tunnelrefraction` by a constant 2 (scale) and its lit term by
    // `DiffuseColour` (offset).
    // Omega's `rt_icetunnel` (`PROGRAM_ICE_GLASS`, mode 10) weights the grab by the vertex
    // colour's red (carried in `colour.a`) instead, and pushes the sample point
    // `Refraction * a` along the normal (`weight.w`) rather than one unit.
    let ice = program_in(weight.z, 10u, 10u);
    let alpha_weight = select(select(1.0, shaded.a, weight.y > 0.5), in.colour.a, ice);
    let w = tint.rgb * weight.x * alpha_weight;
    let reach = select(1.0, weight.w * in.colour.a, ice);
    // The point the program projects: the surface pushed one unit along its
    // normal (`distortion.z` = -1 negating a negated normal, live on the
    // capture). The normal map's perturbation of that normal is not drawn.
    let n = in.normal * inverseSqrt(max(dot(in.normal, in.normal), 1e-12));
    let clip = scene.refraction * vec4<f32>(in.world + n * reach, 1.0);
    let ndc = clip.xy / max(clip.w, 1e-6);
    // Kept inside the rectangle the target was drawn into (`scene.sea_reflection.zw`).
    let uv = min(vec2<f32>(0.5 + 0.5 * ndc.x, 0.5 - 0.5 * ndc.y), scene.sea_reflection.zw);
    let grab = target_in(textureSampleLevel(behind_glass_tex, shadow_sampler, uv, 0.0).rgb);
    return plus_after_fog(fogged(lit * tint.w + grab * w, in.world, in.view_depth, in.slots), shade_out.after_fog);
}
