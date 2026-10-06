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
- Closed 2026-10-06 (wesl-port): every other render-crate shader is WESL with a
  shared module set (ADR-0060), `wesl`'s declaration order is stable, and the
  screen-filter presets deliberately stay plain WGSL (runtime-concatenated).
- FSR 3's passes still repeat upstream's per-pass helpers
  (`load_dilated_motion_vector` x4, `spd_reduce4` x2, ...); folding them into
  `common.wesl` would need a bit-exactness check against the FSR 3 captures.

## Next Steps

1. Split `lit_texel` (above), with the capture harness.
2. Fold FSR 3's repeated per-pass helpers into `common.wesl`, with the captures.
