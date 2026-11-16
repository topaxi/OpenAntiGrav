---
status: measure
titles: [hd]
updated: 2026-11-10
summary: "Deck HD Anulpha: four desktop cuts won on the Deck, HD encode cuts dropped; open: 32-bit prepared texel, unused scratch target"
categories: [rendering]
---

# The Deck's GPU passes on HD Anulpha, after the gather bisect

Per-pass table and the two dropped cuts: `docs/rendering/frame-thread-cpu.md`, "The Deck's GPU
passes on HD Anulpha" (command there). Bisect and tier prices: `docs/rendering/motion-blur.md`, "The
gather on the Deck: bisected, two exact cuts flat". The Deck run scripts were the `render-bundles`
ones (`~/oag-perf/render-bundles/run.sh`, `avg.sh`, `ab.sh`, with `--split-cut horizontal` for `2h`).

## Picture-changing motion blur options (a design decision for the maintainer)

The blur is this project's effect, not the original's, so a cheaper picture is ours to choose. Each
expected saving is per view, for the maintainer's profile (blur high, half resolution). The ms come
from the bisect's linear tap cost (15 to 5 taps saved 1.04 ms at 1080p; 4K scales by the measured
7.15 / 1.78) unless marked unmeasured.

| Option | Saves at 1080p / 4K (estimate) | What the picture loses |
| --- | --- | --- |
| ~~Fewer taps: `TAPS` 15 to 9 (or `TAP_SPACING_PX` 4 to about 7)~~ | landed as per-tier caps (`high` 11, `medium` 9, `low` 7); the Deck gather fell 19 to 22 % at `high` (1.77 to 1.40 ms at 1080p, 8.20 to 6.42 ms at 4K) and 26 to 32 % at `medium`: `docs/rendering/motion-blur.md`, "Fewer taps: each tier caps the gather" | Same length (the taps stratify a fixed span); a slightly coarser speckle along bright streaks. Spacing tied to the reach cap was measured and dropped. |
| 32-bit `prepared` texel (`[reach, depth]` in `Rg16Float`) | unmeasured; at most 0.9 / 3.7 ms (the bisect's no-load row, which also folded the weights), likely far less | Reach rounded to f16 before the cone weights: sub-level differences at silhouettes, likely invisible; needs a frame-compare to say. |
| A quarter-resolution gather rung | about 1.3 / 5.4 ms (three quarters of the gather) | Smears resolved at a quarter of the pixels each way: thin moving objects (a craft's fin, a weapon trail) smear less or not at all, and silhouettes of a still surface over a moving background soften. |
| `medium` instead of `high` (exists today) | measured before the tap caps: little at 4K (pipelined 24.9 to 24.7 ms), 1080p within noise. With the caps `medium` gathers 9 taps against 11 (desktop: gather 1.05 against 1.31 ms at 4K, NVIDIA) | Shorter shutter: less smear. |
| Blur `off` (exists today) | measured: pipelined 8.7 to 6.2 ms at 1080p, 24.9 to 15.3 ms at 4K | All of it. |

None is built. A tier needs frame-compare pictures (`docs/tools/frame-compare.md`) before it is
proposed as a default.

## Open

- ~~**Re-measure the tap caps on the Deck.**~~ Done (`deck-remeasure`): `high` 725 to 587, 1770 to
  1399, 8199 to 6424 us and `medium` 644 to 478, 1660 to 1158, 7932 to 5399 us at 800p, 1080p, 4K
  (`docs/rendering/motion-blur.md`, "The tap caps on the Deck"). The estimate of a fifth was right.

- **Motion blur is ours (maintainer, 2026-11-09):** the blur is our own graphics setting, so a cut that changes its picture (fewer taps, cheaper composite or copy) needs a capture diff that names the change, not byte-identity and not a design decision. Every other pass stays byte-identical.
- **The 64-bit `prepared` texel.** Each gather tap loads `Rgba16Float` (velocity xy, depth, an
  unused w). Not cache misses (bisect). Untested: whether the 64-bit load's rate is the cost. A
  diag loading a 32-bit texture at the same cache-local addresses, compared with the 1.63 ms
  cache-local 64-bit row. A 32-bit `[reach, depth]` texel would change the picture (reach rounded
  to f16), so it is a tier or a measured-diff decision for the maintainer.
- ~~**The blur's composite plus copy**~~ Landed (`blur-composite`): the half-size composite blends
  onto the scene (`SMEAR_OVER`), no copy pass; desktop -15 to -31% of composite + copy at 800p,
  1080p, 4K on NVIDIA and Intel, pictures differ by 1 of 255 (details:
  `docs/rendering/motion-blur.md`, "The half-size composite blends onto the scene"). **Deck
  re-measured** (`deck-remeasure`): composite + copy 305 to 224, 673 to 503, 2587 to 1981 us at 800p,
  1080p, 4K (-23 to -26 %), `docs/rendering/motion-blur.md`, "On the Deck". Not done: in half mode the
  full-size `scratch` target is still allocated though unused (66 MB at 4K); drop it when the
  resolution is half, rebuilding it on a switch to full.
- ~~**HD encode decodes each scene texel four times**~~ Measured and closed with nothing kept
  (`hd-encode`): `docs/rendering/frame-thread-cpu.md`, "HD encode: three exact cuts measured, none
  kept". Since the sRGB-curve decode (`79f28d611`) the pass reads 451 / 989 / 3891 us on the Deck at
  800p / 1080p / 4K, not 0.75 / 2.6 ms. Half of that is the four decodes.
  - Quad sharing is exact (`cmp` equal on the Deck and NVIDIA) but slower. With the per-quad check
    it costs +10 % at every size on the Deck; without it, +0.5 %. The selects that assemble each
    lane's texels eat the saved decode.
  - A GPU-built decode table (4,819 f16 entries) is slower too: 1455 against 989 us at 1080p.
  - Still open, not tried: a compute encode with workgroup-shared texels (about 1.1 decodes per
    pixel). It needs the caller's canvas as a storage texture, which `oag-post` does not own.
    Measure with the recipe on that page.
- **No whole-race `2h` baseline was taken**: the Deck went back to the maintainer. Earlier Deck
  numbers are vertical split.

## Next Steps

1. The 32-bit diag above: `OAG_RENDER_GPU_BENCH=60 OAG_RENDER_PERF=1`, 1080p and 4K, interleaved
   with `base`, 2 rounds.
2. If it pays: render the tier's pictures and diff them (`docs/tools/frame-compare.md`) before
   proposing it.
3. Whole race for any kept change: `ab.sh <tag> "base cand" "1280x800 1920x1080 3840x2160" 3 hd
   "1 2"`, plus Pulse's default circuit.
4. The maintainer picks a row of the options table, or none.
