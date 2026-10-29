# ADR-0078: A mod is turned off by id, live, and the built-in never

## Status

Proposed (2026-10-29). Builds on [ADR-0066](0066-the-menu-is-the-disc-plus-mods.md)
(mods, and the menu as the disc plus mods) and [ADR-0077](0077-a-custom-campaign-compiles-to-the-titles-own-grid.md)
(custom campaigns, which may race another mod's track).

## Context

Until now a mod was on while it was on the search path: the MODS page listed it and
nothing else. The maintainer asked for the page to turn each mod on and off. Four
things had to be decided: what is stored, when a switch takes effect, whether the
built-in `oag-extras` can be switched, and what happens to a campaign that races a
track of a mod that is off. A mod reaches the game through menu rows, strings,
campaigns, pilots, tracks (campaign events and RACE REMIX's CUSTOM title) and the LAN
content fingerprint.

## Decision

1. **The settings file stores the mods turned off**, `[mods] disabled = [<mod id>, ...]`
   (`oag_game::settings::Mods`, a `BTreeSet` for a stable rewrite). A mod installed
   later is on. An id no installed mod has is kept, so a mod moved out of the search
   path and back stays off.
2. **One gate.** `oag_game::mods::Mods` is loaded with the set, and
   `Mods::player` (campaigns, pilots, tracks, the fingerprint) and `Mods::build` (menu
   rows, strings) skip a mod in it. Only `Built::listing`, the MODS page's list, still
   names it, so it can be turned back on. Every consumer goes through `Mods`, so a new
   one cannot read a mod that is off without asking for the raw list.
3. **A switch applies live.** The ENABLED row saves the set and rebuilds the tree,
   listing, campaigns and RACE REMIX's mod tracks the way a boot builds them
   (`prepare::mods_menu`, `prepare::finish_definition`), then rebuilds the menu model
   with the function the menus open with and reopens it on the MODS page
   (`oag_ui::menu::Menu::trail`/`restore_trail`). Nothing a mod adds is held past the
   front end: a race reads its options at launch, a campaign when it opens.
4. **`oag-extras` cannot be turned off.** It carries OPTIONS, the MODS row, QUIT,
   RACEBOX and MULTIPLAYER (`assets/mods/oag-extras/menu.toml`); off, the player would
   have no way back to the page that turns it on. Its ENABLED row holds one value,
   ALWAYS ON (not dimmed: a dimmed value's reason draws in the note slot, over Pulse's
   detail lines), and `mods::switch` refuses its id.
5. **A campaign racing a track of a mod that is off or missing is left out whole**,
   named in the log with the reason (`mod "oag.hillside" is turned off on the MODS
   page`), and its mod's MODS entry gains a detail line: `NEEDS HILLSIDE, OFF` or
   `NEEDS <id>, MISSING`.
6. **Row shape: the MOD stepper plus one ENABLED row**, not a row per mod. The page's
   rows are `assets/ui/menu.toml`'s, each with a `string_id`; the mods are known only at
   run time, and the detail lines already follow the stepper.

## Alternatives considered

- **An enabled set.** A newly installed mod would be off until the player found the
  page, which is not what installing a mod means.
- **Apply on the next launch, with a note.** Simpler: no rebuild path. Rejected
  because the rebuild is the boot's own code run again; a player who turns a mod off
  and sees its row still there would read it as broken.
- **Let `oag-extras` be turned off.** It is a mod like any other in format, but it
  carries the page's own route; turning it off is a one-way door.
- **Leave out only the event that races the missing track**, keeping the rest of the
  campaign. That is optional-dependency semantics: which events may be dropped and
  how unlocks then flow are the open questions of the mod-dependency thread, held for
  the maintainer. Refusing the whole campaign is what a missing mod already did.
- **A switch row per mod.** Dynamic rows have no `string_id`, would need a second
  mechanism for the detail lines, and on Pulse would push the lines off the page.

## Consequences

- The menu model is rebuilt on every switch: the player's place is kept by page id
  and row id or setting, but a scroll position or a half-made choice on another page
  is reset to what the settings hold.
- A race parked under the menus (`Session::suspended_race`) keeps the mods it started
  with until it ends.
- The LAN fingerprint counts only the mods that are on: a machine with a mod turned off
  matches one that does not have it.
- `oag-mod`'s CLI (`just mods-list`) does not read the game's settings and lists every
  mod it finds.
- A campaign mod whose dependency is off shows no campaign; until mods declare
  dependencies, the NEEDS line is found only when its campaign is compiled.
- Pulse's page has room for four detail lines under three rows, so it drops AUTHORS,
  VERSION, then LICENSE as it needs to (`Listing::table_within`); HD shows all.
