---
status: re-first
titles: [hd, omega]
updated: 2026-11-06
summary: "HD Eliminator's race line and lap hexagon are drawn; the TARGET header and the ranked score rows need HD's scoring law (craft+0x6f80) first"
---

# HD's Eliminator HUD: the score rows wait for the scoring law

2026-11-06, `eliminator-hud`. Evidence: `docs/ui/hud.md` ("HD's Eliminator race line"),
`docs/ghidra/functions/ps3-hdfury-eu/eliminator-hud.md`. Guarded by
`crates/raceplay/tests/hd_eliminator_hud_ground_truth.rs`.

## Open

- **HD's Eliminator is score-based; this build's is Pulse's kill count.** The original prints
  `TARGET ( 200 )` over ranked `NAME ( score )` rows, scoring points for damage, eliminations and
  laps and losing points on an elimination (the intro board's own text). The score is
  `craft+0x6f80` (a float read as an int; tie-breaks `+0x6f7c`, then `+0x628c`); the target is
  `g_GameState+0x20`. Recover the writers of `craft+0x6f80`, then the rows (`0x0008c6f8`, not
  decompiled), the header (`Hud_BindWidgets` `0x0009d494`) and the rank brackets and
  `HighlightBar`. Until then the rows, `TARGET`, `MPWeaponIcon0-7`, `VoiceCom0-7` and
  `ScoreFeedBack0-15` stay undrawn: a row of `( 0 )` would stand in for data the original has.
- **The marker half turn:** `Hud_UpdateRaceLine` rotates a marker by pi when `craft+0x6ad8` is
  set; the byte is unidentified (it also gates the absorb in `weapon-ai-fire.md`). Markers are upright.
- **Not captured:** a frame after a kill or any score change; the split-screen Eliminator pair
  (`splitscreen_elimination_hud_{0,1}.xml`, same widgets, unchecked). The Eliminator weapon-pad
  disarm time was not seen in the frames.
- **Omega: checked, applies, not wired for `LapPanel2`** (its `ALWAYS_ON` is "HD's list"); the
  race line draws there through the shared path. Not seen on an Omega frame.
- **Compare pair:** the countdown pair is in the lane's scratch only (original frame and ours, with
  the region metric); adopting it into `data/reference/compare-page/hd-race/` is the lead's.

## Next Steps

1. Find the writers of `craft+0x6f80` (`get_xrefs_to` is unreliable here: run
   `scripts/ps3-toc.py toc` on every caller first) and write the scoring law up.
2. Decompile `0x0008c6f8`; wire the rows through `oag_hud::kill_tags`'s ranking, with HD's score.
3. Capture a frame after a score change on RPCS3 (Autopilot is fine for getting there).
