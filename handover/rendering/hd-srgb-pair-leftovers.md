---
status: ready-to-wire
titles: [hd, omega]
updated: 2026-11-10
summary: "HD's texel decode is sRGB (wired); left: the adaptation's encode, three CPU-side power encodes, Omega's decode (trial lowers both pairs)"
difficulty: easy (60)
---

# The sRGB pair's leftovers

2026-11-10, `hd-contrast`. HD's scene now decodes texels and stores draws with the piecewise sRGB
curve (`docs/rendering/hd-track-tone.md`, "The texture unit's decode is the sRGB curve"). What that
lane measured and did not ship:

## Open

1. **Omega decodes its sRGB GNF texels with a `2.2` power.** Swapping `decode` to `srgb_decode` on
   Omega's scene (`SceneStore::Computed` with `linear_out`) lowered both race pairs
   (`omega/anulpha-pass` 33.00 to 31.95, `omega/vineta-k` 28.36 to 27.94). Needs a census of the
   GNF channel type per bound unit (albedo `Srgb`, lightmap UNORM is documented) and a store or
   override that says so.
2. **The adaptation's encode** in `hd_bloom.wesl` `fs_reduce_first` is the `2.2` power; the bytes the
   original averages are sRGB. Swapped: Anulpha mean grey 3.79 to 4.09 with its mean bias moving
   toward zero. Read the original's adapted value (`FunkLayer + 0x25c`) beside ours' on one frame
   before choosing.
3. **Three CPU-side power encodes** read back through the shader's `decode`:
   `sky_cube::tint_in_texel_domain`, `rcs::picture_tint::tinted`, `rcs::skin::raw_to_texel`. They
   need the title's curve (HD sRGB, Omega power until item 1, 2048 the square root).
4. **Per-circuit spread that is left**: Metropia +1.9, +2.3, +1.0, Sebenco +1.8, +1.0, -0.8,
   Moa Therma -0.7, -1.1, -0.9 (stddev bias, ours minus theirs).

## Next Steps

1. Item 3 first: it is a correctness debt of the wired change (small, three functions, one curve
   argument).
2. Item 2: one RPCS3 pause with the PPU readback and the scene dump, then one shader line.
3. Item 1: the census, then the override.
