# Wipeout 2048's loading screen

**2048 ships HD's loading-screen class, and the retail executable starts a different kind of it for a race load.**
Confidence 90: a call-site census and a decompile, and since 2026-11-16 three captures of a campaign race load on
Vita3K with the screen's own fields read out of the guest's memory. The evidence is
[vita-2048-eu-v104/loading-screen.md](../ghidra/functions/vita-2048-eu-v104/loading-screen.md) (the class and the
census) and [vita-2048-eu-v104/race-load.md](../ghidra/functions/vita-2048-eu-v104/race-load.md) (what the race load
draws and its meter's law). `oag_2048::TITLE.loading` is `Some` since 2026-11-16 (lane `vita-race-load-wire`): the race load, kind 4, is built (box, rule, meter and law, the launched event's card with a game tip and the mode page, below); the one that is authored in full, kind 7, is never started and not drawn.

## Two screens in one class

| kind | what it is | reached |
| ---: | --- | --- |
| **4** | the race load: the launched event's own card over the blurred front end (a still for the attract demo alone), a `Blue2048`/`White2048` percentage box, `FE_LOADINGDOT`, a `%d%%` readout and a seven-triangle meter | by every race start (`FUN_8115f09a`, `FUN_811494fa`) |
| **7** | HD's screen: caption naming the circuit, one of five illustrated features, a dotted progression bar | **by no call site** of the v1.04 executable |

Every call of the starter `LoadingScreen_Start` (`0x812845ce`) was censused: eight calls, with `0` (presents), `4` (race
start, three sites), `6` (boot and shutdown, three sites) and `8` (the epilepsy still). The argument is a literal at each.

**Why nothing is wired.** Kind 7 was wired once (the layout is HD's operand for operand, minus the mode icon, the ten
illustrations and six marks resolve, a white ground) and taken out again: it is a screen no 2048 player has seen, and a
stand-in for the one they have. The evidence page keeps its operands.

## The race load, as measured

Queens Mall (`2048 - Event 2`), three runs, 2026-11-16; every operand, address and the live numbers are on
[race-load.md](../ghidra/functions/vita-2048-eu-v104/race-load.md).

1. **The backdrop is the front end's own event card**, drawn by the same painter with the same page state, over the
   front end's blurred campaign map. Nothing is handed over in the original: the launched event is the current game
   mode, and the painter takes it. Only the attract demo's mode object answers the test that picks a still instead
   (vtable slot `+0x60`: `1` for `ATTRACT`, `0` for the base and the six other mode classes).
2. **A game tip replaces the track photo** for the first seconds (when the profile's `TutorialsActive` is `ON`): a
   398 by 334 panel at (16, 92) with a `?` mark, `TUT_TIP_HEADER`, a picture, a title and a body from a 37-row table in
   the executable; the row is picked by the event (its class, its weapon pads) and the profile's first loads.
3. **Then the card turns to a mode page and slides to the middle.** Once the race manager exists the card gains a
   page describing the mode (`Tut_Event_Race_1` and its siblings under the mode's name and icon), turns to it, drops
   the tip and the photo, and moves 202 px left in thirteen draws (x 420 to 218).
4. **The percentage box**: a 70 by 70 `Blue2048` fill at (16, 16) with `%d%%` in `White2048` at scale 0.6 centred on
   (51, 38), `FE_LOADINGDOT` at (105, 30), a 338 by 4 rule at (86, 82) that does not grow, and seven triangles at the
   bottom right (centres x 895 down to 715, y 475) that pulse and fill in from the left with the fraction.
5. **The meter's law**: shown columns of 166, `+rate` each draw at 30 draws a second, clamped to a ceiling. It starts
   at ceiling 57 % and rate 0.115; the loader raises the ceiling to 75, 95 and 98 %, each time multiplying the rate by
   `(old gap + new gap) / new gap`; the race manager then freezes the rate; the end of the load sets 100 %. Live: about
   10 % after three seconds, then 12.7 % a second to 98 %, a hold, 100 % for an instant.

The frames are not in the repository (they are the original's pixels). `scripts/vita-2048-race-load.py` reproduces
them with the memory samples in about a minute on a private Vita3K profile.

## What is built

| piece | where | state |
| --- | --- | --- |
| box, caption, readout, rule | `assets/fallback/Data/XML/LoadingScreen_2048.toml` (`screen-layout` `fill`, `text` kinds), named by `oag_2048::loading::LOADING` | built, operands pinned by `loading::vita_tests` |
| meter | `oag_ui::loading_meter` (`Draw::SolidTriangles`), the layout's `meter` kind | built |
| law | `oag_title::loading::Rules` (rescale on a raise, a step per 1/30 s, rate frozen at the last stage) on `oag_2048::loading::PROGRESSION`; `loading::fill` | built, reproduces the rates of two captures (`fill::tests`) |
| the launched event's card | `oag_ui::frontend::CardPaint` (the one painter, `Frontend::draw_event_card` calls it too); `Frontend::launched_card` keeps the card a Launch pressed; handed over through `Session::follow_launch` and `Stage::race_loading` | built |
| game tip | `oag_2048::tips::TIPS` (the 37-row table, keys only), picked by `oag_title::loading::GameTips::candidates`, drawn by `CardPaint::draw_tip_panel` | built **without** the weapon-pad rows and without the profile-flag rows (below) |
| mode page and slide | `CardPaint::draw_mode_page`, `loading::card` (turns on the load's last stage, slides `-(1 - (1 - s)^2) * 202`, `s += 0.08` a draw) | built |

**Chosen, not measured.** Which of this build's load stages is which milestone (stages 1 to 3 are 0.75, 0.95 and 0.98,
stage 4, the crafts built, is the race manager that freezes the rate and turns the card). The ground behind the card is
`White2048` (the original's is the blurred campaign map, which a launch here has no map for). The bold face's scale carries
a calibration of 0.927 (this build's `Default` face scaled by the line-height ratio is 8 % wider than `NEOSANS_BOLD`; one
mode-page line, 370 units in the original). The tip's draw is a seeded pick (`races_launched`), not `rand()`.

**Open** (none changes what the screen reads as): the weapon-pad tip rows (need the event's active weapon pads, which no
`EventCard` carries), the first-load rows of a profile (`GameTip_Pick` steps 1 and 2: a counter and per-class flags in the
profile, not persisted here), the controls diagrams (rows 34 to 36, `0x8106d3ce` unread), the prototype challenge's mode page
(`Tut_Proto_Event_<team>`), the HD and Fury cells' own panel (`CampaignEventCard_DrawCellPanel`: a racebox or cell launch draws
the box and meter over the ground and says so in the loader report, never a made-up card), and the **attract demo's still**
(`Title_Screen.gxt` at (-32, 144), 1024 by 256; it ships, it is not wired: the demo's load draws the box and meter only).
Replays and `--race` start loads with no card likewise.

## What ships (asserted)

All nine pictures kind 4 names resolve, and all sixteen kind 7 names (six marks, **both arrows included**, and ten
illustrations): `oag-2048` `loading_ground_truth::every_picture_the_race_load_names_is_in_the_package` and
`the_art_of_the_dormant_feature_screen_ships_whole`. The Omega page's "the two arrows were not listed" and the earlier
"four of six marks" were a census gap. The illustrations are 1024 by 512. The loader picks `_fury` on a byte that is
`0` in the image with no writer (confidence 70), so the base styling is the only one kind 7 reaches.

## Kind 7's operands, for the record

HD's, compared in `LoadingScreen_Draw` (`0x812830c8`): rules (160, 110) and (160, 975), 1600 by 8; caption arrow (160, 73);
caption in `Title` at (194, 62); labels at (160, 48), (160, 125), (960, 125), (160, 670), (1510, 670); four bracket sets; the
picture 896 by 480 at (180, 190) with uv 0.875 by 0.9375; name and heading rows at (175, 175) and (975, 175); the paragraph at
(975, 215), wrap 780, pitch 40; the brand is HD's literal `WIPEOUT(R) HD` (`0x814d0e24`); the mode icon's panel is a label and
brackets with nothing inside. The deck is HD's row for row (mode `6`: two, `0xe`: one, `0xd` and `0x15`: `0 1 3`, `8` and
`0x14`: five, else `rand() % 4`). 2048 reads `FE_HD_BG` and draws it nowhere in kind 7; the picture's ground is white, so the
ground that makes a base picture's overhang invisible is white. The kind draws the picture first and the rows over it.

## Against Omega and HD

- **Omega: checked, differs.** Omega's `LoadingScreen_Start` (`0x01708fe0`, which this page used to call the
  constructor) is called with 8 by `ModeManager_ConstructByMode` (`0x0124fe49`) and `0x01277470` (`0x01277757`), with 7
  by `Game_Main` and at `0x01252b86`, and with 9 and 0 by `Game_Main`: the race start of Omega **is** the HD feature
  screen (its state 8), and its "state 5" (this kind 4 in shape) is **started by no call site**, in the base and the
  1.07 build alike ([ps4-omega-eu/loading-screen-start.md](../ghidra/functions/ps4-omega-eu/loading-screen-start.md)).
  Omega also ships none of the tip art. So nothing of the race load above applies to Omega; state 5 stays unwired
  there for good reason.
- **HD: ported.** Nothing changes in HD's loader.

## Looking at it

```sh
oag-game data/extracted/vita/PCSF00007 --loading-screen 0/1 --loading-step race \
  --loading-card "2048 - Event 2" --loading-stage 0 --ticks 232 --size 960x544 --screenshot tip.png
```

`--loading-card EVENT[:PAGE]` presses Launch on that event's card (a capture has no save, so the first event counts as
passed); `--loading-stage N` is the load stage reported to the bar (0 to 4: 4 turns the card to its mode page); `--ticks`
the screen's age at 60 Hz (232 is 8 %, 1780 is 61 %, 1760 reads 60 %, a few thousand holds 98 %). The original's is one command away
(`scripts/vita-2048-race-load.py`). The comparison page's three race-load pairs (`2048/race-load-tip|mode|late`, stages 0, 4, 4
at 232, 1780 and 6000 ticks) are made by `scripts/vita-2048-race-load-pair.py`: it writes each `pair.json` (`kind = "loading"`,
the exact flags) beside the kept `theirs.png` and renders ours, so `compare-rerender-ours.py` re-renders them afterwards.

Against Omega: **checked, differs** (unchanged: it never starts this screen). Against HD: **ported** for the law (HD's
`Rules::HD` keeps its doubling, per-frame step and base-rate reset; HD's loading frames are byte-identical before and after).
