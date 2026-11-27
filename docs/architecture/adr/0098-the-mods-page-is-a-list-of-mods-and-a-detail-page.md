# ADR-0098: The MODS page is a list of mods, and each mod has a detail page

## Status

Accepted (2026-11-27). Supersedes [ADR-0078](0078-a-mod-is-turned-off-by-id-live-and-the-built-in-never.md)
decision 6, the page's row shape. ADR-0078's other decisions stand.

## Context

ADR-0078 shaped MODS as one MOD stepper, one ENABLED row and the detail lines of the
mod the stepper was on. Seeing which mods are installed and on meant stepping through
them one at a time. The maintainer asked for a list of mods by name with whether each is
enabled on the right, a button that toggles it in the list, and a button that opens
today's page for it, with the enabled mods sorted above the disabled ones if possible.

## Decision

1. **MODS opens a list, one row per mod**: the name, `ON` or `OFF` on the right. Accept
   opens the mod; Square toggles it where it stands. A pointer's click on the ON/OFF
   column toggles and on the name opens.
2. **The old page is the mod's detail page** (`mod_detail`), with its MOD stepper kept:
   BACK returns to the list on the mod the page shows.
3. **Mods that are on are listed first, decided when the list is entered.** A toggle
   never moves a row; the order, cursor and scroll window survive the menu rebuild a
   toggle causes.
4. **A list page is a property of the menu tree**, `item_page` on a page of
   `assets/ui/menu.toml`, not a MODS special case in the menus. The rows are one
   `values_from = "mod_list"` template expanded per mod, as the MUSIC page's are. A
   mod's own pages cannot carry `item_page`.
5. **Accept opens, not the circle**: the circle is Back on every title and pressing but
   two (`docs/ui/back-and-help.md`).

## Alternatives considered

- **Keep the stepper and add a list beside it.** Two ways to choose a mod on one page,
  and no room for either on Pulse.
- **Sort at once on a toggle.** The row under the cursor would jump.
- **A detail page without the MOD stepper.** The page would need the selected mod held
  somewhere other than a row, and its title would have to name it; the stepper already
  does both.

## Consequences

- Reaching a mod's SETTINGS takes one press more than before.
- A title whose menus draw no footer legend (2048) shows no words for Square.
- The detail lines are no longer visible while browsing: the list shows name and switch
  only.
- The list's rows carry no `string_id`: their labels are the mods' own names.
