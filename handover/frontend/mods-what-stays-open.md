# Mods and the main menu: what stays open

## Open

- **2026-10-12 `menu-mods`: landed.** Each title's original main menu is data on `Title`
  (`Title::main_menu`, ground truth against every pressing), every row has a stable `id`, this
  project's own rows are the built-in mod `oag-extras`, and a mod is a folder, `.zip` or
  `.oagmod` (deflate, zstd or stored) with `oag-mod.toml`, `menu.toml` and
  `strings/<language>.toml`. Attached rows draw inline (Pulse, Pure, 2048) or revealed below the
  parent (HD, Omega). Permanent write-ups: `docs/architecture/adr/0066-the-menu-is-the-disc-plus-mods.md`,
  `docs/architecture/menus.md` ("The main menu is the disc's, plus mods"), `docs/tools/mods.md`.
- **Only menu rows are read from a mod.** `tracks/`, `craft/` and `pilots/` are reserved names; nothing
  reads them. A mod cannot add a page, a setting, a race mode, a choice row or an action: the closed
  vocabulary is deliberate, opened one kind of content at a time.
- **No MODS page.** The load report and `just mods-list` show each mod, its version, its licence ("no
  license stated" when it has none) and the rows it added; the game shows the player nothing. A page
  would be a new screen with its own pointer support.
- **A language switch does not re-resolve labels.** The tree's labels are resolved once when it is built,
  as every row's always were, so a live LANGUAGE change leaves a mod row in the language it booted in.
  `Session::resupply_language` clones the built definition; rebuilding it through `Mods::build` is the fix.
- **`--menu <file>` takes the file whole**: no title trimming and no mods.
- **`titles` and `oag` in the manifest**: `oag` is checked against the build, `titles` is information and
  filters nothing.
- **The capture path** (`--menu-page`) builds for the title with 2048's update treated as not mounted, since a
  still has no archives.
- **Replays do not say a mod was active.** A menu row cannot change a simulation, so nothing here needs it;
  the header gap ADR-0061 records stays for the content a mod may carry later.
- **The revealed-row keys and placement are chosen, not measured.** No disc authors a second level on the
  main menu. Hardware check on a touchscreen and a real pad: left/right and up/down were walked in
  headless tests only, the pointer through region tests, and the frames are stills.
- **HD packs and Pure's packs were not read for a menu**: no HD pack is on this machine and Pure's are
  encrypted with no key table in the checkout. The five HD archives agree, and Pulse's and 2048's packs
  carry no menu.

## Next Steps

1. A MODS page (list, licence, rows, enable and disable) with pointer support.
2. Read `tracks/`, `craft/` and `pilots/` from a mod, one kind at a time, through the layer the disc's
   own archives use (`docs/architecture/asset-layers.md`).
3. Rebuild the tree on a live language switch.
