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
both. A file edited by hand is cleaned the same way on use, so a name can always be drawn on every
title.

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

The footer's `<TextInfo type="tag">` (`NewsItemTagText`) now draws the stored tag at its authored
`(30, 235)` in the `Default` face (`NavigationLegend::set_tag`); `docs/ui/menus-original.md`,
"What still differs, ranked", item 3. The face and colour are not measured against the original:
the colour is the file's `FEGlobals->BackgroundColor`, the face is the ticker's.

The online lobby's name field opens on the name the last connection used, else the stored player
name (lower-cased: the lobby's keys are lower case), else the login name.

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
- **Steam Deck**: not wired. Detection by the `SteamDeck` environment variable is plausible and the
  virtual keyboard is opened by the client's `steam://open/keyboard` URL or Steamworks'
  `ShowFloatingGamepadTextInput`, which needs an AppID a non-Steam shortcut does not have. None of
  that was run (no Deck). To test: from a non-Steam shortcut, run `steam steam://open/keyboard`
  in a terminal in Gaming Mode and see whether the keyboard rises over the game; if it does, call it
  when a typing prompt opens and `SteamDeck=1`. Until then the controller path is what works there.
