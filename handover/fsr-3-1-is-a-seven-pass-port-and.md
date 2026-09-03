# FSR 3.1 is a seven-pass port, and this is where it stands

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

- **The passes not yet ported.** See the table in
  [fsr3.md](../docs/rendering/fsr3.md#the-passes); it is the port's spine and
  is kept current as each lands.
- **Nothing is compared against upstream's output.** The WGSL compiles and
  draws on this machine's adapter, which says the bindings agree and the syntax
  parses - it says nothing about whether a texel matches what `ffx_fsr3upscaler`
  would produce. There is no reference implementation to diff against without
  building the SDK, which ADR-0012 declined. The honest instrument is a
  from-play look and a `--presented` capture, and neither has been taken.
- **The widened intermediates are unmeasured.** Holding `R32Float` where
  upstream holds `R16_FLOAT` doubles those targets; the doc page states the
  arithmetic but nobody has looked at what it costs on the Steam Deck, which
  [goals.md](../docs/overview/goals.md) names in the first tier. The same
  question the dynamic-resolution thread left open about the ceiling
  allocation.
- **The fallback ladder's lowest rung cannot be exercised here.** Both adapters
  on this machine report `DownlevelFlags::COMPUTE_SHADERS`, so a green run is
  not evidence that a compute-less adapter degrades to FSR 1 rather than
  failing to boot. Same shape of hole as the missing-`TIMESTAMP_QUERY` case.

## Next Steps

1. Port the next pass in the table in [fsr3.md](../docs/rendering/fsr3.md), in
   the order given - the order is upstream's dispatch order, and a pass reads
   what the ones before it wrote.
2. Once `accumulate` lands, take a `--presented` capture at 50 % render scale
   with `upscaler = fsr3` and put it beside the `fsr1` and `off` ones. Until
   then there is nothing to look at: the chain has no output.
3. Write the ADR for the SPD substitution, but only once the reduction is
   actually built - ADRs are immutable and this one has a choice left in it
   (a fixed six-level chain versus a loop to 1x1).
