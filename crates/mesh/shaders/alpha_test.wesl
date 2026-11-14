// The cutout alpha-test threshold and its pipeline override.

// **The default, for a draw whose file authors no reference of its own.**
//
// Every PSP and PS2 cutout batch authors one:
// `oag_vex::vex::Batch::alpha_test_reference` reads the selector out of
// `pass_mask` and `header_flags`, `mesh::DrawCall::alpha_test_ref` carries it,
// and `mesh_render::cutout` builds one pipeline per distinct value - so a
// `.vex` draw overrides this constant rather than falling back to it. What
// still lands here is a PS3 chunk (Wipeout HD authors per *material* instead,
// through `Model::alpha_test_ref` below), a Vita submesh, and every synthetic
// draw this crate makes up.
//
// `0`, the most permissive of the recovered references: a draw whose real
// reference is unknown keeps a few extra near-zero-alpha texels rather than
// hiding geometry. Under the `<=` below it discards exactly the fully
// transparent texel. A mid-range default such as `0.5` would erase whole
// batches: Wipeout Pure's `Speedup Pad` glow peaks at alpha ~0.23 and its
// batches ask for `0` (`docs/rendering/pads.md`, `docs/formats/vex.md`).
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
// Wipeout HD's `0.5` sits between `127/255` and `128/255`, where the two agree,
// but every reference off a `.vex` batch lands on an 8-bit value exactly (`0`,
// `0x10`, `0x7f`), and at `0` the `<` form discards nothing: Pure's `Speedup
// Pad` would paint its transparent background as a solid square. See
// `crates/render/tests/pad_alpha_test_ground_truth.rs`.
override alpha_test_ref: f32 = ALPHA_TEST_THRESHOLD;
