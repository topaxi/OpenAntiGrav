# Back and Help on every front-end screen

Which pad button leaves a front-end screen and which opens its Help is **one decision per title,
pressing and player's choice**, taken in one place and handed to every screen model that asks.
No screen names the circle or the triangle as its Back.

## What the original does

The disc's front-end XML never says "circle" for Back. It says `backward` and `cancel`, and each
executable binds those abstract controls to a pad bit:

- the PSP's `Input_ParseButtonName` binds both to the **circle**;
- the PS2's binds both to the **triangle** (`docs/ghidra/functions/ps2-pulse-eu/input.md`, "The
  divergence", confidence 90). Pulse on the PS2 then authors Help on the circle where the PSP
  authors it on the triangle, and the footers draw the pair that way round.

A screen that wants a different button for something names it in its own `Redirect`
(`forward="circle"`), and that wins over any rule.

## Census of the authored redirects

Method: every front-end XML of the disc, expanded where shortened (`oag-wad extract <image>:<wad>
-o DIR --expand`; the HD, Omega and 2048 `.xml` entries read by path out of their PSARCs), then
`scripts/fe-redirect-census.py DIR --buttons`, which prints each `Redirect`'s screen, `forward`,
`backward` and target. Pulse's PS2 `WADS2.WAD` (210 XML entries) and PSP `Data.wad` and
`FEData.wad` (215 and 64), Pure's three WADs, HD's `DATA00`-`DATA06`, Omega's merged `data00`-`data09`
and 2048's `data.psarc` were read. The values are the disc's own data (confidence 95); the readings
below leave out the directions, `start`, `none` and the movie-skip screens.

**Where the pressings differ** (every other authored redirect is the same on both):

| Screen | Pulse PSP | Pulse PS2 | Meaning |
| --- | --- | --- | --- |
| `Cell Selection` | `forward="triangle"` to `Cell Help` | `forward="circle"` | Help |
| `Team Selection` | `triangle` to `Team Help`; `square` to `Pre Race Music Select` | `circle` to `Team HelpSingle`; `square` to the music select | Help, Music |
| `Tournament C` | `triangle` to `Tournament Help` | `circle` | Help |
| `Track Creation` | `triangle` to `Track Help` | `circle` to `Track HelpSingle` | Help |
| `Single Player`, `Split ScreenH`, `Split ScreenV`, `Cell Creation`, `Team SelectionSplit`, `Tournament CSplitH`/`V`, `Track CreationSplit` | no such screen in the PSP's XML | `circle` to each screen's own Help; `Cell Creation`'s `square` opens `Create Menu` | Help |
| `Stats - 1` to `Stats - 12` | `forward="circle"` to `Profile` | `forward="triangle"` to `Profile` | Back |
| `DisplayProfileLoadList` and the other seven memory-card lists | none | `forward="triangle"` to `Manage Profile` | Back |

**The same on both**: `Cell Help` and the other Help overlays close on `cross`, `triangle` or
`circle` (three `Redirect`s with `backward=`); `forward="backward"` (the abstract Back control) on
`Cell Selection`, `Cell Setup`, `Cell Creation`, `Records`, `Profile Details` and the PSP's `Manual
Top Level`; `forward="start"` on the pause screens; any of the five face buttons on the movie-skip
screens (`LogoFMV`, `Play Intro`).

**Pure, HD, Omega and 2048** author no Help redirect on the circle or the triangle. Pure has only
`backward` tokens and the demo's `any`. HD: `Stats - n` leaves on `circle`, the studio logo and
the movie screens skip on any face button, the in-game and photo screens on `square`/`start`. Omega
re-ships HD's and 2048's files (`forward="backward"` on `Records` and `Cell Selection`, `forward="cancel"` on
`WOW Download`). 2048 adds `circle` on `Demo Build Vita Lobby` and `Demo_Definition`'s
`dlcTestScreen`, and `start` on its tutorial movies. So the only title whose `backward` is not the
circle is Pulse on the PS2. Omega's and 2048's `backward` binding is the circle by lineage (HD's
`Stats` pages author it) and by their executables' button tables, not measured on the PS4 or the
Vita: **checked, differs: not at all, but not measured there**.

## The resolver

`oag_title::Title::menu_buttons(platform, choice, authored_help)` returns
`oag_title::prompts::MenuButtons { back, help, glyph_swap }`. `FrontEnd::back_button(platform)` is the
title's and pressing's own Back (Pulse's PS2: the triangle, everyone else: the circle);
`choice` is CONTROLS > MENU BACK (`original`, `circle`, `triangle`); `authored_help` is the
screen's own Help redirect (the triangle where it authors none). Back is the choice, or the
original's under `original`. Help is the screen's authored button unless Back takes it, and then
it is the button Back displaced.

| Title and pressing | Help authored on | choice | back | help |
| --- | --- | --- | --- | --- |
| Pulse PS2 | circle | original | triangle | circle |
| Pulse PS2 | circle | circle | circle | triangle |
| Pulse PS2 | circle | triangle | triangle | circle |
| every other title and pressing | triangle | original | circle | triangle |
| every other title and pressing | triangle | circle | circle | triangle |
| every other title and pressing | triangle | triangle | triangle | circle |

`oag_game::prompts::roles` turns it into `oag_ui::roles::Roles { back, help }` (buttons, not prompt
codes), `Session::roles()` and `Session::roles_with_help(authored)` read it from the live settings,
and the session hands the result to the screen model that is up, every tick, before it reads the
pad. A choice therefore applies the moment the CONTROLS row changes. The tables are
`oag_game::prompts::tests::the_roles_a_screen_is_handed_on_every_title_pressing_and_choice` (every
title, pressing and choice) and `oag_title::prompts::tests`.

## What each screen asks

| Screen | Model | Back is |
| --- | --- | --- |
| Menu pages, the pause menu's sub-pages, REPLAYS and every settings page | `oag_ui::menu::Menu` | `Roles::back` (`Menu::set_roles`); at the pause root Back is silent, as `backward="none"` authors |
| Track Select, Ship Select, Team Selection, the split seats' pickers | `oag_ui_screens::picker::Picker` | `Roles::back` |
| Campaign Selection, Grid Selection | `CampaignSelection`, `GridSelection` | `Roles::back` |
| Cell Selection | `CellSelection` | `Roles::back`; Help `Roles::help`, seeded from the screen's own `Redirect` to `Cell Help` (`authored_cell_help`) |
| End of race: `EndRace Menu` | `EndRaceMenu` | `Roles::back`; the original authors no Back here (the results pages and the menu answer a confirm), so it is the Back cue only |
| The on-screen keyboard, the tag entry, yes/no dialogs | `Keyboard`, `TagEntry`, `Confirm` | `Roles::back` cancels |
| Pure's chain, the window layout and seat assignment screens, the LAN and online lobbies and the local join screen | `pure_chain::Choice`, `window_layout::Picker`, `SeatAssign`, `Lobby::set_back` | `Roles::back` |
| The campaign editor | `Session::tick_campaign_editor` | `Roles::back`; the editor's second face action is `Roles::help` |
| 2048's touch front end | `oag_ui::frontend::Frontend` | `Roles::back` (`Frontend::set_roles`); `Roles::help` is its Home key |
| The paused race | `Session::tick` | `Roles::back` leaves the race; the secondary mouse button taps it |
| The ffmpeg download | `Session::sync_ffmpeg` | `Roles::back` cancels |

**What stays on the physical button**, and why:

- **A race that runs.** The circle is the absorb (`oag_raceplay::weapons`) and the triangle is not
  Back there; nothing reads `Roles`.
- **Photo mode, the replay viewer and the screenshot notice.** They read the device through
  `Controls::view_take` and the notice's own spent-buttons list, apart from the front end's
  frame; they are ours and have no original Back to follow. Open: they leave on the circle on the PS2 too.
- **The disc chooser** (`oag-game --launcher`), before any title is known: no title, so the PSP's circle.
- **Escape and the secondary mouse button** emit Back directly on every screen, on every title.
- **The keyboard's keys** stay bound to the buttons they always were (Backspace, `Z` and `Y` are the
  circle, `V` the triangle), so on the PS2 under `original` `V` leaves.

## Footers

A footer's glyph is a stand-in codepoint inside the disc's own prompt string (`oag_title::Prompts`),
not a widget a screen could ask a button of, so it stays a substitution: when `Back` is not the
button the disc authored its footers for (`MenuButtons::glyph_swap`), `oag_game::prompts::substitution`
draws the circle's and the triangle's stand-ins the other way round, and `Help` follows because
it is the displaced button. The same resolver decides both halves, so a footer and the screen
cannot disagree. Stills (Pulse PS2 EU): `original` draws the triangle beside `Back` and the circle
beside `Help` on Track Select; `circle` draws them exchanged.

The window title's key hint (`OAG_HINTS_SHELL_KEYS`, "backspace, circle or escape goes back") is
prose in twenty languages naming the circle: it is wrong on the PS2 and under `triangle`, and open.

## Not built

The pickers' Help screens (`Track Help`, `Team Help`, `Tournament Help`, `Pre Race Music Select`)
are authored on the screens above and not drawn (`docs/ui/selection-screens.md`, "Open"), so the
PS2's footers draw `Help` beside the circle on Track Select and the circle does nothing. Wiring
them is `Roles::help` on `Picker`.

## Tests

- `oag_ui::roles::tests` and a `..._on_the_button_it_is_handed` test on every model above;
- `oag_game::prompts::tests::the_roles_a_screen_is_handed_on_every_title_pressing_and_choice`;
- `campaign_cursor_ground_truth` (disc-backed, `#[ignore]`d): Pulse PS2 EU's race page, track picker,
  ship picker, keyboard prompt and pause page leave on the resolved button and not on the other, for
  every choice (`pulse_ps2_*`); the PSP's four screens and HD's race page still leave on the circle
  (`pulse_psp_screens_still_leave_on_the_circle`, `hd_race_page_still_leaves_on_the_circle`); the
  campaign screens are the tests of the same file above them.
- All 33 menu comparison pairs (2048, HD, Omega, Pulse PSP, Pulse PS2, Pure) re-rendered with the build
  before this change and after it are byte-identical: no pair moved.
