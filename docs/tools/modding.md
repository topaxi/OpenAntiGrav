# Modding guide

For a player. What you can add to OpenAntiGrav today, where the files go, and how to do
each thing. Every step here was run against this build (2026-10-12) unless it says
otherwise; where a thing does not exist yet, the last section says so.

You need the game running on your own disc image first ([installing](../overview/installing.md)).
Mods add to it; they never contain game content, and neither should yours.

## Where files go

Three folder families. `<data dir>` and `<config dir>` are the operating system's per-user
folders, which the game finds the same way for everything it saves.

| OS | `<data dir>` | `<config dir>` |
| --- | --- | --- |
| Linux, Steam Deck (Desktop Mode) | `~/.local/share` (or `$XDG_DATA_HOME`) | `~/.config` (or `$XDG_CONFIG_HOME`) |
| Windows | `%APPDATA%` (`C:\Users\<you>\AppData\Roaming`) | `%APPDATA%` |
| macOS | `~/Library/Application Support` | `~/Library/Application Support` |
| Android | `/sdcard/Android/data/org.openantigrav.game/files/data` | `.../files/config` |

| What | Folder | Section |
| --- | --- | --- |
| Mods (folder, `.zip`, `.oagmod`) | `<data dir>/oag/mods/` | [Mod that adds a menu row](#a-mod-that-adds-a-menu-row) |
| Custom tracks (own format, BallisticNG) | `<data dir>/oag/unity/tracks/` | [Race a Workshop track](#race-a-ballisticng-workshop-track) |
| AI pilots | `<config dir>/oag/pilots/` | [Write an AI pilot](#write-an-ai-pilot) |

Create a folder that is missing (`mods/` and `unity/` are not made for you).
Two more places are searched for mods and tracks: **beside the program** (`mods/` or
`unity/` next to `oag-game`, or next to the AppImage, which is the easy choice on a Steam
Deck) and, from a checkout, `data/mods/` and `data/unity/`. Setting `OAG_MODS` or
`OAG_UNITY` (a path list, `:` on Linux and macOS, `;` on Windows) replaces the whole
list. On Android the files are reached with `adb push` or a file manager that can open
`Android/data`; iOS and a real Windows PC and Mac are described from the code and have
not been run by the maintainers ([installing](../overview/installing.md#2-make-the-images-folder-and-put-your-files-in-it)
says what was tried).

## Race a BallisticNG Workshop track

1. Subscribe to a track on the Steam Workshop for BallisticNG, and find its folder, one
   directory per item holding a `.wtrk` (Linux, Steam: `steamapps/workshop/content/473770/<item>/`).
2. Put the `.wtrk` in `<data dir>/oag/unity/tracks/`, or the whole item folder in
   `<data dir>/oag/unity/` (the game looks in `tracks/` and one and two directories
   down). To read Steam's own copy in place, set `OAG_UNITY` to
   `.../steamapps/workshop/content/473770`.
3. In the game: RACE REMIX, TRACK TITLE `CUSTOM`, pick the track, START.

Notes:

- Only tracks tagged **`Original`** on the Workshop were tested end to end (27 of the 32
  items tried). A `Port/Remake` item reads but is not what the project checked.
- The track is raced with the ship, rules and physics of the title on the CRAFT TITLE row.
  The game draws the track as flat-shaded collision and names what it did not map.
- A file that will not load is listed dimmed with the reason, and START refuses it.
- Without the menu: `oag-game --unity-track <name or path>`.

Depth: [BallisticNG tracks](../formats/ballisticng-tracks.md), [RACE REMIX: CUSTOM tracks](../architecture/menus.md#race-remix-custom-tracks).

## Build and race your own track with Blender

1. Install the add-on (Blender 4.2 or later): zip `scripts/blender/oag_track` from a
   checkout, then Edit > Preferences > Get Extensions > Install from Disk.
2. In the 3D view sidebar (N), tab **OAG**: draw a closed Curve and tag it `spline`, then
   *Build Floor and Walls*, *Add Start*, and optionally pads.
3. *Export OAG Track* writes `name.track.toml` and `name.glb`. Keep the two together.
4. Copy both into `<data dir>/oag/unity/tracks/`, then RACE REMIX, TRACK TITLE `CUSTOM`,
   track `<title> (TRACK FILE)`, START. Or `oag-game --track-file name.track.toml`.

What a track file can carry is a closed list: the racing line, collision floor and walls,
the start and speed and weapon pads. Sky, fog, lights, art and scenery are not in the
format and are not drawn. Banking is not authored yet.

Depth: [the Blender add-on](blender.md), [the track file](../formats/track-file.md).

## Write an AI pilot

A pilot is one small TOML file describing a driver's habits: where it sits on the track,
how hard it drives, how it treats other craft.

1. Copy `assets/ai/example-pilot.toml` from a checkout (or the repository) to
   `<config dir>/oag/pilots/winston.toml`. The filename is the pilot's name.
2. Edit the ranges. Every axis is `[low, high]`; each craft flying the pilot draws its own
   value inside it. An axis you leave out comes from the built-in `balanced`.
3. Start a race. Every `.toml` in the folder is read at the start of a race.

Or use OPTIONS > AI PILOTS in the game, which edits the same files one axis at a time.
A misspelled key is an error naming the key, not a silent no-op, and a number past its
limit is refused by name. A file named `aggressive.toml`, `balanced.toml`, `passive.toml` or
`shy.toml` replaces that built-in, and deleting it brings the built-in back.
Keep the first line `toml-schema = { ... }` so an editor that reads
[TOML Schema](../formats/toml-schema.md) can check the file; without it the pilot still loads, with a warning.

Depth: [AI pilots you can author](../gameplay/ai.md#pilots-you-can-author).

## A mod that adds a menu row

A mod is a folder with `oag-mod.toml` in it. Make a folder `garage/` in
`<data dir>/oag/mods/` with three files.

`oag-mod.toml`:

```toml
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/mod.tosd", version = "1.0.0" }

[mod]
id = "example.garage"
name = "Garage"
version = "1.0.0"
authors = ["A. Modder"]
license = "CC-BY-4.0"
description = "Adds a GARAGE row after OPTIONS that opens the records page."
```

`menu.toml`:

```toml
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/mod-menu.tosd", version = "1.0.0" }

[[entry]]
id = "garage"
parent = "options"
kind = "submenu"
label = "GARAGE"
string_id = "EXAMPLE_GARAGE"
target = "records"
```

`strings/english.toml`:

```toml
[strings]
EXAMPLE_GARAGE = { text = "GARAGE", human = true }
```

Check it, then start the game:

```sh
just mods-check <data dir>/oag/mods/garage    # from a checkout; prints id, version, licence, files
just mods-list                                # what the search finds, and what it left out
```

On Pulse the main menu then ends with a GARAGE row (seen in a headless screenshot,
`oag-game --menu-page main --screenshot out.png`). A row can only open a page the menu
already has, or run an action from a closed list; a mod cannot add a page, a setting or a
mode. Anything wrong (an unknown page, parent or string id) leaves the whole mod out and
the load report names it; the game still starts. `human` is `true` only when a person
wrote or approved the text.

Depth: [mods](mods.md) has every key, the parent ids, the action list and how rows draw on each title.

## Package it

A folder works as it is. To hand someone one file:

- **`just pack-mod <folder>`** (from a checkout) writes `<folder>.oagmod`: text, `.glb` and
  raw data are zstd, and PNG, OGG, JPEG, WebP, MP3, Opus, zip and zst are stored, because
  they are already compressed. A tiny mod can come out larger than its folder; that is
  the zip header, not a fault.
- **Only a stock zip tool to hand**: `zip -r garage.zip garage` (or 7-Zip, `jar`, `bsdtar --format zip`)
  writes deflate or stored entries and the game reads those too. Name it `.zip` or `.oagmod`.
  Zip the folder itself or its contents; either works.
- Put the result in `<data dir>/oag/mods/`. A player unpacking with Info-ZIP `unzip`
  cannot read a zstd one: pack with deflate for them.

Give the mod a licence. A mod with none loads, and the list prints `no license stated`.

## What does not work yet

- **Custom craft.** The `craft/` folder in a mod is reserved and nothing reads it.
  BallisticNG `.csf` ships are read by the engine layer but not raced.
- **Plain Unity tracks.** Only BallisticNG Workshop tracks and this project's own track
  format. A track from some other Unity game is not a thing the engine reads.
- **Tracks and pilots inside a mod.** `tracks/` and `pilots/` in a mod are reserved; put
  them in `unity/tracks/` and `pilots/` as above for now.
- **A MODS page in the game.** There is no list, switch or licence screen; use `just mods-list`
  and the load report. Queued.
- **Anything beyond menu rows in a mod**: no new pages, settings, modes, actions or art.

## Schemas

Every file above names a schema in its `toml-schema` header. The schemas are published, with
an index, at <https://oag.topaxi.com/mods/schemas/> (`index.json` beside it), generated
from `schemas/` at build time. Check a file by hand with
`tosd validate schemas/v1/pilot.tosd <file>` ([TOML Schema](../formats/toml-schema.md)).
