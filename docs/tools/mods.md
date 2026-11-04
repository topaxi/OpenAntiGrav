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
checkout), `assets/mods/official/` (the official mods, the [`official-mods`](https://github.com/OpenAntiGrav/official-mods)
submodule, `oag_mod::OFFICIAL`), `assets/mods/examples/` (the example mods, the [`mod-examples`](https://github.com/OpenAntiGrav/mod-examples) submodule;
both found when the game runs from a checkout), `mods/` beside the program (or the AppImage), and `<data dir>/oag/mods/`
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
# homepage, keywords = [...], titles = ["wipeout-pulse"], oag = ">=0.1", default-enabled = false (optional)
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
  menu can have: `campaign`, `racebox`, `options`, `records`; and the extras':
  `multiplayer`, `mods`, `quit`. A row attaches to a **top-level** row only. A parent the tree knows but this
  title's menu lacks places the row at the top level (the load report says so); an id
  nobody knows is an error.
- **Only existing things.** A row is a `submenu` to a page of `assets/ui/menu.toml` or an
  `action` of the closed set (`oag_ui::menu::Action::all()` in `crates/ui/src/menu/vocabulary.rs` is the source of truth; today `launch_race`, `quit`, `save_pilot`,
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
- **A page of its own.** A `[[page]]` (`id`, `title`/`title_string_id`, `[[page.entry]]` rows in the grammar of `assets/ui/menu.toml`) is added to the tree as `<mod id>/<id>`. An `entry` or a page row of the same mod names it by the short `id` (`target = "picker"`); a mod cannot name another mod's page, and a short id wins over a title page of the same name only for its own mod's rows. A page id is non-empty with no `/`. A page no row of the mod reaches, a `target`, `action` or string id that does not resolve, or a colliding page id is a load error naming the mod and the key (`page "picker" entry 0 target`). A template `choice` row (`values_from`, no values) on a mod page expands like any other (`a_template_choice_row_on_a_mod_page_expands_like_one_on_an_engine_page`). A `choice` row's list may be a closed query instead: `values_from = { query = "tracks_of", of = "<setting key>" }`. The queries are `titles_available`, `tracks_of`, `teams_of`, `variants_of`, `speed_classes_union`, `ai_difficulties` and `custom_tracks`; `of` (only the `*_of` queries take one) names the setting whose value is the argument, empty meaning the booted title, and it must be the `setting` of a choice row on the mod's own pages or the mod is refused naming the row. `tracks_of`, `teams_of` and `variants_of` with an `of` list the title that setting's value names (a title from `titles_available` or `craft_titles`) and are re-listed when the setting changes; with the title not openable here a row keeps its list. Each may name one more setting, its **companion**, which must also be a choice row's setting on the mod's pages and re-lists the row when it changes: `tracks_of` takes `craft_title` (under the CUSTOM title it lists the custom tracks, each greyed with the reason when its host disc is missing or that craft title cannot race it), `teams_of` takes `class` (the roster narrowed to the teams that author it; the title resolves as a craft title, so "Wipeout HD" with only 2048 here lists 2048's reshipped HD teams, ADR-0035) and `variants_of` takes `team` (that team's variants; without it, the title's first team's). A companion on another query, or without an `of`, is a load error. The fixed name `craft_titles` (every title plus a craft title a guest roster stands for) and `race_modes` are also open to a mod row. A choice row may carry `default = "<value>"`: the value a `mod.<id>.<key>` row takes while nothing is saved for it, when its list offers it (with literal `values` it must be one of them). A mod cannot define a query. A mod with a bad page is left out alone, listed with its error, and the game keeps running (`crates/game/tests/mods_pages.rs`). `just check-strings` holds a repository mod (`assets/mods/examples/*`, `assets/mods/official/*`) to its strings: every page title, row label and subtitle names a `string_id` whose English text is in the mod's `strings/english.toml` or, for a word an engine page shares (`OAG_RACE_MODE`, `OAG_MENU_BACK`), in the project's `english.toml` (`scripts/check-strings.py --mod DIR` checks one more folder). [ADR-0082](../architecture/adr/0082-a-mod-owns-pages-settings-queries-and-one-launch-action.md) supersedes ADR-0066 decision 4 for this.
- **Launching a race.** An `action = "launch_race"` row with a `launch = { ... }` table names where each input of the race is read from, by setting key: `track_title`, `track`, `craft_title`, `team`, `variant`, `mode`, `class`, `difficulty`. A key is an engine setting (`race.mode`, `race.class`, `ai.difficulty`) or a `mod.<id>.<key>` of the mod's own settings; a key nothing stores aborts the launch with its name in the log. An input left out is the booted title's own (no `track_title` races the booted disc's circuit). The engine keeps the rules for combining them: a CUSTOM track's host disc, the craft falling back to the track's title, a class no team authors, a class no controller drives. A `launch_race` row without `launch` is the ordinary RACE page's own launch, and only a `launch_race` row may carry `launch`. RACE REMIX's START row (the official mod `oag.remix`) is this action with its `mod.oag.remix.*` keys. A mod's attached `[[entry]]` rows cannot carry `launch` yet, only `[[page.entry]]` rows can. `crates/game/src/main/session/launch_keys/tests.rs` reads those keys off the mod's page and builds a Pulse craft on a Pure track and an HD craft on a Pulse track from them.
- **Not yet.** A mod cannot add a race mode.

```toml
[[page]]
id = "picker"
title = "PICKER"

[[page.entry]]
kind = "toggle"
label = "BLOOM"
setting = "mod.topaxi.example.bloom"

[[entry]]
id = "picker_row"
parent = "racebox"
kind = "submenu"
label = "PICKER"
target = "picker"
```

`assets/mods/oag-extras/` is the built-in example: this project's own RECORDS, MODS and
QUIT (and RACEBOX, OPTIONS and MULTIPLAYER where a title's menu has none), built into the
program, in the same format, carrying the project's own licence. A page with title-scoped
pickers and a launch is the official mod `oag.remix` below.

## Official mods: `assets/mods/official`

The project's own features that the game runs without
([ADR-0083](../architecture/adr/0083-official-mods-are-a-public-folder-and-the-fingerprint-counts-race-changers.md))
live in the public [`OpenAntiGrav/official-mods`](https://github.com/OpenAntiGrav/official-mods)
repository, one folder per mod, mounted as the submodule `assets/mods/official`
(`git submodule update --init assets/mods/official`) and on the default search path. An
official mod is a mod: on unless `[mods] disabled` names it, switched on the MODS page, and
the engine names none of them.

- **`remix/` (`oag.remix`): RACE REMIX.** A REMIX row under RACEBOX (and so a tile on 2048's
  grid, `oag_game::boot::campaign2048::tiles_of`) opening the page `oag.remix/remix`: MODE,
  SPEED CLASS (`speed_classes_union`, default VENOM), AI DIFFICULTY (`ai_difficulties`,
  default ELITE, pinned to "no opponents" for a class no driver covers), TRACK TITLE
  (`titles_available`, with CUSTOM when the custom-track scan found a file), TRACK
  (`tracks_of`), CRAFT TITLE (`craft_titles`), TEAM (`teams_of` with the class), VARIANT
  (`variants_of` with the team) and START (`launch_race`). Every pick is saved under
  `[mod_settings."oag.remix"]`, its own and not the RACE page's. Turned off or absent: no
  REMIX row, no page, no 2048 tile and no CUSTOM title; campaign events and
  `--ballisticng-track` still race custom tracks. It changes no race, so the LAN fingerprint
  leaves it out. `crates/game/tests/official_remix.rs` builds every title's tree with it on
  and off.
- **Delivery** (ADR-0083 decision 5): a checkout finds the submodule; packaged builds
  (AppImage, AUR, Windows, macOS) copy the folder beside the executable, which the release
  recipes do not do yet. Web and phone builds have no official mods.

## Saved picks: `mod_settings`

A mod page row names `setting = "mod.<id>.<key>"`.
The pick is saved to `settings.toml` as a string:

```toml
[mod_settings."oag.example"]
track = "Vostok"
```

- **Own id only.** A row of mod `oag.example` may name `mod.oag.example.<key>` and nothing
  else (another mod's key, an engine setting such as `race.track`): the load refuses it,
  naming the mod and the key (`Definition::check_mod_setting`, called for every `[[page.entry]]` row as the page is added). The mod is left out alone and the game keeps running (`crates/game/tests/mods_pages.rs`). A mod id carries dots and a
  key never does, so the key is what follows the last dot.
- **Kept while off.** Turning a mod off on the MODS page deletes nothing; turning it on
  restores its picks. There is no cleanup and no size cap.
- **Restart.** The table seeds the menu's rows when the menu opens, so a pick survives a restart.
- **Settled.** When the menu opens and when a pick changes, a row's saved value that its list
  no longer offers becomes the value the row shows (its `default` while nothing is saved), and
  a changed pick re-lists the queries that read it, down the chain (`oag_game::remix::settle_queries`,
  `crates/game/tests/query_resolve_ground_truth.rs`). A row with an empty list keeps its saved
  value. So what a launch reads is what the page shows.

## The MODS page

Every title's main menu has a **MODS** row under OPTIONS (added by `oag-extras` like any mod row,
after RECORDS and before QUIT). It opens a page with a **MOD** row that steps through the list,
built-in first, then each mod in search order, then what was left out; an **ENABLED** row that
turns the mod MOD is on on or off; and under them the lines for that mod: id, version (marked BUILT
IN for `oag-extras`), authors, licence (**NO LICENSE STATED** when the manifest has neither
`license` nor `license-file`; `see <file>` for a `license-file`) and what it added. Values over 22
characters are cut with `...`. Where the page has no room for every line (Pulse: four under the
three rows), AUTHORS goes first, then VERSION, then LICENSE (`Listing::table_within`).

**The LAN content fingerprint** ([ADR-0083](../architecture/adr/0083-official-mods-are-a-public-folder-and-the-fingerprint-counts-race-changers.md))
counts only the mods that are on and change a race: a simulation module (`[extension] simulation = true`),
a `tracks/*.track.toml` or a `campaigns/*.toml`. A mod of only pages, rows, strings or pilots is left
out, so two machines that differ in one still match and can race. The filter is
`oag_game::mods::Mods::content_fingerprint`; `oag_net::discovery::content_fingerprint` takes strings and knows no mods.

**Turning a mod off** ([ADR-0078](../architecture/adr/0078-a-mod-is-turned-off-by-id-live-and-the-built-in-never.md)):

- It is stored by id as `[mods] disabled = ["oag.hillside"]` in the settings file. A mod
  installed later follows its manifest's `default-enabled` (default `true`; `false` ships it
  off); an id no installed mod has is kept. A mod that ships off and is turned on is stored
  as `[mods] enabled = [...]`. `oag_game::settings::Mods::is_enabled(id, manifest_default)`
  is the one resolver, and a switch that equals the default is not stored.
- An off mod adds nothing: no menu rows, no strings, no campaigns, no pilots, no tracks on a
  CUSTOM title, and it is left out of the LAN content fingerprint (as is any mod that changes no race, below). It stays on the MODS
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

## Event channels: `[channels]`

A mod's modules and a track's triggers send each other events over named channels
([hooks.md](../architecture/wasm-extensions/hooks.md#events-between-mods-emit-and-mod)):

```toml
[channels]
publishes = ["oag.checkpoint-circuit/checkpoint"]   # what its modules may `emit` on
listens = ["oag.other/open"]                        # what it hears (for the MODS page)
```

Names are `<mod id>/<name>` (lowercase letters, digits, `-`, `_`; the id may also carry `.`; at
most 64 bytes). A published name starts with this mod's own id; a malformed, repeated or foreign
one, or more than 32 of a list, refuses the mod at load naming the key
(`channels.publishes`, `channels.listens`). A mod whose id hashes to the track's sender key (0) is
refused too. `schemas/v1/mod.tosd` has the table. A module's `emit` on a channel its mod does not
publish, or more than 4 `emit`s a tick, faults it for the race, naming the rule.

The MODS page shows a **SENDS** line (the mod's published channels and those of its track files'
`[channels] publishes`) and a **HEARS** line, and a **NO ONE SENDS** line naming each channel the mod
listens to that no mod that is on (or track file of one) sends. A warning, never a refusal: the
sender may simply be turned off. Turned-off mods show neither line.

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
module = "ext/wall_penalty.wasm"
```

A mod may instead (or also) bind a module to one of its own track files:

```toml
[extension.track]
module = "ext/hillside_rain.wasm"
track = "tracks/hillside.track.toml"   # a .track.toml of this mod; the module runs only there
```

It needs `simulation = true` too, loads only in a race of that file (never a disc circuit)
and may send the same commands as a rules module, including the race-wide `set-weather`,
`set-sun` and `grade-sky` outputs. Hillside's rain is one.

A mod may ship text and images for its module's HUD: its own `strings/<language>.toml`
(the same files its menu rows use, English the fallback) and `[icons]` (`name = "path/in/the/mod.png"` or `.svg`). A module's
`hud_string` and `hud_mod_icon` name them; an icon that is missing or unreadable draws
nothing and the load report names the file and the error. An SVG is rasterised at load, on the
load thread, and again on a worker when the window size, HUD scale or Safe Area changes, at the
pixel size the pickup slot is drawn at (`oag_hud::svg`); nothing is drawn as a vector per
frame. `<text>` draws nothing (the text stack is not built in; convert it to paths) and an
`<image>` is never read. Pixel-art icons stay PNGs: a PNG is drawn as authored.

A simulation module can also change the title's HUD through outputs that are never
hashed: `hud-tint`, `hud-shake` and `hud-bar-extra` on a HUD element (`Speed`, `Thrust`,
`Energy`, `Pickup`, `Position`, `Lap`, `Time`; the pickup is shaken but coloured by `pickup-look`), and `hud-text` and `hud-icon` at one of
nine anchors of the HUD grid, up to 32 per slot per tick. Re-send each every tick it
should hold; a title with no widget for an element draws nothing. Energy Turbo's pickup look
is its manifest's `[pickup]`, shows its added thrust as a red share of the speed bar (`Speed`; Pulse has no
thrust bar, and `Thrust` draws nothing there) and shakes the energy bar while it drains. Guests format times with
`oag_ext::Line`. The commands are in
[docs/architecture/wasm-extensions/](../architecture/wasm-extensions/README.md).

A mod that hands out a pickup of its own (`give-pickup`, `pickup-roll`) declares its look in
`oag-mod.toml`, and is faulted for the race at the first such command without it. Each mod
with a `[pickup]` has its own pickup kind (at most 32 a race), so two such mods run side by
side:

```toml
[pickup]
icon = { title = "turbo" }       # the title's own icon for a weapon, or { mod = "<key of [icons]>" }
colour = { rgba = "E0202080" }   # RRGGBBAA, or { title = "shield" }: the title's own backdrop row
```

`title` is a weapon (`rocket`, `missile`, `quake`, `cannon`, `turbo`, `shield`, `autopilot`,
`plasma`, `bomb`, `mine`, `leachbeam`, `repulser`, `shuriken`, `disruptor`); a bad weapon, a
`mod` that is no key of `[icons]` or an `rgba` that is not eight hex digits refuses the mod by name.
The title's fade-in, overshoot, size pulse and fade-out play on it (Pulse, HD, Omega). Pure has no
backdrop widget, so a colour draws nothing there; 2048 keeps its backdrop as authored. A module can
override the look per tick with `pickup-look`.

A rules module also sees every weapon hit on a craft (`hit{slot, amount, weapon}`, an
absorbed one included, with the craft's `shielded` flag) and can start, hold and end the
title's Shield (`give-shield`, `hold-shield`, `clear-shield`): enough to write a breakable
armour. See [wasm-extensions](../architecture/wasm-extensions/README.md#breakable-armour-from-the-shield-2026-10-30).

**The example of triggers, track properties, HUD text and the AI hook is
`checkpoint-circuit/`** in the mod examples (`oag.checkpoint-circuit`): a track of its own
whose pit lane is a fork in the spline, built by `checkpoint_circuit.py` in Blender, and a
track module that reads `mod_checkpoint` off its checkpoint triggers and `mod_pit` off its pit
trigger and pit-lane path, drains a craft's energy past its checkpoint clock, recharges it in
the pit lane, draws the clock and `RECHARGING` from its own `strings/english.toml`, and sends
an AI craft low on energy down the pit lane with `prefer-route{slot, route}`: the host still
throws the fork's coin and the AI drives the route with the player's physics; the module only
picks the way. See
[wasm-extensions](../architecture/wasm-extensions/README.md#and-checkpoint-circuit-2026-10-30).

The game runs modules through its `extensions` feature, on by default on Linux,
Windows, macOS and Android (`cargo run --release -p oag-game`, `just play`, `just apk`).
`OAG_NO_EXTENSIONS=1 just play` or `--no-default-features` builds without it; the web
build and iOS always do (the browser's own engine and wasmtime's Pulley interpreter
are not built yet). A build without it runs no module, and its boot log says so for
each mod that ships one.
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
`assets/mods/examples/wall-penalty/` (one event, one command) and
`assets/mods/examples/energy-turbo/` (a pickup of its own) are the worked examples.
Copy either one's `guest/` to start inside a `mod-examples` checkout; elsewhere, point
its `oag-ext` dependency at your engine checkout's `crates/ext` (the example's is a
relative `path` into the engine the submodule sits in).

**A track's own properties.** A track file can carry numbers for a mod on its speed
pads, weapon pads, triggers and itself (a `mod_<name>` custom property in Blender,
[track file](../formats/track-file.md#properties-and-triggers-for-simulation-mods)); the
engine gives them no meaning. Read one with
`obs.track().speed_pad(pad).and_then(|p| p.prop(oag_ext::prop!("boost")))`, and hear
entries as `Event::SpeedPad`, `WeaponPad`, `TriggerEnter` and `TriggerExit`. The SDK's
track buffer is 32 KiB of zeroed memory: with a 16 KiB stack, `memory_pages = 2` holds
it. `Reply::scale_pad(pad, factor)` (race-wide, held) and `scale_slot_pads(&craft,
factor)` (this tick) scale a pad's push, both in `[0, 2]`
([what they scale](../architecture/wasm-extensions/README.md#and-track-properties-2026-10-30)).

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
