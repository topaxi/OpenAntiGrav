// The cutout alpha-test threshold and its pipeline override.

// **The default, for a draw whose file authors no reference of its own.**
//
// Every PSP and PS2 cutout batch now authors one:
// `oag_vex::vex::Batch::alpha_test_reference` reads the selector out of
// `pass_mask` and `header_flags`, `mesh::DrawCall::alpha_test_ref` carries it,
// and `mesh_render::cutout` builds one pipeline per distinct value - so a
// `.vex` draw overrides this constant rather than falling back to it. What
// still lands here is a PS3 chunk (Wipeout HD authors per *material* instead,
// through `Model::alpha_test_ref` below), a Vita submesh, and every synthetic
// draw this crate makes up.
//
// `0` rather than `0.5` for those, and that is measured rather than
// inherited. `0.5` was this project's own placeholder for the unrecovered
// case and it was wrong on real data: Wipeout Pure's `Speedup Pad` glow
// texture (`speedup_GLOW_KEY.tga`) decodes to an alpha histogram of exactly
// `{0: background, 57..58: glow interior}` - its brightest texel is ~0.23, so
// `0.5` discards the whole batch and the pad renders nothing.
//
// **The pad now gets that reference from its own file, and the value it asks
// for is `0`.** Its 349 batches are the one selector pattern that separates the
// recovered branch from its rival (`pass_mask & 0x80` set, `header_flags &
// 0x20` clear) - see `oag_vex::vex::Batch::alpha_test_reference`. So the pad no
// longer depends on this constant at all, and `pad_alpha_test_ground_truth`
// now guards the recovered value rather than the placeholder.
//
// The constant still stands for the paths that do reach it, on the same
// argument it always had - the most permissive of the recovered references, so
// a draw whose real reference is unknown keeps a few extra near-zero-alpha
// texels rather than this project hiding geometry. Under the `<=` below that
// is `0`, which discards exactly the fully transparent texel; it was written
// `1/255` while the comparison was `<`, and those two say the same thing about
// 8-bit alpha, so the paths that reach this constant keep the picture they
// had.
const ALPHA_TEST_THRESHOLD: f32 = 0.0;

// **Wipeout HD authors its own reference**, and this is where it arrives:
// a `Transparency::Mode2` material carries the `GL_GREATER`/`0.5` pair its
// `Material_ApplyRenderState` programs into `NV4097_SET_ALPHA_FUNC` and
// `SET_ALPHA_REF`, and `mesh_render::build` passes it here off
// `Model::alpha_test_ref`. The default is the constant above. Only
// `GL_GREATER` is reproduced: the comparison below discards at `<=`, and
// `mesh::rcs::cutout` reports a material asking for anything else instead of
// drawing it inverted.
//
// **`<=` and not `<`, and the difference is not cosmetic.** Both `GL_GREATER`
// here and the GE's `GU_ALPHA_GREATER` on the PSP/PS2 path keep what is
// *strictly* above the reference, so the discard is `<=`; `<` is `GEQUAL`.
// While Wipeout HD's `0.5` was the only reference in play the two were
// interchangeable - the alpha is 8-bit and `0.5` sits between `127/255` and
// `128/255`, so no texel could land on the boundary. Every reference recovered
// off a `.vex` batch lands on one exactly (`0`, `0x10`, `0x7f`), and at `0`
// the `<` form discards nothing at all: Pure's `Speedup Pad` painted its
// texture's fully transparent background as a solid plate, a square instead of
// a pad. See `crates/render/tests/pad_alpha_test_ground_truth.rs`.
override alpha_test_ref: f32 = ALPHA_TEST_THRESHOLD;
