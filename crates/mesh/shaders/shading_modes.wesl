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

// **Whether `in.colour` is a baked light rather than a tint** - 1.0 only for a
// Wipeout HD `.rcsmodel`, set from `Model::vertex_colour_is_light`. HD's
// fragment programs *add* the interpolated colour to the lightmap term before
// multiplying the albedo, so the stand-in path below must not multiply by it.
//
// A separate override from `linear_out` on purpose: that one is the *target's*
// colour space and this is the *model's* vertex semantics. Keying this off
// `linear_out` was tried and rendered HD dark wherever it draws into a gamma
// target - the capture and viewer paths both do.
override colour_is_light: f32 = 0.0;
