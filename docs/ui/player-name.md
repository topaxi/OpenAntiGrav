# The player's name and tag

One name is kept for every title. A 3-character tag is kept too and is asked for only on the titles
that use it. Entry works with a controller, a pointer or touch, and a physical keyboard.

## What a title keeps

`oag_title::Title::profile` (`oag_title::profile::Profile`) carries the name and tag lengths with
their provenance. The table is `docs/formats/fexml.md`, "Which titles keep a name and a tag" (read
from executable strings and shipped XML, confidence 55).

| Title | Name | Tag |
| --- | --- | --- |
| Pulse, Pure | 10 | 3 |
| HD/Fury | 10 | none |
| 2048, Omega | none | none |

A title with no name draws no PLAYER NAME row and is never asked; a title with no tag draws no
PLAYER TAG row (`Definition::drop_unavailable_player_rows`). Pulse PS2 shares Pulse's record, so it
carries the PSP's values (**inherited from PSP**, not checked on PS2).

## Where it is stored

`[player] name` and `tag` in `settings.toml` (`oag_game::settings::Player`), validated by
`schemas/v1/settings.tosd`. It is written when a prompt is accepted, through `settings::save`, like
every other setting: no save slot. An answer with nothing drawable in it changes nothing, so a
stored name is never blanked (`Player::set_name`, `set_tag`).

## What a name may hold

Letters, digits and the hyphen, at most 10 characters; a tag is letters and digits, upper case, at
most 3 (`settings::player::allowed`, `clean_name`, `clean_tag`). That is the intersection of what
every consumer can show: Pulse's `<TagInput>` alphabet has no underscore, and the online lobby's
name keys (`a-z0-9-_`) have no space or punctuation. A test checks every allowed character against
both. A file edited by hand is cleaned the same way when `settings.toml` loads
(`Player::cleaned`), so a name can always be drawn on every title.

## When it is asked

On an interactive launch, when the front end's menus open and the title wants a name and none is
stored: the name, then the tag where the title has one. A title that wants a tag when a name is
stored (a name made on HD, then Pulse) asks for the tag alone. Once per run: escape declines and the
rows under OPTIONS (PLAYER NAME, PLAYER TAG) are the way back.

"Interactive" is an allowlist (`Cli::is_interactive`): any capture, script, `--drive`, trace, race,
dry run, prefetch, network flag or `--original-menu` run never gets the prompt. Boot chains carry
their provenance (ADR-0025) and the prompt is ours, so it hides under `--original-menu` with the
rest of our additions.

## How it is drawn

Pulse plays its own authored `<TagInput name="Name">` / `name="Tag"` row
(`oag_ui_screens::tag_entry`, geometry read live off `TAG_INPUT_SCREENS`). Every other title, and a
source that cannot spell the text, gets the grid (`oag_ui_screens::prompt::Keyboard`) with the
allowed keys. Pure ships no front-end XML on its disc, so it takes the grid (checked against the
real disc: `--menu-page options --menu-prompt player-tag-grid`). `--menu-prompt player-name`,
`player-tag`, `player-name-grid` and `player-tag-grid` draw them as stills.

The title ("ENTER YOUR NAME", "ENTER YOUR TAG") is the disc's `FEGlobals->TitleColor`, `0xFF000000`
(black), the same ink the OPTIONS header uses. The prompt's panel is ours and dark, so
`TagEntry::draw` passes the title through `window_layout::legible` (white at 0.6 where the ink is too
dark, as the window picker does for its boxes); a light authored colour is kept. What the original
draws the title over on its own name screen is not measured. Checked on Pulse EU `--menu-page options
--menu-prompt player-name` and `player-tag` (the title was 1.10:1 against the panel, now readable). On
Pure the live prompt's `pulse_tag_entry` fails for a source that is not Pulse's and the grid opens
(white title); `--menu-prompt player-name` on a Pure disc prints that same refusal and
`player-name-grid` draws the Pure grid. The pilot rename shares `TagEntry::draw`; its `--menu-prompt
rename` still draws the grid, so that still is not evidence for it. Tests:
`tag_entry::tests::a_black_authored_title_is_drawn_so_it_reads_on_the_dark_panel` and the disc-backed
`tag_entry_ground_truth::the_name_and_tag_titles_read_on_the_prompts_panel`.


The footer's `<TextInfo type="tag">` (`NewsItemTagText`) now draws the stored tag at its authored
`(30, 235)` in the `Default` face (`NavigationLegend::set_tag`); `docs/ui/menus-original.md`,
"What still differs, ranked", item 3. The face and colour are not measured against the original:
the colour is the file's `FEGlobals->BackgroundColor`, the face is the ticker's.

The online lobby's name field opens on the stored player name (lower-cased: the lobby's keys are
lower case), else the name the last connection used, else the login name. The footer tag draws only
on a title that uses a tag (`Player::footer_tag`).

## Where the name goes in a race

Seat 1 is `Race::player_slot()`, the grid's first human slot (slot 0 in every local session).
`oag_game::split_screen::name_first_seat` hands the stored name to
`Race::set_person_names` as that slot's name, from `build_race_stage` (every windowed race) and
`race_capture::capture` (`--race --screenshot`). The name then shows in three places that all
read `person_names`:

- the multiplayer tag over seat 1's craft in the other seat's view (`Race::tag_name`), where it
  said `Player 1` before. Seat 2 has no stored name and keeps `Player 2`; with none stored
  seat 1 keeps `Player 1`;
- seat 1's row on Pulse's multiplayer end table (`PersonName::Named`) and HD's field grid;
- on HD's grid also a one-person race's own row, which had no name before (HD keeps a name,
  `Title::profile`): unmeasured against the original.

Only a title whose `Title::profile` keeps a name is given it (Pulse, Pure, HD): 2048 and Omega keep
none and ask when it is first needed (open, above), so a stored name does not reach their tags or
end rows yet.

The name never reaches the simulation or a hash. A network race joins after the stage is built and
`NetPlay::begin_join` replaces the whole list with the lobby's names, so the lobby wins. The
earlier claim that seat 1 already took the stored name at the race start was wrong: nothing
called `set_person_names` on a local race. Fixed at the race start in `oag-game`, not by making
`oag-raceplay` read settings. Tests: `player_tags_ground_truth` (the names a race returns) and
`split_screen_ground_truth::the_stored_name_is_the_tag_over_the_first_seats_craft` (a captured
frame; it fails with the `race_capture` call removed). `build_race_stage`'s own call is not
pinned by a test: it is in the binary, which only a window reaches.

## Keyboards

- **Controller and pointer**: the existing `Prompt` paths, unchanged.
- **Physical keyboard**: `typing::decide_with`. A character the open prompt accepts is typed as it
  is (a name keeps its capitals); one it does not is folded to the case its grid offers. Enter
  accepts, Backspace deletes.
- **Phone** (Android, iOS): `soft_keyboard::sync_soft_keyboard` calls winit's
  `Window::set_ime_allowed` while a typing prompt is open; winit 0.30 maps that to
  `show_soft_input` / `hide_soft_input` on Android and to the soft keyboard on iOS. **Untested on a
  device**; the Android target could not be built on the authoring machine (no NDK clang for `ring`).
  To test: install the APK, open OPTIONS > PLAYER NAME, expect the keyboard to rise, type, press
  Enter.
- **Steam Deck**: not wired. `SteamDeck=1` is set by Steam for a game started on a Deck
  (`docs/ui/button-prompts.md`, item 5, itself unverified without a client). A web search
  (2026-11-06) found only the user's own Steam+X shortcut that raises the virtual keyboard, and
  nothing on a program opening it: **`steam://open/keyboard` and Steamworks'
  `ShowFloatingGamepadTextInput` are from memory, unverified**, and the latter needs an AppID a
  non-Steam shortcut does not have. To test: in Gaming Mode, from a non-Steam shortcut, run
  `steam steam://open/keyboard` in a terminal and see whether the keyboard rises over the game; if it
  does, call it when a typing prompt opens and `SteamDeck=1`. Until then the controller path is what
  works there.

## Open

- The Pulse prompts read the disc (`pilot_editor::Session::pulse_tag_entry`, in
  `crates/game/src/main/session/pilot_editor.rs`: opens the archive, parses two XML entries) on the
  render thread each time a name or tag prompt opens. Inherited from the pilot rename; a boot-time read
  of the `Name` and `Tag` geometry beside `read_nav_legend` would remove it.
- The footer tag draws only where a root authors a `type="tag"` `TextInfo` and the title uses a tag:
  Pulse. Pure's root authors none (checked in a `--menu-page main` shot), so Pure keeps a tag it
  never draws; HD and Omega draw none.
