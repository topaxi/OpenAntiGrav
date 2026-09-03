# FSR 3.1 is ported, wired and selectable; nobody has played it

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
- **A dispatch is capped at 65535 workgroups per dimension**, which the
  buffer clear reached at 2880x1800 by being dispatched flat over the texel
  count. Everything walks a texture extent now. Note the shape of this and the
  MSAA one together: **both were found by playing, and both were invisible to a
  fixture small enough to be fast.** A test that runs the chain once at a real
  window size and a real anti-aliasing setting would have caught the pair.
- **MSAA changes the *type* of two inputs, and only playing it found that.**
  The scene's depth and velocity attachments are multisampled under
  `anti_aliasing = msaa4x`, which is a different WGSL binding type and a
  `create_bind_group` validation panic in the frame loop. Every test up to that
  point ran at sample count 1. `prepare_inputs` is now built twice and there is
  a test that runs both.
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

- **It has been captured once and never played.** `just compare-upscalers`
  produces the three frames now and the FSR 3.1 one is a clean, artefact-free
  race frame that sits *between* bilinear and FSR 1 in apparent sharpness -
  cleaner edges than the blit, less crisp than FSR 1 and without FSR 1's
  ringing on the barrier slats. Whether that is right is a from-play judgement
  nobody has made. Three candidate explanations for the softness, in the order
  worth testing: the capture is nearly static (the craft is stationary for 240
  of its 300 ticks, which is the worst case for a temporal upscaler); RCAS runs
  at its default sharpness rather than the settings row's; and accumulation runs
  on gamma-encoded values, which biases a converging history bright in the
  shadows.
- **Accumulation runs in gamma, and upstream's runs in linear.** ADR-0020 makes
  gamma authoritative here and says nothing linearises, so there is no linear
  light to hand FSR 3.1 - see `docs/rendering/fsr3.md`. It is the *consistent*
  choice rather than the accurate one, and its visible cost is unmeasured. A
  ghost trail behind a dark object is where it would show.
- **A camera cut does not reset the history.** `Scene::record_frame` sets
  `reset` on the sequence's first frame only, so a view change or a
  respawn hands the resolve a history of a different scene. It has not been
  seen because nothing cuts the camera mid-race yet.
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

1. **Play it.** `just play --race --upscaler fsr3 --render-scale 50` and look
   at a moving frame; the capture is nearly static and is the wrong instrument
   for the one thing a temporal upscaler is for. Ghosting behind the craft and
   shimmer on the barrier slats are what to watch.
2. Decide whether `fsr3` should be a default anywhere. It is off by default and
   the row already offers it; `Scale::default` is `FULL`, so like `fsr1` it is
   inert until a player lowers the render scale - except that unlike `fsr1` it
   is *not* inert at 100 %, because a temporal resolve still has more samples
   than one frame carries.
3. Reset the history on a camera cut, which nothing does yet - see above.
