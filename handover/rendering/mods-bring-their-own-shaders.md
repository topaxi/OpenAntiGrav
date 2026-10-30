---
status: parked
titles: [all]
updated: 2026-10-30
summary: "Mod-supplied WGSL at fixed render hooks (post-process, craft overlay, particle fragment), naga-validated; after mod-shipped particle effects"
categories: [tooling]
size: XL (30)
difficulty: hard (40)
---

# Mods bring their own shaders

**Maintainer, 2026-10-30:** mods with custom shaders and effects "might be neat". The
first stage, mod-shipped particle effects played by `oag_fx::psys`, is queued as its own
lane on `wasm-extensions`. This thread is the second stage, parked until the first has
landed.

Today a module can only fire the title's own effects: `fire-effect{slot, effect}` takes
`Effect::CollisionSpark` or `Effect::HitSpark`, the disc's `.pob`, drawn at the nozzle
(`docs/architecture/wasm-extensions/README.md`).

## The shape (a sketch, not decided)

- **Fixed hooks only**, each with a fixed bind-group interface the engine owns: a
  screen post-process pass (frame in, frame out), a craft overlay, perhaps a particle
  fragment. No vertex stage, no access to engine buffers beyond the hook's inputs
  (time, resolution, the frame, up to N mod parameters).
- **WGSL, validated at load** with `naga` (as `oag-shader-check` does at build time);
  a shader that fails is refused by name in the loader report, and the hook draws as
  vanilla. The web build runs WebGPU, so WGSL works there unchanged.
- **Parameters by command**: modules re-instantiate each tick, so a shader's inputs
  come from an output command (`set-shader-param{hook, index, value}`), never hashed.
- **Off by default** behind a setting: a mod shader can be slow or hang the GPU, and
  the Steam Deck and phones have tight budgets.

## Open

- Which hooks are worth having first (post-process is the cheapest to bound).
- A per-hook GPU time budget, and what happens past it (disable the hook for the race?).
- Whether a shader may sample a mod texture, and how those are declared in the manifest.
- How the `original` setting value and compare shots exclude mod shaders.

## Next Steps

1. Wait for the mod-shipped particle effects lane to land; reuse its manifest table
   for declaring shaders.
2. Prototype the post-process hook alone: one naga-validated WGSL file from a mod,
   one parameter, a frame-time measurement on the Deck before and after.
