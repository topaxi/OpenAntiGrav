# FSR 3.1's eight passes are ported; nothing selects it yet

The route was decided by
[ADR-0012](../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md):
port the shaders to WGSL, drive no native SDK. Every renderer-side prerequisite
[modern-features.md](../docs/overview/modern-features.md) listed - readable
depth, per-object motion vectors, sub-pixel jitter, a UI-free scene - landed
before this thread opened, so what is left is the port itself.

The permanent record of *what* is being ported and *how it deviates* is
[docs/rendering/fsr3.md](../docs/rendering/fsr3.md). This file carries only
what is not done yet and the traps found on the way.

## Provenance, and why it is pinned

The reference is AMD's FidelityFX SDK at tag **`v1.1.4`**, commit
`c6efa6bf7f2027b3ec94f28578bb5965eabb9e55`. Nothing from it is vendored; it is
read from a scratch checkout outside the repository. The pin matters because
ADR-0012's whole mitigation for "a port is a fork" is that the WGSL stays
diffable against a named upstream, and `licences/AMD-FidelityFX-MIT.txt`
already sets the precedent by naming `v1.20210629` for FSR 1.

`v1.1.4` deliberately rather than SDK 2.x: 1.1.x is the last line that is MIT
source all the way down, with no FSR4 signed binaries anywhere in the tree.

To re-fetch it:

```sh
just fsr-reference   # into ~/.cache/oag-fsr/v1.1.4, never into the repo
```

## Traps found

- **`accumulation` and `luma_history` are render-sized, not
  presentation-sized**, despite their names, and getting that wrong produces no
  error at all - only the top-left corner of each is ever written, every read
  past it comes back zero, and the picture reads as "no history anywhere". Found
  by a readback test at pass 5, not by reading the descriptor table twice. The
  four that really are presentation-sized are `new_locks`,
  `internal_upscaled_color_1`/`_2` and the output.
- **Storable and filterable are different properties.** `R32Float` is
  baseline-storable and *not* baseline-filterable, so every intermediate some
  pass reads through the linear sampler needs `Rgba16Float` or `Rgba8Unorm`
  instead. Settle it by listing the `Sample*` callbacks the passes actually
  call - `SampleInputDepth` is declared and never called, which is the only
  reason the depth attachment's unfilterable binding is never a problem.
- **A dispatch cannot bind one texture as both a storage write and a sampled
  resource**, even when the shader ignores the read. The pyramid's level-0
  dispatch needed pointing at something other than the level it writes.
- **A small fixture catches size rules a realistic one never reaches.** 8x4
  render found the mip-count ceiling (a 4x2 target holds three levels, not six)
  and the resolution bug above.

- **`ffxFsr3UpscalerGetJitterOffset` is already what `oag_render::jitter`
  does.** `halton(index % phaseCount + 1, 2 or 3) - 0.5`, both axes, is
  upstream's function line for line - so `jitter::offset_pixels` turned out to
  be an accidental exact transliteration rather than a stand-in. Only the phase
  *count* was ours, and only that had to change.
- **The reconstructed-previous-depth pass scatters.** `ReconstructPrevDepth`
  writes up to four texels at reprojected positions with an `InterlockedMin`,
  so it cannot be a fragment pass and cannot avoid an atomic. See the doc page
  for why that became a storage *buffer* here rather than a storage texture.
- **Only two upstream passes use `groupshared` at all**, both of them the SPD
  pyramids. Everything else is per-pixel, which is what makes ADR-0012's
  predicted SPD substitution the *only* structural deviation rather than one of
  many.
- **Upstream ships no wave intrinsics in these headers.** `ffxWave*` appears
  nowhere under `gpu/fsr3upscaler/`, so there is no scalar-versus-wave fork to
  choose between - a worry that turned out not to exist.

## Open

- **The game side is not wired, and that is the next piece of work.**
  `Fsr3::output` returns a frame; `upscale::Framebuffer::resolve_scene` has no
  `Upscaler::Fsr3` branch that reads it, so choosing `fsr3` on the menu row
  still resolves through the blit. Landing the resolve and the wiring separately
  is deliberate - a bug in each would otherwise arrive together with nothing to
  bisect between. Three things the wiring has to get right, all of them stated
  in `docs/rendering/fsr3.md` and none of them enforced by a type:
  the colour input is the **sRGB** view (FSR 1 takes the non-sRGB one), the
  output is **linear** and the blit's grade must not decode it twice, and the
  fallback ladder has to run `supported(&adapter)` and fall to FSR 1 rather than
  failing to boot.
- **Nobody has looked at a frame.** The chain is complete and every pass is
  checked against arithmetic worked out on the CPU - which establishes that each
  computes what upstream's source says, not that the source was read correctly.
  A `--presented` capture at 50 % render scale beside the `fsr1` and `off` ones
  is the instrument, and taking it is a decision for the maintainer rather than
  something a green test can stand in for.
- **`compute_motion_divergence` divides by zero on a perfectly static camera.**
  `saturate(reprojected_velocity / velocity_4k)` with both zero is `0/0`.
  Upstream has the same expression and relies on `saturate(NaN)` returning zero;
  a race always has motion so it never bites in play, but a still fixture walks
  straight into it. Noted rather than 'fixed', because changing it would be a
  divergence nobody has measured a need for.
- **Nothing is compared against upstream's own output.** There is no reference
  implementation to diff against without building the SDK, which ADR-0012
  declined.
- **The widened intermediates are unmeasured.** `Fsr3::sizes` reports the
  number and nobody has read it on the Steam Deck, which
  [goals.md](../docs/overview/goals.md) names in the first tier. The same
  question the dynamic-resolution thread left open about the ceiling
  allocation.
- **The fallback ladder's lowest rung cannot be exercised here.** Both adapters
  on this machine report `DownlevelFlags::COMPUTE_SHADERS`, so a green run is
  not evidence that a compute-less adapter degrades to FSR 1 rather than
  failing to boot. Same shape of hole as the missing-`TIMESTAMP_QUERY` case.

## Next Steps

1. Wire `Upscaler::Fsr3` into `upscale::Framebuffer::resolve_scene`, with the
   fallback ladder and the two colour-space rules above. That is the whole
   remaining gap between "ported" and "selectable".
2. Then take a `--presented` capture at 50 % render scale with
   `upscaler = fsr3` and put it beside the `fsr1` and `off` ones. **This is the
   first time anyone will have seen the port's output**, and no test substitutes
   for it.
3. Derive `Dispatch::sharpness` from the existing `[graphics] upscale_sharpness`
   row rather than the default it takes now - the setting already exists and
   already means upstream's stops on both paths.
