---
status: measure
titles: [hd]
updated: 2026-11-09
summary: "Deck HD Anulpha GPU: track shading is the largest pass (exact, 1.00x overdraw); the blur gather bisected, two exact cuts flat; open: 32-bit prepared texel, blur composite+copy, HD encode's four decodes"
categories: [rendering]
---

# The Deck's GPU passes on HD Anulpha, after the gather bisect

Spans, bisect and the two dropped cuts: `docs/rendering/motion-blur.md`, "The gather on the Deck:
bisected, two exact cuts flat". Span command there; Deck run scripts in `~/oag-perf/deck-gpu` on
the Deck (`span.sh LABEL BIN WxH [flags]`, `ab.sh` for whole races).

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
