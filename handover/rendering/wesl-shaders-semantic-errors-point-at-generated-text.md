# WESL shaders: follow-ups after build-time validation

2026-10-06. [ADR-0059](../../docs/architecture/adr/0059-render-shaders-are-wesl-compiled-to-wgsl-at-build-time.md)
adopted WESL for `oag-mesh` (16 modules) and `oag-post` (FSR 3). The trial's
one real cost was that only a syntax error was caught at build. **Closed
2026-10-06:** `oag-shader-check` (a build-dependency) validates every linked
artifact and every plain `.wgsl` with `naga` in six crates' `build.rs`, and a
failure names the module, the declaration and its line - see
`docs/architecture/workspace-layout.md`, "Shader validation at build time".

## Open

- **`lit_texel` is one 828-line function**, so `shade.wesl` is 839 lines. The
  glow, flame and absorb blocks inside it could become functions in their own
  modules. That is a logic-neutral refactor that needs the 18-capture `cmp`
  harness (`capture.sh` in the trial's scratch) run before and after.
- **`wesl` prints declarations in a different order on each build** (a hash map
  inside the linker): two builds of the same tree give `mesh.wgsl` files that
  are line-for-line equal once sorted but not byte-equal. Captures stay
  byte-identical, so it is cosmetic, but a diff of two `$OUT_DIR` artifacts is
  noisy and a reproducible build would want it sorted.
- Other `.wgsl` files with a shared prelude are candidates: `screen.wgsl`'s
  `PRELUDE` is concatenated in Rust today (`oag-post/src/screen.rs`), so it and
  the five `assets/shaders/screen/*.wgsl` presets are validated by tests, not by
  the build. Porting the prelude to a WESL import would bring them in.
- Comments elsewhere that say `mesh.wgsl` name a file that no longer exists.

## Next Steps

1. Split `lit_texel` (above), with the capture harness.
2. Port the screen-filter prelude to a WESL import so the build validates
   presets too.
