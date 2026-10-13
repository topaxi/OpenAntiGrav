# ADR-0066: The main menu is each title's own, plus rows that mods attach

## Status

Accepted (2026-10-12). Relates to [ADR-0058](0058-per-title-behaviour-is-title-data-with-provenance.md)
(per-title behaviour is `Title` data), [ADR-0061](0061-two-asset-layers-around-the-disc.md)
(the override and fallback layers) and [ADR-0064](0064-project-toml-files-declare-a-toml-schema.md)
(every TOML file of ours declares a schema). Does not supersede the menu tree's
"the shape is ours" rule: pages stay ours; what changes is the main page.

## Context

`assets/ui/menu.toml`'s `main` page was one list for every title: RACE CAMPAIGN,
RACEBOX, REMIX, RECORDS, OPTIONS, QUIT, with the campaign row reworded for Pure. No
original has that top level. Pulse's has seven rows, Pure's five, HD's and Omega's
five on a horizontal strip, 2048's a grid of touch tiles. A mod system, which the
project wants for tracks, craft and pilots, needs somewhere to put a menu row that
does not depend on a row's position, and the project's own additions (REMIX, QUIT,
RECORDS) are the first such rows.

## Decision

1. **A title's vanilla top level is its disc's.** `oag_title::MainMenu`
   (`Title::main_menu`) lists the rows the disc authors, by `idstring`, in the disc's
   order, with the `menu.toml` row each opens (`builds`) or `None` where nothing is
   built behind it. Ground truth reads every pressing and every archive that carries the
   file and compares the whole list (`crates/raceplay/tests/main_menu_ground_truth.rs`).
2. **Every row has a stable `id`.** `menu.toml`'s `main` page holds only the rows an
   original menu can have (`campaign`, `racebox`, `options`, `records`); a title shows the
   ones it `builds`. Ids are ours, never positions.
3. **Our own rows are a mod.** `oag-extras` (`assets/mods/oag-extras/`) is embedded in
   the binary and read through the same reader a player's mod goes through. It adds
   REMIX, QUIT, RECORDS, and RACEBOX or OPTIONS or RECORDS where the title's own menu has
   none (`only_if_missing`).
4. **A mod row says where it goes with `parent`** (a top-level row's id, or `top`) and
   optionally `after` a sibling. There is no `children` key. A `parent` the title's menu
   lacks but the tree knows places the row at the top level; an id nobody knows is an
   error naming the mod and the key, as are an unknown action, page or string id. A row
   attaches to a top-level row only, and is a `submenu` or an `action` of the closed
   vocabulary: a mod cannot invent behaviour.
5. **Attached rows draw per title** (`MainMenu::attached`): inline after the parent on
   a flat list (Pulse, Pure, 2048), or **revealed** below the parent while it or one of
   its attached rows has focus on a horizontal strip with no room for more tabs (HD,
   Omega). The reveal is chosen, not measured: no disc authors a second level.
6. **A mod is a folder, a `.zip` or a `.oagmod`** (a zip under our own extension, same
   reader), read in place and never unpacked, with `oag-mod.toml` at its root
   (`schemas/v1/mod.tosd`) and by convention `menu.toml` (`mod-menu.tosd`) and
   `strings/<language>.toml`. Entries may be stored, deflate or zstd (method 93).
   Searched under `<data dir>/oag/mods/`, beside the program and `data/mods/`, or
   `$OAG_MODS`.
7. **No licence is allowed and visible.** A mod with neither `license` (an SPDX
   expression) nor `license-file` loads, and the load report marks it "no license stated".
8. **A broken mod never stops the game.** One that will not open, repeats an id, wants
   another game version, or has a row that does not fit is left out and named in the
   load report. The built-in extras failing is a build defect and an error.

## Alternatives considered

- **Keep one tree and let a mod edit it by index or position.** Indices shift when a
  row is dropped (QUIT on a phone, TOUCH CONTROLS without a touchscreen); an id does not.
- **A `children` list on the parent.** The parent would have to know its extras, which
  is the dependency this removes: a mod must attach to a row it did not write.
- **One extras file per title.** Duplicates the rows. Per-title behaviour is data on
  `Title`; the extras are written once.
- **Unpack a mod to a cache.** A mod is small, text and meshes; reading in place needs
  no cache to invalidate and no write permission, and is what the DLC path (a 10 MiB pack
  read many times) is different from.
- **Draw HD's attached rows inline.** A fifth and sixth tab pass the right edge of the
  screen the disc's own strip fits in.

## Consequences

- Pure's SINGLE PLAYER, Pulse's seven, HD's five and 2048's tiles are each pinned
  to their disc; a disc that disagrees with another pressing is a recorded finding
  (the USA Pure disc has no `Teasers`; the PS2 Pulse disc authors two more `RACE CAMPAIGN`
  variants and no multiplayer or website row; 2048's `data1.psarc` patch authors a fifth tile
  that `data2.psarc` replaces).
- The main page now only means something once a title and the extras are merged
  (`Definition::build`); `Definition::parse` on the shipped file alone reports the
  `remix` and `records` pages unreachable. Every caller builds.
- The duplicated `racebox`, `options` and `records` rows (once in `menu.toml`, once
  in the extras) must be kept equal: a test asserts it.
- A mod changes only the main page. It cannot add a page of its own, a setting, a
  race mode or a choice row until a later change opens that vocabulary.
- A mod that asks for a string id must ship it (`strings/english.toml`) or use a
  project id. A live language switch does not re-resolve already resolved labels,
  which is how every row behaved before this change.
- The revealed-row navigation (left and right step the top level, up and down
  step inside a family and stop at its ends) is ours and unmeasured.
- A mod that changes a table would change the simulation; this change gives mods no
  such reach (ADR-0061 records the replay consequence).
