---
status: ready-to-wire
titles: [hd, omega]
updated: 2026-11-14
summary: "HD's and Omega's texel decodes are sRGB (wired) and the CPU encodes follow; left: the adaptation's encode"
difficulty: easy (60)
---

# The sRGB pair's leftovers

2026-11-10, `hd-contrast`. HD's scene now decodes texels and stores draws with the piecewise sRGB
curve (`docs/rendering/hd-track-tone.md`, "The texture unit's decode is the sRGB curve"). What that
lane measured and did not ship:

## Open

1. ~~Omega decodes its sRGB GNF texels with a `2.2` power~~ landed 2026-11-14 (`docs/rendering/hd-track-tone.md`,
   "Omega's texels decode with the sRGB curve"): every Omega race pair's mean grey fell or held.
2. **The adaptation's encode** in `hd_bloom.wesl` `fs_reduce_first` is the `2.2` power; the bytes the
   original averages are sRGB. Swapped: Anulpha mean grey 3.79 to 4.09 with its mean bias moving
   toward zero. Read the original's adapted value (`FunkLayer + 0x25c`) beside ours' on one frame
   before choosing.
3. ~~Three CPU-side power encodes~~ landed 2026-11-13: they take `SceneStore::texel_encode`
   (`docs/rendering/hd-track-tone.md`, "Left, named").
4. **Per-circuit spread that is left**: Metropia +1.9, +2.3, +1.0, Sebenco +1.8, +1.0, -0.8,
   Moa Therma -0.7, -1.1, -0.9 (stddev bias, ours minus theirs).

## Next Steps

1. Item 2: one RPCS3 pause with the PPU readback and the scene dump, then one shader line.
