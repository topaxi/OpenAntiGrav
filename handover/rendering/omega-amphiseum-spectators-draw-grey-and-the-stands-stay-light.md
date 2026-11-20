---
status: re-first
titles: [omega]
updated: 2026-11-20
summary: "Omega Amphiseum: front-row spectators (emissive, 919 figures) draw grey, upper stands still light, glow constant families unwired"
difficulty: medium (40)
---

# Omega Amphiseum: the spectators draw grey and the upper stands stay light

2026-11-16, lane `omega-amph-crowd`. The pad strips and the white stands are fixed (`docs/formats/rcsmaterial.md`,
"Omega's constant diffuse families"); this is what is left on `omega/the-amphiseum` (box A, stands band, 65.4 of an original 84.2).

## Open

1. ~~**The 919 `emissive` figures**~~ **Not drawn any more** (`omega-sol-silhouettes`, 2026-11-20): the 907 node-bound people are the track's crowd-entity shapes (mesh flags `0x0401`), which the original's loader replaces with `Data/Crowd/Rigs` instances (`omega-status.md`, "The crowd-entity shapes"); Omega draws nothing there. Amphiseum frame 49.20 to 48.71. **Open:** the rigs themselves (skinned `_BoneCount4` crowd materials, 82 rig files, six clips, instanced at the `CrowdEntity` nodes) (the other 12 of the 919 are not looked at).
2. **Upper stands and far crowd** read lighter than the original with white speckle: `nr_crowd_bustle` and the stand seats on Omega (probably
   the same material swap; not read).
3. **Constant-only glow programs** `dc_constantglow`, `dc_glowlightband` (Moa Therma), `cf_constantcolourglow` (Sebenco): same shape as
   `emissive`; wire through `psp2::flat_constant` once the glow-mask halo (`docs/rendering/hd-glow-mask.md`) is drawn on Omega, or they score worse.
4. **Boot-to-boot noise per box** was not measured (the twin original of this pair is not kept). A second cold boot of the native port with
   `scripts/wipeout-pc-omega-pair.py` gives it.

## Next Steps

- Item 1 answered (the loader swaps entities for rigs); the rigs are the open lane. Item 2: `nr_crowd_bustle` and the seats are not entity shapes (flags `0x0101`/`0x0201`), so the swap does not explain them; unread.
- 2048 has `emissive` in two files; its program is unread, loader gate is `ps4` only.
