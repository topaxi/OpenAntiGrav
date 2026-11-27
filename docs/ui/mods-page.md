# The MODS page

MODS is this project's own screen: no original has one. It is reached from OPTIONS (the row
`oag-extras` adds) on every title. The decision is
[ADR-0098](../architecture/adr/0098-the-mods-page-is-a-list-of-mods-and-a-detail-page.md); what a
switch does to the game is [mods](../tools/mods.md#the-mods-page).

## The list

One row per mod the run has: its name on the left, `ON` or `OFF` on the right. The built-in
`oag-extras` reads `ALWAYS ON`. A candidate that did not load reads `OFF`, and a mod whose rows
did not fit keeps its switch; both are marked with their error in the note line. Names over 24
characters are cut with `...`.

| Input | On a mod's row |
| --- | --- |
| Accept (cross, or Start; Enter or `X` on a keyboard) | opens the mod's detail page |
| Square (`C` on a keyboard) | toggles the mod where it stands |
| Left, right | step the switch, as on any value row |
| Back | leaves the list |
| Click or tap on the name | opens the detail page |
| Click or tap on the `ON`/`OFF` column | toggles |
| Hover | selects, and never scrolls the list |

The footer's Square prompt reads `TURN ON/OFF` (`OAG_MODS_TOGGLE`) while the selected row can be
toggled, and is a click target like every footer hint. The built-in and a mod that never loaded
show no prompt, have no toggle column and only open. **2048 shows no prompt**: its menus draw no
footer legend ([back-and-help.md](back-and-help.md), "Footer hints are clicks").

On a text-row title (Pulse, Pure, 2048) the toggle column is the right-hand 22 % of the row. On
Wipeout HD and Omega it is the value block and the label block's two step arrows.

Accept opens rather than the circle, because the circle is Back on every title and pressing but
Pulse's PS2 and Pure's PSP ([back-and-help.md](back-and-help.md), "The resolver").

## The order

The mods that are on are listed first, each group in the order the search found them (the
built-in first). **The order is decided when the list is entered** from outside it: a toggle,
from the list or from the detail page, moves nothing until the list is left and entered again.
BACK from a detail page returns to the same row of the same list.

A switch rebuilds the menus (see [mods](../tools/mods.md#the-mods-page)). The rebuild carries
over the list's order, the cursor's mod, the scroll window and whether a pointer put the cursor
there, so nothing moves under the pointer or the cursor.

## The detail page

The page ADR-0078 gave every mod: a MOD row that steps through the mods, ENABLED, SETTINGS
(only while the mod is on and its `menu.toml` names a settings page) and the detail lines (id,
version, authors, licence, what it adds, channels and problems). Stepping the MOD row moves the
list's cursor with it, so BACK lands on the mod the page showed.

## Headless

`--menu-page mods` draws the list as it is entered, `--menu-page mods:<mod id>` with the cursor on
that mod, and `--menu-page mod_detail:<mod id>` that mod's detail page. In a `--drive` script
the list's rows are selected by mod id (`select oag.hillside; press square`), and the detail
page's switch by `set mods.enabled=off`.

## How it is built

- `assets/ui/menu.toml`'s `mods` page has `item_page = "mod_detail"` and one template row,
  `values_from = "mod_list"`; `oag_game::mods::expand` makes one row per mod
  (`Definition::expand_rows`, as the MUSIC page does).
- `oag_ui::menu` (`menu/items.rs`) gives a page with `item_page` its behaviour: accept reports
  `MenuEvent::OpenItem`, Square steps, entry sorts. A mod's own page cannot be one.
- The session opens the detail page (`session::mods_page`, `open_mod_item`) and applies a row's
  switch through the same `switch_mod` the detail page's ENABLED uses.

## Tests

- `oag_ui::menu::tests::items`: the order on entry and re-entry, accept, Square, the footer
  words, the click targets on a text-row and a HD skin, and what a rebuild keeps.
- `crates/game/tests/mods_list.rs`: the rows built from a mod listing, which mod Square names,
  open and back.
- `drive_ground_truth::the_mods_page_turns_a_mod_off_and_on_and_what_it_adds_goes_and_comes_back_live`
  (disc-backed): Square on the list, the order, the cursor through the detail page and the saved
  `[mods] disabled`.
