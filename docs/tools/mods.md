# Mods

A mod adds content to the game without touching the program. The menu is the first
thing a mod can reach: a mod attaches rows to a title's main menu. Tracks, craft and
pilots have folders reserved (`tracks/`, `craft/`, `pilots/`) and nothing reads them
yet. The decision is [ADR-0066](../architecture/adr/0066-the-menu-is-the-disc-plus-mods.md);
the menu side is [menus](../architecture/menus.md#the-main-menu-is-the-discs-plus-mods).

## A mod is a folder, a zip or a `.oagmod`

| Form | What it is |
| --- | --- |
| a folder | a directory with `oag-mod.toml` at its root |
| `.zip` | the same files in a zip, made by any zip tool |
| `.oagmod` | a zip under this project's extension; the same reader, no difference |

It is **read in place**: a folder file by file, a zip entry by entry, nothing is
unpacked to disk. A zip of the folder itself (`my-mod/oag-mod.toml`, rather than its
contents) is read with the wrapping directory stripped. A path that leaves the mod
(`..`, an absolute path) is refused.

Searched, in this order: `$OAG_MODS` alone when set (a path list), else `data/mods/` (a
checkout), `mods/` beside the program (or the AppImage), and `<data dir>/oag/mods/`
(`~/.local/share/oag/mods/` on Linux). Each directory is searched one level deep: a
folder holding `oag-mod.toml`, or a `*.zip`, or a `*.oagmod`. `just mods-list` shows
what the search finds and what it left out.

## The manifest

`oag-mod.toml`, checked against [`schemas/v1/mod.tosd`](../../schemas/v1/mod.tosd):

```toml
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/mod.tosd", version = "1.0.0" }

[mod]
id = "topaxi.example"        # unique, stable: lowercase letters, digits, '.', '-', '_'
name = "Example"
version = "1.0.0"            # the mod's own semver release
authors = ["A. Modder"]
license = "CC-BY-4.0"        # an SPDX expression ("MIT OR Apache-2.0"), or:
# license-file = "LICENSE"   # a file in the mod (not both)
description = "What it adds."
repository = "https://example.org/example"   # optional
# homepage, keywords = [...], titles = ["wipeout-pulse"], oag = ">=0.1"
```

A mod **with no licence loads**, and the mods list and the load report print `no license
stated`. `oag` is a semver requirement against this build; a mod asking for another
version is left out and named. `titles` is information for now. `just mods-check <path>`
opens a mod and checks its manifest.

## Menu rows: `menu.toml`

Checked against [`schemas/v1/mod-menu.tosd`](../../schemas/v1/mod-menu.tosd):

```toml
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/mod-menu.tosd", version = "1.0.0" }

[[entry]]
id = "garage"                 # unique across the whole menu
parent = "options"            # a top-level row's id, or "top" for the top level
after = "records"             # optional: a sibling under the same parent
kind = "submenu"              # "submenu" (needs target) or "action" (needs action)
label = "GARAGE"
string_id = "TEST_GARAGE"     # optional, resolved through the strings below
target = "records"            # an existing page of the menu tree
subtitle = "..."              # optional, with subtitle_string_id
```

Rules:

- **`parent` is a row id; there is no `children` key.** Ids of the rows a title's own
  menu can have: `campaign`, `racebox`, `options`, `records`; and the extras': `remix`,
  `quit`. A row attaches to a **top-level** row only. A parent the tree knows but this
  title's menu lacks places the row at the top level (the load report says so); an id
  nobody knows is an error.
- **Only existing things.** A row is a `submenu` to a page of `assets/ui/menu.toml` or an
  `action` of the closed set (`launch_race`, `launch_remix`, `quit`, `save_pilot`,
  `new_pilot`, `rename_pilot`, `delete_pilot`, `open_campaign`). An unknown action, page,
  parent, `after` or string id is an error naming the mod and the key. Such a mod is left
  out of the build and named in the load report; the game starts without it.
- **Strings.** `string_id` is looked up in the project's strings and the mod's own
  `strings/<language>.toml` (`[strings]` with `ID = { text = "...", human = false }`,
  `human` meaning a person wrote or approved the text; the file for the player's language,
  falling back to the literal `label`). The id must exist in English somewhere: ship
  `strings/english.toml`.
- **How it draws.** On Pulse, Pure and 2048 an attached row is drawn inline after its
  parent. On HD and Omega it is revealed below the parent tab while the parent or one of
  its attached rows has focus, with the pad (left and right step the top level, up and down
  step inside the family) and the pointer.
- **No behaviour of its own.** A mod cannot add a page, a setting, a mode or an action.

`assets/mods/oag-extras/` is the worked example: this project's own REMIX, RECORDS and
QUIT, built into the program, in the same format, carrying the project's own licence.

## Packaging

Entries may be **stored, deflate or zstd** (zip method 93); the reader takes all three.

| Method | Use it for | Why |
| --- | --- | --- |
| zstd | text (TOML, XML), `.glb`, raw data | smallest, and fast to read |
| stored | PNG, OGG and anything already compressed | compressing it again gains nothing |
| deflate | when only stock zip tools are to hand | every tool reads it |

Writing:

- **`just pack-mod <folder>`** (or `oag-mod pack <folder> [-o file]`) writes a `.oagmod` with
  zstd entries at level 19 and stores `.png`, `.ogg`, `.jpg`, `.webp`, `.mp3`, `.opus`, `.zip`
  and `.zst`. The manifest is the first entry. It checks the manifest first.
- Python 3.14 `zipfile` with `ZIP_ZSTANDARD` writes zstd entries this reader opens.
- Info-ZIP `zip -r`, `7z a -tzip`, `jar` and `bsdtar --format zip` write deflate or stored
  entries, all of which were opened by this reader (2026-10-12). 7-Zip 26.04 cannot write
  zstd into a zip (`E_NOTIMPL`).

Reading what `pack-mod` wrote, for a check: 7-Zip 26.04 (`7z t`), `bsdtar -tf` (libarchive
3.8) and Python 3.14's `zipfile` read it; **Info-ZIP `unzip` 6.0 does not** (it skips the
entries as an unsupported method). Pack with deflate when a player must unpack with `unzip`.
