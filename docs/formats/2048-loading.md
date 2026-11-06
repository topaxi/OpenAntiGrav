# Wipeout 2048's loading screen

**2048 ships HD's loading-screen class, and the retail executable starts a different kind of it for a race load.**
Confidence 80 (a call-site census and a decompile, no running original: the machine has no Vita3K profile). The
evidence is [vita-2048-eu-v104/loading-screen.md](../ghidra/functions/vita-2048-eu-v104/loading-screen.md).

## Two screens in one class

| kind | what it is | reached |
| ---: | --- | --- |
| **4** | the race load: the mode's event card (or a controls still), a `Blue2048`/`White2048` percentage box, `FE_LOADINGDOT` and a `%d%%` readout | by every race start (`FUN_8115f09a`, `FUN_811494fa`) |
| **7** | HD's screen: caption naming the circuit, one of five illustrated features, a dotted progression bar | **by no call site** of the v1.04 executable |

A Vita player's race load is kind 4. Kind 7 is HD's `LoadingScreen_Draw` carried over with its code, its strings and
its ten illustrations, and nothing starts it. **This build draws kind 7** (`oag_2048::loading::LOADING`,
`assets/fallback/Data/XML/LoadingScreen_2048.toml`) because it is the screen 2048's own data authors in full, and
kind 4 (a card plus a percentage box, composed from the front end's event card) is not built. **Chosen, not
measured:** that a race load of this build shows kind 7. A player of the original sees kind 4. Kind 4 is the open
work, below.

## What kind 7 is, in 2048

- **Art.** All sixteen names resolve in `data.psarc`: the six marks (`corner`, `dot`, `square`, `line`,
  `Title_Arrow_HD`, `Subtitle_Arrow_HD`) and the ten illustrations (`Barrel_Roll`, `Side_Shift_Tap`, `Pilot_Assist`,
  `Absorb`, `Flip`, each with and without `_fury`), all `.gxt`. **Both arrows ship**: the Omega page's "the two arrows
  were not listed" was a census gap, and the brief's "four of six marks" was wrong. The illustrations are 1024 by 512.
  Asserted by `loading_ground_truth::every_entry_of_the_loading_table_is_in_the_package` (and
  `the_unreached_fury_pictures_ship_beside_the_base_ones`, in `oag-2048`).
- **One styling is reachable.** The loader picks `_fury` on a byte that is `0` in the image and has no writer
  (confidence 70), so the table carries the base styling alone. The `_fury` pictures ship and nothing reaches them.
- **The layout is HD's, operand for operand**, minus the mode icon: rules, caption, labels, brackets, the picture
  window (896 by 480 at (180, 190)), the name and heading rows, the paragraph and the bar's rectangle. The grid is HD's
  1920 by 1080; how it maps onto the Vita's 960 by 544 surface is unread.
- **No mode icon.** The panel's label and brackets are drawn and nothing inside them (no clip, no still).
- **The brand is HD's literal**, `WIPEOUT(R) HD`, at `0x814d0e24`. (The Omega page said the literal was `WIPEOUT`; it
  is the whole of HD's.)
- **The deck is HD's, row for row**, with a four-wide default and no Fury byte (mode `6`: two, `0xe`: one, `0xd` and
  `0x15`: `0 1 3`, Eliminator `8` and `0x14`: five). Asserted in `deck_tests::the_2048_deck_is_hds_rows_with_a_four_wide_default`.
  2048's own mode flags that also select the Eliminator row cannot be keyed by id and draw the default.
- **The colours** are the four `HD_*` globals of the `HD_Colours` block of `Skin.xml`, which 2048 inherits from
  HD's black-and-red archive: ink `969696`, accent `ac0717`, trough `64646480`. The ground is **chosen, not
  measured**: kind 7 reads `HD_BG` nowhere and fills no ground, and the base pictures are opaque white to their edge,
  which shows nothing only against white. The palette's ground is therefore `HD_White`, the one white global of the same
  block. An earlier Vita3K walk records the load screen as light (mean luma 0.9,
  [vita3k-capture.md](../reverse-engineering/vita3k-capture.md)), which fits a light ground (that screen was kind 4).
- **The bar** fills with the time estimate (`progression: None`, chosen, not measured). 2048's ceiling is 94.62 at 0.115 a
  frame against HD's 149.4 at 0.1.
- **The circuit's name** in the caption comes from the caller: `--loading-screen ... --track` prints the track's own
  stem (`altima`, `Vineta_K`), where a race started from the menus passes the catalogue label.

## Kind 4, not built

Read in the evidence page and summarised here so the next lane starts from it:

1. Its backdrop is `CampaignEventCard_DrawPanel` for a mode whose vtable slot `+0x60` is `0`, and otherwise a
   still at (-32, 144), 1024 by 256: `LoadingScreens/LoadingScreen_{racer,motion,wipeout}_controls.gxt` for an HD-lineage mode by
   control scheme, `Title_Screen.gxt` otherwise.
2. The percentage box is 70 by 70 at (16, 16) in `Blue2048`, a 338 by 4 bar at (86, 82), `FE_LOADINGDOT` at (105, 30)
   and `%d%%` at (51, 38). `Blue2048` and `White2048` are in `Skin.xml`.
3. The readout's ceiling is `94.62 / 166` = 57 %, which is the 56 % a Vita3K load sat at when it stalled.

Omega's "state 5" is the same screen (`omega-loading.md`, "Not wired").

## Against Omega and HD

- **Omega: checked, differs.** Omega's race load is its kind 8 and draws HD's screen (a still per mode in the icon
  panel, brand `MSC_WOOM_CAPS`); 2048's kinds are numbered differently (8 and 9 are the epilepsy and ESRB stills), and
  2048's race load is kind 4. The marks, the pictures' sizes, the strings and the layout operands are the same files and
  numbers, so `LoadingScreen_2048.toml` is `LoadingScreen_Omega.toml` with HD's brand and no mode icon.
- **HD: ported.** Nothing in HD's loader changes. The generic loader's style pick is unchanged; 2048 names one
  style, so a saved `FURY` falls back to it with the loader's note.

## Looking at it

```sh
oag-game data/extracted/vita/PCSF00007 --no-audio --loading-screen 0/0 --loading-step race --ticks 0 \
    --loading-draw 1 --mode single_race --track 'Data\art\published\environments\altima\track.vex' \
    --size 1920x1080 --screenshot /tmp/2048_loading.png
```

`--loading-draw` picks the feature (`1` is Pilot Assist), `--mode eliminator` the five-wide deck. The original is
unavailable here: no Vita3K frame of a race load exists.
