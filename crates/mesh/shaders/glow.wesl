// The bloom glow-mask stamp: PSP constant and PS2 per-texel rules.

import package::alpha_test::{alpha_test_ref};
import package::types::{VertexOutput};

// **Whether this model stamps the bloom's glow mask** - 1.0 only for a
// drawable built with `mesh_render::GlowMask::Stamped`. The original writes
// that channel through the GE stencil, never through the blend, so an opaque
// surface stamps a constant the batch names rather than its own alpha - see
// `oag_mesh::mesh::glow` and docs/rendering/glow-mask.md.
override glow_stamp: f32 = 0.0;

// **Whether that stamp is the PS2's rule** - 1.0 only for a drawable built with
// `mesh_render::GlowMask::StampedByTexel`. The PS2 writes the fragment's own
// alpha (texel times vertex colour) from a batch with the glow bits and nothing
// from any other, where the PSP writes a constant the batch names. See
// `GlowMask::StampedByTexel` and docs/rendering/ps2-bloom.md.
override glow_texel: f32 = 0.0;

// **Whether the mask is what the material's own program writes** - 1.0 only for
// a drawable built with `mesh_render::GlowMask::StampedByProgram`, Wipeout HD's
// rule. The per-vertex `glow` carries the program's alpha lane: `>= 0` is a
// constant the program writes, `< 0` is the picture's alpha times `-glow` (the
// program multiplies a texture's alpha channel by a constant). See
// `oag_rcs::rcsmaterial::fragment::Program::output_alpha` and
// docs/rendering/hd-glow-mask.md.
override glow_program: f32 = 0.0;

// The samples a texel-sourced program mask reads, written by `lit_texel` as it
// fetches them: the first texture, the second (scrolled where the material adds
// it) and the pad's third binding. Zero wherever the draw does not fetch one.
var<private> glow_first: vec4<f32>;
var<private> glow_second: vec4<f32>;
var<private> glow_pad: vec4<f32>;

// The mask `GpuVertex::glow` names under `glow_program`: `>= 0` is a constant
// the program writes; `< 0` is a texel's channel times a constant, packed
// `-(scale + 8 * (4 * source + channel))` by `mesh::rcs::glow::texel_glow`,
// `source` 0 the coverage alpha, 1 the second texture, 2 the first, 3 the pad's.
fn program_glow(g: f32, coverage: f32) -> f32 {
    if g >= 0.0 {
        return g;
    }
    let m = -g;
    let k = u32(floor(m / 8.0));
    let scale = m - f32(k) * 8.0;
    let channel = k & 3u;
    var value = coverage;
    switch k >> 2u {
        case 1u: { value = glow_second[channel]; }
        case 2u: { value = glow_first[channel]; }
        case 3u: { value = glow_pad[channel]; }
        default: {}
    }
    return clamp(scale * value, 0.0, 1.0);
}

// `slots::GLOW_BATCH`: the batch carries the glow bits.
const SLOT_GLOW_BATCH: u32 = 8192u;

// What this fragment stamps, before any alpha test: the batch's constant on the
// PSP's rule, the fragment's own alpha on a glow batch on the PS2's.
fn stamp_value(in: VertexOutput, shaded_alpha: f32) -> f32 {
    let ps2 = select(0.0, shaded_alpha, (in.slots & SLOT_GLOW_BATCH) != 0u);
    return mix(mix(in.glow, ps2, glow_texel), program_glow(in.glow, shaded_alpha), glow_program);
}

// The opaque and cutout pipelines' alpha: `1.0` as it always was, or the
// batch's stamp where the model stamps.
fn stamped_alpha(in: VertexOutput, shaded_alpha: f32) -> f32 {
    return mix(1.0, stamp_value(in, shaded_alpha), glow_stamp);
}

// **The glow mask's stamp for a blended batch** - drawn after the batch's own
// blended draw, through a pipeline that masks colour off and writes alpha only,
// so it reaches the target as the one thing the blend cannot do: replace the
// mask with a constant. The original does this in the same draw with the GE
// stencil (`ALWAYS`, ref = the texture's glow byte, `REPLACE` on depth pass)
// left on under the blend, and the test below is its alpha test
// (`GU_GREATER`, the batch's own reference, on texel alpha times vertex
// alpha - the value the blend reads). A batch without the glow bits carries
// `glow == 0` and never stamps: the original disables the stencil test for it,
// which keeps the mask. See `docs/rendering/glow-mask.md`.
//
// **The additive class also tests colour** - `Gfx_BuildBatchStateList`'s
// `0x200` branch turns the GE colour test on, `NOTEQUAL` against black, and a
// GE dump of Outpost 7 shows it on every additive glow draw and off on the
// alpha-over ones - so a black texel of an additive quad stamps nothing.
// `stamp_colour_test` is that branch's constant, set on the pipeline pair
// that draws the additive batches. Whether the GE tests before or after the
// fog is unread; this tests the lit texel, before the fog.
override stamp_colour_test: f32 = 0.0;

fn stamp_discards(shaded: vec4<f32>, glow: f32) -> bool {
    let black = max(shaded.r, max(shaded.g, shaded.b)) < 0.5 / 255.0;
    return shaded.a <= alpha_test_ref || glow <= 0.0 || (stamp_colour_test > 0.5 && black);
}
