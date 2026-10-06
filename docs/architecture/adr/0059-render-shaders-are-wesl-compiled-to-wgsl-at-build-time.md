# ADR-0059: Render shaders may be WESL, compiled to plain WGSL at build time

## Status

Accepted, after a half-day trial on 2026-10-06 that the maintainer approved
as a trial and asked to be recorded either way. Applies to the two crates it
was tried in, `oag-mesh` and `oag-post`. Every other `.wgsl` file stays WGSL
until someone has a reason to move it; nothing here obliges that.

## Context

WGSL has no include and no module system. The renderer worked round that two
ways:

- `oag-post` built every FSR 3 pass as `format!("{COMMON}\n{pass}")`, and
  prepended a different two-line `INPUTS` string to `prepare_inputs` for the
  multisampled build.
- `oag-mesh` kept the whole mesh pipeline in one 2,103-line `mesh.wgsl`:
  structs, 25 bindings, 20 `override` pipeline constants, the vertex stage,
  an 828-line `lit_texel` and ten fragment entry points.

[WESL](https://wesl-lang.dev) (WGSL Extended) adds `import`, `@if`/`@else`
and visibility to WGSL and compiles to plain WGSL, so `wgpu` sees no new
format. The `wesl` crate (0.6.0 here) runs from a `build.rs`.

## Decision

A render-side crate **may** keep a shader as `.wesl` modules under
`<crate>/shaders/` and compile it in `build.rs` to `$OUT_DIR/<name>.wgsl`,
which Rust pulls in with `include_str!(concat!(env!("OUT_DIR"), ...))`.
Nothing past `build.rs` knows WESL exists. The trial moved:

- `oag-post`: FSR 3's `common` plus nine passes (`crates/post/shaders/fsr3/`).
  `format!` concatenation is gone. Each pass imports what it uses from
  `package::fsr3::common`. The one `@if` in the tree, `inputs.wesl`'s `msaa`,
  replaces the `INPUTS`/`INPUTS_MULTISAMPLED` strings; `build.rs` compiles
  `prepare_inputs` twice.
- `oag-mesh`: `mesh.wgsl` into 16 modules under `crates/mesh/shaders/`
  (`types`, `bindings`, `shadow`, `fog`, `zone`, `pad`, `flame`, `absorb`,
  `glow`, `alpha_test`, `shading_modes`, `texture_lod`, `spu_light`,
  `velocity`, `shade`, and the entry-point root `mesh`). The split moved
  whole top-level items and edited none; the 20 `override`s keep their names
  and defaults and moved with the item that reads them.

### Conventions for a new WESL shader

1. **`ManglerKind::None`, always.** Pipeline constants, entry points and
   bindings are looked up by name from Rust. The default mangler would
   rename an imported `override`. The price is that **a name must be unique
   across a crate's modules**, and a collision is silent, not an error.
2. **Entry points live in the root module**, because WESL strips what no
   entry point reaches. A module that only holds an `override` is kept
   because something it imports reaches it.
3. **Imports are explicit lists.** WESL has no wildcard import, so a pass
   names every item it uses (2 to 33 names in the FSR 3 passes). Write the
   list when you write the pass; the compiler will not suggest it.
4. **`@if` is for what `format!` used to do** (a type swap, a prelude
   variant). It is **not** used to replace an `override`; that is a separate
   decision with its own pixels to compare, not part of this one.
5. A shader a single file holds comfortably, or that shares nothing, stays
   WGSL. Do not convert for the sake of it.

## Evidence

All of it from `wesl-trial` against `main` at `1fa02252b`, a debug build,
Pulse PSP/PS2, Pure, HD, 2048 and Omega.

- **Pixels.** 18 headless captures at 640x360 (race frame and a menu page on
  each of six titles; Pulse PSP and HD under `--reconstruction fsr3`,
  `fsr3 --msaa 4x` and `smaa`, `--presented`) are **byte-identical** (`cmp`)
  between `main` and the branch. The capture tool is deterministic: two runs
  of the same binary were also `cmp`-equal. The MSAA frames differ from the
  non-MSAA ones, so the `@if(msaa)` artifact is drawn, not just built.
- **Per-item.** Comments and whitespace stripped, and float and hex literals
  compared by value, each top-level item of `mesh.wgsl` equals its item in
  the compiled output: 89 of 89, none dropped, none added, and the
  `@group/@binding` set and the 20 override names are unchanged. For FSR 3,
  each compiled pass equals the old `common + pass` text item by item, with
  the unreached `common` items stripped (47 to 82 per pass) and none changed.
  WESL reprints literals (`1.0e-6` to `1e-6`, `6.10e-05` to `6.1e-5`,
  `0x7ef19fffu` to `2129764351u`); every one is the same value. There is no
  `const_assert` in either crate, which stripping could drop.
- **Size.** `mesh.wgsl` as `wgpu` sees it went from 115,592 bytes to 27,199
  (comments gone). Parse plus validate in `naga`, release, median of 30: 1.94
  ms to 1.75 ms, at a load average of about 9.7, against a race scene build
  of 195 to 372 ms. The shader is not what that build spends its time on, so
  no measurable change in race-load time is expected and the windowed
  `--measure-race-load` run was not repeated.
- **Build cost.** Twenty crates join the lockfile, all build-time only
  (`wesl`, `wgsl-parse`, `wgsl-types`, `logos`, `lexical-*` and others).
  `cargo build -p oag-mesh -p oag-post` from a clean target directory: 25.4
  s without, 40.6 s with, at a load average of 9 to 13 (contended, and one
  sample each). An edit to one shader file and `cargo build -p oag-mesh`:
  1.8 to 3.1 s before, 1.6 s after, so an incremental build does not notice
  it. `just check-deps`, `check-unused-deps` and `clippy --workspace
  --all-targets -D warnings` pass.
- **Developer experience: the bad part.** A deliberate error in `fog.wesl`,
  five ways:

  | Error | Caught where | What the message points at |
  | --- | --- | --- |
  | Syntax (missing `;`) | `build.rs`, build fails | `shaders/fog.wesl:13:5` with a source excerpt and `in package::fog`: the best case |
  | Undeclared variable | **not at build**; `create_shader_module` panics at race start | `wgsl:633:55` in the **generated** text, with the excerpt |
  | Type mismatch (`max(f32, 1u)`) | same, by `naga` | `wgsl:379:16`, generated text, excerpt and an explanation |
  | Unknown field (`.nearr`) | same | same |
  | Misspelt import name | **not at all by WESL**; surfaces as the undeclared variable above | the use, not the import |

  So only syntax errors get a module and line. Every semantic error comes at
  runtime from `naga` with a line number into text that no longer exists in
  the tree, and the module is not named, only the excerpt. With one 2,103-line
  file before, the same error pointed straight at `mesh.wgsl:N`. Turning on
  `wesl`'s `eval` feature (`validate`) did not catch the undeclared variable
  either, and added seven crates and a minute of build time, so it was
  dropped.
- **What WESL did not split.** `lit_texel` is one 828-line function and WESL
  modules hold whole items, so `shade.wesl` is still 839 lines. Making the
  glow, flame and absorb blocks into functions is a logic-neutral refactor
  but a refactor, and it was not part of a no-logic-change trial.

## Consequences

- FSR 3's concatenation hack, and its `INPUTS` strings, are gone, and the
  mesh pipeline can be read in sixteen files instead of one. That is the gain.
- A shader author now writes an import list per pass and keeps names unique
  by hand.
- **Semantic shader errors are worse to read than before**, as the table
  shows. The mitigation not done here is a `naga` parse and validate of each
  artifact inside `build.rs`, which would turn the second to fourth rows into
  build-time failures at the generated line, and a map from that line back to
  its module. That is the first follow-up, and it adds `naga` as a build
  dependency.
- Twenty extra build-time crates and about 15 s on a clean build of these two
  crates. Neither crate was measured on a release build or on an idle machine.
- `crates/mesh/shaders/mesh.wesl` is only the root. Older comments and docs
  that say `mesh.wgsl` name a file that no longer exists; those inside
  `oag-mesh` and `oag-post` are retargeted to the module holding the item and
  the rest are left as history.
- Reversible: the compiled output is plain WGSL and the old files are in
  history.
