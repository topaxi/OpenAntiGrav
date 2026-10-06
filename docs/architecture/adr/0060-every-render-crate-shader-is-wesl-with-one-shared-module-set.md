# ADR-0060: Every render-crate shader is WESL, with one shared module set

## Status

Accepted 2026-10-06, on the maintainer's request ("port/dedupe the rest of the
wgsl files"). It supersedes [ADR-0059](0059-render-shaders-are-wesl-compiled-to-wgsl-at-build-time.md)'s
convention 5 ("a shader a single file holds comfortably, or that shares
nothing, stays WGSL") and its "applies to the two crates it was tried in".
Everything else in ADR-0059 stands.

## Context

After ADR-0059, `oag-post`'s other 11 shaders, `oag-fx`'s seven, `oag-render`'s
five, `oag-game`'s four, `oag-present`'s one and `oag-view`'s one were plain
WGSL, and the same helpers were typed again in each: the fullscreen-triangle
`vs_main` and its `VertexOutput` in seven post and present shaders, the
`linear_out` override and its `pow(c, 2.2)` decode in nine, the scene-space
`Uniforms`/`VertexInput` block in six, Rec. 709 `luma` in two and `bytes()` in two.

## Decision

1. **Each of those crates keeps its shaders as `<crate>/shaders/*.wesl`** and
   compiles them in `build.rs` through `oag_shader_check::link_each` (one
   artifact per module, named after it) or `link`. Nothing past `build.rs`
   knows WESL exists.
2. **Shared helpers live in `crates/shader-check/shaders/`** and every crate
   that calls `link` imports them as the external package `oag_shaders`:
   `import oag_shaders::fullscreen::{VertexOutput, fullscreen_triangle};`.
   `oag-shader-check` mounts that directory with a `wesl` `Router`, so it is a
   build-dependency's own files, not a new crate and not a runtime dependency.
   Items are `public`. Modules: `fullscreen`, `colour` (`luma_rec709`), `bytes`,
   `target` (`linear_out`, `gamma_to_target`), `scene_vertex`, `quad`.
3. **Only bodies equal in meaning are shared.** Kept separate on purpose:
   `screen.wgsl`'s Rec. 601 `luma`; `flash`'s unflipped triangle; `view`'s
   scaled one; `motion_blur`'s position-only `VertexOutput` (it uses
   `fullscreen_clip`); the `fs_blur`/`fs_composite`/`fs_copy` entry points,
   whose bodies differ and which WESL cannot import anyway (entry points live
   in the root module); `oag-fx`'s `exhaust`/`psys` `VertexOutput`s; FSR 1's and
   FSR 3's `min3`/`max3`, which `rcas.wesl` already argues are deliberately
   twin copies so each diffs against its own upstream file.
4. **The screen-filter presets stay plain WGSL** (`assets/shaders/screen/*.wgsl`
   and `oag-post`'s `screen.wgsl` prelude). `Preset::wgsl` concatenates prelude,
   generated `param_*` accessors and the body at runtime, and a user's own
   preset is read from the config directory the same way; `wesl` is a
   build-time tool, so an import would need a WESL compiler in the shipped game.
   Moving `to_linear`, `to_gamma` and `warp` into the prelude would make every
   preset, and any a user wrote, that defines its own copy fail as a
   redefinition. They stay validated by `Preset::validate` and `oag-game`'s
   screen tests, and the duplicated CRT helpers stay duplicated.
5. **Declaration order is made stable.** `wesl` visits imported modules through
   a `HashMap`, so one source printed its declarations in a different order each
   build. `oag-shader-check` now stable-sorts the declarations (root module
   first, then imported modules by path, each in its own source order) before
   writing, so two builds of an unchanged shader are `cmp`-equal. WGSL module
   scope is order-independent.

## Consequences

- The artifacts' text changed (order only, plus the shared items), so a build
  artifact is not comparable with one from before this ADR. Pixels are:
  90 headless captures across six titles are `cmp`-identical.
- A shared module is one edit for every crate. `oag-shader-check` emits
  `rerun-if-changed` for its `shaders/` directory, so editing one re-links all
  of them; `wesl` itself reports only the crate's own files.
- `ManglerKind::None` still makes names unique by hand across a crate's
  modules and the shared set. A shared name added later can collide with a
  crate's own and the collision is silent.
- `wesl`'s parser is stricter than `naga`'s: `a * b >> c` was accepted by `naga`
  and rejected by `wesl`, so `ps2_bloom` parenthesises it (same meaning).
- Not done: `lit_texel` (839 lines) is still one function; FSR 3's passes still
  repeat upstream's per-pass helpers (`load_dilated_motion_vector` x4, ...).
