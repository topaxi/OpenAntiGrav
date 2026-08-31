# Mode 2's alpha test is decoded end to end; `oag_render` still alpha-blends it

2026-08-31, split off
[material-states-upper-bits-bit-7-is-measured-mode.md](material-states-upper-bits-bit-7-is-measured-mode.md)
once its `Transparency::Mode2` half resolved cleanly - see that file's history
and [material-state.md](../docs/ghidra/functions/ps3-hdfury-eu/material-state.md)
for the reverse-engineering. This thread is the implementation half: format
and mechanism are both settled, `oag_render` has not caught up.

**What is settled.** `Material_ApplyRenderState` (`0x005d8f68`) reads
`Material::state` bit 1 into `NV4097_SET_ALPHA_TEST_ENABLE` and two new
material fields - `alpha_func` at `+0x18`, `alpha_ref` at `+0x1c`, both now
decoded by `oag_formats::rcsmodel::material` - into
`NV4097_SET_ALPHA_FUNC`/`SET_ALPHA_REF`. Disc-wide, every material measured
holds `alpha_ref = 0.5` and every `Transparency::Mode2` material holds
`alpha_func = GL_GREATER` (`0x0204`). Mode 2 is a plain `GL_GREATER`/`0.5`
alpha-test cutout, confirmed against `nr_crowd_bustle`'s own fragment
microcode and its texture's alpha histogram (a clean bimodal `0`/`255` split,
not the near-uniform spread an earlier reading assumed).

**What is not.** `oag_render` has no fixed-function alpha test - `wgpu` does
not expose one, so it needs a shader-side `discard`. `Material::blend()`
(`crates/formats/src/rcsmodel/material.rs`) currently returns
`Blend::Factors` for `Transparency::Mode2`, identically to
`Transparency::Blended`, because both satisfy `is_see_through()` and the
method has never distinguished them. `mesh::rcs::surface`
(`crates/render/src/mesh/rcs.rs:333`) builds its `blend_state` straight off
that, so every mode-2 surface on the disc - the six materials, 537 chunks
census-2026-08-20 - is drawn alpha-*blended* today, not alpha-*tested*.

## Open

- Whether the current mis-render is visible at all. `crowd_avatars_22x4.gtf`'s
  alpha is exactly `0` or `255`, never between, so a blend factor of `0` or
  `1` and a hard test likely produce the same per-pixel colour - the two
  modes differ in **depth-buffer interaction**, not colour, for this specific
  texture. Whether that shows up as a visible sorting artefact (a crowd
  billboard blending through nearer geometry it should instead have tested
  against and won or lost depth normally) is unconfirmed - no screenshot
  comparison has been done.
- Whether every other mode-2 material's texture is equally bimodal. Only
  `nr_crowd_bustle`'s has been histogrammed; `jd_alphalambert_test`,
  `fence_alpha`, `emissive_alpha_heathaze_test`, `cf_alpha4glow` and
  `uv_anim_diffuse_alpha_emissive` have not, so whether the "colour is close
  either way" reasoning generalises past the crowd is unmeasured.

## Next Steps

- Histogram the other five mode-2 materials' alpha channels
  (`oag_formats::gtf`, same method as the crowd check) before wiring
  anything, to know whether any of them actually needs the hard cutoff to
  look right rather than merely to sort right.
- Add a `discard` path to `mesh.wgsl`: the fragment shader needs the
  material's `alpha_func`/`alpha_ref` as a per-draw value (a uniform or a
  vertex-carried constant - whichever fits the existing pipeline's binding
  layout) and a comparison against sampled alpha before the final colour
  write. Only `GL_GREATER` is attested on the disc for a *consumed* mode-2
  material, so a first pass can hardcode that comparison and treat a
  differing `alpha_func` as unimplemented-and-reported rather than guessing
  a general comparison-function switch nothing has exercised.
- Change `Material::blend()` to return something other than `Blend::Factors`
  for `Transparency::Mode2` - a new `Blend` variant carrying `alpha_func`/
  `alpha_ref` reads cleanest, since `Blend::Opaque` would lose the cutout
  entirely (drawing the crowd's transparent background as solid) and
  `Blend::Factors` is the wrong mechanism by construction now. Thread this
  through `mesh::rcs::surface` and whatever pipeline/bind-group change the
  `mesh.wgsl` discard above needs.
- Verify with a screenshot: `just view <image>:.../talons_junction/track.vex
  --track --screenshot out.png` (or equivalent for a circuit that draws
  `nr_crowd_bustle`) before and after, to confirm the change is not a
  no-op and does not regress the crowd's visible silhouette.
