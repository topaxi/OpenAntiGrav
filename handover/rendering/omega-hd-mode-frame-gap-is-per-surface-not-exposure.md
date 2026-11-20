---
status: re-first
titles: [omega]
updated: 2026-11-20
summary: "Omega HD-mode per-surface gap: sky dome and the RT_snow1 snow program wired; open: the other NOVA-lit families, bloom, Modesto E, snow shadow map"
difficulty: medium (45)
---

# Omega's HD-mode frames: the scene's light level differs by surface, exposure does not

2026-11-20, `omega-hd-exposure` then `omega-luma-read`. Evidence, boxes, the ring layout and the per-pair numbers:
[`omega-status.md`](../../docs/formats/omega-status.md), "HD-mode frame brightness: the tone map is not the gap" and "the
original's luma ring, read".

## Open

- **Settled by `omega-luma-read` (native port, eight boots, ring read through `/proc/<pid>/mem`):** the original's whole-frame scene luma
  (`LAvg`, settled at the grab) is Sebenco 0.2290/0.2299, Moa 0.4720/0.4740, Modesto 0.4197/0.4299, Amphiseum 0.1324/0.1325 against ours'
  0.1756, 0.3851, 0.2621, 0.1333: ratios 1.30, 1.23, 1.60 to 1.64, 0.99. `E` equals ours on Sebenco, Moa and Amphiseum (pinned) and is
  0.60 against ours' 0.868 on Modesto. Neither "equal" nor "1.6 times" holds across the four.
- Inverting the original's frames with its own `E`, `t1` and curve puts the excess in surfaces: Sebenco sky and far building about 2.3 to
  2.5 times ours, floor and wall 1.1 to 1.4; Modesto 1.4 to 2.2 everywhere (sky most); Moa sky and floor 1.0, stands 1.4, stripe road 1.6;
  Amphiseum ceiling and stands 0.67 to 0.73 (ours brighter). Bloom (ours draws none) inflates the bright rows, saturation deflates them.
- Not known: which scene term carries it. The sky and fog consumers in the 1.07 eboot are unread (`FUN_015eda20` is only the registrar);
  `Lighting.Sky brightness` does not predict the sign; no texel of Sebenco's luma image reaches 0.97 where the other circuits reach 1.6 to 8
  (whether the HDR fog colour `0.646 1.18 1.85` is clamped on far geometry, or the sky draws with no brightness term, is open).
- **Read live by `omega-crowd-live` (native port, Sebenco Climb three boots, Sol two):** the bound `constantAmbientColour`, `liveLighting0diffuse` and direction, specular, `fogColour` (HDR `0.646 1.18 1.85`, unclamped in the constant) and the lighting object's `Sky brightness 1.51` equal the file's authored values, which ours feeds (ratio 1.000; [`omega-status.md`](../../docs/formats/omega-status.md), "Crowd and scene light inputs, read live"). So the per-surface excess is not a bound light constant: it is inside the sky, far-building and fog programs and their textures. That narrows the first step below to reading those programs, not their inputs.
- Moa's stands belong to the queued crowd-lighting lane (its Sol counterpart is closed: inputs equal ours').

## Next Steps

1. **DONE 2026-11-20 (`omega-sky-program`)**: the sky dome is `SkyBrightness * texture` and is wired for Omega
   (`Looks::sky_brightness`; [`omega-status.md`](../../docs/formats/omega-status.md), "The sky dome's pixel program"). Sky-only pixels,
   ours minus theirs: Moa -36.4 to -2.3, Sebenco -36.8 to -12.8; Modesto -14.5 to +23.1 (+0.2 with `E` pinned to the original's 0.6).
   Fog's curve is read off `RT_snow1`'s vertex program and is ours'.
2. **DONE 2026-11-20 (`omega-snow-program`)**: the Sebenco mountain (`RT_snow1`, 84,203 triangles) is drawn by its own program, the vertex-lit
   permutation `@0xaa24` (`NOVAColor`, no lightmap, no shadow map): a drift-plus-detail normal, `F * FresnelStrength * FresnelColour * VertexColour2 * tex * D`
   with a diffuse wrap of 2.62, and a sun lobe ([`omega-status.md`](../../docs/formats/omega-status.md), "The snow program"). Box `1100,40,300,160`
   100.1 to 133.2 (theirs 162.3), snow pixels 93.9 to 176.2 (theirs 186.3); the other 23 Omega race pairs, HD and 2048 are pixel-identical.
   **Left open:** the same NOVA-lit family on other Omega materials (`NOVAColor` + `VertexColour2` on the PS4 declaration; the loader reads both now), which
   stay generic; `reflectiveBlurCoeffs.z` (read as 0), the vertex-light count of the permutation the race binds (`@0xf6e4` is one light), the shadow-map
   permutations (`@0xc2c4`, `@0x110e4`, `S` on the track), `rt_snow_ref` (three more samplers and a cube reflection).
3. **An exact linear sky gain**: the dome's gain is a gamma-domain multiply by the sRGB encode of the gain (1.9 to 3.4 % low at a
   mid texel, 6 to 11 % low at 0.2); an exact multiply after `decode` needs a per-draw uniform slot (the block has none and 34 test files
   mirror it). Also: read the frame-constant table's own `SkyBrightness` (recipe in `omega-status.md`).
4. Modesto's `E` (0.868 against the original's 0.60) is the symptom of its dim scene: the other surfaces (left structure, right wall,
   floor, 1.4 to 2.2 times ours) are the lane that closes it. Bloom (ours draws none on the sky) is the red channel's 20 grey on Sebenco.
5. Acceptance numbers for any wiring: ours' `LAvg` on the four pairs should reach 0.229, 0.472, 0.42 and 0.132 (the readback patch
   is not kept; `omega_tonemap`'s `frameL` is the quantity), and Modesto's `E` should fall to about 0.6.
