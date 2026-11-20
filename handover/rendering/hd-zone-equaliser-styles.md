---
status: re-first
titles: [hd, omega]
updated: 2026-11-20
summary: "HD Zone billboard equaliser: style 0 (stages 0-2) wired; styles 1-8 and a lit-target live read open; Omega's loop unread"
---

# HD Zone billboard equaliser: the styles past stage 2

`docs/ghidra/functions/ps3-hdfury-eu/zone-equaliser.md` reads the stage pass's draw into the shared
512 x 512 target; `oag_raceplay::zone_equaliser` draws style 0 (stages 0 to 2) and the bare grid for
every other style.

## Open

- Styles 1 to 8 (`g_ZoneEqualiserStyle` from requested stages 3 to 14): branches of
  `Environment_UpdateStageBlend` after `0x003dbc9c`, unread. Style 6 runs 32 iterations, style 1 starts
  at `held[1]`, styles 4 and 6 compare `raw[2]` against 0.7.
- What `FUN_005e5d40` and the texture at stage block `+0x3370` lay over the bars (ours draws the grid).
- A live read of the target with the bars lit: four boots caught the bands live once (no texels) and the
  bare grid twice (silence). Recipe on the page.
- Omega: `Scene_PrepareFrame_q` (`0x015c8ea0`) reads HD's `-508.0`/`526.0` pair; its loop and binding
  are unread, `Adverts::zone_equaliser_bars` is `false`.

## Next Steps

1. One RPCS3 boot with the race released and music on (`scripts/rpcs3-hd-zone-equaliser.py`, Zone is
   four RIGHTs from Single Race; read the RACE TYPE row before `cross`), compare the target to the law.
2. Read style 7 (stages 3, 4 and 11: the most stages after style 5) and style 5.
