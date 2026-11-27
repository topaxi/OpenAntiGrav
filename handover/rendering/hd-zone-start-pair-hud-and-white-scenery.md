---
status: re-first
titles: [hd]
updated: 2026-11-27
summary: "HD Zone start pair (hd/zone-vineta-k-z5): board glyph, three over-bright hull panels, grid flaps and countdown tone remain"
---

# HD Zone start pair: what is left after the Zone frame's light and post chain

The adopted pair `hd/zone-vineta-k-z5` (Vineta K, Zone start, stage 1 Sub
Venom) moved 53.04 -> 28.77 mean grey on the sky dome, the light-bar term, the
zone equaliser and the pre-GO HUD gate, and 28.77 -> 7.42 (rmse 0.165 ->
0.085) on the Zone frame itself
(`docs/ghidra/functions/ps3-hdfury-eu/zone-frame.md`): the Zone surface lit by
the stage's ambient instead of the circuit's lightmap (17.04), then the Zone
`HDR and Bloom` block and the resolve's `Bloom feedback` `-0.4` (7.42). The
white scenery, the sky's 17 % and the dome's flat band are closed by those
three. By region after: sky 0.10, scenery 1.93, road and pads 1.50, the board
1.32, the craft 1.92.

## Open

- **The gantry board shows glyphs ours does not** (ours' board assembly starts at tick 63). A `--countdown` pair now records the
  original's phase and seconds in it (`original.race_phase`), so ours' `--ticks` can be matched to that moment; it is still 0
  (`docs/tools/compare-page.md`, "The countdown frame"). 1.24 mean grey on the 2.4 s grab.
- **Countdown tone and grid flaps, not modelled.** The original's adapted luminance climbs 0.311 to 0.359 over the 3 s countdown and ours holds one
  tone; the original shows two raised flaps on the hull's top at the grid and ours draws none (same page; the flaps not checked
  against any other grid work).
- **The hull's light in a Zone frame: read, three panels overshoot.** The probe tint
  (twice the stage's `Track.Base Colour Highlight`) and `zoneship01`'s rim term
  (`fresnelBase` `0.1234`, `base + (1 - base) * (1 - N.V)^5` added untinted) are wired:
  pair 7.42 -> 7.27 -> 6.57, craft box 38.56 -> 33.58 -> 15.07, the flat pod panels
  within 1 byte (`docs/ghidra/functions/ps3-hdfury-eu/zone-hull-light.md`). Left: the
  nose, the flaps and the pods' outer sides are too bright in ours (left pod box
  `(98.7, 110.3, 124.5)` against `(88.6, 98.7, 111.2)`); the nose's excess predates
  the rim and is ours' reflection there. Next reads on that page, "Still open".
- **The rim on Omega and 2048: applies, not wired.** Both ship `zoneship01` with
  `fresnelBase` declared and authored (Omega `0.12269939`, 2048 `0.1234`); their
  programs are unread and neither hull draws the term (same page, "2048 and Omega").
- **A pulse's history in the feedback term.** The resolve's feedback buffer is
  the zoom ring's history target; ours resolves against the frame's own
  quarter-res scene, so a damage pulse in a Zone race is not in the term.
- **Radial bloom.** A Zone race enables it (contribution `0.04`); the port has
  no radial pass.
- **Detonator and Zone Battle** take their own arms of the same block
  (zone-frame.md, "The Zone arm"); `oag_hd::race::ZONE_FRAME` is applied in
  Zone alone.
- **2048 on HD-lineage circuits draws the same grey Flash sky** (zone-sky.md,
  "2048 and Omega"): checked, applies, not wired; the port draws the circuit
  dome there. Omega's Flash frame shows peach instead: differs, unread.
- **2048 and Omega have no Zone frame law read.** Both end the Zone frame in
  `wo_composite_zone_fp` (Omega also `wo_composite_zone_hdfury_fp`), unread.
- **`_ne.a * zoneBaseInner`.** Two of Vineta K's
  `diffuse_normal_specular_emmissive` Zone variants add it; which variant a
  chunk runs is unread, and the frame shows none.

## Next Steps

1. Once the 2026-11-27 countdown grab is adopted, run the 8-phase `--flame-clock` sweep on it (engine-flare.md, "The cold-boot pairs, regrabbed
   with the clock").
2. Match ours' `--ticks` to the recorded countdown seconds and see whether the board then draws the original's glyphs.
3. The hull's three bright panels: decode the normal and tangent streams of the live
   `zoneship01` draws (641..644, `va-06xx-*.bin`) and compare `N` on the nose with
   ours' `hull_normal`; check whether the flaps carry their own node matrix
   (zone-hull-light.md, "Still open"). Recipe:
   `HOOK_TEX_RE='0\.69804' scripts/rpcs3-hd-zone-frame.py --small 2500`.
4. Read `zoneship01`'s program on Omega (GCN) and 2048 (GXP) and wire `fresnelBase`
   on their Zone hulls.
