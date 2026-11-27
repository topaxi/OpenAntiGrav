# The pause menu

What each title's own pause screen is, and what this build does (2026-11-07, lanes `pause-menu`,
`pause-menu-2`, `pause-menu-3`, `pause-menu-omega` and `pause-menu-pure`; 2026-11-25, `pause-menu-per-mode`).
Scores follow the [rubric](../reverse-engineering/confidence-rubric.md). **Pulse, HD, Fury, Omega, Pure and
2048 are wired** (see [2048](#2048) and [Other titles](#other-titles)).

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
`Confirm` legend and the heavy tint match; ours leaves out GAME MANUAL and END SESSION.

**The HUD, the footer and the header (2026-11-23, `pulse-gap-pairs`, confidence 90).** A fresh capture of a Time Trial on Moa Therma White
(the `time-trial-moa-therma-white-venom-grid` state, START, 4 s) against ours played to the same race clock
(`scripts/pulse-pause.drive`, `docs/tools/compare-page.md`, "Pulse list screens and the pause menu"):
the HUD (record time, lap hexagon, `current` clock) stays on screen under the tint, dimmed with the scene (the clock box
peaks at luma 23 on both sides, a ninth of white), so `PauseMenu::hud_under_scrim` is **on for Pulse** (it was off as "unmeasured";
`Session::draw` reads the table for a title whose pause page has no disc layout too); the footer is `Confirm` alone and **no player tag**
(ours drew `AAA` there; the tag belongs to the frame bar the pause screen does not draw, so `MenuStage::nav_legend_overlay` leaves it
off a pause page); the tint is the authored `0xe8000000` (a least-squares fit of the paused frame on the frame before it reads 0.077
against the authored 0.090 of white, correlation 0.93). **The header bar slides in**: `InGame Pause SP`'s `<Animation delay="1">` runs
its `<Key>`s `Y -24` to `0` over one second after a one second delay, and a frame grabbed 2 s after START caught the bar 7 px
above its rest (a shift fit of the header rows, 8.9 grey against 72 unshifted), which was 64 % of the pair's differing pixels (3,440 of
5,353). Ours draws the bar at rest, so the pair is grabbed 4 s after START, once the slide has ended; **the slide itself is not
wired** (the header is the front end's frame bar, static). GAME MANUAL is drawn by `--original-menu` only
(`Definition::open_on_pause_screen_as_authored`: a row that opens nothing), as a compare shot lists what the original lists.

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
- **Pulse's pause rows bloom (2026-11-24, `fe-worst-residuals`, confidence 85).** The original's pause screen ends its frame in the
  front end's bloom like every other front-end page: each row and the music line carry a halo (a PPSSPP frame of the Time Trial pause,
  the focused `CONTINUE` and the idle rows, at 4x). Ours stamped the same `Draw::GlowText` and never ran the bloom, because the
  pass ran only on a clearing draw and a pause is drawn over the parked race (`LoadOp::Load`). `Renderer::set_glow_over_scene` lets a
  `Load` list bloom its stamps and `MenuStage::render` sets it for a frozen race only (`oag_game::render::glow_tests`,
  `a_load_pass_blooms_its_stamps_only_when_the_renderer_is_told_to`); the overlays, the results table's emitted stamp and the track panel
  also draw with `Load` and stay unbloomed (their stamps are unmeasured). The scratch frame holds the list alone, so a halo is
  the text's colour blurred over black, then added to the scrimmed race. Result on `pulse-psp/menu-pause`: mean grey 4.01 (main's tree;
  the page's 4.51 predates the music line) to 2.70, the rows box `(40,20,150,120)` 11.84 to 5.75. Pure draws no glow (its skin
  has no `MenuGlow`, the other titles likewise): their pause pairs are unchanged. The focused row's stamp rides the pulse's phase, unpinned
  on the drive route. **Pure's pause pair (5.06) shares none of this**: its residual is the race under the scrim and its HUD
  (the HUD corners `(0,0,150,50)` 11.41 and `(370,0,110,45)` 12.54, the lap counter `(0,225,60,47)` 16.75, the same boxes score 10.93, 13.21 and
  16.79 on the `pure-psp/vineta` race pair; the band `(0,95,480,100)` is 2.70).
- **A campaign race pauses** now: ESCAPE used to leave it straight for `Cell Selection`.
- **Rows left out**, named in the load report at `debug`: `IG_PAUSE_GMAN` (the game manual),
  `IG_PAUSE_ENDSES` (ending a Speed Lap session), `IG_PAUSE_SKIP` (a tournament leg).
- **The `MusicList`** (read 2026-11-24, PSP USA `BOOT.BIN` and PPSSPP, confidence 90 unless named;
  [music-list.md](../ghidra/functions/psp-pulse-usa/music-list.md)). The row at `y = 220` shows the
  selected song's **name** (no artist) in the `small` face and `TextColor`, after a two-half arrow pill
  (`pulse_assets.mip`, UV 282/306, at the text's x - 32 and - 20); the left half is dark at the first song,
  the right half at the last. It starts on the playing song. **LEFT and RIGHT step it whichever row the
  cursor is on** (`workWhenNotFocus`), one step a press, **no wrap**: a step plays `UPDOWN`, a press at
  an end plays `DECLINE` and changes nothing. The cursor never lands on it. **Nothing plays while
  paused**; leaving the screen by resuming (CONTINUE, START) starts the picked song **from its start**
  when it is not the one that was playing (the now-playing marquee then shows it), and the next race
  goes on from it. Picking the playing song again resumes it where it was (85). Leaving by GAME OPTIONS
  drops the pick (measured: back on the screen the row shows the playing song again); RESTART RACE and
  QUIT RACE drop it too (static, 70). This **corrects** the first reading (2026-11-23), which had LEFT/RIGHT
  on the focus step with `LEFTRIGHT`: those are UP/DOWN.

  **Built (2026-11-24):** `oag_title::PauseMenu::music_list` (`oag_pulse::pause_menu::MUSIC_LIST`) carries the
  numbers; `oag_ui::menu::music_list` is the widget (state, step, draw, pointer) on the pause page's root; the
  session gives it the race playlist (`oag_sound::Audio::race_playlist`, each song's title, the booted disc's
  order then the other enabled sources, a folder file by its file name: **chosen**) and, on resuming, hands the
  pick to `Audio::pick_race_song`, which starts it from its beginning as a new song (and drops a track fetch in
  flight, which would land on the old song); the same song picked again resumes where it was. Only a resume plays the
  pick: PHOTO MODE (which leaves the race frozen) and any other page drop it, the latter on every page change, ESCAPE's
  out-of-band back included. The step is `Nav::Moved` (`UPDOWN`), an end `Nav::Decline`. **Pointer** (chosen, the
  original has none): a click on the back or forward half of the pill steps that way with the pad's sounds, a
  click on the title steps forward; neither moves the row cursor. The pill's y offset (- 4) is measured off the frames;
  a half at its end keeps half its alpha (`ListWidget_ShadeArrows`, read on both binaries; it reads 0.56 of the lit
  half over the dark frame, ours `(31, 90, 100)` against the original's `(32, 91, 100)`). The next race goes on from the picked song through this
  build's playlist (`docs/architecture/race-music-resume.md`). Tests: `oag_ui::menu::tests::music_list`,
  `oag_sound::pause_pick::tests`, `oag_sound::tests::song_name::a_paused_pick_resumes_as_a_new_song_and_the_playing_song_does_not`
  (the now-playing display re-shows a picked song after the resume, not the same song resumed),
  `oag_pulse::pause_menu::tests::each_pressing_draws_its_own_music_list` and
  `main_menu_ground_truth::pulses_pause_screens_author_the_music_list_the_table_draws`.

  **The row list reaching the disc's `y` (2026-11-25, ours).** The disc authors its screens clear of the list
  (the PSP's longest, Speed Lap, has seven rows: the seventh's capitals stand on `y = 219`, the list's line
  starts at 220). This build can draw a longer page than the disc's own (PS2 below), so the list keeps the
  disc's `y` while the last row on screen has its capitals' baseline above it (`RowInk::baseline`, the foot of
  `H`: 19 under the pen on the PSP's menu face, 21 on the PS2's; the capitals' full ink reaches 23 with the
  tail of `Q`, which would move the disc's own screen) and otherwise starts one row pitch and the note gap
  under the last row. The arrows and their pointer targets follow the line; the rows' do not move. Frames at
  1440x816 before and after: every PSP page (five, six and seven rows), the PS2's five-row page and Pure and
  HD are byte-identical; only the PS2 six and seven row `--original-menu` pages changed, where the song name
  was drawn over QUIT RACE and RESTART RACE. Tests: `oag_ui::menu::tests::music_list` and
  `pause_menu_ground_truth::the_track_list_is_clickable_where_the_page_leaves_room_for_it`.

  **The pair** (`scripts/pulse-pause.drive`, the original's frame from the
  `time-trial-moa-therma-white-venom-grid` state, START, 4 s, clock 0.40.9; ours played to it, the same frame
  for both columns): whole frame **1.56 % to 1.05 %** of pixels over 40 grey (2,033 to 1,371 px, mean grey 4.17 to
  3.71); the music row (20, 215, 300, 25) **674 px to 12 px**. After RIGHT the original reads `STEADY RUSH` (USA
  disc, song 1) and ours `SLINGSHOT (WIPEOUT MIX)`: ours renders the EU disc, whose song 1 is another. **The
  marquee after a resumed pick** shows on the drive route too (2026-11-24): the null output (`--no-audio`, and a
  machine with no sound device) never advanced its mixer outside `--dump-audio`, so no voice ever ended, the 32 slots
  filled with the race's effects and the race song was refused (`start_race_voice` logged `voice=None`), leaving the
  now-playing display blank even at the race start. `oag_sound::Audio::tick` now renders one tick of the mix and drops
  it on a null output (`tests::no_audio::no_audio_retires_a_voice_when_its_sound_ends_and_frees_its_slot`); the drive
  then draws `APHEX TWIN - FENIX FUNK ...` after GO and the picked song's line after the resume.

  **PS2 Pulse: checked, differs, wired.** The PS2 executable carries the `MusicList` class, and its four screens
  author the widget at `y="270"` in `font="menu"` (its own grid and the rows' face), no `Wrap`, no `AutoRepeat`
  (the ground-truth test above reads all three pressings). `oag_pulse::pause_menu::MUSIC_LIST_PS2` carries that;
  the step, the ends, the no-audition rule, the resume and the arrows are **the PSP's code, read on the PS2 too**
  ([ps2-pulse-eu/music-list.md](../ghidra/functions/ps2-pulse-eu/music-list.md): the same `MusicList` overrides,
  `List` constructor, step functions, arrow cuts and shade, 8 bytes further on), so only the authoring differs. **Seen on PCSX2** (2026-11-24, `data/saves/pulse-ps2/amphiseum-black.p2s`, START, 4 s): the screen
  lists four rows (CONTINUE, GAME OPTIONS, RESTART RACE, QUIT RACE) and the music row stands well below them at
  about y 285 of 448, `FENIX FUNK 5 (WIPEOUT EDIT)` in the rows' face after a smaller pill; ours on PS2 (no
  `--original-menu`) puts the row at about y 284. Ours named the PS2's songs `TRACK N` at first: the PSP pressing that names them was
  looked up on the boot's unsurveyed fields, not its background survey's answer (`MusicDiscs::counterpart`, shared with
  the now-playing line); now ours shows `BREAK YA SELF (WIPEOUT REMIX)` after RIGHT (the EU disc's song 1). One
  difference stays: the `--original-menu` still draws the PSP's six rows on PS2 (GAME MANUAL, PHOTO
  MODE), so the list stands under the sixth (above) rather than at the original's `y` (the pause table is the
  PSP's, `pulses_ps2_pause_screens_*`). **Pure: checked, differs.** Its executable has no `MusicList` class (no string) and its `InGame
  Pause` screen authors no music widget; `oag_pure`'s table has none.

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
style, white and teal, and a Fury-style player sees the same screen in red; the end screens follow
the saved style, [endrace-screens.md](endrace-screens.md), "HD's FE style on the end screen"):

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
- **The HUD under the tint**: `PauseMenu::hud_under_scrim`, on for HD, Pulse (measured 2026-11-23, above) and Pure; off for Omega. The parked
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

**A 2048 finding is checked against Omega, and the reverse: checked, differs** (2026-11-25). Omega's binder reads
the same mode ids as HD's, but 2048's pause screens are not that file's widgets (see [2048](#2048)): Omega ships
2048's `vita/vita_InGame_Definition.xml` in `data00`, `data08` and `data09` and its `skin.xml` loads it, but as a
3,193-byte stub holding the in-race `InGame2048` screen alone, whose redirect names a `ChoosePauseMenu` the file does
not hold. None of 2048's pause screens is in Omega
(`main_menu_ground_truth::omegas_copy_of_2048s_ingame_file_holds_none_of_its_pause_screens`), so Omega's pause stays
HD's `Block` and `List` file.

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
  implementation on every title, maintainer). So ours has one row more than the original on every screen, except in a
  compare shot: `--original-menu` leaves it out (`original_menu_leaves_photo_mode_out_of_the_pause_screen`).
- **END SESSION** (Free Play) leaves the race the way QUIT does, for the main page: **chosen, not measured**. The
  original shows the one-row `RETURN TO MAIN MENU` screen first; this build races Free Play as a single race with no
  session results to show (`docs/formats/pure-campaign.md`, "The modes"). Free Play is chosen by the campaign cell's
  name (`Session::pause_mode`, key `free_play`), not by the engine's mode.
- **The words** are the disc's own (`Continue`, `Restart`, `Quit`, `End Session`, `Press X to Confirm`, drawn by the
  front end's face). The PSP footer's cross glyph is the font's.

**Measured against the original** (`scripts/pause-band-metric.py`, both frames at 480x272; ours is a 1440x816
capture box-filtered down): the two rules at rows 75 and 175 on both, x 0 to 480; the first row's ink at rows 103
to 114 against 103 to 114 and its x from 188 to 291 against 188 to 293 (that script's threshold-95 metric; per glyph on the pair the widths are equal and ours starts 1 px left, `docs/tools/compare-page.md`, "The band's words and the HUD top, measured again"); the second row's 121 to 132 on both and the
third's 139 to 150 against 140 to 150; the cursor row's ink 252 against 253 and the others 156 against 157; the
Tournament screen's first row 104 to 112 against 104 to 113, x 197 to 282 on both. The mean absolute colour
difference over the first row's box is 22.8 of 255 per channel on Single Race and 42 on Tournament, where the
backgrounds differ and a one-pixel glyph shift is worth about that: the placement numbers above are the ones to read.

Residuals:

- **The cursor wraps** in ours (the menu's rule on every title); the original's does not. Not changed here.
- **Our rows**: PHOTO MODE is extra, so the third row on the original (QUIT) is the fourth in ours.
- **The race's now-playing line** (the music title at the foot, "COLD STORAGE - ONYX"): the original's frames show none.
  The disc authors that text as widgets of the `InGame` screen
  (`InGameArtistText`, `InGameTrackText`, `InGameLabelText`), and the pause screens are siblings of it, so the original
  cannot draw it while paused. **Fixed**: `PauseMenu::song_line_under_scrim` is `false` on Pure (`true` on the titles
  with no such measurement) and the HUD drawn under the tint leaves the line out; the pair's song box went from 335 to 54
  px over 40 grey (`docs/tools/compare-page.md`, "Pure Main Menu and pause menu pairs").
- **Craft livery**: the Feisar in ours is gold, the original's blue and yellow. Not this lane's.

Tests: the three screens' rows against both pressings, the redirect's rows against `by_mode` and the band, rules and
menu place (`main_menu_ground_truth::pures_pause_screens_are_the_three_menus_the_ingame_redirect_picks_between`);
the table (`oag_pure::pause_menu::tests`); the draw and the pointer
(`oag_ui_screens::pause::tests::menu_widget`); the game driving a real race (`pause_menu_ground_truth::pure_screens`:
the rows per mode, START, CIRCLE, RESTART, QUIT and PHOTO MODE by pad).

## 2048

**Wired** (`Title::pause_menu` is `oag_2048::pause_menu::PAUSE_MENU`). The screens are in
`Data\Plugins\Frontend\NEWGUI\InGame_Definition.xml` (36 screens). **They are not HD's and Omega's `Block` and
`List` widgets: checked, differs.** Each is a page of `<TouchButton>` icon tiles, the file's touch idiom
([`2048-frontend.md`](../formats/2048-frontend.md)), nested in `PauseMenuTop` (a `BackendController` and a
`SceneBlur`), with a confirm screen nested in it for each tile that destroys the race:

| Screen | Tiles (the widget's `idstring`, else its `name`) | Opened by |
| --- | --- | --- |
| `PauseMenuRace` | `IG_PAUSE_OPT`, `MSC_PHOTO_PHOTOMODE`, `quitButton`, `restartButton`, `continueButton` | the redirect's default: Single Race, Time Trial, Head to Head, Zone, Detonator |
| `PauseMenuTournament` | `IG_PAUSE_OPT`, `MSC_PHOTO_PHOTOMODE`, `quitButton`, `continueButton` (no restart) | mode 4 |
| `PauseMenuSpeed` (in `PauseMenuSpeedTop`) | `IG_PAUSE_OPT`, `MSC_PHOTO_PHOTOMODE`, `CHOOSE_GHOST`, `quitButton` (END SESSION's icon), `restartButton`, `continueButton` | mode 10 |

`PauseMenuOnline`, `PauseMenuPSNDEMO` and the demo screens are a network race's and a retail demo's, never opened here.
**Which mode opens which** is the executable's:
[`PauseMenu_BindRedirectForMode`](../ghidra/functions/vita-2048-eu-v104/pause-menu.md) (`0x8112e050`, 80), the
same switch on the same mode ids as HD's (**ported**, 80). `QUIT` and `RESTART` open `PauseQuitConfirm`/
`PauseRetryConfirm` (`TournamentPauseQuitConfirm`, `SpeedPauseQuitConfirm`/`SpeedPauseRetryConfirm`): a dimming
(`0x30000000`), a white panel, the tile's icon, `FE_QUIT`/`FE_RETRY`/`IG_PAUSE_ENDSES`, `FE_ARE_YOU_SURE` and a tick
tile (the action) and a cross tile (`PreviousScreen`). **The three copies of the file**: the v1.04 patch's
(43,418 bytes, served ahead of the base) holds all of this; the base package's (40,167 bytes) has no
`PauseMenuTournament` and no `TournamentPauseQuitConfirm` and reads the same in the screens it has. The files in
Omega and `LegacyInGame_Definition.xml` (`Race End Photo`, `Race End Alone`: the race's end, not a pause) are not
this.

### What the original draws

One Vita3K frame, a race event on Vineta K in the default screen
(`data/reference/2048-circuit-names/vineta-k-pause.png`, 960 by 544; the frame the circuit-names lane kept):

- **The tiles are the authored boxes** at `(92, 145)` and `(250, 145)`, 95 square, and `(560, 430)`, `(690, 430)`,
  `(820, 430)`, 120 by 95, in `Blue2048` (`0xff19295d`), each with its icon centred at the texture's own size; the
  header is a `Blue2048` 70 square at `(15, 15)` with the white pause icon, `GAME PAUSED` in `Blue2048` at `(105, 30)`
  and a 405 by 5 rule under it, over a white panel (`0x50ffffff`, `(0, 15, 960, 510)`).
- **`smalltext="true"` labels** under the two upper tiles: `OPTIONS` caps `110` to `168` by `253` to `264`,
  `PHOTO MODE` `254` to `339`: 11 units tall with the cap line 13 under the tile, against the regular label's 16 and
  18 (`oag_ui::frontend::label_under_tile`, `SMALL_LABEL_RATIO`; confidence 80, one frame, two labels).
- **The scene is blurred** (`SceneBlur`), and **an event card** stands at `(416, 87)` to `(940, 420)` (`VINETA K`,
  `TARGET (NOVICE)`, three medals and the page dots): the screen's `<Callout CurGameMode="true">` at `(198, -5)`,
  the same card the campaign map opens, whose layout is native code (`docs/formats/2048-frontend.md`, the event card).
- No cursor: the original's touch tiles answer touch only.

### What ours does

The page is the disc's own screen: `oag_ui_screens::pause::PauseScreen::read_touch` reads the screen and the screens
it is nested in (the panel, the heading and the icon are `PauseMenuTop`'s children or the parent's; `PauseMenuSpeed`
stands in `PauseMenuSpeedTop`), and draws one box and icon per tile through the same touch idiom the front end's tiles
use (`oag_ui::frontend::label_under_tile`). Rows are keyed by the widget's `idstring`, else its `name`, else (the
confirm screens' tick and cross) its `redirect` (`PauseRow::disc_id`; an icon-only tile is worded by the HD pause
strings 2048 also ships, `PauseRow::word`). Pad (UP and DOWN in the file's order, CROSS), pointer and touch
(`oag_ui_screens::pause::regions`, a tap is a click) drive the same menu rows as on the other titles: START and
CONTINUE resume, RESTART loads the race again, QUIT leaves it; PHOTO MODE is the disc's own tile. **QUIT and
RESTART ask first** (`PauseRow::opens`, `Definition` pages `pause_screen:<screen>`, opening on the way back,
`Page::opens_on`): the page they ask on stays drawn under the dimming, a tap on the tick does the action, a tap on the
cross or CIRCLE goes back. The page being left in a transition is the disc's screen on 2048 (it faded as the front
end's list, ghosting for about 25 ticks); the other titles' transitions are unchanged.

- **GHOST** (`CHOOSE_GHOST`, Speed Lap) opens `PauseMenuGhost`, a sibling screen: the OPTIONS and PHOTO MODE tiles
  again, the ghost tile in `Orange2048` (the page you are on: the disc authors `Colour="FEGlobals->Orange2048"`), a
  `Touchlist` `GhostOption2048` at (430, 145) 510 by 225 with four entries (`GHOST_NONE`, `GHOST_MY_BEST`,
  `GHOST_FRIEND_CHALLENGE`, `GHOST_GLOBAL_CHALLENGE`, each with an icon) and a tick back to `PauseMenuSpeed`. **Built**:
  NONE and MY BEST are `display.ghost_ship_visible` off and on (one `Choice` row, `pause_ghost_list`; a tap on an entry
  is `Part::Value(i)`, LEFT and RIGHT step it, CIRCLE or the tick go back); the friend's and the world's ghost need
  the online service this build has not, so they are **drawn in `Grey2048` and answer nothing** (the disc authors
  no disabled look for a list entry, so the grey, **chosen**, is the palette's own inactive colour, as the options
  sliders use it). The chosen entry is filled in `Orange2048` and the others in `Blue2048`; the strip spaces its four
  entries evenly across the authored rect (**chosen**: no entry authors its own, as on the front end's touch lists).
  The page was drawn from the disc's strings and icons (`every_mode_reads_words_and_draws_its_tiles_...`) and not
  pictured: no headless route reaches a Speed Lap pause on 2048 and the Vita has no frame of it here.
- **OPTIONS** opens this build's OPTIONS page, as on HD; the disc's is the touch `pause_options` tree
  (`InGame_PauseOptions.xml`: camera, audio, controls, pilot assist as touch lists and sliders). **Chosen.**
- **The scene blur and the event card (`Callout CurGameMode`) are not drawn**: the blur is not in this build's
  front end, and the card is native code whose layout is unlocated (open). The scene under the panel is sharp.
- **Pad and keyboard** (**chosen**: the original's tiles answer touch only, `docs/ghidra/functions/vita-2048-eu-v104/pad-focus.md`:
  no `TouchButton` reads a pad; the pause screens author `forward="start"` and `backward="none"`, so START resumes
  and CIRCLE does nothing on the root, both as here, 85). UP and DOWN walk every tile in the file's order (so every
  tile is reachable and a drive's `select` works), LEFT and RIGHT move along a row by position
  (`oag_ui_screens::pause::tile_neighbour`; a press with no tile that way, and on a list row where they step the
  value, stays the menu's: `MenuStage::step_pause_tiles` takes a button only when it moves), CROSS or ENTER activates, CIRCLE or ESCAPE backs out of a confirm screen
  (ESCAPE on the root resumes, as on every title), START resumes. The cursor fills the focused tile in `Orange2048`,
  the tile's own pressed colour, **only while the pad spoke last** (`draw_list_for`): a mouse or finger over the tiles
  leaves them at rest, as on the front end's touch grids, and a click or tap is the tile's press. A confirm screen opens
  on its cross, not where its cursor was left. Test: `pause_menu_ground_truth::y2048_screens::the_pad_walks_the_tiles_by_position_and_start_resumes`.
- **The HUD under the panel** is off, **inherited from Omega** (`hud_under_scrim`); the Vita frame was taken on the
  campaign's pause with the HUD hidden by its own card, so this is not measured. **The slide-in transitions**
  (`transition`, `delay` on every widget) are not played.
- Words are the disc's: `GAME PAUSED`, `OPTIONS`, `PHOTO MODE`, `QUIT`/`RETRY`, `Are you sure?`.

Measured against the Vita frame (`GAME PAUSED` caps `105` to `346` by `40` to `67` against ours `105.7` to `347.7`
by `40.7` to `66.7`; the OPTIONS label `109.3` to `168` by `253.7` to `263.7` against `110` to `168` by `253` to `264`;
ours is the first event of the campaign, not Vineta K, so the card and the scene differ).

Tests: the rows against the file in every copy and the Omega stub
(`main_menu_ground_truth::y2048s_pause_screens_are_touch_pages_tile_for_tile_in_the_served_copy`,
`omegas_copy_of_2048s_ingame_file_holds_none_of_its_pause_screens`); the mode-to-screen table
(`oag_2048::pause_menu::tests`); the draw and the confirm page (`oag_ui_screens::pause::tests::touch_tiles`); the
rows, words, icons and boxes per mode off the disc
(`pause_menu_ground_truth::y2048_screens::every_mode_reads_words_and_draws_its_tiles_with_their_icons_off_the_disc`);
the game driving the first event: START, QUIT and RESTART asking, a tap on a tile
(`pause_menu_ground_truth::y2048_screens`).

## Other titles

Nothing is unwired. Pure's screens share nothing with 2048's or Omega's (a `<Menu>` of entries on a band, not
`Block` or `<TouchButton>` widgets), so Pure's finding is **checked, differs** against both; 2048's are touch
tiles, **checked, differs** against HD and Omega (see [2048](#2048)).
