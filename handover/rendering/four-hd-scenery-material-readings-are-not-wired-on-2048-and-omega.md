---
status: ready-to-wire
titles: [2048, omega, hd]
updated: 2026-11-11
summary: "Four HD scenery material readings (alpha_emissive, two glass windows, the k-less billboard) apply to 2048 and Omega and are not wired; HD: specular map unbound, unsaturated N.L per block"
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

1. **`alpha_emissive` families** (`picture.a * glowtinrt` added after the
   light; HD: 6 families, 155 slots, 9 circuits). 2048 `data.psarc` 12 entries,
   `dlc1.psarc` 7; Omega `data00` to `data08`. Omega's ambient-only pixel
   program was dumped and is the same combine; the lit one was not read.
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
   is unbound: the specular is drawn unweighted.
6. **The unsaturated `N.L`** of the bumped lightmapped block: the program
   multiplies the sun by `N.L` with no clamp. Wiring it on every bumped
   lightmapped slot read worse (Moa's box 1.60 against 1.47, the frame 1.92 to
   2.18), so which programs leave it unsaturated has to be read per block.

## Next Steps

- For 1 to 4, per title: dump the program (`gcn_dump` for Omega, the GXP
  reader for 2048), confirm the combine matches HD's, add the family's row to
  the `psp2` loader, and measure on a pair of that title (Omega in-game shots
  are not trusted: use the `wipeout_pc` port or a material census). Item 1
  first: it covers the most slots.
- For 5: bind unit 1 where the program declares it and measure on Metropia
  (17 slots) and Ubermall (2).
- For 6: list the lightmapped programs with a `DP3` whose result has no
  `_SAT`, and wire it for those alone.
