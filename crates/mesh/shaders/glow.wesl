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

// `slots::GLOW_BATCH`: the batch carries the glow bits.
const SLOT_GLOW_BATCH: u32 = 8192u;

// What this fragment stamps, before any alpha test: the batch's constant on the
// PSP's rule, the fragment's own alpha on a glow batch on the PS2's.
fn stamp_value(in: VertexOutput, shaded_alpha: f32) -> f32 {
    let ps2 = select(0.0, shaded_alpha, (in.slots & SLOT_GLOW_BATCH) != 0u);
    return mix(in.glow, ps2, glow_texel);
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
