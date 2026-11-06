---
status: measure
titles: [hd]
updated: 2026-11-06
summary: "Which archive copy HD loads is unresolved; Results lists every craft by team now, open: whether times refresh, the badge, Omega still on the built-in scoreboard; Podium is multiplayer-only"
---

# HD's end screens: read and drawn (Results/Menu/Rewards/Podium); Podium multiplayer-only

2026-09-18: located, not read. **2026-09-21: `EndRace Results`/`EndRace Menu`
read widget by widget and drawn.** Full write-up in
[hd-endrace-screens.md](../../docs/formats/hd-endrace-screens.md) (reading) and
[endrace-screens.md](../../docs/ui/endrace-screens.md)'s own HD section
(drawing, captures, live-walk findings).

**The one to know, updated**: `oag_title::FrontEnd::endrace_entry` is now the
per-title axis this thread's own "Next Steps" called for - Pulse keeps
`Data\Plugins\PI001\GUI\EndRace_Definition.xml`, HD's is
`oag_hd::frontend::names::ENDRACE_DEFINITION`
(`Data\Plugins\Frontend\Gui\EndRace_Definition.xml`, five of seven archives,
no two alike by MD5). `oag_game::endrace::load`/`load_hd` mirror
`crate::campaign::load`/`load_hd`'s own title dispatch.
[`oag_ui_screens::endrace::hd`](../../crates/ui-screens/src/endrace/hd.rs) draws Results (the
whole field's own standings, `Grid{col}.{row}` - **four columns by ten rows**,
not the "eight rows by ten columns" this thread originally guessed before the
reading pass) and Menu (one `<Block>` per option, at its own authored
position). Two real bugs turned up by a live `--menu-page` capture rather than
by reading alone, both fixed: HD's own finishing-place idstring is
`ER_{n}PLACE`, not Pulse's `ER_{n}STP`, and the Target/loyalty placeholder
text was leaking through an unfiltered fallback arm. See the drawing doc's own
"Two real bugs" section.

Verified end-to-end without a GPU or a live race in
[`crates/game/tests/hd_endrace_ground_truth.rs`](../../crates/game/tests/hd_endrace_ground_truth.rs):
a real HD single race, autopiloted to its own finish (headless, ~18s), its
real `Board`, the real disc's `EndRace_Definition.xml`, and
`hd_results_draw_list` fed both.

## Open

- ~~`EndRace Rewards` is read but not drawn.~~ **2026-09-25: settled as
  never entered by the original** (no redirect in any XML of any archive
  names it; the EBOOT has no `EndRace Rewards` type string, no
  `EndRaceRewards_Screen.cpp` and zero `reward` strings - confidence 85,
  `docs/formats/hd-endrace-screens.md`). Drawn off its own widgets by
  `oag_ui_screens::endrace::hd::hd_rewards_draw_list`, reachable only by
  `--menu-page endrace-rewards`; the live flow stays Results -> Menu.
  **Still open under it**: `DFENGINE.SPRX` (encrypted) was not grepped; a
  live RPCS3 end-of-race walk is what would lift 85. The `Rewards` loyalty row
  draws nothing, correctly (HD never enters it).
- ~~HD's loyalty law is unrecovered~~ **2026-10-02 (`hd-endrace-loyalty`):
  recovered and drawn on `Results`.** `Race_ComputeLoyaltyAward` (`0x00023f98`,
  82) and the Results ticker (`0x00224488`/`0x00225030`) agree on every rate;
  they are **not Pulse's** (`ps3-hdfury-eu/endrace-loyalty.md`).
  `oag_hd::loyalty` is the law, `Session::build_endrace` banks it, `Results`
  draws `<award> POINTS` and the total. **No bar** - HD's Results code has no
  `loyaltybar`. **Still open under it**: no live RPCS3 run (static, so every
  row <= 84); perfect laps and perfect zones are never counted by this project;
  a race with no cell uses the `Easy` rung (chosen); the ticker's animation
  is not reproduced; what byte `0x009384e1` and `g_GameState+0xe4` are.
- ~~`EndRace Podium` is inventoried, not modelled.~~ **2026-10-06
  (`hd-endrace-podium`): read and drawn** off `DATA05`'s copy
  (`load_hd_podium`) by `--menu-page endrace-podium`, plinths included. The
  slot setter (`0x00220108`, 78) and caller (`0x00220820`, 76) are in
  `ps3-hdfury-eu/endrace-podium.md`, and **its entry is found**:
  `FUN_000459d8` in the `MPRaceManager` family goes to it at `0x00045e4c`
  (74), so it is multiplayer-only and the live single-player flow correctly
  stays Results -> Menu. **Still open under it**: what starts the `+0x34ec`
  deadline, the `DAT_008a5644`/`DAT_008b0458` bytes, the ship portraits
  (`pod_img`) and badge panels, its title string (`FE_ENDRACE_PODIUM`, only in
  `DATA05`/`06`'s table), and what `+0x50` of a race record is (a team idstring on a race, 75; the podium prints it as a
  literal name). Omega: checked, applies, not wired
  (Results, Menu, Podium, no Rewards in all three copies).
- **Which copy the runtime actually loads is still unresolved** - the same
  open question [hd-frontend.md](../../docs/formats/hd-frontend.md) records
  for `skin.xml`'s six copies. This build serves `DATA02`'s, by
  `oag_assets::Archives::holder_of`'s own mount order, not a measurement.
- ~~**No live capture reaches a finished `EndRace Results` yet.**~~ **2026-11-06
  (`hd-endrace-rows`): reached headless on three routes** (Fury campaign, HD campaign, RACEBOX
  single race) with `oag-game --drive ... --autopilot`, the real `Session` and no window, a
  race in about two minutes; the scripts and frames are in `docs/ui/endrace-screens.md`'s "HD's
  `Results` names every craft". A live RPCS3 run of the original is still what would lift the
  static readings.
- **2048 and Pure were not looked at.** Neither `oag_title::FrontEnd`
  instance's `endrace_entry` is anything but `None` - a gap, not a
  measurement that either ships no such screen.
- **The `Grid{n}.h` white-ink variant (three widgets, not four or ten) is
  unexplained** - see the formats page's own confidence-60 row. Left
  undrawn; whoever next opens this screen with more time or a capture is
  the one to settle what subset of columns it is for.
- ~~**Row geometry is chosen, not measured**~~ **Read, 2026-09-28, for the
  race family**: `EndRaceResults_LayoutGrid`/`_FillRaceRows` (`0x0022c068`/
  `0x0022b688`) put row `r` at `96 + 45 r`, columns at `40`/`200`/`545`,
  stretch the frame to a `487` bottom bar and hide `GridBottomBlock` - the
  file's frame is the Time Trial layout, which is why the 8th row used to sit
  on the footer. See
  [endrace-results-grid.md](../../docs/ghidra/functions/ps3-hdfury-eu/endrace-results-grid.md).
  **Still open under it**: Time Trial / Speed Lap rows keep the chosen
  `row_y` (fillers `0x002239a8`/`0x00227b30` unread); `Gridi.r` (the row's
  team badge) is not drawn. ~~`Grid1.r` unidentified~~ **2026-11-06 (`hd-endrace-rows`): a team
  idstring, drawn** (the live Results listed no names before), with `ER_RACING` for a craft
  still racing; see `docs/ui/endrace-screens.md`'s "HD's `Results` names every craft".
  **Open under it**: whether the original refreshes the other crafts' times while `Results` is
  up (ours holds the board at the player's finish); Omega's finished race still shows the
  built-in overflowing `scoreboard::Overlay` because its `endrace_entry` is `None`.
- **2026-09-28: `EndRace Menu` draws its cursor the executable's way** - the
  focused option Block turns `ActiveColor` (`0xff8ac0ca`), eases 60 wider and
  blinks a 32x32 arrow (`Block_Update`, `0x0018d588`,
  [menu-blocks.md](../../docs/ghidra/functions/ps3-hdfury-eu/menu-blocks.md)),
  and the options step in screen order. Live-verified on Xvfb with pad and
  pointer; see `docs/ui/endrace-screens.md`'s HD section. **Open**: the
  original's default focus on HD is unmeasured (this build keeps
  `RETURN TO GRID`); the `Endrace Difficulty` `<List>` at the same `y` as
  `race_again` is still undriven.

## Next Steps

1. ~~Recover HD's loyalty law~~ Done 2026-10-02, see Open above. Next on
   it: count perfect laps/zones in the race so the award is whole, and
   animate the ticker if it ever matters.
2. A live RPCS3 walk to a finished race would confirm Results -> Race End
   Save -> Menu with no Rewards in between, and settle which `skin.xml`/
   `EndRace_Definition.xml` copy is served.

## From the HANDOVER.md index (moved 2026-09-25)

2026-09-21; `Grid{col}.{row}` is four columns by ten rows, not eight by ten; no live capture reaches a finished `EndRace Results` yet (a full race renders at ~5s/frame under this project's own Xvfb sandbox), substituted by a disc-backed, GPU-free ground-truth test
