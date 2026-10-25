// Shadow, sun-occlusion and self-shadow terms.

@if(ray) import package::bindings::{sun_tlas};
import package::bindings::{scene, self_shadow_tex, shadow_depth_sampler, shadow_depth_tex, shadow_exempt, shadow_sampler, shadow_tex, sun_delta, sun_known, sun_occlusion_tex, uniforms};

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

// How far along the sun a traced shadow ray starts, in world units, so a surface does not
// shadow itself. **Measured on Moa Therma White, then chosen**: the circuit carries a
// second surface hovering about 0.12 to 0.4 above parts of its road, never drawn but in the
// structure, which at the first value (0.12) shadowed the road in triangle-shaped patches;
// 0.4 left a few, 0.8 none. Six tenths clears it with room and stays far under a craft's
// hover of about 3.5, so no craft loses its shadow. What that surface is is not read.
const TRACED_START: f32 = 0.6;

// How far a traced shadow ray looks for an occluder. The sun is at infinity and a bridge's
// shadow can fall from far above, so this is past any circuit's height rather than a
// cascade's box. **Ours.**
const TRACED_REACH: f32 = 2000.0;

// How much of this point the `traced` tier says is in shadow, `0.0` to `1.0`: rays toward
// the sun, everything casting and everything receiving, no cascade, no resolution and no
// bias slope. How many rays is the player's `traced_quality` (`scene.shadow.traced_rays`):
//
// - **One** (`medium`): the nearest occluder decides, and its distance lightens the
//   shadow - full within `TRACED_FADE_NEAR`, down to `TRACED_FADE_FLOOR` of it by
//   `TRACED_FADE_FAR` - the way a wide penumbra thins its umbra. The edge stays hard.
// - **More** (`high`): that many rays spread across a sun `TRACED_SUN_SPREAD` wide, the
//   fraction blocked. The edge blurs with the caster's distance, a real penumbra. Three
//   probes go first and settle every point outside a penumbra, so the full set is cast
//   only along shadow edges.
//
// The direction is read out of the same matrix the `mapped` tier fits, so the two tiers
// agree on where the sun is by construction (a circuit's authored sun, or Pulse's borrowed
// axis): depth in that orthographic matrix grows away from the light, so its z row is the
// direction *from* the light and the sun is the other way.
@if(ray) fn traced_coverage(world: vec3<f32>) -> f32 {
    let m = scene.shadow.matrix;
    let away = vec3<f32>(m[0][2], m[1][2], m[2][2]);
    if dot(away, away) <= 0.0 {
        return 0.0;
    }
    let towards = -normalize(away);
    let rays = max(1, i32(scene.shadow.traced_rays + 0.5));
    if rays == 1 {
        let t = traced_nearest(world, towards);
        if t < 0.0 {
            return 0.0;
        }
        return mix(1.0, TRACED_FADE_FLOOR, smoothstep(TRACED_FADE_NEAR, TRACED_FADE_FAR, t));
    }
    // A basis across the sun's disc, and the disc's samples on a golden-angle spiral
    // turned by a hash of the point, so neighbouring pixels sample different parts of the
    // disc and the penumbra reads as fine grain rather than as a stack of hard bands.
    let side = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(towards.y) > 0.9);
    let u = normalize(cross(towards, side));
    let v = cross(towards, u);
    let turn = traced_hash(world) * 6.2831853;
    // **Probes first**: the disc's centre and two opposite points of its rim. Where all
    // three agree the point is wholly lit or wholly shadowed - most of a frame - and the
    // answer stands; only a point where they disagree is in a penumbra and casts the
    // full set. The centre catches a caster thin enough to pass between the two rim rays.
    let rim = (u * cos(turn) + v * sin(turn)) * TRACED_SUN_SPREAD;
    let probes = select(0, 1, traced_blocked(world, towards))
        + select(0, 1, traced_blocked(world, normalize(towards + rim)))
        + select(0, 1, traced_blocked(world, normalize(towards - rim)));
    if probes == 0 {
        return 0.0;
    }
    if probes == 3 {
        return 1.0;
    }
    var blocked = 0.0;
    for (var i = 0; i < rays; i = i + 1) {
        let radius = sqrt((f32(i) + 0.5) / f32(rays)) * TRACED_SUN_SPREAD;
        let angle = f32(i) * 2.3999632 + turn;
        let direction = normalize(towards + (u * cos(angle) + v * sin(angle)) * radius);
        blocked = blocked + select(0.0, 1.0, traced_blocked(world, direction));
    }
    return blocked / f32(rays);
}

// The `medium` fade: a caster within `TRACED_FADE_NEAR` world units shadows fully, one past
// `TRACED_FADE_FAR` at `TRACED_FADE_FLOOR` of that. **Ours, all three**: near enough that
// a craft's own shadow on the road keeps its full weight, far enough that a canopy or a
// deck overhead reads paler than the craft under it.
const TRACED_FADE_NEAR: f32 = 4.0;
const TRACED_FADE_FAR: f32 = 60.0;
const TRACED_FADE_FLOOR: f32 = 0.35;

// The `high` sun's half-width, as the tangent of its half-angle: 2.5 degrees. **Ours**:
// the real sun's quarter degree leaves a canopy's edge as hard as one ray does; this blurs
// it across about four units at fifty, and a craft's at a few tenths.
const TRACED_SUN_SPREAD: f32 = 0.0437;

// The distance to the nearest opaque thing along `direction` from `world`, or `-1.0` for
// none within `TRACED_REACH`. Traverses to the closest hit, which a fade needs.
@if(ray) fn traced_nearest(world: vec3<f32>, direction: vec3<f32>) -> f32 {
    var query: ray_query;
    rayQueryInitialize(
        &query,
        sun_tlas,
        RayDesc(0u, 0xFFu, TRACED_START, TRACED_REACH, world, direction)
    );
    while rayQueryProceed(&query) {}
    let hit = rayQueryGetCommittedIntersection(&query);
    return select(-1.0, hit.t, hit.kind != RAY_QUERY_INTERSECTION_NONE);
}

// Whether anything opaque lies along `direction` from `world`: the first hit found ends
// the traversal, since only whether, not where, is asked.
@if(ray) fn traced_blocked(world: vec3<f32>, direction: vec3<f32>) -> bool {
    var query: ray_query;
    rayQueryInitialize(
        &query,
        sun_tlas,
        RayDesc(RAY_FLAG_TERMINATE_ON_FIRST_HIT, 0xFFu, TRACED_START, TRACED_REACH, world, direction)
    );
    while rayQueryProceed(&query) {}
    return rayQueryGetCommittedIntersection(&query).kind != RAY_QUERY_INTERSECTION_NONE;
}

// A value in `0..1` from a world position's own bits (a PCG step over each axis), stable
// for a point of a surface from frame to frame, so the `high` grain does not crawl.
fn traced_hash(world: vec3<f32>) -> f32 {
    var h = bitcast<vec3<u32>>(world);
    h = h * 747796405u + 2891336453u;
    h.x = h.x + h.y * h.z;
    h.y = h.y + h.z * h.x;
    h.z = h.z + h.x * h.y;
    h = h ^ (h >> vec3<u32>(16u));
    return f32((h.x ^ h.y ^ h.z) & 0xFFFFFFu) / 16777216.0;
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
    // The `traced` tier (mode 3) asks the acceleration structure instead of a map. Only
    // the `ray` variant has one; the scene block is shared, so a drawable built without
    // it can still be handed mode 3 and reads no shadow rather than a map nobody drew.
    if scene.shadow.mode > 2.5 {
        @if(ray) { return traced_coverage(world); }
        @if(!ray) { return 0.0; }
    }
    if coverage_mode {
        return craft_shadow_coverage(world);
    }
    // The depth mode: two layers of one map, the near box around the player
    // and the far one out towards the horizon. **Inside the near box the near
    // layer alone**; across its rim the two blend; past it the far layer
    // alone, faded out across its own rim. Before the far layer existed a
    // receiver past the near box read lit, and under a long roof that drew a
    // straight-edged end of shadow that travelled with the craft.
    let near = depth_layer_coords(scene.shadow.matrix, world);
    let far = depth_layer_coords(scene.shadow.far_matrix, world);
    let near_weight = clamp(near.w / CASCADE_RIM, 0.0, 1.0);
    var shadow = 0.0;
    if near_weight > 0.0 {
        shadow = depth_layer_shadow(near, 0, scene.shadow.depth_bias);
    }
    if near_weight < 1.0 && far.w > 0.0 {
        let far_weight = clamp(far.w / CASCADE_RIM, 0.0, 1.0);
        let far_shadow = depth_layer_shadow(far, 1, scene.shadow.far_depth_bias) * far_weight;
        shadow = mix(far_shadow, shadow, near_weight);
    }
    return shadow;
}

// How much of a layer's own half-width, in `uv`, the blend between the depth
// map's two layers spans: the near layer hands over to the far one across the
// outer tenth of its box, and the far one fades to lit across the outer tenth
// of its own. **Ours**: wide enough that neither edge reads as a line, narrow
// enough that the near layer's sharper texels cover most of its box.
const CASCADE_RIM: f32 = 0.1;

// Where `world` lands in one layer of the `mapped` depth map: `uv` in `xy`, its
// depth in that layer's `0..1` in `z`, and in `w` how far inside the layer's
// rectangle it is - `0.0` at the edge, `0.5` at the centre, negative outside the
// rectangle or past the box's depth.
//
// **Outside reads as uncovered, never as the border**: the clamped sampler
// would smear the border texel across the whole circuit.
fn depth_layer_coords(matrix: mat4x4<f32>, world: vec3<f32>) -> vec4<f32> {
    let clip = matrix * vec4<f32>(world, 1.0);
    if clip.w <= 0.0 {
        return vec4<f32>(0.0, 0.0, 0.0, -1.0);
    }
    let ndc = clip.xyz / clip.w;
    // Behind the box's near plane or past its far one: nothing to sample.
    if ndc.z < 0.0 || ndc.z > 1.0 {
        return vec4<f32>(0.0, 0.0, 0.0, -1.0);
    }
    // The same `0.5 - ndc.y * 0.5` flip `craft_shadow_coverage` documents.
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    let inside = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    return vec4<f32>(uv, ndc.z, inside);
}

// How much of a point one layer of the depth map shadows, `0.0` to `1.0`, from
// `depth_layer_coords`' answer: four taps a texel apart, each compared, and
// their mean. **The softening is in the comparison, not in the sample** -
// filtering the depth itself would average two surfaces into one that is
// neither. Four rather than sixteen because this runs on every lit pixel of the
// frame; it is a choice, and it is ours, like everything else in this tier.
fn depth_layer_shadow(coords: vec4<f32>, layer: i32, bias: f32) -> f32 {
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
            coords.xy + offset,
            layer,
            // A depth texture's own `textureSampleLevel` takes an `i32` mip
            // level, not the `f32` a colour texture's does.
            0
        );
        lit = lit + select(0.0, 1.0, coords.z - bias <= nearest);
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
// the generic one - a light cone, and the like - because `sun_known` is clear
// there. Those materials' own `ShadowToAlpha` programs are unread. **The glass
// floor's facing ramp and the magstrip floor are exempt** (`shadow_exempt`): the
// first has no shadow lookup, the second's redraw has no sun to take out.
//
// **Called from every entry point that draws a lit surface**, which is five of
// them and not one: a race draws through the `_velocity` pair, so a version of
// this that only reached `fs_main` shadowed the scenery a blended material drew
// and left the road untouched. That is exactly what the first cut did.
fn shadowed(colour: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    if shadow_exempt > 0.5 {
        return colour;
    }
    let coverage = shadow_coverage(world);
    if scene.shadow.mode < 1.5 && sun_known > 0.5 {
        return max(colour - coverage * sun_delta, vec3<f32>(0.0));
    }
    return colour * (1.0 - coverage * scene.shadow.strength);
}
