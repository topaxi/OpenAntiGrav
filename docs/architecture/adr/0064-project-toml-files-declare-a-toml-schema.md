# ADR-0064: Every TOML file of this project declares a TOML Schema

## Status

Accepted (maintainer, 2026-10-12). Supersedes the `track.format` and
`track.version` clause of [ADR-0063](0063-the-own-track-format-is-glb-plus-a-toml-manifest.md)
(its glb-plus-manifest decision stands). Convention and how to use it:
[`toml-schema.md`](../../formats/toml-schema.md).

## Context

The TOML files this project owns (the menu tree, a player's pilots, a track
manifest, and soon mod and craft files) each grew their own way to say what they
are and which revision: `version = 1` in the menu, `format = "oag-track"` plus
`version = "1.1"` in the track manifest, nothing in a pilot. A player editing a
file by hand got no completion and no check before launching, and a mod author
had no document to write against.

## Decision

1. **The schema language is [TOML Schema](https://toml-schema.org/)**, language
   version `1.0.0`, schema files with the `.tosd` extension. Chosen over JSON
   Schema (which Taplo and Tombi read, see below) because it is TOML itself: a
   schema is a file a modder can read without learning a second syntax, and a
   closed table rejects an unknown key, which is the rule our loaders already
   enforce for a misspelled pilot axis.
2. **A document carries a `[toml-schema]` table and nothing else for versioning**:
   no own `version` key, no `format` key.

   ```toml
   [toml-schema]
   location = "https://oag.topaxi.com/mods/schemas/v1/pilot.tosd"
   version = "1.0.0"
   ```

   `version` is the TOML Schema *language* version (the specification defines it
   so), not ours. A file whose top level is plain keys (menu, pilot) writes the
   same table as one line, `toml-schema = { location = "...", version = "1.0.0" }`,
   because a `[toml-schema]` header would swallow every key after it. Parsed, the
   two are the same table.
3. **Kind and major come from `location`**: its last two path segments are
   `v<major>/<kind>.tosd` (`oag_schema::parse_location`). There is no minor: a
   compatible addition goes into the same `v1` schema, a breaking change gets
   `v2/` beside it. The host is not read, so a copy served elsewhere or a
   `file:///` path works.
4. **A loader refuses by name** (`oag_schema::check`, the one reader): a missing
   table, a `location` that does not end in `v<digits>/<kind>.tosd`, another
   kind ("names a "menu" file, but this is read as "pilot""), another major
   ("pilot schema v2 is not supported; this build reads v1") and a language
   version other than `1.0.0`. The menu and track loaders refuse a missing
   header. The **pilot** loader loads a header-less file with a warning that
   names the line to add, because those files exist in players' config
   directories; saving from OPTIONS > AI PILOTS writes the header.
5. **The schemas live in `schemas/v1/` and deploy with the Pages build** to
   `https://oag.topaxi.com/mods/schemas/v1/<kind>.tosd` (`scripts/stage-schemas.sh`,
   called by `scripts/build-web.sh`; `web/_headers` gives them
   `application/toml`, CORS and a one-hour cache).
6. **Validation is the reference Rust library, as a pinned git dev-dependency.**
   `tosd` 1.0.0-rc.2 is not on crates.io (`publish = false` upstream), so
   `oag-schema` has an optional `validate` feature that depends on
   `github.com/brunoborges/toml-schema` at the commit behind tag `rust-v1.0.0-rc.2`.
   Only dev-dependencies turn the feature on, so no runtime build links it. This
   keeps the schema-versus-loader tests hermetic: `cargo nextest run` and CI need
   no binary and no download. The `tosd` CLI release (checksummed archives) is
   the same code; use it for `tosd validate schemas/v1/pilot.tosd my-pilot.toml`.
7. **Schema and loader cannot drift unseen.** For each kind a Rust test runs a
   list of accepted and rejected examples through both and asserts the same
   verdict (`crates/raceplay/tests/pilot_schema.rs`, `crates/ui/tests/menu_schema.rs`,
   `crates/trackfile/tests/schema.rs`); `just check-schemas` validates every
   shipped file and checks the staging. Building the lists found two real loader
   defects (a pilot axis or junction pair of three numbers was read as its first
   two and the rest dropped); both loaders now refuse them.

## Consequences

- **The specification is a release candidate** (language 1.0.0, CLI 1.0.0-rc.2).
  A later rc or the final release may change a keyword. The pin means nothing
  moves under us; moving it is a deliberate change that reruns the drift tests,
  and a schema change that must break readers goes to `v2/`.
- **Editors do not read it yet.** Taplo and Tombi take JSON Schema (and their own
  `#:schema` directives), not `.tosd`, so a hand editor gets no completion from
  the header today. The header still documents the file and `tosd validate`
  checks it. A JSON Schema export for editor use is open.
- **The schemas say shape, not meaning.** A pilot's per-axis numeric limits, a
  menu's dangling `target`, a track's index ranges and glb counts stay the
  loader's; the examples in the drift tests stay inside what both can say.
- **One deliberate difference**: the track reader tolerates (and reports as
  unread) a key it does not know, so an older build can read a file written
  against a later revision of the same `v1`; the schema is closed and refuses it.
- The git dependency needs network on a first build and adds a crate
  (`toml-schema`, `url`, `regex`) to dev builds. The default build is unchanged.
- Old track manifests (`format`/`version` keys, no header) are refused with the
  line to add; the committed fixture was regenerated, and the 1.0/1.1 minor
  distinction is gone (no file with `_OAG_AI_BYTES` absent was ever shipped).
