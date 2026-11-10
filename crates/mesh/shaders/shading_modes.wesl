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
