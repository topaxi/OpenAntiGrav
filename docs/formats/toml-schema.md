# TOML Schema: how our TOML files declare themselves

**Status: built (2026-10-12).** Decision and rationale:
[ADR-0064](../architecture/adr/0064-project-toml-files-declare-a-toml-schema.md).
Code: [`oag-schema`](../../crates/schema). The schemas: [`schemas/v1/`](../../schemas/v1).
**Chosen, not measured**: this is this project's own convention, not an original's format.

## The header

Every TOML file this project owns carries

```toml
[toml-schema]
location = "https://oag.topaxi.com/mods/schemas/v1/pilot.tosd"
version = "1.0.0"
```

and no other versioning key. `version` is the TOML Schema *language* version. The kind
(`pilot`) and the major (`1`) are the last two path segments of `location`. A file made of
plain top-level keys writes it as one line, which parses to the same table and does not
swallow the keys after it:

```toml
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/pilot.tosd", version = "1.0.0" }
```

## What a loader does

`oag_schema::check(header, kind, major)` is the one reader. It refuses, naming what it found:

| Found | Message |
| --- | --- |
| no table | `no [toml-schema] table; add toml-schema = { ... }` (the line is spelled out) |
| `location` not ending in `v<digits>/<kind>.tosd` | `location "..." does not end in v<major>/<kind>.tosd` |
| another kind | `names a "menu" file, but this is read as "pilot"` |
| another major | `pilot schema v2 is not supported; this build reads v1` |
| language version other than `1.0.0` | `version "1.0", but the schemas are TOML Schema 1.0.0` |

A pilot without the header still loads, with that first message as a warning; the other
kinds refuse. OPTIONS > AI PILOTS writes the header on save. There is no minor version: a
compatible addition goes into the same `v1` schema, a breaking change gets `v2/`.

## The kinds

| Kind | File | Loader | Schema |
| --- | --- | --- | --- |
| `menu` | `assets/ui/menu.toml` (and `--menu <path>`) | `oag_ui::menu::Definition::parse` | [`menu.tosd`](../../schemas/v1/menu.tosd) |
| `pilot` | `$XDG_CONFIG_HOME/oag/pilots/*.toml`, `assets/ai/example-pilot.toml` | `oag_raceplay::pilots::parse` | [`pilot.tosd`](../../schemas/v1/pilot.tosd) |
| `music-state` | `$XDG_CONFIG_HOME/oag/music.toml` ([race music resume](../architecture/race-music-resume.md)) | `oag_game::music_state` | [`music-state.tosd`](../../schemas/v1/music-state.tosd) |
| `track` | `<name>.track.toml` ([track file](track-file.md)) | `oag_trackfile::read` | [`track.tosd`](../../schemas/v1/track.tosd) |

| `mod` | `oag-mod.toml` at the root of a mod ([mods](../tools/mods.md)) | `oag_mod` manifest reader | [`mod.tosd`](../../schemas/v1/mod.tosd) |
| `mod-menu` | a mod's `menu.toml` ([mods](../tools/mods.md)) | `oag_ui::menu` mod rows | [`mod-menu.tosd`](../../schemas/v1/mod-menu.tosd) |

Craft files get theirs when that format lands.

## Checking a file by hand

```sh
tosd validate schemas/v1/pilot.tosd ~/.config/oag/pilots/winston.toml
```

`tosd` (1.0.0-rc.2, [releases](https://github.com/brunoborges/toml-schema/releases),
checksummed archives) does not fetch the `https` `location`; pass the schema file. Taplo and
Tombi do not read `.tosd` yet, so an editor gets no completion from the header today.

## What the schemas do not say

The schema says shape; the loader says meaning. Left to the loader: a pilot axis's numeric
limit (`pilots::limit`), a menu's dangling `target` or unknown `action`, a track's index
ranges and glb counts. The track reader also tolerates a key it does not know and names it
as unread, where the closed schema refuses it.

## The gate

`just check-schemas` (`scripts/check-schemas.sh`): every shipped file validates, each kind's
accepted and rejected examples get the same verdict from the schema and from the loader
(`pilot_schema`, `menu_schema`, `schema` test binaries), and `scripts/stage-schemas.sh`
puts the schemas at `dist/mods/schemas/v1/` where `scripts/build-web.sh` deploys them to
Cloudflare Pages, and `scripts/gen-schema-index.py` (run by it) writes the index page and
`index.json` at `https://oag.topaxi.com/mods/schemas/` from `schemas/` alone: kind and version
from the path, the description from the schema's own leading comment. The gate fails if a
`schemas/v*/*.tosd` is missing from the generated index or the index names one that does not
exist. The player-facing guide to these files is [modding](../tools/modding.md).
