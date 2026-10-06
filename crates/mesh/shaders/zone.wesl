// Zone-race track grading: the Zone sets, their sampling and the window glow.

import package::bindings::{scene, zone_nearest_sampler, zone_sampler, zone_scene_tex, zone_scene_tex_outer, zone_tex, zone_tex_outer, zone_vis_sampler, zone_vis_tex};
import package::types::{ZoneSet};

// **The Zone recolour**, read out of the Zone variant compiled into the disc's
// own materials rather than out of any engine program - see
// `docs/ghidra/functions/ps3-hdfury-eu/zone-shader.md`:
//
//     zoneUV  = zoneColourTint.xy * (1 - meshUV)
//     rim     = 1 - dot(N, toEye)
//     surface = zoneTex(zoneUV).rgb * zoneEffect.rgb
//             + zoneBase.rgb    * rim^10
//             + zoneBaseAlt.rgb * rim^5
//
// **The albedo is not in that sum.** On a circuit the Zone variant replaces
// the material's shading rather than tinting it, which is why the original's
// Zone frame reads near-monochrome: crowds, banners and concrete all go, and
// what is left is the stage's own two colours over the stage's own texture.
//
// A census of all 20,214 Zone-bearing fragment blocks on the disc finds
// two shapes and no block in both: this one, 20,084 blocks, which never
// carries the `blackMask` literal at all; and a `zoneAnisoPalette` shape of
// 130 blocks which always does. All twelve racing circuits are 100% this
// shape; the other lives only in `zone_1`..`zone_4`, HD's Zone arenas. The
// `albedo + zoneCol * (1 - blackMask)` rule this file used to carry was the
// arena shape, read correctly from an arena material and generalised.
//
// The zone texture is sampled at the *mesh's own* UV, not in screen space and
// not projected through `zoneOrigin` - which is the counter-intuitive half of
// the reading, and is confirmed from the file side by the engine's own schema
// naming the two lanes "Texture U scale" and "Texture V scale".
//
// **This returns the bare sample, not the finished term**, because the two
// shading paths below sum in different colour spaces and `pow(a * b, g)` is
// not `pow(a, g) * b`. `zoneTex` is a texture and wants the same decode every
// other sample here gets; `zoneEffect` is a shader parameter and wants none -
// it is authored well past 1.0 (to 3.0 on stage 12), which is a multiplier's
// range and not a colour's. `zoneBase*` are parameters on the same terms.
//
// **Which of the two publications a fragment reads is the chunk's own
// bit.** HD publishes the parameters twice - `zoneModeTrack*` beside the
// `Track.*` colours, `zoneMode*` beside the `Scene.*` ones - and
// `FUN_003ff860` picks per chunk on bit 0 of its render-block flags, which
// `mesh::rcs` carries here as `slots::ZONE_TRACK` (bit 9 of `in.slots`).
// Both textures are sampled and one selected, rather than branched on: a
// `textureSample` wants uniform control flow, and a flat-interpolated
// per-vertex word is not that.
//
// **Which of the Inner and Outer copies a fragment reads is a world-space
// sphere: `distance(worldPos, zoneOrigin) < zoneColourTint.w`** - read with
// opposite compiler polarity in two materials, and the same rule both times
// (`zone-shader.md`, 82). Inside it the stage that just committed, outside
// it the one before: a stage change on HD is a sphere growing out of the
// local craft that repaints the world as it passes, not a cross-fade. The
// radius and origin come from the game side every frame by the law read out
// of `Environment_UpdateStageBlend` and reproduced live - see
// `mesh_render::Zone`, "the stage-transition wavefront", for the numbers
// and for the one input still unmeasured (the radius's unit against this
// renderer's world). Before any transition the two pairs are equal and this
// selects the same colours either side, which is also every draw outside a
// Zone race.
//
// **Scope, stated because it is an approximation.** This applies to every
// surface `mesh.wesl` draws for a Zone race, and the original applies it per
// material. Craft are the exclusion that mattered and they are handled at the
// call site rather than here: `data/materials/ships/*` carries no Zone block
// at all, so `race::scene::frame` binds a default (disabled) Zone to the
// ships. The sky cube, the pads and the collision wireframe do still reach
// this path, which is the same shape of blanket approximation as the shared
// specular exponent and the blanket sun term above. **The stage texture now
// follows the sphere too**: `zone_tex_outer`/`zone_scene_tex_outer` are the
// stage being swept out's own `zoneTexInner`/`zoneTexOuter` publication,
// rebuilt on the same stage-change edge `oag_raceplay::Scene::rebind_zone_art`
// rebinds bind group 2 on rather than every frame - see
// `oag_mesh::mesh_render::zone::rebind`.
fn zone_is_track(slots: u32) -> bool {
    return (slots & 512u) != 0u;
}

fn zone_inside(world: vec3<f32>) -> bool {
    return distance(world, scene.zone.origin.xyz) < scene.zone.radius;
}

fn zone_set(slots: u32, world: vec3<f32>) -> ZoneSet {
    let inside = zone_inside(world);
    if zone_is_track(slots) {
        if inside {
            return scene.zone.track;
        }
        return scene.zone.track_outer;
    }
    if inside {
        return scene.zone.scene;
    }
    return scene.zone.scene_outer;
}

// Samples all four stage textures unconditionally and `select`s between them
// - never branches to decide *which* texture to sample - the same shape
// `zone_set` above already reads its four `ZoneSet`s in, and needed here for
// the same reason: `textureSample` wants uniform control flow, and a `select`
// on its *result* (or, as below, on which pre-sampled result to keep) satisfies
// that where an `if` around the call itself would not.
fn zone_sample(uv: vec2<f32>, slots: u32, world: vec3<f32>) -> vec3<f32> {
    let zone_uv = scene.zone.uv_scale * (1.0 - uv);
    let track = textureSample(zone_tex, zone_sampler, zone_uv).rgb;
    let track_outer = textureSample(zone_tex_outer, zone_sampler, zone_uv).rgb;
    let general = textureSample(zone_scene_tex, zone_sampler, zone_uv).rgb;
    let general_outer = textureSample(zone_scene_tex_outer, zone_sampler, zone_uv).rgb;
    let inside = zone_inside(world);
    return select(
        select(general_outer, track_outer, zone_is_track(slots)),
        select(general, track, zone_is_track(slots)),
        inside,
    );
}

// The two rim-lit summands, `zoneBase.rgb * rim^10 + zoneBaseAlt.rgb * rim^5`.
//
// Both exponents are **inline literals** in every one of the 20,084 blocks
// that carry this shape, not parameters, so nothing per-stage drives them and
// there is no uniform for them to come from.
//
// **`max(rim, 0)` is load-bearing.** The microcode's own `ADD R3.w, -R0,
// {1, 0, 0, 0}` is *unsaturated*, so `rim` runs negative on a back-facing or
// badly-normalled fragment; the hardware's `LG2` -> `MUL` -> `EX2` chain floors
// there at zero, while WGSL's `pow` with a negative base is undefined and hands
// back NaN. A NaN here survives every arithmetic below and paints a hole no
// unit test on the formula would catch.
//
// Undecoded on both shading paths, like the set's `effect`: these are shader
// *parameters*, authored in the `.effectSettings` file's own float domain, not
// texture samples that owe an sRGB decode.
fn zone_base_term(rim: f32, colours: ZoneSet) -> vec3<f32> {
    let r = max(rim, 0.0);
    return colours.base.rgb * pow(r, 10.0) + colours.base_alt.rgb * pow(r, 5.0);
}

// The visualiser glow: `saturate(N.y - 0.5) * (1 - windowDepth) * E.w *
// zoneTexVis[band].rgb`, read out of the Zone shader's own fragment microcode
// - `docs/ghidra/functions/ps3-hdfury-eu/zone-shader.md`. Re-read
// instruction by instruction against the project's own fixed decoder on
// 2026-08-31 (the condition-code and parameter-patch defects that page's own
// "Two `ps3-microcode.py` defects" section fixed the same day) and found
// **complete**: every register in the block traces to a named parameter, a
// known texture unit or the fog term already read elsewhere in this file, so
// nothing multiplies into it that this project has not already read.
// Confidence 84 on the structure, unchanged by the re-read since it only
// confirmed rather than moved it.
//
//     band = zoneTex<I|O>Nearest(zoneUV).a
//     glow = max( saturate(N.y - 0.5) * (1 - windowDepth) * E.w
//                * zoneTexVis[band].rgb , 0 )
//
// `zoneUV` is the same coordinate `zone_sample` builds - the microcode reuses
// it rather than a second one. `E.w` is the chunk's set's `effect.w` - `EQ
// brightness`, `Track.` or `Scene.` per `zone_set`: the disc's own per-stage
// drive scalar, `0.0` before the race starts and `20.0` from `Sub Venom` on.
//
// **`zoneTexVis` here is not the original's own table.** HD/Fury zero-fills
// it at load and rewrites it every frame from `Environment_UpdateStageBlend`
// - confirmed, not a static ramp - but *what* it writes is a float compared
// against the recovered per-stage threshold ladder, and whether that float is
// itself audio-reactive or a plain stage-progress fraction is unread; closing
// it needs an RPCS3 watchpoint on the value the comparison reads. See
// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`'s
// twenty-sixth pass. This samples `zone_vis_tex`, which
// `oag_mesh::mesh_render::zone::write_vis` fills every frame from a genuine
// spectrum of this project's own mixer output - the honest reading of "feed
// the shader audio data" while the original's own feed is unread, not a
// transcription of anything on the disc. See that function's own doc
// comment.
//
// **One additive term of the microcode's own glow is left out**:
// `5.0 * saturate(1 - 0.1 * (distance - zoneColourTint.w))`. Both of its
// inputs are bound now - `zone_inside` reads the same origin and radius -
// and it is still not drawn, for a different reason than before: as read,
// the term is `5.0` for every fragment *inside* the sphere (the argument
// saturates to `1` wherever `distance <= radius`) and only tails off over
// ten units past the boundary, which would flood the new stage's whole
// interior white. The live RPCS3 frame at radius `799` shows nothing of the
// kind - the near track is plainly the new stage's colours. So the reading
// of that term is what is in doubt, not its inputs, and it stays out until
// the microcode is re-read rather than being tuned into an edge glow that
// looks right.
//
// `window_depth` is `@builtin(position).z` read at fragment time - WebGPU's
// own window-space depth in `0..1`, the same convention the microcode's own
// `f[POS]` carries.
fn zone_glow(n: vec3<f32>, window_depth: f32, uv: vec2<f32>, slots: u32, world: vec3<f32>) -> vec3<f32> {
    let zone_uv = scene.zone.uv_scale * (1.0 - uv);
    // The band comes from whichever stage texture this chunk publishes - the
    // scene set's is alpha 255 everywhere, so a scene chunk indexes the last
    // entry - on whichever side of the sphere this fragment is, and `E.w`
    // from the matching colour group: `band = zoneTex<I|O>Nearest(zoneUV).a`
    // above already names both sides. Four samples, `select`ed rather than
    // branched into, for the same uniform-control-flow reason `zone_sample`
    // is.
    let track_band = textureSample(zone_tex, zone_nearest_sampler, zone_uv).a;
    let track_outer_band = textureSample(zone_tex_outer, zone_nearest_sampler, zone_uv).a;
    let scene_band = textureSample(zone_scene_tex, zone_nearest_sampler, zone_uv).a;
    let scene_outer_band = textureSample(zone_scene_tex_outer, zone_nearest_sampler, zone_uv).a;
    let inside = zone_inside(world);
    let band = select(
        select(scene_outer_band, track_outer_band, zone_is_track(slots)),
        select(scene_band, track_band, zone_is_track(slots)),
        inside,
    );
    let vis = textureSample(zone_vis_tex, zone_vis_sampler, vec2<f32>(band, 0.5)).rgb;
    let up = clamp(n.y - 0.5, 0.0, 1.0);
    let drive = zone_set(slots, world).effect.w;
    return max(up * (1.0 - window_depth) * drive * vis, vec3<f32>(0.0));
}
