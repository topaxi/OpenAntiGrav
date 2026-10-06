# HUD legibility and HUD skins across titles

Investigation done 2026-10-06, no behaviour changed; findings and confidence on
[hud-skins.md](../../docs/ui/hud-skins.md). Short version: Pulse's glyph ink
matches the original at 1x; the poor reading is a 3-8x linear magnification of
a 480x272 raster whose contrast is a 7 px baked halo, plus 0.6-scale readouts
that lose ink in our sampler. HD, Omega and 2048 ship Pulse's HUD atlas at 4x
but not Pulse's typeface at high resolution.

## Open

- The bright-backdrop frame (boost pad) behind the washed-out bottom-left
  readouts is not reproduced; the bloom term over the HUD is measured only on
  dark-backed frames (mean 0-1.5, max 29 of 255).
- The original's sampling of text at fractional scale (`BestTime` at 0.6) is not
  read; ours loses the separator dots and 28 % of the body peak.
- Pure was measured at font-header level only (27 px and 12 px outlined fonts);
  no Pure frame was captured.
- `arcade_hud_old.xml` and `hud_timers.xml` (HD, Omega, 2048) are unreferenced
  from data; whether any executable draws them is unchecked.
- 2048's route to `2097_hud`/`wo3_hud` and Omega's default HUD are unmeasured.

## Next Steps

Ranked by player value:

1. **Fix the 0.6-scale text** (small): read how Pulse's text draw samples a
   scaled glyph (`Gfx_` text path, PPSSPP software frame of `BestTime`), then
   match it; verify with the glyph-profile method, not region MAE (the original
   differs from itself by 19-26 levels over those regions).
2. **A HUD scale filter for raster-HUD titles** (nearest, integer, sharp
   bilinear), config first: the measured 3x linear stretch is the main
   difference from a PSP screen. Already queued as the lead's HUD scale-filter
   lane; this page is its evidence.
3. **Config `[hud] skin`** and a `skins: &[HudSkin]` table on `Title` (design in
   hud-skins.md part C), HD's `2097` and `wo3` first (same shape as the default,
   unlock rows `loyalty 6000` and `10000` read), then the same two on Omega.
4. **Pulse sprites from the 4x sheet** when HD, Omega or 2048 is also mounted:
   an asset substitution, not a skin; sprites only, the text stays 25 px. Needs a
   product decision first (a Pulse draw depending on a second disc).
5. Reproduce the bright frame on both sides (`psp-drive.py place --pad`) to size
   bloom over the readouts; capture one Pure frame.
