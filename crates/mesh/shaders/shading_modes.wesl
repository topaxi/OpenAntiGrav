// Whole-pipeline shading switches: Omega's nova prelit curve, colour-is-light.

// 0.0 on a pipeline whose scene can never light by Omega's nova prelit curve,
// so `lit_texel` folds `scene.light.nova` away and the driver deletes the
// curve's second `pow` a channel: 0.55 ms of HD's 22.5 ms opaque track at
// 3200x1800 on the Raphael iGPU. **1.0, the uniform deciding, by default**,
// so every pipeline built outside a race scene - the viewer, a test, a front
// end - shades as it always did. Set by `race::Scene::new` through
// `mesh_render::BuildCacheScope::lit_by`. A constant rather than a rewrite of
// the curve: selecting the `pow`'s input instead moves pixels.
override nova_prelit: f32 = 1.0;

// 0.0 on a pipeline whose scene has no authored rig (`Light::stand_in`, every circuit with no
// `.envsettings`) or is no Zone race, so `lit_texel` reads a constant zero for
// `scene.light.enabled` or `scene.zone.enabled` and the driver deletes the paths behind them.
// **1.0, the uniform deciding, by default**, as `nova_prelit`. Neither changes mid-race: a Zone
// grade only ever grades a rig that is on (`ZoneGrade::light`), and a race with no grade writes
// a zero `Zone`. Set through `mesh_render::BuildCacheScope::{lit_by, zoned}`.
override light_rig: f32 = 1.0;
override zone_race: f32 = 1.0;

// 1.0 on a circuit's pipeline in a Zone race whose frame lights every chunk by the stage's
// ambient and sun (`mesh_render::BuildCacheScope::zone_stage_light`, only where
// `specialise::zone_light_exact` holds): `lit_colour` states that light outright, so the driver
// drops the lightmap and vertex light chain it overwrites. **0.0, the uniform deciding, by
// default.**
override zone_stage_light: f32 = 0.0;

// 1.0 on a pipeline every pixel of which takes the authored rig's result: its scene's rig is
// on for the whole race and every vertex it draws has `lit` 1.0, so `lit_texel`'s last mix
// has a weight of exactly one and the stand-in result it weighs at zero, with the decode of
// it, is never seen. **0.0, the mix deciding, by default.** Set for a circuit's plain draws
// (`mesh_render::Prepass::plain`) by `mesh_render::specialise::plain_is_lit`, and for a craft hull
// whose every vertex is lit (`BuildCacheScope::craft`), under a scope whose light is an authored
// rig (`BuildCacheScope::lit_by`).
override lit_by_rig: f32 = 0.0;

// **Whether any vertex this pipeline draws can be a 2048 vertex-lit one** (`sun_mask` 2 or above,
// `NOVAColor`). 0.0 where none is (`mesh_render::specialise::nova_vertices`), so `lit_texel`'s
// `nova_vertex` is the constant false; 1.0, the vertex deciding, by default.
override nova_vertices: f32 = 1.0;

// **Whether any vertex this pipeline draws carries a tangent** (`mesh_render::specialise::
// tangents_present`): 0.0 where none does, so every tangent the fragment stage reads is the zero
// vector `vs_main` leaves and the normal-map branches on it go; 1.0, the varying deciding, by
// default.
override tangents_present: f32 = 1.0;

// **Whether the scene's rig has a generic specular at all**: 0.0 where its `specular_scale` is
// zero for the whole race (`BuildCacheScope::lit_by`; 2048's and Omega's circuits author none),
// so `lit_texel`'s generic specular, a product with that scale, is the zero it would compute.
// 1.0, computed, by default.
override rig_specular: f32 = 1.0;

// **1.0 on a circuit's plain pipeline when the player chose CIRCUIT LIGHTING `simplified`**
// (`oag_display::display::CircuitLighting`, `BuildCacheScope::simplified_lighting`): a departure
// from the original for speed. No specular, no normal map, no Wipeout HD SPU vertex lights; the
// albedo, baked light, ambient, sun on the surface's own normal, glows, fog and shadows stay.
// 0.0, the original's shading, by default.
override simplified_lighting: f32 = 0.0;

// **Whether `in.colour` is a baked light rather than a tint** - 1.0 only for a
// Wipeout HD `.rcsmodel`, set from `Model::vertex_colour_is_light`. HD's
// fragment programs *add* the interpolated colour to the lightmap term before
// multiplying the albedo, so the stand-in path below must not multiply by it.
//
// A separate override from `linear_out` on purpose: that one is the *target's*
// colour space and this is the *model's* vertex semantics. Keying this off
// `linear_out` renders HD dark wherever it draws into a gamma target - the
// capture and viewer paths both do.
override colour_is_light: f32 = 0.0;
