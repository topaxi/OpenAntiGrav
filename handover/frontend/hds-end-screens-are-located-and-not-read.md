# HD's end screens: read and drawn (Results/Menu/Rewards); Podium still open

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
[`oag_ui::endrace::hd`](../../crates/ui/src/endrace/hd.rs) draws Results (the
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
  `oag_ui::endrace::hd::hd_rewards_draw_list`, reachable only by
  `--menu-page endrace-rewards`; the live flow stays Results -> Menu.
  **Still open under it**: `DFENGINE.SPRX` (encrypted) was not grepped; a
  live RPCS3 end-of-race walk is what would lift 85. The loyalty row draws
  nothing (HD's loyalty law is unrecovered), and so does the `Results`
  loyalty block, which is where HD actually shows loyalty - recovering that
  law from `EndRaceResults_Screen.cpp`'s code (strings at `0x7845b0`) is the
  next useful step for either screen.
- **`EndRace Podium` (`DATA05`/`DATA06` only) is inventoried, not modelled.**
  Its three `pod_head.{1,2,3}` widgets all carry the identical idstring
  `IG_HUD_1ST`, which reads as an authoring placeholder rather than something
  this build could draw correctly - see the formats page. Its eight badge
  panels are an achievement/online system with no analogue in this project's
  `oag_race`/`Session` state.
- **Which copy the runtime actually loads is still unresolved** - the same
  open question [hd-frontend.md](../../docs/formats/hd-frontend.md) records
  for `skin.xml`'s six copies. This build serves `DATA02`'s, by
  `oag_assets::Archives::holder_of`'s own mount order, not a measurement.
- **No live capture reaches a finished `EndRace Results` yet.** Xvfb `:94`
  with `xdotool` (mouse-only, `WAYLAND_DISPLAY` unset - see
  `docs/architecture/menus.md`'s "DISPLAY against GRAPHICS" for why that
  matters) reached a real race launch (`Main Menu` -> `RACE CAMPAIGN` ->
  `Grid Selection` -> `Cell Selection` -> `LOADING... VINETA K` -> racing),
  the same point `docs/ui/campaign-screens.md`'s own 2026-09-21 walk reached
  by an identical route. The race itself renders at ~5s/frame in this
  sandbox (scene built in 53.7s, first frame 5.2s after) - independently
  confirmed twice now (that walk's own 107.6s/741ms-8056ms-per-frame
  numbers, and this pass's) - so a full race is hours of wall clock here, not
  a code bug. The exact command for a machine with a working display is in
  `docs/ui/endrace-screens.md`'s own Live section.
- **2048 and Pure were not looked at.** Neither `oag_title::FrontEnd`
  instance's `endrace_entry` is anything but `None` - a gap, not a
  measurement that either ships no such screen.
- **The `Grid{n}.h` white-ink variant (three widgets, not four or ten) is
  unexplained** - see the formats page's own confidence-60 row. Left
  undrawn; whoever next opens this screen with more time or a capture is
  the one to settle what subset of columns it is for.
- **Row geometry is chosen, not measured** - the disc authors ten row slots
  and every `Grid{col}.{row}` cell at `x="0" y="0"`, so this build divides
  the grid's own measured frame by `oag_gameplay::MAX_SHIPS` (eight) rather
  than ten. See `oag_ui::endrace::hd::row_y`'s own doc.

## Next Steps

1. Recover HD's loyalty law from `EndRaceResults_Screen.cpp`'s code (the
   `loyalty1.1`/`loyalty1.2`/`loyalty2`/`ER_POINTS` strings at `0x7845b0` in
   `EBOOT.elf`; use `scripts/ps3-toc.py` for xrefs). That fills the `Results`
   loyalty block, the screen HD actually shows it on.
2. A live RPCS3 walk to a finished race would confirm Results -> Race End
   Save -> Menu with no Rewards in between, and settle which `skin.xml`/
   `EndRace_Definition.xml` copy is served.

## From the HANDOVER.md index (moved 2026-09-25)

2026-09-21; `Grid{col}.{row}` is four columns by ten rows, not eight by ten; no live capture reaches a finished `EndRace Results` yet (a full race renders at ~5s/frame under this project's own Xvfb sandbox), substituted by a disc-backed, GPU-free ground-truth test
