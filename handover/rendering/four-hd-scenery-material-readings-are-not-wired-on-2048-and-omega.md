---
status: ready-to-wire
titles: [2048, omega, hd]
updated: 2026-11-12
summary: "HD readings on 2048 and Omega not wired (alpha_emissive done); HD specular map wired, unsaturated N.L read but drawing it is worse"
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

5. ~~**The specular map of `dn_diffuse_specular_emiss`**~~ **Wired 2026-11-12, `hd-specular-nl`**: the 17 Metropia slots (and Ubermall's 2, out of view at its start line)
   bind `SpecularTexture` third with `ALPHA_EMISSIVE_SPECULAR_MAP` (`docs/formats/rcsmaterial.md`, "The specular map of `dn_diffuse_specular_emiss`").
   `mr_diffuse_emissive_alpha_glow_specular` declares no `SpecularTexture` on HD (units 1 and 2 are shadow maps): nothing to wire. **Left:** 14 other HD
   families declare the sampler (`pb_window` 156 programs, `cf_diff_spec`, `base_diffusespecular` and the rest, listed on that page), not read; the
   `_glow` twin declares it and is not wired on HD.
6. ~~**The unsaturated `N.L`**~~ **Closed, 2026-11-12 (`rsx-bit24`)**: bit 24 of the `MUL`'s dword 2 is `src1_prec_mod = 4`, a saturate of the source (RPCS3 `SRC1`; `docs/formats/rcsmaterial.md`, "Bit 24 is a source saturate"). `Program::sun_diffuse_unsaturated` now skips a reader whose `Instruction::source_precision[k]` for the dot's operand is `PRECISION_SATURATE`, and all 2,349 programs fall to 0 of 7,211: HD clamps the sun term everywhere, nothing to wire. **Left:** Omega's `@0x228`/`@0x260` clamp modifiers and 2048's equivalent have not been censused; the other precision codes (1 half, 2 fixed12, 3 fixed9) are decoded in the reader but no census says where they occur.

## Next Steps

- For 1 to 4, per title: dump the program (`gcn_dump` for Omega, `scripts/vita-usse.py
  disasm` for 2048: it needs no Vita3K shader cache), confirm the combine matches HD's, add the family's row to
  the `psp2` loader, and measure on a pair of that title (Omega in-game shots
  are not trusted: use the `wipeout_pc` port or a material census). Item 1
  is done.
- For 6: closed (above).
