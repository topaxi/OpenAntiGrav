# The pause menu

What each title's own pause screen is, and what this build does (2026-11-07, lanes `pause-menu`,
`pause-menu-2`, `pause-menu-3`, `pause-menu-omega` and `pause-menu-pure`). Scores follow the [rubric](../reverse-engineering/confidence-rubric.md). **Pulse, HD, Fury, Omega and
Pure are wired; 2048 is not** (see [Other titles](#other-titles)): there, pausing still opens the
front end's main page over the parked race, as before.

## Pulse

The screens are in `Data\Plugins\PI001\GUI\InGame_Definition.xml`, identical on the EU and USA
PSP discs (`oag-raceplay`'s `main_menu_ground_truth`):

| Screen | Rows (`idstring`) |
| --- | --- |
| `InGame Pause SP` | `IG_PAUSE_CONT`, `OPT_GAME_OPT`, `EXT_PHOTO`, `IG_PAUSE_GMAN`, `IG_PAUSE_REST`, `IG_PAUSE_QUIT` |
| `InGame Pause SP Time Trial` | the same six |
| `InGame Pause SP Speed Lap` | `CONT`, `OPT_GAME_OPT`, `EXT_PHOTO`, `IG_PAUSE_ENDSES`, `GMAN`, `REST`, `QUIT` |
| `InGame Pause SP Tournament` | `CONT`, `OPT_GAME_OPT`, `EXT_PHOTO`, `GMAN`, `IG_PAUSE_SKIP`, `QUIT` |
| `InGame Pause MP`, `MP Tournament Host` | no photo row; a network race's, never opened here |

**Which mode opens which** is the executable's, not the XML's: `InGame Pause Redirect`
(`type="PauseMenu"`) only names `InGame Pause SP` as its default.
[`PauseMenu_BindRedirectForMode`](../ghidra/functions/psp-pulse-usa/pause-menu.md) writes
Tournament (mode 4), Time Trial (5) and Speed Lap (10) their own screen and leaves every other
single-player mode (Single Race, Head to Head, Zone, Eliminator) on the default. Live on PPSSPP:
Time Trial, Speed Lap and Single Race read the expected state names (90); Tournament is the
static reading (85).

The screen is `top left title` (the dark tint `0xe8000000`, 91% black, and a header bar sliding
in with `IG_PAUSE_GPAUSED`), the `Option` menu (left aligned, 15 apart, from `y = 32`), a
`MusicList` at `y = 220`, a `Redirect` for each row and a second one: `forward="start"`, default
`InGame` (**START resumes**) and `backward="none"` (**CIRCLE does nothing**, measured on the
root, `menu-sounds.md`). `NavigationButtons="1"`: the footer is `Confirm` alone. No news ticker
and no frame bar.

### What ours does

Frames, same track (Talon's Junction), the originals from PPSSPP on the USA disc and ours from the
EU disc, original beside ours: Time Trial, Speed Lap and Single Race, read side by side. The rows, the heading, the row pitch, the
`Confirm` legend and the heavy tint match; ours leaves out GAME MANUAL, END SESSION and the
`MusicList`.

- **Rows.** `oag_title::Title::pause_menu` (`oag_pulse::pause_menu`): each screen's rows by
  `idstring` in the disc's order, and what each does here (`PauseAction`). Worded off the
  disc's own string table (the English "Continue" and so on, drawn in capitals by the menu
  face); a row whose words do not resolve is left off, not drawn as an id. The page is built by
  `oag_ui::menu::Definition::open_on_pause_screen` when the menus open over a held race
  (`Session::add_pause_screen`), and is the root of that tree, so CONTINUE, START and (while
  the menus are the pause's) back out of it resume the race.
- **START and ESCAPE** (and Back on a phone) open it from a one-view race that is not a network
  race. START no longer freezes the race with no menu on a title that has one. A split-screen race,
  a LAN or online race and a title with no `pause_menu` keep what they did (START freezes, the
  split's ownership rule and the countdown stand; ESCAPE opens the main page). Online pause is
  the maintainer's open decision (the pause-in-multiplayer note) and is untouched.
- **CONTINUE, START** resume the race where it stood. **GAME OPTIONS** opens this build's OPTIONS
  page (the disc's `InGame Settings` is a save-or-cancel page of the same settings); BACK from it
  returns to the pause screen. **PHOTO MODE** is the disc's own `EXT_PHOTO` row, on the Pulse
  screens; where a screen has none, this project's PHOTO MODE row follows OPTIONS
  (`PauseMenu::offers_photo`; photo mode is one implementation on every title). It left the main
  page for this one wherever a pause screen applies. **RESTART RACE** loads the race again on the
  options it ran with, from the grid (`Session::restart_parked_race`, the path RACE AGAIN uses).
  **QUIT RACE** drops the race: a campaign race lands on its cell (`Session::quit_campaign_race`,
  measured), any other on the main page. Every row works by pad and by pointer: the page is a
  `Menu` page like the rest, and a click is what CROSS is (`menu::tests::pause`).
- **CIRCLE does nothing on the root** (`backward="none"`, measured silent), as on the disc;
  below it, in OPTIONS, it is back. **ESCAPE** (and Back on a phone) is not a button of the
  original and resumes: **chosen, not measured**. START and ESCAPE and CONTINUE play `ACCEPT`
  (measured for START and the rows).
- **Silent.** Opening it stops the engine note and the race music where they were (the position
  is kept and resumes, `Audio::hold_race_music`); the front end's menu loop does not start
  (measured on the original, `photo-mode.md`, "What entering stops"). A window losing focus,
  minimising and Android's `Suspended` open it the way ESCAPE does.
- **Not animated:** the header bar slides in from `Y = -24` on the original (`delay="1"`); ours
  is there from the first frame.
- **A campaign race pauses** now: ESCAPE used to leave it straight for `Cell Selection`.
- **Rows left out**, named in the load report at `debug`: `IG_PAUSE_GMAN` (the game manual),
  `IG_PAUSE_ENDSES` (ending a Speed Lap session), `IG_PAUSE_SKIP` (a tournament leg).
- **Not built: the `MusicList`.** The original shows the playing track at the foot
  (`Fenix Funk 5 (Wipeout Edit)`) and changes it with LEFT and RIGHT; ours draws nothing there.
  Open.

Tests: the disc's lists per screen and the default, both pressings and PS2
(`main_menu_ground_truth::pulses_*_pause_screens_*`); the mode-to-screen table
(`oag_pulse::pause_menu::tests`); the page, its words, Start and the pointer
(`oag_ui::menu::tests::pause`); the game driving a real race: the rows per mode, ESCAPE and
START, RESTART, QUIT, PHOTO and OPTIONS (`pause_menu_ground_truth`).

### PS2

**Checked, differs.** `SCES_547.48`'s `InGame_Definition.xml` authors the same four screens
without `EXT_PHOTO` and `IG_PAUSE_GMAN`, the same default. The PS2 gets the same rows built: the
manual is built nowhere and PHOTO MODE is this project's row. (Test:
`pulses_ps2_pause_screens_are_the_psps_without_photo_and_the_manual`.)

## HD and Fury

The screens are in `Data\Plugins\Frontend\Gui\ingame_definition.xml` (58 screens, in five
archives). **They are not Pulse's `Option` menu**: each row is a sibling `Block` or `List` widget with
its own `y`, in document order, and the `aVertMenu` and `aBlock` elements the file spells with an `a`
prefix are switched off. The five single-player screens this build opens (the split-screen
`Split Player`, `Tournament Split Player` and `Zone P1`/`P2` pairs and the `MP` screens are for races
this build does not pause into the disc's screen):

| Screen | Live widgets (`idstring`) |
| --- | --- |
| `InGame Pause SP` | `IG_PAUSE_CONT`, `FE_PILOT_ASSIST`, `OPT_GAME_OPT`, `FE_AUDIO_OPT`, `MSC_PHOTO_PHOTOMODE`, `ONL_VIEW_INV`, `IG_PAUSE_REST`, `IG_PAUSE_QUIT` |
| `... SP Time Trial` | the same with `FE_GHOST` after `FE_PILOT_ASSIST` |
| `... SP Speed Lap` | `CONT`, `PILOT_ASSIST`, `FE_GHOST`, `OPT`, `AUDIO`, `PHOTO`, `ONL_VIEW_INV`, **`IG_PAUSE_ENDSES` and no RESTART or QUIT** |
| `... SP Tournament` | `CONT`, `PILOT_ASSIST`, `OPT`, `AUDIO`, `PHOTO`, `ONL_VIEW_INV`, `QUIT` (no RESTART) |
| `InGame Pause Zone` | `CONT`, `OPT`, `AUDIO`, `PHOTO`, `ONL_VIEW_INV`, `REST`, `QUIT` (no Pilot Assist) |

**Which mode opens which** is the executable's:
[`PauseMenu_BindRedirectForMode`](../ghidra/functions/ps3-hdfury-eu/pause-menu.md) (`0x0023d660`, 85):
Tournament (4), Time Trial (5), Speed Lap (10) and Zone (6, 14) their own screen, every other
single-view mode (Single Race, Head to Head, **Eliminator**) the redirect's default `InGame Pause SP`.
Live on RPCS3 (Fury EU): Single Race, Time Trial, Speed Lap and Zone read the expected state names
(90) and drew exactly the rows above; Tournament and Eliminator are the static reading.

**The disc has three generations of the file** (`DATA00`/`DATA06`: Fury, with `FE_GHOST`; `DATA05`:
without it; `DATA02`/`DATA03`: the base game, without `FE_GHOST` and `ONL_VIEW_INV`, and a Speed Lap
screen with RESTART and QUIT). The game reads the Fury one and the table follows it. `FE_GHOST`'s
words are in `DATA06`'s English table and in none of the base game's, so the page's strings are the
served table plus those gaps (`oag_game::boot::PauseScreens::strings`, the way the selection screens
fill theirs).

### What the original draws

RPCS3 frames, same disc, in **both FE styles** (the original's "Menu Style" row,
`docs/formats/hd-frontend.md`, "The FE Style switch is live"; the first round's frames were in the HD
style, white and teal, and a Fury-style player sees the same screen in red):

- **Geometry**, from the file. The title arrow at `(480, 251)` and `GAME PAUSED` at `(520, 240)`; each
  row a `Block` (`x = 480`, `width = 520`, `height = 40`) or a `List`, fifty apart from `y = 310`; the
  focused block grows by 60 (`Block_Update`) and carries the blinking marker arrow. A `List` is a label
  block with two step arrows at its right end and a value block ten units on (280 wide, 340 focused).
  The two rules (`y = 292`, `y = 790`, 960 wide) and the footer are the **shared screen `InGame Main
  Menu`**'s: the START glyph and `IG_PAUSE_RESU` (`RESUME GAME`) at `x = 480`/`525`, `FE_CONFIRM` at
  `x = 1230`, all at `y = 800`. So the panel is centred because 480 to 1440 is centred on 1920; the
  frame's inset (HD's `Safe Area Setting`, 96 % on the capture's profile) pulls it in about 18 units.
- **Ink.** The focused block's `ActiveColor` and the others' `Color` are `FEGlobals->HD_Blue` and
  `HD_Grey`: in the HD style `0xff8ac0ca` and `0xff646464` (measured on the focused pixel, (138, 192, 202),
  and the others, (100, 100, 100)), in the Fury style red and `0xff969696`. The blocks are opaque in the HD
  style and translucent in the Fury one (`BlockArt::fury`), as everywhere else in the front end.
- `FE_PILOT_ASSIST` and `FE_GHOST` as on/off lists (`FE_ON`, `FE_OFF` in the file's order; the step arrows
  stop at the ends of the list). `ONL_VIEW_INV` is **greyed and skipped by the cursor** offline: its label
  and border are white at about a third (border 153, glyphs 151 to 165 over `HD_Grey` 100) and its fill is
  fainter than the others' on the Fury palette (73 against 105).
- The **race HUD stays on screen under the tint**, dimmed with the rest, which leaves 0.25 of the frame's
  brightness (black at alpha 0.75: `scrim_argb = 0xc0000000`, measured on three patches before and after).

### What ours does

The page is the disc's own screen: `oag_ui_screens::pause` lays the widgets out from the file's
positions (read at boot into `oag_game::boot::PauseScreens`, one per screen of the table), the rules,
title and footer from `InGame Main Menu`, and draws the blocks with `oag_ui::menu::block` and the
menu's own focus easing. The rows and what they do are the menu's (`Definition::open_on_pause_screen`),
so pad, pointer and START work as on Pulse; a widget finds its row by the disc `idstring` through the
title's table and `pause_row_id`. The pointer's regions are the same boxes
(`oag_ui_screens::pause::regions`), the step arrows included. A title or screen whose file does not read
keeps the front end's list, said in the boot report.

- **Rows.** CONTINUE, PILOT ASSIST, GHOST (Time Trial and Speed Lap), GAME OPTIONS (this build's OPTIONS
  page), **AUDIO OPTIONS** (its AUDIO page, `PauseAction::Audio`), PHOTO MODE (the disc's own row on every HD
  screen), VIEW INVITES (`PauseAction::Inert`: `Entry::Inert`, drawn, never chosen, no pointer region),
  RESTART RACE, QUIT RACE. START resumes, CIRCLE on the root does nothing, as on Pulse.
- **PILOT ASSIST** is the on/off view of the setting the CONTROLS page edits (`controls.pilot_assist`): ON is
  `extreme`, the level HD flies with, OFF is `off`; a stored `normal` shows as ON. A switch from this page
  reaches the parked race when the player resumes (`Session::apply_pause_setting`), **and the recording goes
  on**: the switch is an event between the two ticks it was made between (`Race::change_pilot_assist`, the `EVNT`
  chunk of the [replay file](../formats/replay-file.md)), so the file still reproduces. The CONTROLS page still applies from the next race, as before.
- **GHOST** is `display.ghost_ship_visible`, applied to the parked race at once (`the_ghost_list_hides_the_ghost_ship_of_the_race_it_resumes`).
  It is one setting across titles, so a player who turned ghosts off on Pulse now has them hidden on HD too. HD lists it itself: its own
  GAME OPTIONS page has `GHOST SHIPS VISIBLE` and `MULTIPLAYER TAGS` (frame, `Menu Style` capture), so
  `oag_hd`'s `GameOptions::ship_visible` is on and this build's OPTIONS page keeps that row.
- **END SESSION** (Speed Lap) leaves the race the way QUIT RACE does: **chosen, not measured**. The original
  goes to `Race End Photo` (CONTINUE, PHOTO MODE), `EndRace Results` and `EndRace Menu`: it **finishes**
  the race. This build's Speed Lap never finishes, so there is nothing to finish.
- **The HUD under the tint**: `PauseMenu::hud_under_scrim`, on for HD only (Pulse's is unmeasured). The parked
  race draws its HUD (not the track panel or end screens, `RaceStage::draw_view_hud`) before the menu's tint.
  It stays behind this build's OPTIONS and AUDIO pages too; whether the original keeps it behind its own
  `InGame Settings` is not measured.
- **Palette.** The page takes the front end's: this build serves `DATA00`'s Fury palette on every page
  (`docs/formats/hd-frontend.md`), so the focused row is red and the blocks translucent. **That is the
  original's own Fury-style pause screen** (RPCS3, Fury style, Single Race and Time Trial); the cyan frames
  are the HD style, which this build's front end does not switch to (open: the FE Style row).
- **Eliminator**: it takes the default `InGame Pause SP` screen, and its HUD (TARGET, the ranked rows,
  the race line) is drawn again after a resume. A pause and resume leaves the race as it was and the L1
  flip works after it (`an_eliminator_race_resumes_with_its_flip_intact`: the state hash at one tick is
  the unpaused race's, and a flip pressed after the resume changes it).
- **A split-screen race** still gets the main page (START's freeze); the disc's `Split Player 1`/`2`
  screens are the model.

**Measured against the original, Fury style** (`scripts/pause-panel-metric.py`, both frames mapped to the
1280x720 game area, Time Trial pair): the two rules at rows 202 and 521 on the original, 203 to 204 and 519
to 520 on ours (x 333 to 947 against 335 to 945); the focused row's span 340 to 685 against 341 to 685; the
mean absolute colour difference over the nine rows' box interiors (three samples each, backgrounds differ)
**4.6** of 255 per channel, the focused ink (117, 11, 22) against (118, 8, 20). The Single Race pair reads
5.8 over eight rows. Residuals: the original's marker arrow blinks (off in the frame taken), the step
arrows stop at a list's ends where ours wrap, and the greyed row's fill.

Tests: the rows per screen against all five archives' copies and the layout (rule rows and spans, row
`x`/`y`, `ActiveColor` resolving to each archive's `HD_Blue`)
(`main_menu_ground_truth::hds_pause_screens_are_the_five_single_player_widget_lists`); the mode-to-screen
table and that every screen can be left (`oag_hd::pause_menu::tests`); the draw (`oag_ui_screens::pause`:
chrome positions, focus ink and growth, list parts, the inert row and its regions); the rows, the lists'
settings and the cursor skip (`menu::tests::pause`); the game driving a real race
(`pause_menu_ground_truth::hd_screens`: rows per mode, PILOT ASSIST reaching the resumed race, AUDIO and
back, END SESSION, restart, Eliminator).

## Omega

**Wired** (`Title::pause_menu` is `oag_omega::pause_menu::PAUSE_MENU`). Omega's `ingame_definition.xml`
(`data00`, `data08`, `data09`, identical on these screens) is HD's Fury file **without `ONL_VIEW_INV`**, and
carries the same `InGame Main Menu` chrome (rules at `y = 292` and `790`, `IG_PAUSE_RESU` at `525, 800`), so
the HD layout above is the one drawn (`oag_omega::pause_menu` sets `layout` to the same file and chrome). Its
binder ([`PauseMenu_BindRedirectForMode`](../ghidra/functions/ps4-omega-eu/pause-menu.md), `0x0155dc00`, 80) is
the same switch on the same mode ids, preceded by a rewrite of the mode from a campaign event object. The table
is HD's less that row, with PILOT ASSIST and GHOST as the same lists (the ground-truth test asserts the three
copies and the equality with HD's less the row).

### What the original draws

shadPS4 0.20 on the EU pressing (`scripts/shadps4-omega-pair.py`'s rig: its own headless `sway`, one uinput
pad, muted; the walk is MAIN MENU, RACEBOX, the race type row, Track Select, Ship Select, START RACE, then the
START button, "OPTIONS" on a PS4 pad), Vineta K, the first Feisar craft, Venom class, 1920x1080. Reference
frames: `data/reference/omega-pause-menu/`.

- **Single Race, Time Trial and Speed Lap** were walked live and draw the rows the file authors (confidence
  90 each). Single Race: CONTINUE, PILOT ASSIST (OFF), GAME OPTIONS, AUDIO OPTIONS, PHOTO MODE, RESTART RACE,
  QUIT RACE. Time Trial adds GHOST (ON) after PILOT ASSIST. Speed Lap is Time Trial's without RESTART and QUIT,
  ending in END SESSION. The footer is the START glyph with `RESUME GAME` and `CONFIRM`, as on HD. START opens it
  over the countdown board or the road.
- **Not walked live**: Zone (shadPS4 0.20 died at the Zone race start in the one try, an access violation on
  the render thread; the native port was not tried), Eliminator (the default screen by the binder, as on HD),
  Tournament (a campaign event rewrites the mode first; static reading, confidence 80). The tables for those
  are the file's.
- The pad is the PS4's: the original is driven by CROSS and by the OPTIONS button, which the virtual pad's
  START presses. Input over shadPS4 on this machine lags by tens of seconds under load and drops short presses,
  so the walk polls the screen between steps (`scripts/shadps4-omega-pause-capture.py`, on the rig of
  `scripts/shadps4-omega-pair.py`).
- **Ink and blocks** (answers the dark-ink defect the first reading expected): the title, the labels and the
  footer are **white**; the blocks are **opaque black** with the focused one **red** `(239, 0, 0)`, the list value
  boxes the same. There is no HD-style cyan on this profile: the red and black is the palette this build
  already serves. Whether Omega has a `Menu Style` row that switches it was not looked for on its options page.
- **The tint is lighter than HD's**: the scene is about half as bright, not a quarter. A least-squares fit of
  the paused frame on the frame before it over 348,000 pixels outside the panel and the HUD (correlation
  0.99, so the same scene) gives a slope of **0.50**; seven hand-picked patches read 0.38 to 0.52 (the fit is
  the better number). `scrim_argb = 0x80000000`, black at alpha 0.5.
- **No HUD under the tint**: the lap hexagon, position, best time and speedometer are gone from a paused frame
  and back on the resume (HD keeps its HUD, dimmed). `hud_under_scrim` is off.
- The START glyph is the **OPTIONS button** (a pill with a tiny "OPTIONS" label above it, `FE_START_BUTTON`
  is `"I"` in `PS_BUTTONS.fnt`); ours draws a triangle there (below).

### What ours does

The page is the one HD gets (`oag_ui_screens::pause` over the file's widgets; `Definition::open_on_pause_screen`),
with Omega's table: rows, pad, pointer, START, CIRCLE on the root doing nothing and the settings reaching the
parked race are HD's code. Omega's `GameOptions::ship_visible` is on so the GHOST list has the setting to switch
(its OPTIONS page here gains `GHOST SHIPS VISIBLE`, **inherited from HD**; the original's own options page was
not read, only the pause screen's `FE_GHOST` row, which proves the setting exists, `docs/ui/menus-original.md`).

**Measured against the original** (`scripts/pause-panel-metric.py`, Time Trial pair, `--orig-box 0 0 1920 1080
--inset 1.0`; ours is the same fit on its own paused frame: slope **0.48** at alpha 0.52, so 0.5 gives 0.5): the rules at rows 195 to 196 and 527 to 528 on the original, 196 to 197 and 526 on ours, both
spanning x 320 to 960 at 1280x720; the focused row (239, 0, 0) against (238, 7, 7); the mean absolute colour
difference over the eight rows' box interiors **1.2** of 255 per channel (24 samples; the backgrounds differ).

Residuals:

- **The START glyph** draws HD's triangle where the original draws the OPTIONS pill. Not chased: whether Omega's
  `PS_BUTTONS.fnt` (`data00`'s copy or `data08`'s) draws `"I"` as the pill and ours reads the other copy is open.
- The marker arrow blinks off in the frames taken; the list step arrows were not compared.
- The Fury style's red is Omega's own; the pause frames have no second style to switch to.

**A 2048 finding is checked against Omega, and the reverse**: Omega's binder reads the same mode ids as HD's, so
this table is the model for 2048's `InGame_Definition.xml` (Vita), which is not wired.

Tests: the rows per screen against all three archives' copies and the equality with HD's less the row
(`main_menu_ground_truth::omegas_pause_screens_are_hds_without_the_invites_row_in_every_archive`); the mode
table (`oag_omega::pause_menu::tests`); the game driving a real Omega race
(`pause_menu_ground_truth::omega_screens`: the rows per mode, END SESSION, START and the PILOT ASSIST list
reaching the race, RESTART).

## Pure

**Wired** (`Title::pause_menu` is `oag_pure::pause_menu::PAUSE_MENU`). The screen START opens is **not
`InGame Pause`**: that screen is a `Paused` text on a band with no rows. START goes through
`InGame`'s redirect (`forward="start"`) in `Data\Plugins\PI001\GUI\InGame_Definition.xml` (identical on the EU
and USA PSP discs), and the **mode is in the file**, as an `Entry` row on the front end's own state, where
Pulse's and HD's is the executable's switch:

| Screen | Rows (`idstring`) | Opened by |
| --- | --- | --- |
| `InGame Main Menu` | `Continue`, `Restart`, `Quit` (scale 1.2) | the default: Single Race, Time Trial, Zone |
| `InGame NoRestart Main Menu` | `Continue`, `Quit` (scale 1.0) | `Single Player->Mode` = `Tournament`, `Main Menu->Mode` = `Multiplayer` |
| `InGame Free Play Menu` | `Continue`, `Restart`, `End Session` (scale 1.0) | `Single Player->Mode` = `Free Play` |

Each screen draws the same band: an image `0xc0000010` over `y = 75` to `195` across the whole 480, two 1-pixel
rules `0xff3192bb` at `y = 75` and `175`, one `<Menu name="Option">` centred at `x = 240` (`vertalign="middle"`,
`y = 106`) and the footer `Press X to Confirm` at `y = 175`. START resumes (`forward="start"`, default `InGame`)
and `backward="none"`: CIRCLE does nothing. Nothing outside the band is tinted.

### What the original draws

PPSSPP 1.20.4, software renderer, 480x272, the USA disc, a fresh profile, Vineta K, Feisar, class Vector,
walked by pad to the grid and START pressed on the countdown (Single Race) or after it (Time Trial, Tournament,
Free Play). Reference frames: `data/reference/pure-pause-menu/` (`p1` Single Race, `p2`, `p3` its cursor on
RESTART and QUIT, `t2` Time Trial, `u2` Tournament, `f3` Free Play, `c1` the same race a moment before).

- **Single Race, Time Trial, Tournament, Free Play and Zone** were all walked live and draw the rows the file
  authors (confidence 90 each; `zone` in the reference frames: CONTINUE, RESTART, QUIT, the default).
- **END SESSION** (Free Play) was walked live too: it ends the session onto a fly-past with one row, `RETURN TO MAIN
  MENU` (`free-play-end-session-return` in the reference frames), and CONFIRM lands on the main menu (confidence 85:
  one run, no laps set). It is not QUIT's straight drop.
- **The band is the authored image and nothing else.** Pixels above and below it are the unpaused frame's; inside
  it the scene is `0.25` of itself plus `(0, 0, 16)` (`0xc0000010` at alpha 0.75: the brightest text-free pixel of the
  band reads (63, 63, 75) against the law's (64, 64, 80)). **The HUD stays on screen at full brightness outside
  the band** (CURRENT, POS, LAP, the speed bar) and is dimmed inside it like the scene.
- **Ink**: the cursor row white (253), the others grey (157, 155 over the band's 20), the footer white. The focused
  glyph is 11 pixels tall at scale 1.2 and 10 at 1.0.
- **Pitch**: rows stand `15 * scale` apart (18 at 1.2: ink rows 103 to 114, 121 to 132, 140 to 150; 15 at 1.0: 104
  to 113, 119 to 128, 134 to 143), the first row's middle at the widget's `y`.
- **The cursor does not wrap**: DOWN on QUIT stays on QUIT and UP on CONTINUE stays on CONTINUE (checked on Single
  Race, three presses each way), though the file authors `wrap="true"`.

### What ours does

The page is the disc's own screen over the HD/Omega drawer (`oag_ui_screens::pause`), which now also lays out a
`<Menu>` widget: `PauseLayout::chrome` is `None` (each screen carries its own band), and `PauseLayout::menu_rows`
gives the two numbers the file does not author, the pitch (`15`, measured) and the alpha of a row without the cursor
(`0.5`: the authored alpha that lands the measured grey 157 on our blend). Every row of the page is drawn down the
widget in page order, the one with the cursor at full ink, and each is a pointer target across the band
(`oag_ui_screens::pause::regions`). The scrim is transparent and `hud_under_scrim` is on. Pad, pointer, START, CIRCLE
doing nothing and the rows' actions are the menu's (`Definition::open_on_pause_screen`), as on the other titles.

- **PHOTO MODE** follows CONTINUE (`PauseMenu::offers_photo`; the screen authors none, photo mode is one
  implementation on every title, maintainer). So ours has one row more than the original on every screen.
- **END SESSION** (Free Play) leaves the race the way QUIT does, for the main page: **chosen, not measured**. The
  original shows the one-row `RETURN TO MAIN MENU` screen first; this build races Free Play as a single race with no
  session results to show (`docs/formats/pure-campaign.md`, "The modes"). Free Play is chosen by the campaign cell's
  name (`Session::pause_mode`, key `free_play`), not by the engine's mode.
- **The words** are the disc's own (`Continue`, `Restart`, `Quit`, `End Session`, `Press X to Confirm`, drawn by the
  front end's face). The PSP footer's cross glyph is the font's.

**Measured against the original** (`scripts/pause-band-metric.py`, both frames at 480x272; ours is a 1440x816
capture box-filtered down): the two rules at rows 75 and 175 on both, x 0 to 480; the first row's ink at rows 103
to 114 against 103 to 114 and its x from 188 to 291 against 188 to 293; the second row's 121 to 132 on both and the
third's 139 to 150 against 140 to 150; the cursor row's ink 252 against 253 and the others 156 against 157; the
Tournament screen's first row 104 to 112 against 104 to 113, x 197 to 282 on both. The mean absolute colour
difference over the first row's box is 22.8 of 255 per channel on Single Race and 42 on Tournament, where the
backgrounds differ and a one-pixel glyph shift is worth about that: the placement numbers above are the ones to read.

Residuals:

- **The cursor wraps** in ours (the menu's rule on every title); the original's does not. Not changed here.
- **Our rows**: PHOTO MODE is extra, so the third row on the original (QUIT) is the fourth in ours.
- **The race's now-playing line** (the music title at the foot, "COLD STORAGE - ONYX") draws under the pause in ours;
  the original's frames show none. The disc authors that text as widgets of the `InGame` screen
  (`InGameArtistText`, `InGameTrackText`, `InGameLabelText`), and the pause screens are siblings of it, so the original
  cannot draw it while paused. Ours draws it because `hud_under_scrim` brings the whole race HUD under the band.
  Open: leave the music line out of the HUD drawn under a Pure pause.
- **Craft livery**: the Feisar in ours is gold, the original's blue and yellow. Not this lane's.

Tests: the three screens' rows against both pressings, the redirect's rows against `by_mode` and the band, rules and
menu place (`main_menu_ground_truth::pures_pause_screens_are_the_three_menus_the_ingame_redirect_picks_between`);
the table (`oag_pure::pause_menu::tests`); the draw and the pointer
(`oag_ui_screens::pause::tests::menu_widget`); the game driving a real race (`pause_menu_ground_truth::pure_screens`:
the rows per mode, START, CIRCLE, RESTART, QUIT and PHOTO MODE by pad).

## Other titles

Not wired: **2048** (`InGame_Definition.xml`, 36 screens, and `LegacyInGame_Definition.xml`; Vita, a
binder of its own). A 2048 finding is checked against Omega and the reverse: Omega's binder above reads the
same ids, so 2048's is the first thing to compare. Pure's screens share nothing with 2048's or Omega's (a
`<Menu>` of entries on a band, not `Block` widgets), so Pure's finding is **checked, differs** against both.
