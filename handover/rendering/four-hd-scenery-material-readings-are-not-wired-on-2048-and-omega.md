---
status: ready-to-wire
titles: [2048, omega, hd]
updated: 2026-11-11
summary: "Three HD scenery material readings apply to 2048 and Omega, not wired (alpha_emissive wired); HD: one specular map unbound, unsaturated N.L per block"
difficulty: medium (35)
---

# Four HD scenery material readings are not wired on 2048 and Omega

2026-11-11, lane `hd-worst-pairs`. Metropia's buildings and Moa Therma's floor
were fixed on HD by reading five fragment-program families
(`docs/formats/rcsmaterial.md`, the last five sections). Each was checked
against 2048 and Omega as the rule asks; this thread holds what that left.

## Open

**Checked, applies, not wired (2048 and Omega re-ship the material; neither
title's program was read):**

1. ~~**`alpha_emissive` families**~~ **Wired 2026-11-11, `w2048-alpha-emissive`** on 2048 and Omega
   (`docs/formats/rcsmaterial.md`, "The picture's alpha times a colour": 2048 171 of 171 programs, Omega 249 of 250 plain,
   `psp2::alpha_emissive`). **Wired 2026-11-12, `omega-metropia-specular`:** (a) the 14 Metropia `dn_diffuse_specular_emiss`
   materials (17 with the three whose maps share a file) draw `alpha_emissive` as the picture and weight the specular by their
   `SpecularTexture` on the third binding, on Omega and 2048 (`oag_rcs::rcsmodel::psp2::specular_picture`, `docs/formats/rcsmaterial.md`,
   "Omega's `dn_diffuse_specular_emiss` and 2048's"). **Left:** (a2) the `_glow` twin's own emissive term (2048: `T^2 * T.a`, no `glowtinrt`; Omega's unread) is not drawn, and no `_glow` surface is on a measured frame; (b) none of the three 2048 grid pairs has these surfaces in frame
   (on/off pixel-identical), so the 2048 effect has no original to measure against: a 2048 Anulpha Pass or Metropia pair would
   (ours at the Anulpha grid shows an amber strip light on the stand); (c) Omega Anulpha Pass pair, box `1065,268 100x35`: G 73 to
   80 against the original's 80.
2. **`pb_glass_2nduv_reflect_specular1_glow`** (Metropia's tower). Omega
   `data02`, `data05`; 2048 `dlc1.psarc`. Combine not read, no row in the
   `psp2` loader.
3. **`cf_vex_billboard` compiled without `k`** (Metropia, Chenghou, Anulpha's
   `cf_vex_billoard_glow`). Omega `data02` 6 entries, `data05` 30; 2048
   `dlc1.psarc` 4, `data.psarc` 3. Omega keeps the split between the compiles.
4. **`glass_2uv_rgb_reflect_glow`** (the reflection gated by the mask's red).
   Omega `data02`, `data05`; 2048 `dlc1.psarc`. Programs not read.

The two-lane normal map (Moa's `diffuse_normal_specular`) is **checked,
differs** on Omega (a three-lane `.gnf`, already bound) and needs nothing.

**Open on HD itself:**

5. **The specular map of `dn_diffuse_specular_emiss` and
   `mr_diffuse_emissive_alpha_glow_specular`** (`specular * map.rgb`, unit 1)
   is unbound on HD: the specular is drawn unweighted. Omega and 2048 now bind it third for the first family
   (`ALPHA_EMISSIVE_SPECULAR_MAP`, `shade.wesl`); HD's `alpha_emissive` binds nothing third, so the same flag needs its unit 1 sampled
   there. `mr_..._specular` is not read on Omega or 2048 (its Omega program folds `glowtinrt` into the alpha-weighted specular).
6. **The unsaturated `N.L`** of the bumped lightmapped block: the program
   multiplies the sun by `N.L` with no clamp. Wiring it on every bumped
   lightmapped slot read worse (Moa's box 1.60 against 1.47, the frame 1.92 to
   2.18), so which programs leave it unsaturated has to be read per block.

## Next Steps

- For 1 to 4, per title: dump the program (`gcn_dump` for Omega, `scripts/vita-usse.py
  disasm` for 2048: it needs no Vita3K shader cache), confirm the combine matches HD's, add the family's row to
  the `psp2` loader, and measure on a pair of that title (Omega in-game shots
  are not trusted: use the `wipeout_pc` port or a material census). Item 1
  is done.
- For 5: bind unit 1 where the program declares it and measure on Metropia
  (17 slots) and Ubermall (2).
- For 6: list the lightmapped programs with a `DP3` whose result has no
  `_SAT`, and wire it for those alone.
