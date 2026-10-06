# WESL shaders: semantic errors point at generated text, not at the module

2026-10-06. [ADR-0059](../../docs/architecture/adr/0059-render-shaders-are-wesl-compiled-to-wgsl-at-build-time.md)
adopted WESL for `oag-mesh` (16 modules) and `oag-post` (FSR 3). The trial's
one real cost: only a syntax error is caught at build, with module and line.
An undeclared name or a type mismatch surfaces at `create_shader_module`, at
race start, as `wgsl:633:55` into `$OUT_DIR/mesh.wgsl`, without the module
named. `wesl`'s `eval`/`validate` did not catch it either.

## Open

- **Validate each artifact with `naga` inside `build.rs`** (parse, then
  validate) so the failure is a build error, and map the generated line back to
  its module. Adds `naga` as a build dependency of both crates; check what it
  does to a clean build, which was 25 s to 41 s without it.
- **`lit_texel` is one 828-line function**, so `shade.wesl` is 839 lines. The
  glow, flame and absorb blocks inside it could become functions in their own
  modules. That is a logic-neutral refactor that needs the 18-capture `cmp`
  harness (`capture.sh` in the trial's scratch) run before and after.
- Other `.wgsl` files with a shared prelude are candidates: `screen.wgsl`'s
  `PRELUDE` is concatenated in Rust today (`oag-post/src/screen.rs`).
- Comments elsewhere that say `mesh.wgsl` name a file that no longer exists.

## Next Steps

1. Add the `naga` check to `crates/mesh/build.rs` and `crates/post/build.rs`;
   re-run the five-error table in the ADR and confirm rows two to four fail the
   build.
