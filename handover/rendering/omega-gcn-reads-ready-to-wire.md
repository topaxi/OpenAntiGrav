---
status: re-first
titles: [omega]
updated: 2026-11-04
summary: "Omega 1.07 composite: vignette, ColorScale, tint wired; open: the bloom chain behind BloomTexture, tint and ColorScale triggers"
difficulty: medium (40)
---

# Omega's shader reads: two wired, the composite's bloom open

2026-10-27, `gcn-operand-decoder`. `oag_rcs::gcn` decodes every GCN program Omega ships
([gcn.md](../../docs/formats/gcn.md); `cargo run -p oag-rcs --example gcn_dump`). Three HD findings that sat as
"Omega: not checkable" were read with it. None is wired.

## Open

1. **DONE 2026-10-27 (`omega-normal-map`)** - **Omega tangent frame and normal map.** `psp2` reads the tangent (type `0xe`) and now binds the
   normal map third and signs the frame: 24,590 of 25,977 tangent-carrying submeshes on Omega's circuits (`omega_normal_map_ground_truth`).
   The sampler is the one the pixel programs decode `2x - 1` (`oag_rcs::gcn::signed_samplers`, usage records in
   [gcn.md](../../docs/formats/gcn.md)). **Left open:** 771 submeshes on materials that blend two normal maps (which frame belongs to which),
   203 whose normal map file is not in the archives or will not decode, and the Pro `_1`/`_2` decodes. Evidence:
   [hd-track-tone.md](../../docs/rendering/hd-track-tone.md), "Omega's normal maps are bound".
2. **DONE 2026-10-27 (`omega-tangent-hologram`)** - **Amphiseum hologram board on Omega.** Wired: `mesh::rcs::psp2::static_hologram`
   routes the plain `dc_hologramwithstatic` to `PROGRAM_STATIC_HOLOGRAM` with the record's own rates; the `/2` is HD's
   `output_scale` (0.5 only where `lit_output_halved` applies, unread on Omega, so its scale is 1.0). Pinned by
   `psp2_static_hologram_ground_truth` and `hd_static_hologram_lit_path` (both scales). No Omega Amphiseum reference frame
   exists. Evidence: hd-track-tone.md, "The Amphiseum hologram board", Omega paragraph.
3. **PARTLY DONE 2026-11-04 (`omega-composite`)** - **The 1.07 composite.** Wired: `ColorScale = (1, 1, 1)`, the circuit's
   `Vignette.*` keys as the vignette and the tint (0 at rest) outside it, between the curve and the scanout encode
   (`oag_post::omega_tonemap::law::Composite`; evidence [tonemap.md](../../docs/ghidra/functions/ps4-omega-eu/tonemap.md),
   "The composite's constants"). **Left open: `Bloom` and `LowResAdditive`.** The disc authors `Bloom.*` (bright-pass,
   `Blend factors`) but the chain behind `BloomTexture` is unread: read `FUN_0177fbc0` (the chain) and `FUN_016247c0` (its
   targets), the bright-pass program by reflection names, the blur and per-level blend, level count and sizes. `BloomScale`
   is level 0's `Blend factors` entry. Also open: what triggers `ScreenTint` above zero (the `s^5` flash writers) and the
   `ColorScale` fade ([colorscale.md](../../docs/ghidra/functions/ps4-omega-eu/colorscale.md)).

## Later

- Name the PS4 Pro opcodes the Sea Islands tables do not list (VOP1 84-91, VOP2 50-58, VOPC 201-206); they occur only in
  `_1`/`_2` (Pro) materials, which no model names, so no wiring needs them.
- What selects `_1` against `_2` at run time (eboot).
- 2048's tangent `.w` and hologram (`hologram.rcsmaterial`) programs: needs USSE operands (`gxp.md`).

## Next Steps

1. Lane 1 first: it touches every Omega circuit's normal-mapped surfaces.
