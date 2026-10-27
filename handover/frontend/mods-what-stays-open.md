---
status: ready-to-wire
titles: [all]
updated: 2026-10-12
summary: "Mods: enable and disable on the MODS page; tracks, craft and pilots read from a mod"
difficulty: medium (30)
---

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
- **2026-10-12 `mods-page`: landed.** OPTIONS, then MODS: a read-only page (one MOD row over the list,
  detail lines for the mod it is on, a mod that did not load dimmed with its error). No enable or
  disable. Write-up: `docs/tools/mods.md` ("The MODS page"). The list is rebuilt only when the
  tree is, so a language switch leaves its words as booted (the thread's language item below).
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
- **Does 2048's DLC hide the HD and Fury tiles?** Not read. The patch is what grows the grid (four tiles to six);
  `g_bDlc1Mounted`/`g_bDlc2Mounted` are read at dozens of front-end sites and none was traced to
  `GameModeChoice`. A Ghidra `get_xrefs_to` on the two globals, then the tile builders among the readers.
- **HD packs and Pure's packs were not read for a menu**: no HD pack is on this machine and Pure's are
  encrypted with no key table in the checkout. The five HD archives agree, and Pulse's and 2048's packs
  carry no menu.

## Next Steps

1. Enable and disable on the MODS page (a stored per-mod switch, then the tree rebuilt without it).
2. Read `tracks/`, `craft/` and `pilots/` from a mod, one kind at a time, through the layer the disc's
   own archives use (`docs/architecture/asset-layers.md`).
3. Rebuild the tree on a live language switch.
