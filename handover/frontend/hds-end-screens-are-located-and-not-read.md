# HD's end screens: read and drawn (Results/Menu); Rewards/Podium still open

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

- **`EndRace Rewards` is read (present on `DATA02`-`05`) but not drawn.** Out
  of this pass's own scope. HD draws its own loyalty total directly on
  `Results` (a block Pulse has no equivalent for), which is not how Pulse's
  Results->Rewards->Menu flow works at all - untangling the relationship
  between that block and the separate `Rewards` screen is unresolved.
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
