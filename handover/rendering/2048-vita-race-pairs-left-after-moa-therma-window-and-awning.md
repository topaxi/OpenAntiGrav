---
status: measure
titles: [2048, omega]
updated: 2026-11-20
summary: "Vita v flip wired (modesto 28.6 to 21.8); open: Moa road needs the light lane, Omega GCN reads, amphiseum road, empire flare"
size: M (30)
difficulty: medium (50)
---

# 2048's worst race pairs after the reflective window and the awning

2026-11-19, lane `vita-race-pairs`. Evidence and numbers: `docs/formats/2048-material-params.md`, "The reflective window and
the awning". `2048-vita/moa-therma` whole frame 32.55 to 27.56, tower box 56.12 to 20.81, top band 42.21 to 26.60. The other
fifteen `2048-vita` pairs re-render pixel-identical.

## Open

- **Omega's `fc10_lambert_2uv_emissive` is checked, applies, not wired.** Omega's 2048-mode Arena (`#73`) and Cathedral
  (`#464`, `#824`, `#1106`) ship the file with the same sampler layout (window atlas, `sky5.gnf` second, lightmap third); read its
  GCN programs (`cargo run -p oag-rcs --example gcn_dump -- <data0N.psarc> <entry>`) for the `D.a * R^2` term and the
  `0.5 + 0.5 R.xy` lookup, then set `Kind::ps4` in `mesh::rcs::psp2::emissive_add` and re-render `omega/arena-2048` and
  `omega/downtown-2048` (the Cathedral pair).
- **Moa Therma's road and walls are `diffuse_normal_specular` #16, and the missing terms read the light's uniforms**
  (box `(0,175,960,544)` 28.02, bias -17.8, 354k px; skip scan in `docs/formats/2048-material-params.md`, "Moa Therma's
  road"). Law read: normal-mapped N, `s.w * max(0, R.L) * lm^2` added to the prelit sum, and a `liveLighting0specular * D.a * lm.a
  * max(0, R.L)^Constant1` lobe; ours has none of the three on Vita. `s.w` is `shadowLightSpecularDiffuse.w` and the lobe's colour is
  `liveLighting0specular`: the 2048 violet-band lane's writer (`2048-live-light-specular.md`), so not wired here. Once that
  lane lands the light values, wire the three terms (the normal map needs the Vita sampler choice and tangent binding
  `normal_map.rs` does only for PS4; the lobe is the Omega panel-map road's `mode 15` shape without the SDF) and re-render
  `moa-therma` and the four Vita pairs. The hull box `(300,255,660,525)` (38.12, bias -27.0) was not split from the road under it;
  the craft's hdships materials are not in the `OAG_SKIP_MATERIAL` filter, so a craft mask needs a second render.
- **Amphiseum** (27.94 after the flip, whole frame bias +11.4): the left road box fell 47.96 to 30.17 but the original's blue
  panels and dark road under the white lines are still not ours (ours brighter, bias +10.8 to +13.9 over the road): not read.
- **Omega: the `1 - v` flip is checked, applies, not wired.** Omega's Modesto `Track_Wall` (`data02.psarc`) starts its GCN vertex
  program `v_sub_f32 v0, 1.0, v9` on `Uv1`; its Amphiseum and Tech De Ra track materials were not read. A GCN reader that
  mirrors `oag_rcs::gxp::texcoord_flip` (the `v_sub_f32 1.0` on the `Uv1` input) would let `psp2::uv_flip::plan` act on a PS4 model.
- **Arena (a base circuit) now takes the window term too**, with no kept pair to score it: a Vita3K Arena grid pair would
  measure it.
- **The base copies of `fc10_Lambert_2UV_Emissive`** (54 `ReflectMap` programs in `data.psarc`) were not walked for the
  dlc1 copy's shape; walk them before relying on the term in a base circuit.
- **Lane 3 on every other Vita family** moves `modesto-heights` +0.07 and `empire-climb` +0.09 against `moa-therma` -0.03,
  so they stay on lane 0; each family's gate lane is unread.

## Next Steps

1. Omega: read the GCN programs of `fc10_lambert_2uv_emissive` and wire it (about 30 minutes of reading, an hour to wire and
   re-render two pairs).
2. `amphiseum` road: skip-scan again (the flip is in, so what is left is the white lines and the original's blue pit panels).
3. `empire-climb` top middle: scan for the sky and the flare; then Moa's road once the light lane lands `s.w` and the specular colour.
