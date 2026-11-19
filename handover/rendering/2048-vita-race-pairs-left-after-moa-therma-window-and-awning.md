---
status: measure
titles: [2048, omega]
updated: 2026-11-19
summary: "Moa Therma window and awning wired (32.55 to 27.56); left: Omega fc10 (GCN unread), Moa hull and walls dark, one top box per other Vita pair"
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
- **Moa Therma, what is left** (boxes at 960x544, HUD masked, luma, ours minus theirs): hull `(300,255,660,525)` 38.12
  (bias -27.0), road and walls `(0,175,960,544)` 28.02 (-17.8), left awning `(0,0,420,175)` 30.33 (-18.6). The whole frame is
  dark: this is the `2048-grid-dark-bias-is-the-craft-not-a-frame-term.md` family of causes (the craft's paint, the lightmap
  scale), now with a cleaner top band to read it against.
- **One top box on each other pair** (4 x 6 grid of mean |err| and bias, before this change): `modesto-heights` right wall
  `(600..960, 136..272)` 51 to 60, bias +24 to +51 (ours brighter: not read); `amphiseum` left road `(0..160, 272..408)` 52,
  bias -19 and the white lines across ours' road where the original's is dark with blue panels (not read); `empire-climb`
  top middle `(320..640, 0..136)` 47 to 50, bias -41 to -21 (the sun's flare and haze, `lens-flare.md`, 2048 not wired).
- **The base copies of `fc10_Lambert_2UV_Emissive`** (54 `ReflectMap` programs in `data.psarc`) were not walked for the
  dlc1 copy's shape; walk them before relying on the term in a base circuit.
- **Lane 3 on every other Vita family** moves `modesto-heights` +0.07 and `empire-climb` +0.09 against `moa-therma` -0.03,
  so they stay on lane 0; each family's gate lane is unread.

## Next Steps

1. Omega: read the GCN programs of `fc10_lambert_2uv_emissive` and wire it (about 30 minutes of reading, an hour to wire and
   re-render two pairs).
2. `modesto-heights` right wall: skip-scan with `OAG_SKIP_MATERIAL` (the method in "The reflective window and the awning")
   to name the owner, then read its program with `scripts/vita-usse.py disasm`.
3. `amphiseum` road: the same scan; the white lines are a draw the original does not show at this pose or a material whose
   emissive add is not read.
