// Shadow, sun-occlusion and self-shadow terms.

import package::bindings::{scene, self_shadow_tex, shadow_depth_sampler, shadow_depth_tex, shadow_sampler, shadow_tex, sun_delta, sun_known, sun_occlusion_tex, uniforms};

// **Wipeout HD's engine-flare program**, off by default and on only for a model
// whose material declares the whole parameter set it reads - see
// `oag_mesh::mesh::Flame`, which carries the arithmetic and the evidence, and
// `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`, which carries the
// disassembly the arithmetic came from.
//
// **Every number is the material's, passed in as a pipeline constant**, because
// they are a property of the *model* exactly as `colour_is_light` is, and a
// uniform field would have to be mirrored by every pipeline in this crate and
// by the asset viewer. The defaults are zero rather than the shipped values on
// purpose: a model that reaches this path without its parameters must draw
// nothing recognisable, not a flame with numbers this file invented.
// Which shadow maps this model's surfaces may read: 0 reads none, 1 reads the
// `mapped` tier's depth map, 2 reads that *and* Wipeout HD's coverage map.
// Three states because the two tiers have different receivers and the sky is
// out of both - see `mesh_render::ShadowReceiver`.
override receives_shadow: f32 = 0.0;

// The GE's fog is a linear ramp between `near` and `far` - `Gu_Fog`
// (`0x08811748`) sends `far` and `1/(far - near)` and nothing else, so there is
// no curve to reproduce here. What the original varies is the *parameters*,
// re-sampled every frame from the camera's position inside the fog volume;
// that happens on the CPU before this uniform is written. See
// `docs/ghidra/functions/psp-pulse-usa/fog.md`.
//
// **Radial distance from the eye, where the hardware uses view-space depth.**
// The two differ towards the screen edges, by up to `1 / cos(fov / 2)` - about
// 15 % at the corners of Pulse's authored field of view. Reproducing view-space
// z needs the view matrix separately, which this uniform block does not carry;
// recorded as a known divergence rather than silently accepted.
// How far, in the craft map's `0..1` depth, a receiver may sit towards the sun
// of the craft's centre and still be shadowed by it: the fallback box's half
// thickness, 6 of the box's 140 units (`Shadow_BuildShipShadowMatrices`).
// **Chosen, not measured.** The original stores depth and compares it, so a
// road that is nearer the sun than the hull is lit; the coverage map stores no
// depth, and without this a deck or a tunnel roof *above* the craft along the
// sun would be darkened by a craft passing under it.
const CRAFT_SLAB_NEAR: f32 = 0.5 - 6.0 / 140.0;

// How far past the craft's centre, along the sun, a receiver may sit and still
// be shadowed by it: 50 of the box's 140 units, the radius the original redraws
// track chunks within (`0x008b7e24` = 50.0, `Shadow_CompileShadowedTrackRedraw`).
// **Chosen, not measured**: the original culls chunks by a sphere around the
// ship, this cuts a slab, which is the nearest cut a per-pixel lookup has.
const CRAFT_SLAB_FAR: f32 = 0.5 + 50.0 / 140.0;

// How much of this surface point the craft shadow maps cover, `0.0` to `1.0`.
//
// **Wipeout HD redraws the road once per ship, each through that ship's own
// map**, blending `SRC_ALPHA / ONE_MINUS_SRC_ALPHA` over the frame
// (`Shadow_CompileShadowedTrackRedraw`). Two redraws leave the road
// `(1 - a1) * (1 - a2)` of the way back to the sunlit colour, so the coverage
// of several craft is `1 - product(1 - coverage_i)`, which is what this loop
// computes in one pass. A craft with no map (an empty rect), a point outside
// its box and a point on the sun side of it all contribute nothing.
//
// **`ndc.y` is up, and a map's row 0 is its top.** `oag_core::math::
// camera`'s projections are glam's `rh::proj::directx` pair, which is
// Y-up NDC with a `0..1` depth (glam's own `camera/rh/proj.rs` says so),
// and the caster pass rasterises `ndc.y = +1` into texel row 0 like any
// wgpu render target. So the lookup is `0.5 - ndc.y * 0.5`. The other
// sign mirrors the map about its own horizontal axis, and that is a bug
// that hides itself: a caster near the middle still lands near the
// middle, and only the *side* the shadow falls on is wrong - which, for
// a low sun, reads as the shadow sitting on the wrong side of the craft
// and its silhouette turning the wrong way as the craft turns. Pinned by
// `tests/shadow_map_coverage.rs`, which reads a caster's texel back
// through this same formula.
//
// **A projective sample of a coverage map, not a depth compare** - which is
// what Wipeout HD's own track material does: `TXP R1.x, f[TC0] unit2`, then
// `1 - R1.x`, with nothing compared against anything. See
// `mesh_render::ShadowMap`.
fn craft_shadow_coverage(world: vec3<f32>) -> f32 {
    let atlas = vec2<f32>(textureDimensions(shadow_tex));
    var open = 1.0;
    for (var i = 0; i < 8; i = i + 1) {
        // A craft with no map has a zero radius (`CraftShadow::NONE`), so this
        // skips it too. Member by member rather than the whole record: a road
        // point near no craft reads only the centre.
        // Most road is far from most craft: one subtract and one dot reject it
        // before the projection's twelve multiplies. The sphere holds the whole
        // slab (`CRAFT_CULL_RADIUS` in `shadow::map`), so this skips only what
        // the slab would, and is the original's own "track chunks within 50
        // units of the ship" in form.
        let centre = scene.craft_shadow[i].centre;
        let offset = world - centre.xyz;
        if dot(offset, offset) > centre.w {
            continue;
        }
        let craft = scene.craft_shadow[i];
        // Orthographic, so `w` is 1 and the divide is the identity.
        let ndc = (craft.matrix * vec4<f32>(world, 1.0)).xyz;
        if ndc.z < CRAFT_SLAB_NEAR || ndc.z > CRAFT_SLAB_FAR
            || abs(ndc.x) >= 1.0 || abs(ndc.y) >= 1.0 {
            continue;
        }
        let local = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
        // Kept half a texel inside the craft's own square, so the filtered
        // read never takes a neighbouring craft's texels from the atlas.
        let half_texel = 0.5 / (craft.rect.zw * atlas);
        let uv = craft.rect.xy + clamp(local, half_texel, vec2<f32>(1.0) - half_texel) * craft.rect.zw;
        // Level 0 named rather than derived: the map has one mip, so the two
        // read the same texel, and Chrome's WGSL compiler refuses an implicit
        // derivative here because this loop is not uniform (naga allows it).
        open = open * (1.0 - textureSampleLevel(shadow_tex, shadow_sampler, uv, 0.0).r);
    }
    return 1.0 - open;
}

// How much of this surface point the shadow map covers, `0.0` to `1.0`: the
// craft maps' union in the coverage mode, the `mapped` tier's single compared
// depth map in the depth mode.
//
// Outside a map's own frustum the sample is clamped to its border, which the
// caster pass leaves at zero, so a surface the map does not cover is lit.
fn shadow_coverage(world: vec3<f32>) -> f32 {
    if scene.shadow.strength <= 0.0 {
        return 0.0;
    }
    // **Who may read which map.** In the coverage mode the receiver is the
    // track alone, which is what Wipeout HD's own materials say; in the depth
    // mode every lit surface reads, which is the whole point of a tier that
    // is not the original's.
    let coverage_mode = scene.shadow.mode < 1.5;
    let allowed = select(1.0, 2.0, coverage_mode);
    if receives_shadow < allowed {
        return 0.0;
    }
    if coverage_mode {
        return craft_shadow_coverage(world);
    }
    let clip = scene.shadow.matrix * vec4<f32>(world, 1.0);
    if clip.w <= 0.0 {
        return 0.0;
    }
    let ndc = clip.xyz / clip.w;
    // Behind the map's near plane or past its far one: nothing to sample.
    if ndc.z < 0.0 || ndc.z > 1.0 {
        return 0.0;
    }
    // The same `0.5 - ndc.y * 0.5` flip `craft_shadow_coverage` documents.
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    // Outside the map's own rectangle the clamped sampler would smear its
    // border across the whole circuit, so a receiver the map does not cover is
    // lit rather than edge-coloured.
    if uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 {
        return 0.0;
    }
    // The depth mode: four taps a texel apart, each compared, and their mean.
    // **The softening is in the comparison, not in the sample** - filtering
    // the depth itself would average two surfaces into one that is neither.
    // Four rather than sixteen because this runs on every lit pixel of the
    // frame; it is a choice, and it is ours, like everything else in this
    // tier.
    let texel = 1.0 / f32(textureDimensions(shadow_depth_tex).x);
    var lit = 0.0;
    for (var i = 0; i < 4; i = i + 1) {
        let offset = vec2<f32>(
            f32(i % 2) * 2.0 - 1.0,
            f32(i / 2) * 2.0 - 1.0
        ) * texel * 0.5;
        let nearest = textureSampleLevel(
            shadow_depth_tex,
            shadow_depth_sampler,
            uv + offset,
            // A depth texture's own `textureSampleLevel` takes an `i32` mip
            // level, not the `f32` a colour texture's does.
            0
        );
        lit = lit + select(0.0, 1.0, ndc.z - scene.shadow.depth_bias <= nearest);
    }
    return 1.0 - lit * 0.25;
}

// How far a Wipeout HD hull is pushed towards the sun before its depth is
// compared against its own self-shadow map, in that map's `0..1` units - a
// 140-unit-deep box, so this is a seventh of a unit.
//
// **Ours, and it can be small**: the caster pass culls front faces, so a
// sunlit face compares against the hull's far side rather than against
// itself, and the slope-scaled bias in `shadow::self_shadow::Maps::new`'s
// rasteriser handles the panels seen edge-on. This is the floor under it for
// a thin panel whose two sides nearly coincide.
const SELF_SHADOW_BIAS: f32 = 0.001;

// How much sun reaches this point of a Wipeout HD hull, `0.0` to `1.0` - the
// product of the two maps the original's `ShadowMap` hull variant multiplies
// its sun by:
//
//     TXP R0.x, f[TC0] unit1      <- directionalLight0ShadowTex, compared
//     TXP R0.z, f[TC0] unit2      <- directionalLight0LightmapTex, projected
//     MUL H0.w, R0.xxxx, R0.zzzz
//     MAD H0.xyz, H0, H0.wwww, {constantAmbientColour}
//
// `docs/ghidra/functions/ps3-hdfury-eu/ship-sun-occlusion.md`. The first is
// the craft's own depth from the sun, front faces culled, so a wing shadows
// the fuselage under it; the second is the track within ten units of the
// craft drawn from the sun as its own `lightmap.a`/`colourSet.w`, over black,
// so a craft over nothing reads 0. Both project through the one matrix, as
// the original's one `f[TC0]` does, so a texel of each is the same point of
// the hull.
//
// `1.0` for every draw that names no layer, which is everything but a hull
// under the `original` tier on a title that renders these maps - **and that
// early return is the whole of every other title's inertness**: nothing
// below it runs for a draw that names none.
fn sun_occlusion(world: vec3<f32>) -> f32 {
    let layer = i32(uniforms.sun_occlusion_layer + 0.5) - 1;
    if layer < 0 {
        return 1.0;
    }
    let clip = scene.sun_occlusion[layer] * vec4<f32>(world, 1.0);
    if clip.w <= 0.0 {
        return 1.0;
    }
    let ndc = clip.xyz / clip.w;
    // The same `0.5 - ndc.y * 0.5` flip `shadow_coverage` documents: the
    // pass rasterises `ndc.y = +1` into row 0.
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    // Off the map's own rectangle the original's clamped sample would return
    // the border, which its black clear leaves at 0 - a craft's hull always
    // fits its own box, so this is the box's margin and reads as no sun.
    // `textureSampleLevel` rather than `textureSample`: the layer index is
    // dynamic and this sits behind a branch, and a map with one mip has no
    // level to pick anyway.
    let occlusion = textureSampleLevel(sun_occlusion_tex, shadow_sampler, uv, layer, 0.0).r;
    // The self-shadow tap: the four compared taps `shadow_coverage`'s depth
    // mode takes, half a texel apart, and their mean. The original's is one
    // hardware-compared `TXP`; how the RSX filters that is unread, and four
    // is the softening this side already uses for a depth compare.
    let texel = 1.0 / f32(textureDimensions(self_shadow_tex).x);
    var lit = 0.0;
    for (var i = 0; i < 4; i = i + 1) {
        let offset = vec2<f32>(
            f32(i % 2) * 2.0 - 1.0,
            f32(i / 2) * 2.0 - 1.0
        ) * texel * 0.5;
        let nearest = textureSampleLevel(self_shadow_tex, shadow_depth_sampler, uv + offset, layer, 0);
        lit = lit + select(0.0, 1.0, ndc.z - SELF_SHADOW_BIAS <= nearest);
    }
    return occlusion * lit * 0.25;
}

// The shadow term.
//
// **The `original` tier is the original's own composite.** Wipeout HD redraws the
// track chunks near a craft through their `ShadowToAlpha` permutation, which is
// the same surface lit without the sun (no `Sun` bit in its word, and no
// shipped variant sets both) with `1 - shadowMapTex` as the fragment's alpha,
// blended `SRC_ALPHA / ONE_MINUS_SRC_ALPHA` over the frame
// (`Shadow_CompileShadowedTrackRedraw`, `0x004053e0`). So the result is the
// sunlit colour with the sun taken back out in proportion to the map's
// coverage: `colour - coverage * sun`, and there is no darkness number in it -
// a covered texel shows exactly the ambient the road would have had at night.
//
// **Everywhere else the coverage multiplies the colour by `1 - coverage *
// strength`**, and `strength` is chosen, not measured: a scene with no authored
// rig (no sun exists to take out), the `mapped` tier (this project's, not the
// original's), and a track chunk whose colour is a special combine rather than
// the generic one - the glass floor's facing ramp, the magstrip floor, a light
// cone - because `sun_known` is clear there. Those materials' own `ShadowToAlpha`
// programs are unread.
//
// **Called from every entry point that draws a lit surface**, which is five of
// them and not one: a race draws through the `_velocity` pair, so a version of
// this that only reached `fs_main` shadowed the scenery a blended material drew
// and left the road untouched. That is exactly what the first cut did.
fn shadowed(colour: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    let coverage = shadow_coverage(world);
    if scene.shadow.mode < 1.5 && sun_known > 0.5 {
        return max(colour - coverage * sun_delta, vec3<f32>(0.0));
    }
    return colour * (1.0 - coverage * scene.shadow.strength);
}
