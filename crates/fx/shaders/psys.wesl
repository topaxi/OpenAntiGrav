// Draws `.pob` particles - collision sparks and every other authored effect.
//
// Structurally a smaller copy of exhaust.wesl - see that file's own header
// for why the model matrix in the shared uniform block is unused (the
// vertices already arrive in world space).
//
// Two ways to shape a particle, chosen per vertex by `normal.x`:
//
// - **The emitter's own sprite** (`normal.x == 1`), sampled off the sheet
//   `psys::sprite` packs every PSP `.pob`'s embedded sprites into, and
//   modulated the way the GE modulates it: `GU_TFX_MODULATE` with
//   `GU_TCC_RGBA`, set once by `Gfx_Init` and never changed on the particle
//   path - texel times particle colour, alpha included. See
//   `psys/sprite.rs`'s module doc.
// - **A class-7 capped streak's sprite** (`normal.x == 2`), one quad whose
//   `texcoord.y` is the along-coordinate `0..1`; the fragment folds it into
//   `ParticleSystem_DrawCappedStreak`'s `v` - `0 -> 0.5` over the first cap,
//   `0.5` along the body, `0.5 -> 1` over the last - and into the cell's
//   `v` range, carried in `normal.yz`. `lit` is each cap's share of the
//   length. See `psys/streak.rs`.
// - **The procedural profile** (`normal.x == 0`), for every emitter with no
//   sprite to sample (a PS2 or HD source, a non-Pulse streak). It models the one
//   sprite measured before the pixels were located, the collision spark's
//   `orange_glow2.tga`: the original stretches a single row of that radial
//   glow along the streak's body, with the sprite's halves forming the two
//   end caps - so the body keeps a constant-width bright core however long
//   the streak grows. The vertex stream's `lit` slot carries the caps'
//   share of the half-length; `0.5` collapses the profile to the plain
//   radial falloff a round billboard wants.

import oag_shaders::scene_vertex::{Uniforms, VertexInput};
import oag_shaders::target::{linear_out, gamma_to_target};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var sheet: texture_2d<f32>;
@group(0) @binding(2) var sheet_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) colour: vec4<f32>,
    @location(1) texcoord: vec2<f32>,
    @location(2) cap: f32,
    @location(3) sampled: f32,
    @location(4) cell_v: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip = uniforms.view_projection * vec4<f32>(in.position, 1.0);
    out.colour = in.colour;
    out.texcoord = in.texcoord;
    out.cap = in.lit;
    out.sampled = in.normal.x;
    out.cell_v = in.normal.yz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // `u` runs along the streak, `v` across it. Inside the two caps (the
    // first and last `cap` of the u range) the along-axis term ramps 0 -> 1
    // toward the tip; over the body it is 0, so only the cross-axis
    // profile remains and the core stays a constant-width line however
    // long the quad is. For a billboard `cap` is 0.5 - the caps meet in
    // the middle and the expression reduces exactly to the radial
    // `distance(uv, centre) * 2` this shader used for everything before.
    let tip = min(in.texcoord.x, 1.0 - in.texcoord.x);
    let along = clamp((in.cap - tip) / max(in.cap, 1e-4), 0.0, 1.0);
    let across = abs(in.texcoord.y * 2.0 - 1.0);
    let d = length(vec2<f32>(along, across));
    let shape = clamp(1.0 - d, 0.0, 1.0);
    // The profile is measured off the shipped sprite itself, radially
    // averaged from a PPSSPP texture dump of the 64x64 glow the collision
    // effect binds (orange_glow2.tga - identified live: every psys draw in
    // a crash reads the same texture address, and the dump's pixels carry
    // the authoring-path's promise). Measured: alpha falls essentially
    // linearly (fit (1-r)^0.92, a touch faster mid-range, hence 1.2), and
    // the texel colour dims linearly with radius while keeping its
    // saturated hue. Splitting the falloff between rgb and alpha the same
    // way the sprite does is what keeps the bright part of a spark small -
    // a flat-rgb quad reads twice as wide at the same alpha curve, which
    // was the reported "too big, not line-like" look.
    //
    // **The colour is the palette's, never whitened.** This used to carry a
    // `mix(rgb, white, pow(shape, 8.0) * 0.85)` term, from the same dump's
    // innermost ~15 % reading. It was fitted on the collision sparks, whose
    // quads are all under a unit across, and it is a fixed *fraction* of
    // whatever quad it lands on - so on a rocket explosion's 30-to-100-unit
    // billboards the same fraction manufactured a 15-unit disc of pure
    // white. Held against the original in PPSSPP (`data/reference/`, taken
    // with `scripts/psp-fire-weapon.py`), that is plainly wrong: that
    // track-hit fireball stays amber to its hottest point and the scene
    // behind reads through it. The emitter's 256-entry table is the colour
    // authority, so inventing a white here was overriding the asset.
    //
    // **A white core is not always wrong, though.** A craft-hit Rocket at 51
    // units does peak white-hot for a few frames (measured 2026-09-24, see
    // `rocket-visuals.md`), and that white comes out of the asset: a
    // near-white colour table times the emitter's own soft white sprite.
    // Judge a white against a matched capture, not against this comment.
    // Removing it barely moves the sparks - `bits` is white in its own
    // palette either way.
    let procedural = vec4<f32>(in.colour.rgb * shape, pow(shape, 1.2) * in.colour.a);
    // Sampled unconditionally - a texture sample must sit in uniform control
    // flow - and selected afterwards.
    // Each cap covers `in.cap` of the length (at most half), so the two
    // ramps never overlap and the body between them sits at 0.5.
    let s = in.texcoord.y;
    let f = max(in.cap, 1e-4);
    let capped_v = 0.5 * clamp(s / f, 0.0, 1.0) + 0.5 * clamp((s - (1.0 - f)) / f, 0.0, 1.0);
    let folded = vec2<f32>(in.texcoord.x, mix(in.cell_v.x, in.cell_v.y, capped_v));
    let uv = select(in.texcoord, folded, in.sampled > 1.5);
    let texel = textureSample(sheet, sheet_sampler, uv);
    let sprite = in.colour * texel;
    let picked = select(procedural, sprite, in.sampled > 0.5);
    let rgb = picked.rgb;
    // On the linear float target the palette's gamma-authored colour is
    // decoded so the blend and the encode after it round-trip; on a gamma
    // target this is the identity. See `mesh_render::is_linear_target`.
    let out_rgb = gamma_to_target(rgb, linear_out);
    return vec4<f32>(out_rgb, picked.a);
}
