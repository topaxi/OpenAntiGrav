# Wipeout: Omega Collection's loading screen

**HD's screen, carried forward and re-spelt for the PS4's `.gnf`, with one change: the mode icon is a still, not a
clip.** Drawn by `oag_game::loading::Screen` from `oag_omega::loading::LOADING` and
`assets/fallback/Data/XML/LoadingScreen_Omega.toml`; the screen itself is [hd-loading.md](hd-loading.md)'s.

## What the disc and the executable say

Read off the base `eboot.bin` in Ghidra (`FUN_01708fe0`, the loading screen's constructor, and `FUN_017062e0`,
`LoadingScreenThread`'s body, a state machine on `object+4`) and off the patch `eboot.bin`'s string pool (the
same run, same order). State 8 is the race load, state 5 a different screen (see "Not wired").

| thing | Omega | evidence |
| --- | --- | --- |
| five illustrations | `Data/Fe/Images/{Barrel_Roll,Side_Shift_Tap,Pilot_Assist,Absorb,Flip}.gnf`, each also `.2x.gnf`; 1024x512 base level, the size HD's are | string run; `data00` and `data08`; `omega_names_a_loading_screen_whose_files_are_on_the_disc` |
| marks | `corner`, `dot`, `square`, `line`, `Title_Arrow_HD`, `Subtitle_Arrow_HD` as `.gnf` | string run; constructor `FUN_0177ab00` calls |
| strings | `FE_LOADINGDOT`, `FE_FEATURE_IMAGE`, `FE_FEATURE_DESC`, `FE_PROG_BAR`, `FE_MODE_ICON`, and the feature ids HD's table names | all in `data09.psarc`'s English `entries.xml` |
| brand | the string id `MSC_WOOM_CAPS` ("WIPEOUT(TM) OMEGA COLLECTION"), not HD's literal | the run; the layout's `label-title` |
| styling | **one**: no `_fury` file is named; the front end always takes `WOHD_Colours` ([omega-frontend.md](omega-frontend.md)) | string run; `frontend::MENU_SKIN` |
| layout | the rules (160,110) and (160,975), the arrow (160,73), the caption (194,62), the label squares, the brackets and the picture window (0.875 by 0.9375 of 1024x512 at (180,190), 896x480) are the same operands as HD's | `FUN_017062e0` state 8 against `LoadingScreen_HD.toml` |
| mode icon | **a still**, not a Bink clip: one `.2x.gnf` per mode id, tinted `0xff0707ee` (ABGR: `HD_Blue` of `WOHD_Colours`), drawn at (1518,708), 230x234 | `FUN_01708fe0`'s `switch`; no `.bik` ships in any of the nine archives |
| mode to still | 4 and 0x11 tournament, 5 time trial, 6 zone, 8 and 0x14 eliminator, 10 speed lap, else single race; 0xd and 0x15 (Zone Battle) and 0xe (Detonator) pick an untinted `Data/FE/Images/<mode>.2x.gnf` | the `switch` |

The `.gnf` row order is Omega's front-end rule (`FrontEnd::bottom_up_gnf`): `corner`, `square` and `line` are HD's
bytes in HD's order and are reversed on the way into the sheet. `omega_marks_are_drawn_the_way_up_hd_draws_them` pins
it (the corner is an L, so a flipped sheet fails).

## What this build does with it

- Style, palette and rows follow the existing generic loader: the four `HD_*` globals are read through the title's
  `colour_template` (a title with none reads the file as before), the sheet reverses the front end's bottom-up
  `.gnf`, and a feature whose picture does not decode leaves `features` with its illustration.
- A mode id with no still draws none (`ModeStills::entry`); Zone Battle and Detonator draw nothing rather than a
  wrongly tinted glyph.
- **Chosen, not measured:** the deck is every feature for every mode (`deck: None`; the feature type is an argument
  of the constructor and its caller is unread); the bar fills with the time estimate (`progression: None`); the
  bar's, the label text's and the paragraph's numbers are HD's and were not compared with Omega's draw;
  the base styling's callout glyph (the untinted `Images/` branch is taken when a flag at `DAT_01f99bc0` is 1, which
  nothing here sets); the `.2x` pictures are not used (the layout's grid is 1920x1080 and the 1x level matches HD's
  picture window; which one the original takes by resolution is unread).

## Not wired

- **State 5** of `LoadingScreenThread`: a backdrop, a `FE_LOADINGDOT` caption and a `%d%%` readout in a `Blue2048`
  box (`White2048` text). It is what the strings `White2048`/`Blue2048` before `FE_LOADINGDOT` belong to; it is not
  Omega's race load. On 2048 the same screen (its kind 4) **is** the race load, started by every race start
  ([2048-loading.md](2048-loading.md)); what raises Omega's is unread.
- The three `LoadingScreens/LoadingScreen_{racer,motion,wipeout}_controls.gnf` (states 0, 1, 9, 10: full-screen stills
  with a fade) and `loading_screen_demo_bar.gnf`.
- The tournament variant (`+0x598 == 1`: `ER_TOUR_STAN`, `FE_TOUR_TRACKS`, a track list).

## Against 2048 and HD

- **2048: ported, with a difference in kind** ([2048-loading.md](2048-loading.md)). 2048's `eboot.elf` carries the same
  class and string run, `data.psarc` ships all ten illustrations and **all six marks** (both arrows too: the earlier
  census missed them) as `.gxt`, and `LoadingScreen_2048.toml` is this page's layout with HD's brand literal
  (`WIPEOUT(R) HD`, not Omega's string id) and no mode icon. `oag_2048::loading::LOADING` wires it. **Checked, differs:**
  2048's kinds are numbered differently (8 and 9 are the epilepsy and ESRB stills, 4 is the race load, 7 is HD's feature
  screen and **no call site starts it**), so what a Vita player sees on a race load is 2048's kind 4, this page's
  unbuilt state 5.
- **HD: ported the other way.** The generic loader changes (colour template, bottom-up rows, an undecodable picture
  drops its feature) leave HD's tests unchanged; `oag_title::loading::Labels::brand` was dead data and is gone (the
  brand lives in the layout file on both titles).

## Looking at it

```sh
oag-game data/extracted/ps4 --no-audio --loading-screen 0/0 --loading-step race --ticks 0 \
    --mode single_race --size 1920x1080 --screenshot /tmp/omega_loading.png
```

The original is unavailable here: no shadPS4 frame of Omega's race load exists.
