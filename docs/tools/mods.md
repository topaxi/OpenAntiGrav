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
checkout), `assets/mods/examples/` (the example mods, the [`mod-examples`](https://github.com/OpenAntiGrav/mod-examples) submodule,
found when the game runs from a checkout), `mods/` beside the program (or the AppImage), and `<data dir>/oag/mods/`
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
version is left out and named. `titles` names the titles the mod is for, by
`oag_title::Title::slug` (`wipeout-pulse`, `wipeout-hd`, `wipeout-omega-collection`);
a mod's campaigns are offered only on those (empty: any). `just mods-check <path>`
opens a mod and checks its manifest.

## Campaigns, pilots and tracks: `campaigns/`, `pilots/`, `tracks/`

A mod may carry custom campaigns, one file each under `campaigns/`; pilots for them
under `pilots/` (the pilot file's own format, `pilot.tosd`), which an event names by
file stem before the player's roster; and circuits under `tracks/` ([track
file](../formats/track-file.md): the `.track.toml` and the `.glb` it names), which an
event races with `file = "tracks/<name>.track.toml"`, or another installed mod's with
`file = "<mod id>:tracks/<name>.track.toml"`. A track file is raced from a
**folder** mod only, since its reader takes a path on disk. Every folder mod's
`tracks/*.track.toml` is also a row on RACE REMIX's CUSTOM title, on any title the mod
is for, so a mod may carry tracks and nothing else. On Pulse, HD/Fury
and Omega, RACE CAMPAIGN then opens a page listing the disc's campaign first and each
installed campaign after it. The format is [custom campaign](../formats/custom-campaign.md);
a campaign that does not compile, or names a disc this machine cannot open, is not
offered and the log says why. The MODS page's ADDS line names each campaign offered.
The worked examples live in their own repository, [`mod-examples`](https://github.com/OpenAntiGrav/mod-examples),
mounted at `assets/mods/examples` as a git submodule (`git submodule update --init`;
its tracks are in Git LFS). `example-campaign/` is NIGHT RUN: two tiers (six events and
five), the first drawn with locks and the second revealed as it opens, a mod pilot
(`pilots/rival.toml`), a track it bundles (`tracks/practice-octagon.track.toml`) and
Hillside out of `hillside/`, a track-only mod (`oag.hillside`). Both are for Pulse only:
Hillside's art borrows Pulse 03_Track's textures by name. The game run from the
checkout (`just play`) searches `assets/mods/examples` too, relative to the working
directory, so they are offered there; an installed game has no such folder, so copy
both folders into a mods directory to play them there. `OAG_MODS` replaces the whole
search path, examples included. **Track and model files never go into this
repository** (`just audit-leakage` refuses a tracked `.glb`, `.gltf` or `.blend`
outside `crates/trackfile/tests/fixtures/`): an example goes to `mod-examples`, a real
mod to a repository of its own.

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
  `multiplayer`, `quit`. A row attaches to a **top-level** row only. A parent the tree knows but this
  title's menu lacks places the row at the top level (the load report says so); an id
  nobody knows is an error.
- **Only existing things.** A row is a `submenu` to a page of `assets/ui/menu.toml` or an
  `action` of the closed set (`oag_ui::menu::Action::all()` in `crates/ui/src/menu/vocabulary.rs` is the source of truth; today `launch_race`, `launch_remix`, `quit`, `save_pilot`,
  `new_pilot`, `rename_pilot`, `delete_pilot`, `open_campaign`, `multiplayer_local`). An unknown action, page,
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

## The MODS page

Every title's main menu has a **MODS** row under OPTIONS (added by `oag-extras` like any mod row,
after RECORDS and before QUIT). It opens a page with a **MOD** row that steps through the list,
built-in first, then each mod in search order, then what was left out; an **ENABLED** row that
turns the mod MOD is on on or off; and under them the lines for that mod: id, version (marked BUILT
IN for `oag-extras`), authors, licence (**NO LICENSE STATED** when the manifest has neither
`license` nor `license-file`; `see <file>` for a `license-file`) and what it added. Values over 22
characters are cut with `...`. Where the page has no room for every line (Pulse: four under the
three rows), AUTHORS goes first, then VERSION, then LICENSE (`Listing::table_within`).

**Turning a mod off** ([ADR-0078](../architecture/adr/0078-a-mod-is-turned-off-by-id-live-and-the-built-in-never.md)):

- It is stored by id as `[mods] disabled = ["oag.hillside"]` in the settings file. A mod
  installed later is on; an id no installed mod has is kept.
- An off mod adds nothing: no menu rows, no strings, no campaigns, no pilots, no tracks on RACE
  REMIX's CUSTOM title, and it is left out of the LAN content fingerprint. It stays on the MODS
  page, its ADDS line reading **NOTHING, TURNED OFF**, so it can be turned back on. The gate is
  one place, `oag_game::mods::Mods` (`player()` and `build()`).
- **It applies at once.** The menus are rebuilt as a boot builds them and reopen on the MODS page
  with BACK leading where it led. A race parked under the menus keeps what it started with.
- **`oag-extras` cannot be turned off**: it carries OPTIONS, this page's row and QUIT. Its
  ENABLED row holds one value, **ALWAYS ON**. A candidate that never opened has one dimmed OFF:
  there is nothing to switch.
- **A campaign that races a track of a mod that is off or missing is left out whole**, the log
  names the reason (`mod "oag.hillside" is turned off on the MODS page`), and its mod's entry
  gains a line: **NEEDS HILLSIDE, OFF**, or **NEEDS &lt;id&gt;, MISSING**. Another campaign that
  does not compile gets a **LEFT OUT** line naming its file.

A candidate the search left out (a broken zip, a duplicate id, a mod for another game version) and
a mod whose rows did not fit the menu are listed too, **dimmed, with the error in the note line
under the rows** and no detail lines: the mechanism a custom track that will not load uses. It
is the same list the boot load report is written from (`oag_game::mods::Built::listing`); a mod
turned off is a line of its own there. The pointer and touch move both rows as they move any
choice row. A flat-list front end (Pulse, Pure, 2048) draws it as a page; HD and Omega reveal MODS
below OPTIONS with the pad or pointer. `--menu-page mods` draws the page headless with the
profile's switches, and `--menu-page mods:<mod id>` puts the MOD row on that mod. `just mods-list`
(`oag-mod`) does not read the game's settings and lists every mod it finds.

## Simulation modules: `[extension]`

A mod may carry one WebAssembly rules module that runs in every race it is on for
(its `titles`), except the original campaigns:

```toml
[extension]
simulation = true        # required with a rules module
api = 1                  # the ABI the module was built against
memory_pages = 2         # its maximum memory, 64 KiB pages (1 to 256; default 16)
state_bytes = 16         # its state blob, at most 1,024 bytes

[extension.rules]
module = "ext/kiss_curse.wasm"
```

The game must be built with the `extensions` feature: `just play-mods` (`just play`
with it), or `cargo run --release -p oag-game --features extensions`. Plain
`just play` runs no module; its boot log says so for each mod that ships one.
The manifest is read with the others; the `.wasm` is opened only by the race that
runs it. No module runs in an original campaign or a network race. A race running a
module is MODDED and records no replay. The ABI, the event and the command are in
[docs/architecture/wasm-extensions/](../architecture/wasm-extensions/README.md)
("What is built").

**Write a module on the guest SDK, `oag-ext` (`crates/ext`), first.** It is the
whole interface a Rust mod needs: implement `oag_ext::Extension` (a `tick` reading an
`Observation`, updating the state, sending commands through a `Reply`), declare the
state with `oag_ext::blob!`, and name the type in `oag_ext::export!`. No offsets, no
codes, no `unsafe`; a command for an empty slot does not compile, and the common
mistakes the host would fault on come back as `Err(Refused)`. The state's length is
exported as `oag_state_len` and must equal `state_bytes` above. With the SDK's
`test` feature as a dev-dependency, `oag_ext::test::Race` runs the mod's `tick`
natively under `cargo test`, with no wasm toolchain. Build with
`cargo build --release --target wasm32-unknown-unknown`, a 16 KiB stack
(`-C link-arg=-zstack-size=16384`) and `panic = "abort"`.
`assets/mods/examples/kiss-curse/` (one event, one command) and
`assets/mods/examples/energy-turbo/` (a pickup of its own) are the worked examples.
Copy either one's `guest/` to start inside a `mod-examples` checkout; elsewhere, point
its `oag-ext` dependency at your engine checkout's `crates/ext` (the example's is a
relative `path` into the engine the submodule sits in).

A module built for another layout revision of ABI 1 is refused at load, by name
(`built for ABI 1 rev N, this host is ABI 1 rev M`): rebuild it against the SDK of
the engine you run. A module in another language writes the buffer itself; the layout
is `oag-ext-abi`'s (`crates/ext-abi`), and its `oag_abi` must return
`(1 << 8) | REVISION`.

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
