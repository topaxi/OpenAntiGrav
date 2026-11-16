# ADR-0082: A mod owns pages, settings, queries and one launch action

## Status

**Accepted** (2026-11-02, maintainer). **Supersedes
[ADR-0066](0066-the-menu-is-the-disc-plus-mods.md) decision 4 and its closing
consequence ("a mod changes only the main page. It cannot add a page of its own, a
setting, a race mode or a choice row") for declarative content only.** A mod's menu
file may now carry pages, settings, parameterised value sources and a race launch, all
from a closed vocabulary. For a mod's attached rows on the title's own pages, decision 4
stands. [ADR-0080](0080-a-wasm-mod-may-own-a-screen-and-save-a-small-blob.md) (a wasm
mod owns a screen) is not replaced: it stays the answer for editor-style mods whose
page cannot be a closed list of pickers. ADR-0066 and ADR-0080 are not edited. Builds
on [ADR-0078](0078-a-mod-is-turned-off-by-id-live-and-the-built-in-never.md) (mod ids,
the `[mods]` settings table) and [ADR-0034](0034-a-race-may-open-two-titles-at-once.md)
(a race may open two titles). The first planned user is
[ADR-0083](0083-official-mods-are-a-public-folder-and-the-fingerprint-counts-race-changers.md).

## Context

A mod menu file takes `entry` rows only (`ModFile` in
`crates/ui/src/menu/definition/build.rs`, `deny_unknown_fields`), attached to a
top-level row. RACE REMIX is a project feature that needs more: its own page, five
saved picks, value lists that depend on an earlier pick (the tracks of the chosen
title, the teams of the chosen craft title) and a launch from several settings.
All four are closed today:

- the page `remix` is engine-owned (`assets/ui/menu.toml`), only its row is in the mod;
- settings are a fixed struct (`oag_game::settings::Remix`), applied by a match on the
  key string;
- `ValueSource` is a unit enum of fixed names (`remix_tracks`, `remix_teams`,
  `remix_variant`, `remix_speed_classes`, `remix_ai_difficulty`, `titles`,
  `craft_titles`, ...) and `values_from` is a bare string with no argument;
- `Action::LaunchRemix` reads the `remix.*` settings by name.

The music-sources page already added one generic mechanism: a template `choice` row
with `values_from` and no values, expanded to one row per item the caller holds
(`Definition::expand_rows`, `crates/ui/src/menu/expand.rs`). That is a row-count
mechanism and stays; it does not take an argument and does not pick which list a
source means.

The ordinary RACE page has the same shape as Remix with one title: a track, a team, a
variant, a mode, a class. It should not need a second launch path.

## Decision

1. **A mod menu file may carry `[[page]]`.** The grammar is the shipped
   `assets/ui/menu.toml`'s. A page id is namespaced by its mod: `<mod id>/<page id>`
   (`oag.example/picker`). An `entry` of the same mod names a page of its own mod by the
   short id; a mod cannot name another mod's page by short id, and two mods cannot
   collide. Reachability is checked per mod: an unreachable page, an unknown id or a
   string id the mod does not ship is a load error naming the mod and the key (the
   rule `Definition::parse` already applies to the shipped file). A mod adds no row to
   a disc's own page and no behaviour to it, as before.
2. **A mod setting namespace.** A row may name `setting = "mod.<id>.<key>"`. Values
   are stored as strings in `[mod_settings."<id>"]` in the settings file
   (`BTreeMap<String, BTreeMap<String, String>>`, stable rewrite). Apply is one
   generic arm, not a match per key. The ordinary pages keep their own `race.*`
   sections; the namespace is additive. A turned-off mod's values are kept (ADR-0078
   decision 1), so turning a mod back on restores its picks. Pre-release, no migration:
   the fixed `[remix]` section is removed in place when the Remix mod lands.
3. **A closed `Query` set replaces the fixed value-source names.** In TOML,
   `values_from = { query = "tracks_of", of = "mod.oag.example.track_title" }`. `of`
   names a setting key whose current value is the argument; an empty `of` means the
   booted title. The set is closed and lives in `oag-ui`'s vocabulary:
   `TitlesAvailable`, `TracksOf`, `TeamsOf`, `VariantsOf`, `SpeedClassesUnion`,
   `AiDifficulties`, `CustomTracks`. The existing bare names (`titles`, `craft_titles`,
   `tracks`, `teams`, `race_variant`, `speed_classes`, the seven `remix_*`) remain as
   aliases for the queries with the matching `of`, so the shipped file and the extras
   keep parsing unchanged until the Remix lane moves them. The answer to a query is
   supplied by the composition root, as `Menu::supply` is today; a query with nothing
   to offer leaves no choice, the answer a source with nothing already gets. A mod
   cannot define a query: "a mod cannot invent behaviour" holds for lists.
4. **Template expansion is unchanged and composes with queries.** A template row
   (`values_from`, no values) still expands to one row per item the caller holds
   (`expand_rows`). A query-backed `choice` row is one row whose values come from the
   query. A page can carry both.
5. **One launch action, `launch_race`.** The page entry names its inputs by setting
   key: `track_title`, `track`, `craft_title`, `team`, `variant`, `mode`, `class`,
   `difficulty`. A missing key means the booted title's value or the title's default.
   The ordinary RACE page, a mod page and any later page launch through this one
   action. When the track title and the craft title differ, the launch opens two
   sources (ADR-0034, `oag_source::remix::Remix::Split`); when they are the same
   it opens one. There is no `launch_two_source` and no `launch_remix`. The engine
   keeps the rules of a two-source race (the CUSTOM track host, the craft fallback of
   [ADR-0035](0035-a-craft-pick-may-fall-back-to-a-title-that-reships-the-same-roster.md),
   the no-driver flag); a mod chooses the keys, never the rules.
6. **Strings.** A mod page's text is string ids in the mod's `strings/<language>.toml`,
   in `english.toml`'s `{ text, human }` convention, as for a mod row today.
   `scripts/check-strings.py` extends to every mod under the repository's official
   mods folder.
7. **Pointer and touch.** A mod page is a page of the same row types, so it is
   reached by pad, mouse and touch like any other; a mod gets this for free.

## Alternatives considered

- **Keep ADR-0066 decision 4 and leave Remix in the engine.** Safe, and the
  project's own feature keeps shaping the engine's menu tree. Rejected: the stated
  goal is that the game runs without Remix and the engine carries no Remix names.
- **An ADR-0080 wasm screen for Remix.** Takes the picker logic out of the engine,
  but needs the frontend world, which is proposed and unbuilt, and would turn five
  closed pickers into a canvas of host widgets. Kept for editor-style mods, where the
  page is not a closed list.
- **`launch_two_source` as engine plumbing named by the mod.** The scout's first
  option. Rejected by the maintainer: the ordinary RACE page would still need its own
  launch, and a Remix-only action keeps two ways to start a race.
- **Open `Query` to mods (a mod supplies a list).** That is a script, which is
  ADR-0079's job, with its fuel, memory and determinism rules. Not smuggled in
  through TOML.
- **Per-key typed mod settings.** A schema per mod would be validated at load, but
  the vocabulary would grow with every mod. Strings in a namespace, validated by the
  row that reads them (a choice only holds its own values), are enough.

## Consequences

- The menu vocabulary grows by three things (pages, a settings namespace, `Query`)
  and shrinks by one action (`LaunchRemix`). `ValueSource` becomes `Query` with aliases;
  every caller of the old names is touched when the aliases are retired.
- **The picker and launch logic stays in the engine** as generic queries and one
  action. A declarative mod moves the page and the policy, not the two-source rules.
  Only a wasm screen would take that logic out of the engine; "remove Remix's logic"
  means its names, page, settings and policy rows.
- A mod page can look like a disc page. The MODS page and the row's source mod
  must keep the origin visible.
- A turned-off mod leaves values in `[mod_settings]`. There is no cleanup and no
  size cap on the namespace; open if it grows.
- Setting keys are strings, so a typo in `of` is a load error, not a compile error.
- A mod that adds a launch can start a race the title would not offer. The launch
  validates its keys against what the queries return, so a key outside them is
  refused, not launched.
- The mod-author skill and `docs/tools/mods.md` change with each implementing lane,
  not with this ADR.

**Revisit if:** a mod needs a list the closed set cannot give (then a new query is an
engine change, or the mod moves to ADR-0080), or a page needs a control that is not a
choice, a toggle or a submenu.
