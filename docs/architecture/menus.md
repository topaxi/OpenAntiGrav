# The menus

**Ours, not the disc's.** Every other document in this tree describes something
recovered from a Wipeout Pulse executable or a Wipeout Pulse asset. This one
describes a thing this project invented, and says why inventing it was the right
call rather than a shortcut.

Implemented in [`crates/game/src/menu.rs`](../../crates/game/src/menu.rs), with
the tree itself in [`assets/ui/menu.toml`](../../assets/ui/menu.toml) and the
disc-side lists in [`crates/game/src/catalogue.rs`](../../crates/game/src/catalogue.rs).

## Why not the front-end XML

The disc carries `Data\Plugins\PI001\GUI\MainMenu_Definition.xml`, and
[`screen.rs`](../../crates/game/src/screen.rs) already parses that dialect - the
language picker is drawn from it. Reproducing the main menu from it is
technically the shorter path, and it is still the wrong one.

The original's menus are the menus *Pulse* has: on a PSP, for a player with a
Memory Stick, with no mouse, no window, no monitor to pick a refresh rate for
and no keyboard to rebind. This project needs the menus a PC game has. Extending
a recovered tree with branches the original never had produces a structure that
is neither faithful nor ours, and - worse for a project whose main deliverable is
its documentation - **every entry we add starts to look like a recovered one.**
A reader six months from now cannot tell which rows came off the disc.

So the tree is ours and it is written down in one file that says so. What the
disc's own menu XML contains is a separate, still-unread question, and it belongs
on a page under [`docs/formats/`](../formats/README.md) when someone reads it.

## What still comes off the disc

Not the *structure*, but the *contents*, wherever the contents are a property of
the release rather than of this project:

| Row | Where its options come from |
| --- | --- |
| TRACK | `Data\Plugins\PI001\Definition.xml`, and the label from the language's string table |
| LANGUAGE | the language plugins `PI008`-`PI012` |
| TEAM | `oag_formats::handling::TEAMS`, pinned to the eight shipped `handlingstats.xml` files by a test |
| SPEED CLASS | `oag_physics::SpeedClass::ALL` |
| WINDOW MODE / SIZE / ASPECT / RENDER SCALE | `oag_game::display`, pinned to its own `ALL`/`OFFERED` lists by a test |

That split is what keeps [ADR-0006](adr/0006-no-copyrighted-content.md)
satisfied while the Race page still says "Talon's Junction White": the words are
read out of the player's own disc at runtime and this repository contains a
plugin id at most.

**A circuit is an entry, not a directory**, and getting that wrong is the trap
this arrangement exists to avoid. `16_Track` and `32_Track` are two rows on the
menu and one folder on the disc - the second is the first driven the other way,
distinguished by `Reversed="True"` on its `Values` node. Listing
`Data\Environments\*` would therefore show twelve circuits where the game offers
twenty-four, and no amount of path arithmetic recovers the difference. The
settings file stores the plugin id and only the source can say which `.vex` that
id loads.

## The format

A **flat list of pages keyed by id**, not a nested tree. Three consequences, all
of them the reason:

- TOML past two levels of nesting is unpleasant to hand-edit, and this file is
  meant to be hand-edited.
- The back stack becomes a `Vec<usize>` of page indices, which cannot hold a
  dangling page.
- "Every target resolves, every action is known, every page is reachable" is one
  pass over a flat map.

```toml
version = 1
root = "main"

[[page]]
id = "main"
title = "OPENANTIGRAV"

[[page.entry]]
kind = "submenu"
label = "OPTIONS"
target = "options"
```

Six entry kinds: `submenu`, `action`, `choice`, `toggle`, `binding`, `back`.

**`action` and `values_from` are closed sets**, checked at load against enums in
`menu.rs`. A typo in the asset is a startup error naming what it did know, not a
row that does nothing when pressed. Adding an action means adding it to the enum
*and* handling it in the composition root, which is the point: "defined but not
wired up" cannot ship.

### What the loader refuses

Each has a test, and the last is the only one that cannot be caught a row at a
time:

| Refused | Why it matters |
| --- | --- |
| An unknown `version` | A definition written against a different entry vocabulary would otherwise load with rows silently missing |
| A duplicate page id | A `target` would be ambiguous |
| A dangling `target` | |
| An unknown `action` | Reported with the set it does know |
| An unknown `button` | |
| `values` **and** `values_from` | A list has to come from exactly one place, or it is undecidable which wins when the source turns out to be empty |
| Neither `values` nor `values_from` | A choice with nothing to choose |
| A page nothing links to | A menu somebody wrote and forgot to hang off anything. Parses, resolves, and ships silently without this |

Two more tests pin the asset against the code's own lists, which is the class of
mistake the format check cannot see: the TEAM row must *be*
`handling::TEAMS`, and every anisotropy value must parse as `Anisotropy`. Both
would otherwise fail deep inside an archive lookup at race load, with a message
about a missing WAD entry rather than about a menu.

**All of this runs in CI**, unlike most of this repository's interesting tests,
because the thing under test is ours and needs no disc image.

## The seams

The menus never read or write settings, and never touch a disc. Three narrow
calls carry everything:

- `Menu::supply(source, options)` - what a row *may* be set to, for lists that
  come off a disc. **A row keeps the value it was already on** if the new list
  still has it. Resetting to the first option instead is the bug this rule
  exists to stop: a row seeded to the player's language and then handed its list
  would draw the wrong one, and the first nudge of it would persist that as a
  deliberate choice.
- `Menu::seed(setting, value)` - what a row *is* set to. A value the row does not
  offer is ignored rather than added, so a stale config file cannot smuggle an
  unreachable option onto the list.
- `MenuEvent::Changed { setting, value }` - what the player moved. The
  composition root applies and persists it.

A `choice` option carries a stored `value` and a displayed `label`, and they are
different strings exactly when the list came off a disc: `16_Track` against
"Talon's Junction White". That distinction is what lets the menus name a circuit
without this repository containing its name.

## What is not built

- **Rebinding.** `oag_input::keys::map_key` is a hardcoded `match`; a `binding`
  row displays what it returns and cannot change it. Rebinding needs a table,
  persistence and conflict handling of its own. The row exists so the format is
  settled before that lands and so the gap is visible in the game rather than
  only here. `keys::bound_keys` asks `map_key` itself about a candidate set
  rather than keeping a second table, so what the page shows cannot drift from
  what the game does.
- **Localised labels.** Row labels are literal text. `string_id` is accepted and
  not read, so adding localisation later is not a format change.
- **Applying a language without relaunching.** The LANGUAGE row writes the
  setting, and the string table it selects is loaded once at boot; the change
  therefore lands on the next launch. Anisotropic filtering is the same, and is
  less noticeable because a race is built after the menus anyway.
- **Anything a HUD needs.** When `oag-ui` exists (M5, see
  [workspace layout](workspace-layout.md)), this module moves into it.

## What deliberately bypasses the menus

`--race`, the `--screenshot` capture path and `oag-trace` all go straight to a
track. A physics test must not depend on menu navigation, and a capture harness
is not a shell. `--menu-page <id>` draws one page with `--screenshot` for
looking at a layout without walking to it.

## Persistence

`settings.toml` gains `[race]` (class, team, circuit id) and `language`. A
setting is written on the keypress that changes it, not on the way out, because
there is no way out that is guaranteed to run - a player quits with the window
button as often as with the menu, and the file is a few hundred bytes.

`language` is what skips the picker on the second run. The picker state is still
entered and left the same frame rather than being skipped in the state machine,
so the transition sequence `boot_ground_truth.rs` asserts on is unchanged. A
language this source does not carry falls back to asking, with a note.
