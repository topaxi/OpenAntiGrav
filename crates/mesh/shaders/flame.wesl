// Engine flame surface overrides (HD's rim and scroll).

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

override flame_shading: f32 = 0.0;

override flame_rim_power: f32 = 0.0;

override flame_rim_scale: f32 = 0.0;

override flame_rim_min: f32 = 0.0;

override flame_alpha_scale: f32 = 0.0;

override flame_colour_scale: f32 = 0.0;

// `Speed`, the sixth number the material authors: what multiplies `scene.time`
// into the noise tap's `v`. 2.0 on all fourteen craft.
override flame_speed: f32 = 0.0;
