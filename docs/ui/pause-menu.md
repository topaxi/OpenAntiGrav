# The pause menu

What each title's own pause screen is, and what this build does (2026-11-07, lane `pause-menu`).
Scores follow the [rubric](../reverse-engineering/confidence-rubric.md). **Pulse is wired; HD, Fury,
Omega, 2048 and Pure are not** (see [Other titles](#other-titles)): there, pausing still opens the
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

Frames, same ship and track (Talon's Junction, the Feisar default), original beside ours: Time
Trial, Speed Lap and Single Race, read side by side. The rows, the heading, the row pitch, the
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
- **CIRCLE** resumes the race, where the original does nothing on the root: the abstract back
  button is shared with ESCAPE, which resumes. **Chosen, not measured.**
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

## Other titles

Not wired. What is read: HD's `ingame_definition.xml` has 58 screens and a `Photo` block on the
split-player, tournament and Zone pairs; 2048 has `InGame_Definition.xml` (36 screens) and
`LegacyInGame_Definition.xml`; Pure's has `InGame Pause` and no photo screen, and no Pure pause
menu has been seen live; Omega's front end is HD's (check, do not assume). Next in the order
the lane set: HD and Fury, Omega, 2048, Pure.
