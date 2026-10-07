// Draws a decoded .vex model.
//
// Lighting is a fixed two-light rig rather than the game's own: the point here
// is to see the geometry clearly and spot decoding errors, not to reproduce
// Pulse's look. A single light leaves faces pointing away from it unreadably
// black.


import package::alpha_test::{alpha_test_ref};
import package::bindings::{anims, emissives, node_anims, scene, uniforms};
import package::fog::{fogged};
import package::glow::{stamp_discards, stamp_value, stamped_alpha};
import package::shade::{lit_texel};
import package::shadow::{shadowed};
import package::spu_light::{spu_light_sum};
import package::types::{MrtOutput, VertexInput, VertexOutput};
import package::velocity::{velocity_of};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // The `Anim Transform` above this vertex, if any. Slot 0 is the identity,
    // so static geometry passes through unchanged. Applied *before* the model
    // matrix because it is part of the file's own scene-graph chain: the
    // vertex is baked in the anim node's space, and this is the matrix that
    // takes it back to the model's. See `oag_formats::vex::anim_anchors`.
    let node = node_anims.transform[in.xform];
    let placed = node * vec4<f32>(in.position, 1.0);
    let world = uniforms.model * placed;
    out.clip = uniforms.view_projection * world;
    // **The normal goes through the node matrix's inverse transpose**, not the
    // matrix itself: a per-axis scale skews a normal turned by the plain
    // matrix. The model matrix is uniform-scale, so it needs none. The cofactor
    // matrix (columns `c1 x c2`, `c2 x c0`, `c0 x c1`) is the inverse transpose
    // times the determinant, so only its sign is kept - a mirroring node must
    // not flip the normal - and the fragment stage renormalises. Identity for
    // slot 0 (and for a rotation or a uniform scale it is the matrix's own
    // direction), so static geometry and every uniformly-scaled node draw as
    // they did. Wipeout 2048 is the title that needed it: 51 moving, lit
    // meshes across its fourteen race circuits sit under a non-uniformly
    // scaled skeleton node. `tests/moving_normal_inverse_transpose.rs` is the
    // pixel check against the baked path.
    let c0 = node[0].xyz;
    let c1 = node[1].xyz;
    let c2 = node[2].xyz;
    let handed = select(-1.0, 1.0, dot(c0, cross(c1, c2)) >= 0.0);
    let cofactor = mat3x3<f32>(cross(c1, c2), cross(c2, c0), cross(c0, c1));
    let turned = (cofactor * in.normal) * handed;
    out.normal = (uniforms.model * vec4<f32>(turned, 0.0)).xyz;
    out.colour = in.colour;
    out.lit = in.lit;
    out.glow = in.glow;
    // **Pulse's hull, lit the way the GE lights it**: per vertex, the circuit's
    // own ambient plus each directional light's `N . L`, clamped to `0..1` and
    // carried to the fragment as the vertex colour the texel is modulated by.
    // The material is white, so nothing else multiplies it. It replaces both
    // the stand-in rig and the grey an uncoloured vertex otherwise carries,
    // which is why `lit` is cleared. See `mesh_render::HullLights` and
    // docs/ghidra/functions/psp-pulse-usa/scene-light.md.
    if scene.light.hull_ambient.w > 0.5 && in.lit > 0.5 {
        let n = normalize(out.normal);
        var sum = scene.light.hull_ambient.rgb;
        for (var i = 0u; i < 4u; i = i + 1u) {
            let l = scene.light.hull_direction[i];
            sum = sum + scene.light.hull_diffuse[i].rgb * max(dot(n, l.xyz), 0.0) * l.w;
        }
        out.colour = vec4<f32>(clamp(sum, vec3<f32>(0.0), vec3<f32>(1.0)), in.colour.a);
        out.lit = 0.0;
    }
    // Not run through the texture-animation transform below: an animated
    // surface scrolls its diffuse across itself, while its patch in the
    // circuit's lightmap atlas stays where the bake put it.
    out.lightmap_texcoord = vec4<f32>(in.lightmap_texcoord, in.texcoord2);
    // `uv * scale + offset`, the GE's own `TEXSCALE`/`TEXOFFSET` arithmetic -
    // the four words `TexAnim_CompileTransformList` (`0x08927358`) writes into
    // each animated material's display list. The direction is the authored
    // sign and is not negated here: a ship's blink lights author `v` running
    // 0 to -1, which is the backward sweep through the palette's rows that a
    // real capture confirmed, and the arrows on `16_Track` author `v` running
    // 0 to +1, which runs the other way on purpose.
    let anim = anims.transform[in.anim];
    out.texcoord = in.texcoord * anim.xy + anim.zw;
    out.world = world.xyz;
    out.view_depth = out.clip.w;
    out.sun_mask = in.sun_mask;
    out.slots = in.slots;
    out.specular_exponent = in.specular_exponent;
    out.cur_clip = out.clip;
    out.prev_clip = uniforms.prev_mvp * placed;
    out.spu_light = spu_light_sum(out.world, out.normal);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = lit_texel(in);
    // This pipeline is opaque (`blend: None`), so the alpha channel is never
    // blended with - but it is still written to whatever render target is
    // bound, so it stays a hardcoded 1.0 here rather than the texture's and
    // vertex colour's combined alpha, matching every prior opaque render
    // exactly. `fs_main_blend` below is the one that actually reads it.
    return vec4<f32>(
        fogged(shadowed(shaded.rgb, in.world), in.world, in.view_depth),
        stamped_alpha(in, shaded.a)
    );
}

// Used only by the blended pipeline - see
// `oag_mesh::mesh::Model::transparent_draws`. Identical to `fs_main` except
// it outputs the texture's alpha times the vertex colour's alpha instead of a
// hardcoded 1.0, which is what lets the blend state built around this entry
// point actually blend rather than replace.
@fragment
fn fs_main_blend(in: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = lit_texel(in);
    // **A screen-grab refraction** (`slots::REFRACTION`, `mesh::rcs::refraction`):
    // out = fog(C + grab * W), as two blended passes. The grab pass outputs
    // `f * W` for `dst = dst * src` - fog's `f` is `fogged(1) - fogged(0)`.
    // The other pass outputs the lit colour `C`, scaled by the material's own
    // colour multiplier, for `dst = dst + src`.
    if (in.slots & 1048576u) != 0u {
        let refraction_slot = in.slots >> 23u;
        let tint = emissives.tint_offset[refraction_slot];
        let weight = emissives.scale[refraction_slot];
        if (in.slots & 2097152u) != 0u {
            let alpha_weight = select(1.0, shaded.a, weight.y > 0.5);
            let w = tint.rgb * weight.x * alpha_weight;
            let f = fogged(w, in.world, in.view_depth) - fogged(vec3<f32>(0.0), in.world, in.view_depth);
            return vec4<f32>(f, 0.0);
        }
        return vec4<f32>(
            fogged(shadowed(shaded.rgb * tint.w, in.world), in.world, in.view_depth),
            0.0,
        );
    }
    return vec4<f32>(
        fogged(shadowed(shaded.rgb, in.world), in.world, in.view_depth),
        shaded.a
    );
}

// Used only by the cutout pipeline - see
// `oag_mesh::mesh::Model::alpha_tested_draws`. Unlike `fs_main_blend`, this
// keeps depth write on (see `mesh_render::build`): a batch tagged
// `is_alpha_tested()` is meant to be treated as opaque wherever its texel
// clears the threshold, and fully absent everywhere else, not smoothly
// blended - `discard` is what lets the pixels below the threshold contribute
// neither colour nor depth, so surfaces behind a cutout's "empty" corners
// still show through and still get occluded correctly by whatever the
// cutout's solid pixels do draw. `shaded.a` is now the texture's alpha times
// the vertex colour's, per `lit_texel` above; measured directly against every
// alpha-tested batch on `01_Track`/`16_Track` that no vertex reference in
// this path carries baked alpha at or below `ALPHA_TEST_THRESHOLD`, so this
// fold-in does not newly discard any fragment that used to pass on real
// track data.
@fragment
fn fs_main_alpha_test(in: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = lit_texel(in);
    if shaded.a <= alpha_test_ref {
        discard;
    }
    return vec4<f32>(
        fogged(shadowed(shaded.rgb, in.world), in.world, in.view_depth),
        stamped_alpha(in, shaded.a)
    );
}

@fragment
fn fs_main_stamp(in: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = lit_texel(in);
    let glow = stamp_value(in, shaded.a);
    if stamp_discards(shaded, glow) {
        discard;
    }
    return vec4<f32>(0.0, 0.0, 0.0, glow);
}

@fragment
fn fs_main_stamp_velocity(in: VertexOutput) -> MrtOutput {
    let shaded = lit_texel(in);
    let glow = stamp_value(in, shaded.a);
    if stamp_discards(shaded, glow) {
        discard;
    }
    return MrtOutput(vec4<f32>(0.0, 0.0, 0.0, glow), velocity_of(in));
}

// The velocity-writing twins of `fs_main` and `fs_main_alpha_test`, for the
// pipelines built against the race's two attachments (`mesh_render::Velocity`).
// The blended pipelines have no twin on purpose: they write no depth, so the
// velocity at their pixels belongs to the surface behind them - their second
// target carries an empty write mask instead, keeping velocity and depth
// describing the same surface at every pixel.
@fragment
fn fs_main_velocity(in: VertexOutput) -> MrtOutput {
    let shaded = lit_texel(in);
    return MrtOutput(
        vec4<f32>(
            fogged(shadowed(shaded.rgb, in.world), in.world, in.view_depth),
            stamped_alpha(in, shaded.a)
        ),
        velocity_of(in),
    );
}

@fragment
fn fs_main_alpha_test_velocity(in: VertexOutput) -> MrtOutput {
    let shaded = lit_texel(in);
    if shaded.a <= alpha_test_ref {
        discard;
    }
    return MrtOutput(
        vec4<f32>(
            fogged(shadowed(shaded.rgb, in.world), in.world, in.view_depth),
            stamped_alpha(in, shaded.a)
        ),
        velocity_of(in),
    );
}

// The depth prepass's fragment stage - `mesh_render::prepass`. Every target's
// write mask is empty, so what this returns reaches nothing; it exists only
// because a pass with colour attachments wants a fragment stage that names
// them.
@fragment
fn fs_depth_only() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0);
}

@fragment
fn fs_depth_only_velocity() -> MrtOutput {
    return MrtOutput(vec4<f32>(0.0), vec2<f32>(0.0));
}
