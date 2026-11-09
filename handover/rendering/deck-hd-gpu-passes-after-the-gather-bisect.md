---
status: measure
titles: [hd]
updated: 2026-11-09
summary: "Deck HD Anulpha: blur gather bisected, two exact cuts flat; open: 32-bit prepared texel, composite+copy, HD encode decodes"
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
| Fewer taps: `TAPS` 15 to 9 (or `TAP_SPACING_PX` 4 to about 7) | about 0.6 / 2.4 ms | A long smear is sampled every 7 px or so instead of 4: more visible banding or grain along fast streaks, mostly at the frame's edges at full boost. Short smears unchanged (already 5 to 9 taps). |
| 32-bit `prepared` texel (`[reach, depth]` in `Rg16Float`) | unmeasured; at most 0.9 / 3.7 ms (the bisect's no-load row, which also folded the weights), likely far less | Reach rounded to f16 before the cone weights: sub-level differences at silhouettes, likely invisible; needs a frame-compare to say. |
| A quarter-resolution gather rung | about 1.3 / 5.4 ms (three quarters of the gather) | Smears resolved at a quarter of the pixels each way: thin moving objects (a craft's fin, a weapon trail) smear less or not at all, and silhouettes of a still surface over a moving background soften. |
| `medium` instead of `high` (exists today) | measured: little at 4K (pipelined 24.9 to 24.7 ms), 1080p within noise | Shorter shutter: less smear. |
| Blur `off` (exists today) | measured: pipelined 8.7 to 6.2 ms at 1080p, 24.9 to 15.3 ms at 4K | All of it. |

None is built. A tier needs frame-compare pictures (`docs/tools/frame-compare.md`) before it is
proposed as a default.

## Open

- **The 64-bit `prepared` texel.** Each gather tap loads `Rgba16Float` (velocity xy, depth, an
  unused w). Not cache misses (bisect). Untested: whether the 64-bit load's rate is the cost. A
  diag loading a 32-bit texture at the same cache-local addresses, compared with the 1.63 ms
  cache-local 64-bit row. A 32-bit `[reach, depth]` texel would change the picture (reach rounded
  to f16), so it is a tier or a measured-diff decision for the maintainer.
- **The blur's composite plus copy**: 0.66 ms at 1080p, 2.37 ms at 4K. The copy is an identity
  carry-home (`copy_texture_to_texture` when the formats match), and a premultiplied blend could
  draw the composite straight onto the target. The copy alone (0.14 / 0.48 ms) will not show in
  whole-race fps; the blend is not proven byte-identical.
- **HD encode decodes each scene texel four times** (`final_half_texel` puts every pixel on a
  2x2 average; 12 `pow` for the decode plus 3 for the encode): 0.75 ms at 1080p, 2.6 ms at 4K.
  Quad sharing through subgroup ops would cut the worst lane from 4 decodes to 3, exactly. Not built.
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
