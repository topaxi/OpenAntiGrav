# The HUD's four remaining items

The layout was never an RE problem - Pulse ships five layouts as `Data\XML\*_HUD.xml`. What is left is on [hud.md](../../docs/ui/hud.md).

## Open

- ~~**The mode-code string-key substitution rule is read for one widget pair.**~~
  **Closed 2026-09-30 for that pair**: all four captions
  (`IG_HUD_BRONZE`/`SILVER`/`GOLD`/`RECORD`) are wired, `RECORD` against
  `min(stored best, <RaceTimes>/<LapTimes>)`; see
  [hud.md](../../docs/ui/hud.md#medal-targets-closed-2026-09-28). **Still
  chosen, not measured**: the stored best is keyed by circuit, mode and class,
  where the original's is per team; and no live frame has a stored best on it.
  The other 25 of the binary's 38 `IG_HUD_*` keys were not chased.
- 25 of the binary's 38 `IG_HUD_*` keys still appear in no layout and have no
  read mechanism (down from 26, now that `IG_HUD_RECORD` is accounted for).
- Zone, Eliminator, `<Mode3D>` and text outlines are scoped out (details on `hud.md`)

## Next Steps

- **Measure a stored best on a live frame**: set a Time Trial best on the
  original (a finished race), then read `PLAYER_HUD+0x30`/`+0x34` on the next
  one - it would settle the `min` and the per-team keying
  (`Profile_GetBestRaceTime`, [race-progress.md](../../docs/ghidra/functions/psp-pulse-usa/race-progress.md#the-stored-best-fun_088091a0-and-the-record-branch-closed-2026-09-30)).
- Read the other 25 `IG_HUD_*` keys' triggers, one at a time.
