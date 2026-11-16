---
status: re-first
titles: [omega]
updated: 2026-11-16
summary: "Omega Amphiseum: front-row spectators (emissive, 919 figures) draw grey, upper stands still light, glow constant families unwired"
difficulty: medium (40)
---

# Omega Amphiseum: the spectators draw grey and the upper stands stay light

2026-11-16, lane `omega-amph-crowd`. The pad strips and the white stands are fixed (`docs/formats/rcsmaterial.md`,
"Omega's constant diffuse families"); this is what is left on `omega/the-amphiseum` (box A, stands band, 65.4 of an original 84.2).

## Open

1. **The 919 `emissive` figures** (slot 350, 907 node-bound 582-triangle people, `EmissiveColour` 0.5566 grey) draw as grey silhouettes
   where the original shows coloured spectators. Their program is a constant, so the colour is supplied by something else: a runtime material
   swap per crowd entity, or a pass the original draws them in. Needs Ghidra on how the engine binds crowd entities.
2. **Upper stands and far crowd** read lighter than the original with white speckle: `nr_crowd_bustle` and the stand seats on Omega (probably
   the same material swap; not read).
3. **Constant-only glow programs** `dc_constantglow`, `dc_glowlightband` (Moa Therma), `cf_constantcolourglow` (Sebenco): same shape as
   `emissive`; wire through `psp2::flat_constant` once the glow-mask halo (`docs/rendering/hd-glow-mask.md`) is drawn on Omega, or they score worse.
4. **Boot-to-boot noise per box** was not measured (the twin original of this pair is not kept). A second cold boot of the native port with
   `scripts/wipeout-pc-omega-pair.py` gives it.

## Next Steps

- Ghidra: how `Crowd` entities get their material (item 1), then item 2.
- 2048 has `emissive` in two files; its program is unread, loader gate is `ps4` only.
