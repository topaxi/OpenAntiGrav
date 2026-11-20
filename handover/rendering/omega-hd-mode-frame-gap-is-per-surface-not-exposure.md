---
status: re-first
titles: [omega]
updated: 2026-11-20
summary: "Omega HD-mode gap is pre-curve scene radiance, no post stage; bloom mask is 3.5 of Sebenco 18 grey; open: Sebenco_snow, snowfade, mask"
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
  Amphiseum ceiling and stands 0.67 to 0.73 (ours brighter). Bloom inflates the bright rows (ours draws it on Modesto, Moa and Amphiseum, none on Sebenco; the original's adds 3.5 grey there, below), saturation deflates them.
- Not known: which scene term carries it. The sky and fog consumers in the 1.07 eboot are unread (`FUN_015eda20` is only the registrar);
  `Lighting.Sky brightness` does not predict the sign; no texel of Sebenco's luma image reaches 0.97 where the other circuits reach 1.6 to 8
  (whether the HDR fog colour `0.646 1.18 1.85` is clamped on far geometry, or the sky draws with no brightness term, is open).
- **Read live by `omega-crowd-live` (native port, Sebenco Climb three boots, Sol two):** the bound `constantAmbientColour`, `liveLighting0diffuse` and direction, specular, `fogColour` (HDR `0.646 1.18 1.85`, unclamped in the constant) and the lighting object's `Sky brightness 1.51` equal the file's authored values, which ours feeds (ratio 1.000; [`omega-status.md`](../../docs/formats/omega-status.md), "Crowd and scene light inputs, read live"). So the per-surface excess is not a bound light constant: it is inside the sky, far-building and fog programs and their textures. That narrows the first step below to reading those programs, not their inputs.
- Moa's stands belong to the queued crowd-lighting lane (its Sol counterpart is closed: inputs equal ours').

- **Measured by `omega-frame-offset` (native port, bloom knocked out live, [`omega-status.md`](../../docs/formats/omega-status.md), "HD-mode frame offset: which stage carries it"):** no post stage
  (bloom, curve, `E`, encode) is the frame-wide offset. The original's bloom adds **3.5 grey** to Sebenco (2 boots, 10 samples, sd 0.4), almost all of it the per-material
  mask term `AlphaScale * mask.a` that ours does not draw (ours draws 0.00 there); the lit Senco sign alone is 9 to 12 grey (boot means). Ours' `LAvg` on the current tree was not re-read (the 0.1756 is from `13b22ac77`, before the sky, snow and rock programs; an inversion of the current frame gives at least 0.187, so the ring ratio is about 1.2, not 1.30): scaling ours' scene by 1.18 to 1.23 through the original's curve plus that bloom gives 97.4 to 99.6 grey against theirs 103.8, so about 8 to 10 of the 18 grey is scene radiance before the curve, 3.5 is bloom and **4 to 6 grey is unattributed** (a global gain understates it: the sky and far scenery are the dark surfaces).
  Moa, Modesto and Amphiseum read below the method's noise (1 to 2.5 grey); do not wire the mask against Amphiseum (ours already +4.3).

## Next Steps

0. **Bloom mask (small, 3.5 grey on Sebenco):** read `exp mrt1`'s second half off each material's program (`oag_rcs::gcn`, recipe in
   [`tonemap.md`](../../docs/ghidra/functions/ps4-omega-eu/tonemap.md), "The mask, not drawn") and add `AlphaScale * mask` to `fs_bright`; acceptance is the original's
   knockout numbers above (Sebenco +3.5, building-left box +9 to +12, snow sky +1.4, floor and wall 0) with Amphiseum not rising. The knockout script
   (`scripts/wipeout-pc-bloom-knockout.py`, `scripts/omega-bloom-knockout-effect.py`) gives the acceptance numbers for any circuit; the `E` write (`cL`) answers late and
   contaminates later grabs, `highthr` wedges the port: do not use them.

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
2b. **DONE 2026-11-20 (`omega-snow-rest`)**: Sebenco's other "snow" materials, censused ([`omega-status.md`](../../docs/formats/omega-status.md), "The rock program"):
   the box's 18 % is `dc_rocks1` (matched by its drift-normal picture), not snow-named materials (`Sebenco_snow*` 1.1 %, `snowfade` overlays 1.5 %). `DC_Rocks` is drawn by
   its lightmapped program `@0xca64` (two normal maps, wrapped sun, `Fsl`, lobe): the rock's grain matches (high-pass 4.78 to 6.99, theirs 6.75), the mean does not
   (-25.4 to -27.5 on its mask, box 133.2 to 133.0): the rock sits 22 grey under at every distance from the snow, and the whole frame 18. **Left open:** `Sebenco_snow*`
   (own program, 154 to 369 instructions), the `snowfade` overlays (3.5 % of the frame, `PROGRAM_UV_OVERLAY` first), the cube and `S`.
   **`S` read 2026-11-20 (`omega-shadow-map`)**: three exponential cascades along `Sun direction`, the sun term removed whole inside a
   shadow ([`shadows.md`](../../docs/rendering/shadows.md), "Omega's own `S`"); it moves only where the sun reaches a surface past its
   baked `LM.a` and a caster stands in it, so it is not a frame-wide offset. Wiring is in the 2048-road thread.
3. **An exact linear sky gain**: the dome's gain is a gamma-domain multiply by the sRGB encode of the gain (1.9 to 3.4 % low at a
   mid texel, 6 to 11 % low at 0.2); an exact multiply after `decode` needs a per-draw uniform slot (the block has none and 34 test files
   mirror it). Also: read the frame-constant table's own `SkyBrightness` (recipe in `omega-status.md`).
4. Modesto's `E` (0.868 against the original's 0.60) is the symptom of its dim scene: the other surfaces (left structure, right wall,
   floor, 1.4 to 2.2 times ours) are the lane that closes it. Bloom is 3.5 grey of Sebenco's 18, not the red channel's 20 (`omega-frame-offset`, above).
5. Acceptance numbers for any wiring: ours' `LAvg` on the four pairs should reach 0.229, 0.472, 0.42 and 0.132 (the readback patch
   is not kept; `omega_tonemap`'s `frameL` is the quantity), and Modesto's `E` should fall to about 0.6.
