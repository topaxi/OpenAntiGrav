# HUD legibility and HUD skins across titles

Investigation done 2026-10-06, no behaviour changed; findings and confidence on
[hud-skins.md](../../docs/ui/hud-skins.md). Short version: Pulse's glyph body
matches the original at 1x, but our glyph quad is clipped to the metric box so
the baked outer halo is cut off, and the authored `borderExtendPixels` (HUD 5,
HUDSmall 3) is read by nothing; on top of that a 480x272 raster is stretched
3-8x with linear filtering. HD, Omega and 2048 ship Pulse's HUD atlas at
exactly 4x but not Pulse's typeface at high resolution.

## Open

- How deep the original's halo is, and its blend over the extended area: only
  its presence is measured (backgrounds differed between the two sides).
- The bright-backdrop frame (boost pad) behind the washed-out bottom-left
  readouts is not reproduced; the bloom term over the HUD is measured only on
  dark-backed frames (mean 0-1.5, max 29 of 255). The 3x linear stretch has no
  comparative number against a sharper filter yet.
- Pure: no frame attempted; font headers only.
- `arcade_hud_old.xml` and `hud_timers.xml` (HD, Omega, 2048) are unreferenced
  from data; whether any executable draws them is unchecked.
- 2048's route to `2097_hud`/`wo3_hud` and Omega's default HUD are unmeasured.

## Next Steps

Ranked by player value:

1. **Draw the halo: extend each HUD glyph quad by `borderExtendPixels`** (5
   for `HUD`, 3 for `HUDSmall`; read it from the language definition, not a
   constant), then measure against a fresh PPSSPP frame on a flat bright
   backdrop. Small change in `crates/game/src/render/text.rs`, and it may
   resolve `hud.md`'s "outline reads crisper" note. Menu fonts declare 3 and 5
   too; check they are not harmed.
2. **A HUD scale filter for raster-HUD titles** (nearest, integer, sharp
   bilinear), config first: the 3x linear stretch is the second factor. Already
   queued as the lead's HUD scale-filter lane; this page is its evidence.
3. **Config `[hud] skin`** and a `skins: &[HudSkin]` table on `Title` (design in
   hud-skins.md part C, chosen not measured), HD's `2097` and `wo3` first (same
   shape as the default, unlock rows `loyalty 6000` and `10000` read), then the
   same two on Omega.
4. **Pulse sprites from the 4x sheet** when HD, Omega or 2048 is also mounted:
   an asset substitution, not a skin; sprites only, the text stays 25 px. Needs a
   product decision first (a Pulse draw depending on a second disc).
5. Low: the 0.6-scale readout loses ink only below about 800x450; reproduce the
   bright frame on both sides and capture one Pure frame.
