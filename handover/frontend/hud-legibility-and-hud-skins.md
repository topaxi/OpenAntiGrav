# HUD legibility and HUD skins across titles

Investigation done 2026-10-06; the halo and the stretch filter landed the same day
(`hud-crisp` lane), findings and confidence on
[hud-skins.md](../../docs/ui/hud-skins.md) (part D is what was built). Short version: Pulse's glyph body
matches the original at 1x, but our glyph quad is clipped to the metric box so
the baked outer halo is cut off, and the authored `borderExtendPixels` (HUD 5,
HUDSmall 3) is read by nothing; on top of that a 480x272 raster is stretched
3-8x with linear filtering. HD, Omega and 2048 ship Pulse's HUD atlas at
exactly 4x but not Pulse's typeface at high resolution.

## Open

- The halo is about half closed against PPSSPP (rings 2-4 still 0.03-0.11
  shallower, far field lighter), and `best` peaks at 200-218 in the original
  against our 254; the blend is not derived. What the original does with a
  tightly packed face (Pulse PSP's menu faces, 2048's NEOSANS) is not captured;
  our reach cut to half the gap is chosen, not measured.
- The bright-backdrop frame (boost pad) behind the washed-out bottom-left
  readouts is not reproduced; the bloom term over the HUD is measured only on
  dark-backed frames (mean 0-1.5, max 29 of 255).
- Pure: drawn and screenshotted, no PPSSPP Pure frame measured.
- `arcade_hud_old.xml` and `hud_timers.xml` (HD, Omega, 2048) are unreferenced
  from data; whether any executable draws them is unchecked.
- 2048's route to `2097_hud`/`wo3_hud` and Omega's default HUD are unmeasured.
- `hud_scale` has no settings row and no per-title default; `sharp-bilinear`
  everywhere raster is a chosen default (Deck: `integer` shrinks the HUD 25 %).

## Next Steps

1. ~~Draw the halo (`borderExtendPixels`)~~ and ~~a HUD scale filter for
   raster-HUD titles~~: landed (`hud-skins.md` part D). Optional: a settings row
   for `hud_scale`, as a follow-up.
2. **Config `[hud] skin`** and a `skins: &[HudSkin]` table on `Title` (design in
   hud-skins.md part C, chosen not measured), HD's `2097` and `wo3` first (same
   shape as the default, unlock rows `loyalty 6000` and `10000` read), then the
   same two on Omega.
3. **Pulse sprites from the 4x sheet** when HD, Omega or 2048 is also mounted:
   an asset substitution, not a skin; sprites only, the text stays 25 px. Needs a
   product decision first (a Pulse draw depending on a second disc). Not built
   in the `hud-crisp` lane.
4. Low: the 0.6-scale readout loses ink only below about 800x450; reproduce the
   bright frame on both sides and capture one PPSSPP Pure frame.
