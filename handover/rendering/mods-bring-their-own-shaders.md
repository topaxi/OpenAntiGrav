---
status: ready-to-wire
titles: [all]
updated: 2026-11-16
summary: "Mod shaders landed (mod-shaders.md); open: device cost, compare/original exclusion, bounded loops, overdraw"
categories: [tooling]
size: M (40)
difficulty: medium (40)
---

# Mods bring their own shaders: what is left

The feature landed: a mod ships fragment shaders (`.wgsl` or `.wesl`) that the engine draws on the surfaces a
mod can name, with up to four parameters, the mod's own texture, the engine's per-view picture and a static work
bound that refuses a shader over `check::MAX_WORK` (32,768) by name. Everything is an output, never hashed.
Layout, the ABI and the proofs: `docs/architecture/wasm-extensions/mod-shaders.md`; the examples are `oag.tether`
and `oag.slipstream`.

## Open

1. **Device cost.** The bound is per pixel and counts a texture read like an add; nothing measures a shader on the
   device, so a shader at the bound over a 4K fill can stall a weak GPU (the doc's "Not covered"). Wanted: a
   per-hook GPU time measurement (the perf probe's timestamps) and a rule past it (drop the shader for the race,
   with a notice).
2. **The `original` setting and compare shots.** Not decided whether either excludes mod shaders; a compare shot
   should render the original's look (no mod draws), the same as it hides our menu additions.
3. **Bounded loops are refused.** A `for` to a constant would be safe; recognising the counter in naga's IR and
   folding the bounds is left open.
4. **Overdraw** (many particles of a large quad each running the shader): only the live-cap limits it.

## Next Steps

1. Item 2 first (small, a decision plus a flag in the compare path).
2. Item 1 on the Deck: one heavy shader at the bound over a full-screen fill, frame time before and after.
