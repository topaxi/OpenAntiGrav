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
(`forward="circle"`), and that wins over any rule. Our parser reads the abstract names as the PSP's
(`oag_core::buttons::button_from_name`: `backward`, `cancel` are the circle), which is harmless today: the only
consumer of a parsed `Redirect` button (`Screen::redirect_for`) is 2048's boot chain
(`oag_ui::frontend::wipeout2048`), and no Pulse screen's Back goes through an XML redirect.

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
| `Single Player`, `Split ScreenH`, `Split ScreenV`, `Cell Creation`, `Tournament CSplitH`/`V`, `Track CreationSplit` | no such screen in the PSP's XML | `circle` to each screen's own Help; `Cell Creation`'s `square` opens `Create Menu` | Help |
| `Team SelectionSplit` | no such screen in the PSP's XML | **no face-button `Redirect`**: `Team HelpSplit` is named only by a `Watch` on the directions (which freezes the page while it is up); what presents it is not in the XML | none authored |
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
circle is Pulse on the PS2. For Omega and 2048 the resolver is **ported** (the same table, tested for
both: `the_roles_a_screen_is_handed_on_every_title_pressing_and_choice`); that their `backward` is the circle is
**inherited from HD by lineage and open**: HD's `Stats` pages author it, and the static read of the button-name
parser in Omega's `eboot.bin` and 2048's executable was not done.

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
| Track Select, Ship Select, Team Selection, the split seats' pickers | `oag_ui_screens::picker::Picker` | `Roles::back`; Help `Roles::help`, seeded from the screen's own `Redirect` to its Help (`picker::help::Help::button`) |
| Campaign Selection, Grid Selection | `CampaignSelection`, `GridSelection` | `Roles::back` |
| Cell Selection | `CellSelection` | `Roles::back`; Help `Roles::help`, seeded from the screen's own `Redirect` to `Cell Help` (`authored_cell_help`) |
| End of race: `EndRace Menu` | `EndRaceMenu` | `Roles::back`; the original authors no Back here (the results pages and the menu answer a confirm), so it is the Back cue only |
| The on-screen keyboard, the tag entry, yes/no dialogs | `Keyboard`, `TagEntry`, `Confirm` | `Roles::back` cancels; their hint lines name it (`OAG_KEYBOARD_HINT`, `OAG_TAG_ENTRY_HINT`, below) |
| Photo mode | `Session::tick_photo` | `Roles::back` leaves; the camera reset is `Roles::help`, the face button Back displaced; the legend names both |
| The replay viewer | `Session::tick_viewer` | `Roles::back` leaves (inside a title; the viewer has no Help) |
| The disc chooser's key prompt | `Launcher::update` | `oag_game::prompts::chooser_back`: no title is known, so MENU BACK `circle` or `triangle` as chosen, else the platform-neutral circle |
| Pure's chain, the window layout and seat assignment screens, the LAN and online lobbies and the local join screen | `pure_chain::Choice`, `window_layout::Picker`, `SeatAssign`, `Lobby::set_back` | `Roles::back` |
| The campaign editor | `Session::tick_campaign_editor` | `Roles::back`; the editor's second face action is `Roles::help` |
| 2048's touch front end | `oag_ui::frontend::Frontend` | `Roles::back` (`Frontend::set_roles`); `Roles::help` is its Home key |
| The paused race | `Session::tick` | `Roles::back` leaves the race; the secondary mouse button taps it |
| The ffmpeg download | `Session::sync_ffmpeg` | `Roles::back` cancels |

**What stays on the physical button**, and why:

- **A race that runs.** The circle is the absorb (`oag_raceplay::weapons`) and the triangle is not
  Back there; nothing reads `Roles`.
- **The screenshot notice.** It dismisses on the notice's own spent-buttons list, apart from the
  front end's frame.
- **Photo mode and the replay viewer inside a race** read the device through `Controls::view_take`,
  apart from the front end's frame, but take the buttons from `Roles` all the same: they are ours and
  have no original Back, so they follow the pressing's (Pulse PS2: the triangle) and the player's
  choice like every screen. The running race itself reads no roles.
- **The disc chooser** (`oag-game --launcher`), before any title is known, has no pressing's own
  Back to follow: it takes MENU BACK `circle` or `triangle` when the player chose one, and the
  platform-neutral circle for `original`.
- **Escape and the secondary mouse button** emit Back directly on every screen, on every title,
  photo mode and the viewer included.
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

## Key hints

The line printed when the menus open (`OAG_HINTS_SHELL_KEYS`, `crates/game/src/app/hints.rs`) names
the keys bound to the resolved Back (`Controls::bound_keys`, the live table, so a rebind shows) and
the pad button's word (`OAG_HINTS_BUTTON_CIRCLE`/`_TRIANGLE`) through its `%s`: `backspace, z, y,
circle or escape goes back` on every title and pressing but Pulse's PS2 under `original` or
`triangle`, where it reads `v, triangle or escape goes back`. The on-screen keyboard's and the tag
entry's hint lines (`OAG_KEYBOARD_HINT`, `OAG_TAG_ENTRY_HINT`) take the button's word the same way,
and the campaign editor's fill key names the face button Back displaced. English, German and
Brazilian Portuguese carry the placeholder (machine translations, `human = false`); French names the
two button words in its `[untranslated]` table and falls back to English with the rest of its hints.
German's old sentence named the key `Rücktaste`; the key names now come from the binding table and
are not translated.

## The pickers' Help overlays

`Track Creation` and `Team Selection` each author a `Redirect` whose `forward` is a face button and
whose `goto` is the screen's Help: `Track Help`, `Team Help` (`triangle`) on the PSP, `Track
HelpSingle`, `Team HelpSingle` (`circle`) on the PS2. `oag_ui_screens::picker::help::Help` reads that
screen off the same XML (`Layout::help`), the picker opens it on `Roles::help` and closes it on
cross, triangle or circle (the three `Redirect`s the overlay authors: Back closes it and does not
leave), or on a click or the secondary button, and `oag_game` draws it with
`campaign::cell_help_draw` in a pass after the page, its footer and the 3D preview, so the overlay's
dimming covers the craft too (`Renderer::renew_quad_buffer` gives that second pass its own buffer). `--drive` reports it as `screen.help`
(`open` or `closed`, only on a screen that has one) and `screen.help_text` (the selected team's own
paragraph).

- **Team Help's second paragraph is the selected team's own description.** `TeamSelection_Update`
  writes the `Team Ship Help` widget from the team's `HelpText` each time the selection changes
  (read on the PSP, `docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`, confidence 90):
  the PSP's team definition authors `helpText="MSC_TEAMDES_<team>"` per team and ours shows it
  (`Picker::with_help_texts`, `Help::draw`); the PS2's authors none, and its executable writes the
  empty field all the same, so the PS2's paragraph is blank.
- **Not drawn, and why.** `Tournament C` has no screen model in this build (so `Tournament Help` has
  nothing to open from) and `Pre Race Music Select` is a `MusicList` widget, not a Help overlay:
  both are new screens (thread `ps2-picker-help-screens`). Their footers are unchanged.
- **Pulse's split screens.** `Track CreationSplit`'s `Track HelpSpilt` is `Track HelpSingle` with a
  488-high dim (a diff of the two screens), and a split race's Track Select is read from `Track
  Creation`, so its Help is the one above (a split-race drive is not possible: `--drive` has no step
  that seats a second pad). `Team SelectionSplit` authors no face-button `Redirect` to `Team HelpSplit`
  (census table), and the PS2 executable's `TeamSelection_Update` only checks whether that screen is
  present and names its Help state by the split flag, nothing opens it (`race-box-screens.md`, "What
  presents `Team HelpSplit`", 85): the shared Ship Select has no Help on the disc, so its halves open
  none.

## Tests

- `oag_ui::roles::tests` and a `..._on_the_button_it_is_handed` test on every model above;
- `oag_game::prompts::tests::the_roles_a_screen_is_handed_on_every_title_pressing_and_choice`, and
  `oag_title::prompts::tests::back_and_help_never_share_a_button` (Pulse's PS2 under `original`
  with a screen that authors no Help once handed Back and Help the triangle);
- `oag_game::app::hints::tests` (the hint sentence on every title, pressing and choice, and that
  every language words the Back button through `%s`);
- `oag_ui_screens::picker::tests` (`help_opens_on_the_roles_help_...`, `on_the_ps2_the_circle_helps...`);
- `campaign_cursor_ground_truth` (disc-backed, `#[ignore]`d): on Pulse PS2 EU's race page, track picker,
  ship picker, keyboard prompt and pause GAME OPTIONS page, for each choice of MENU BACK one launch presses
  the button that is not Back (`expect` the screen still up), then Back (`expect` it left): `pulse_ps2_*`.
  The PSP's race page, track picker, prompt and pause page, and HD's race page, still leave on the circle
  (`pulse_psp_*`, `hd_race_page_still_leaves_on_the_circle`). The campaign screens are the tests of the same
  file above them. Photo mode and the replay viewer leave on the triangle under `original` and on the circle
  under `circle` on Pulse PS2 EU, on the circle on the PSP (`..._photo_mode_and_viewer_...`; Escape leaves photo mode
  on every choice, and the camera reset sits on the other face button); Track Select and Ship Select open
  Track Help and Team Help on `Roles::help`, Back closes the overlay and then leaves, for each choice on both
  pressings (`..._opens_..._help_on_the_...`).
- All 33 menu comparison pairs (2048, HD, Omega, Pulse PSP, Pulse PS2, Pure) re-rendered with the build
  before this change and after it are byte-identical: no pair moved.
