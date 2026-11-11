---
status: ready-to-wire
titles: [all]
updated: 2026-11-02
summary: "ADR-0082/0083 accepted; lanes 1-5, 6a, 9 (desktop packaging) done; open: 6b resolve of, launch_race, the move, AUR/mobile/web delivery"
size: XL (40)
difficulty: hard (45)
---

# RACE REMIX as an official mod

2026-11-02, the maintainer's goal: move RACE REMIX out of the engine into a
standalone mod in `OpenAntiGrav/official-mods` (to be created), then remove
Remix's page and logic from the engine. The engine keeps the plumbing: a race
whose track and craft come from two titles.

`re-first` here means a decision first (two ADRs), not reverse-engineering.
Scout reading, checked against `main` at ef51bd6be. Confidences in brackets are
the scout's own estimates, not measurements.

## Decided (maintainer, 2026-11-02)

Recorded as ADR-0082 (the mod menu vocabulary) and ADR-0083 (official mods are a public folder,
the fingerprint rule, delivery per target). Lane 1 is done. ADR-0083 leaves web,
Android and iOS delivery unread; settle those before lane 9 ships to them.

- **Launch: option 2.** One `launch_race` action with setting keys (track title, track, craft
  title, team, variant, mode, class, difficulty), used by the ordinary RACE page, Remix and any
  mod page; differing track and craft titles open two sources. No `launch_two_source`.
- **Delivery: folder on disk**, found by the mod search path and copied beside the executable
  in packaged builds; the game runs without it.
- **LAN fingerprint counts only race-changing mods** (simulation modules, tracks, campaigns);
  page-only and string-only mods are left out. ADR-B amends ADR-0078's consequence.
- **`OpenAntiGrav/official-mods` is public** (no token, CI may check the submodule out).

## Remix specifics (not in the ADRs)

Mod id `oag.remix`, folder `remix/` in `official-mods`. It carries the REMIX row, the `remix`
page, the `OAG_REMIX_*` and `OAG_MENU_REMIX*` strings in every language file, its picks under
`[mod_settings."oag.remix"]` and its entry in the 2048 tile list. The built-in `oag-extras`
loses its REMIX row (a row in both must not show twice). The saved `[remix]` picks are lost on
the move (pre-release, no migration). With the mod off there is no REMIX row, no 2048 tile and
no CUSTOM title; campaign events and `--ballisticng-track` still race custom tracks.

## Open

**The blockers are the mod vocabulary and one LAN rule, not the Remix code.**
Today a mod cannot own a page, a setting, a launch or a parameterised choice row,
and the LAN fingerprint counts every mod that is on.

1. **A mod menu file takes rows only.** `crates/ui/src/menu/definition/build.rs:55-79`:
   `ModFile` has `toml-schema` and `entry` only, both `deny_unknown_fields`. A
   `[[page]]` in a mod fails the parse. `docs/tools/mods.md:124` says "A mod cannot
   add a page, a setting, a mode or an action", and ADR-0066 decision 4 and its
   consequences say the same.
2. **The page is engine-owned.** `assets/ui/menu.toml:387-480` (`id = "remix"`,
   `title_string_id = "OAG_REMIX_PAGE_TITLE"`). Only the row is in the mod:
   `assets/mods/oag-extras/menu.toml:43-48` (`parent = "racebox"`, `target = "remix"`).
   `Definition::parse` makes an unreachable page an error, so the shipped file
   alone is only whole after the extras merge (`definition.rs:258-272`).
3. **Targets and actions are closed.** `crates/ui/src/menu/vocabulary.rs:13-21`
   (`Action::LaunchRemix`), and `ValueSource` (`vocabulary.rs:173`, unit variants
   `RemixTracks`, `RemixTeams`, `RemixSpeedClasses`, `RemixAiDifficulty`,
   `RemixVariant`, `Titles`, `CraftTitles`, `Teams`, `RaceVariant`, `SpeedClasses`).
   `values_from` is a bare string name with no argument.
4. **Settings are a fixed struct.** `crates/game/src/settings/race.rs:199`
   (`pub struct Remix`: `track_title`, `track`, `craft_title`, `team`, `variant`).
   `crates/game/src/settings.rs:128` (`pub remix: Remix`) and the flat keys at
   `settings.rs:1087-1091`. Apply is a match on the string key:
   `crates/game/src/main/session/apply.rs:771-787`. No generic `mod.<id>.*` table
   exists. The only mod-keyed table is `[mods] disabled` (ADR-0078).
5. **LAN fingerprint counts every mod that is on (new, blocks the replay/lobby
   verdict).** `crates/game/src/mods.rs:503-511` (`content_fingerprint`) hashes
   `id@version` of every `player()` mod that is on, skipping only mods under
   `oag_mod::EXAMPLES`. Callers: `crates/game/src/main/session/online.rs:147-149`,
   `session/lan.rs:176`, `main/cli/extra.rs:719-721`. `oag_net::discovery::content_fingerprint`
   (`crates/net/src/discovery.rs:168`) is what a LAN host compares. So a machine
   with `oag.remix` on and one with it off fingerprint differently, and a join is
   refused. **A menu-only mod would gate multiplayer.** Verified: a LAN row with
   `Compat::OtherContent` is unselectable and shows "DIFFERENT MODS OR CONTENT"
   (`crates/game/src/main/session/lan.rs:491-505`, `crates/game/src/lan_list.rs:26`),
   and the online rooms refuse it too (`crates/net/src/online/rooms.rs:294`).
   Today's examples are simulation or track mods, so this does not show yet.

6. **Plain gap, the one the maintainer must see.** A declarative mod moves the
   page and the policy. The picker and launch logic stays in the engine, as generic
   queries and actions (sections 2c and 2d). Only an ADR-0080 wasm screen would take
   that logic out of the engine, and that ADR is proposed, not built. "Remove Remix's
   logic" therefore means: remove Remix's names, page, settings and policy rows, not
   the two-source rules.

### What the engine code is, by reading

| Piece | Where | Verdict |
| --- | --- | --- |
| Session pickers and launch | `crates/game/src/main/session/remix.rs` (609 lines) | Remix policy; moves into the mod's vocabulary |
| Supply/apply call sites | `crates/game/src/main/session/menus.rs` (41 `remix` lines), `apply.rs` (17) | Remix policy |
| Settings | `crates/game/src/settings/race.rs:183-199`, `settings.rs:128,1087-1091` | Moves to a mod namespace |
| Catalogue of a title's circuits and roster | `crates/game/src/remix.rs` (143 lines) | Reads `oag_source::remix::Remix`; becomes the generic `TracksOf`/`TeamsOf` source, stays (lane 8b) |
| Source pair | `crates/source/src/remix.rs` (`Remix::{Single,Split}`, `open` :46, `open_server` :68) | **Stays** (plumbing) |
| Two-source load | `crates/raceplay/src/load.rs:61-124` (`open` :75, `open_server` :114-115) | **Stays** |
| Speed-class union | `crates/title/src/speed.rs:223-240` (`is_offered_outside_remix`, `union`) | Title data (ADR-0058); name says Remix, rename later |
| 2048 REMIX tile | `crates/ui/src/frontend/touch.rs:120-121` (`Remix` variant), `crates/game/src/main/session/launch2048.rs:44` (`push_page("remix")`), `crates/game/src/boot/campaign2048.rs:200-208` (hardcoded `["racebox","remix","multiplayer"]`) | **Engine-coded**; must become mod-driven |
| Page-id hooks | `crates/game/src/main/session/custom_tracks.rs:103` (`page.id == "remix"`), `crates/game/src/capture/menu_page.rs:188` | Engine-coded by id |
| Strings | `assets/ui/strings/english.toml:69-70,206-211` (`OAG_MENU_REMIX*`, `OAG_REMIX_*`), the same 8 ids in `german.toml`, `french.toml`, `portuguesebr.toml` | Move with the page into the mod's `strings/` |

Counting `remix` mentions is not a measure of what leaves. Most of the
`session/remix.rs` mentions are comments about Remix's rules, and most of its code
is generalised by lanes 6 and 7, not deleted. No line count is claimed for the move.

### 1. Replays, lobby, saves, records: nothing names "remix"

- **Replay header** (`crates/replay/src/header.rs:20-61`): `title`, `track`,
  `mode`, `class`, `team`, `seed`, `options` (a free map). No remix or source
  field. Filled by `crates/game/src/ghosts.rs:80-81` from a records `Key`.
  **No mod id is needed.**
- **Records and ghost key** (`crates/game/src/main/stage.rs:528-534`): `title.name`,
  the circuit, the mode and the class. Reading, not traced to the binding: the
  comment at `load.rs:112` says the title is the track's, so a remix record files
  under the track's title and the craft's title is not recorded. **Unchanged by the
  move**, and a pre-existing gap if it matters.
- **Lobby** (`crates/net/src/wire.rs:110-134`, `extract.rs:76-141`): the server opens
  one source (`Remix::open_server`, `load.rs:114-115`) and offers that title's
  tracks, teams and classes. No remix token on the wire. **Remix cannot be hosted
  online today**, so no wire field changes when it becomes a mod.
- **LAN fingerprint**: a mod-presence gate, see Open item 5. **Needs a decision in
  ADR-B**: count only mods that change a race (a simulation module, a track, a
  campaign), not page-only mods.
- **Saves**: `[remix]` in settings is the only persisted remix state. Renaming the
  key loses the player's last picks. Pre-release, formats change in place with no
  migration.

Verdict: no replay or record format needs a mod id. The fingerprint is the one
real breakage, and the page id `"remix"` in three hooks.

### 2. Mod API additions for a declarative (no wasm) Remix mod

For each: what exists, the smallest shape, the files, and whether ordinary RACE
could use it too.

**(a) Mod-owned pages.** Exists: nothing; `ModFile` takes `entry` only.
Smallest shape: a mod `menu.toml` may carry `[[page]]` with the grammar of
`assets/ui/menu.toml`. Page ids are namespaced by mod (`oag.remix/remix`), so two
mods cannot collide; an `entry` may name a page of its own mod by short id.
Entries stay from the closed vocabulary. Files: `crates/ui/src/menu/definition/build.rs`
(ModFile, merge), `definition.rs` (reachability per mod), `definition/raw.rs`,
`schemas/v1/mod-menu.tosd` (edit in place), `docs/tools/mods.md`,
`scripts/check-strings.py:239` (extend to official mods). RACE could use it, though
nothing about the ordinary page changes.

**(b) Mod settings namespace.** Exists: `[mods] disabled` only. Smallest shape: a
`BTreeMap<String, BTreeMap<String, String>>` at `[mod_settings."<id>"]`, read by
rows with `setting = "mod.<id>.<key>"`. Apply becomes one generic arm instead of
a match per key (`apply.rs:771-787`). Files: `crates/game/src/settings.rs`,
`apply.rs`. The ordinary RACE page keeps its `race.*` section; the namespace is additive.

**(c) Parameterised value sources.** Exists, as seven fixed names:
`titles`, `craft_titles`, `remix_tracks` (tracks of `remix.track_title`),
`remix_teams` (teams of `remix.craft_title`), `remix_variant` (variants of
`remix.team`), `remix_speed_classes` (union across titles), `remix_ai_difficulty`.
Smallest shape: a closed `Query` enum (`TitlesAvailable`, `TracksOf`, `TeamsOf`,
`VariantsOf`, `SpeedClassesUnion`, `AiDifficulties`, `CustomTracks`), written in
TOML as `values_from = { query = "tracks_of", of = "remix.track_title" }`. An empty
`of` means the booted title, which is how the ordinary RACE page's `tracks` and
`teams` would be expressed. Files: `vocabulary.rs` (`ValueSource` to `Query`),
`definition/build.rs` (parse the table), `menus.rs` (the supply arms at
`:1001-1002`, `:1091-1103`), `remix.rs` (the bodies at :142-320 already take a
title argument). A closed set, so ADR-0066's "a mod cannot invent behaviour" holds.

**(d) Parameterised launch.** Exists: `Action::LaunchRemix` (`vocabulary.rs:21`)
reads `remix.*` by name; `launch_remix` (`remix.rs:380-470`) builds two-source
options. Smallest shape, one of two:
- *Option 1 (scout's recommendation):* keep the two-source launch as **engine
  plumbing** under a generic action `launch_two_source`, whose page entry names its
  inputs by setting key (`track_title = "remix.track_title"`, `track`,
  `craft_title`, `team`, `variant`, plus `mode`, `class`, `difficulty`). The mod
  picks the keys; the engine owns the rules (CUSTOM host, craft fallback, the
  no-driver flag).
- *Option 2:* a mod supplies only the page and calls `launch_race` with keys; the
  two-source rule moves into the launch. More general, larger.
Ordinary RACE can use `launch_race` with the same key list (booted title for track
and craft). Files: `vocabulary.rs`, `menus.rs:513-519` (the Fired arms), `remix.rs`.

**(e) CUSTOM tracks as a query.** Exists: `oag_game::custom_tracks` scans the
`--ballisticng-track` directories (`crates/game/src/custom_tracks.rs`) plus
installed mods' track files (`set_mod_files`, :94-118). Smallest shape: the
`CustomTracks` query above; the page-id hook at `custom_tracks.rs:103` becomes
"this page carries a CustomTracks row".

**(f) Launch-side hooks by id.** `touch.rs:120-121`, `launch2048.rs:44`,
`campaign2048.rs:200-208`, `custom_tracks.rs:103`, `capture/menu_page.rs:188`.
The 2048 tile list is hardcoded; it has to come from the racebox rows the mods add,
or it breaks when the mod is off. Lane 6.

### 3. ADR conflicts and what the new ADR decides

Conflicts:
- **ADR-0066 decision 4 and its consequence** ("a mod cannot add a page, a setting,
  a race mode or a choice row"): direct conflict. ADR-0080 supersedes it for wasm
  screens only, and says so. ADRs are immutable, so the new one supersedes it for
  declarative pages too.
- **ADR-0078 decision 4** (`oag-extras` cannot be turned off): does not cover
  `oag.remix`, which is a player-style mod and can be turned off. With it off, REMIX,
  its 2048 tile and the CUSTOM title disappear; the ordinary RACE page is unchanged,
  and campaign and CLI custom tracks still race.
- **ADR-0078 consequences, "the LAN fingerprint counts only the mods that are on"**:
  this is what the page-only exclusion changes, so ADR-B must amend it.
- **ADR-0079 decision 6** ("no menu vocabulary"): concerns wasm; declarative pages
  do not touch it.
- **ADR-0080** (proposed, frontend world): not needed for a declarative Remix. Its
  screen would be a canvas of host widgets, not the closed pickers the page already
  has. Rejected for this goal, kept for editor-style mods.
- **ADR-0034** (two titles open at once): the plumbing it describes stays.

Decisions to record, as two ADRs (scout's split):

- **ADR-A, the mod menu vocabulary.** Mod-owned `[[page]]` with namespaced ids; a
  `[mod_settings."<id>"]` namespace; the closed `Query` set; `launch_two_source`
  (option 1 above, or option 2 if the maintainer picks it). Supersedes ADR-0066
  decision 4 and its consequence, for declarative content only. The ordinary RACE
  page may use each piece.
- **ADR-B, Race Remix is the official mod `oag.remix`.** Decisions:
  - id `oag.remix`; folder `remix/` in `OpenAntiGrav/official-mods` (its own
    `oag-mod.toml`); submodule at `assets/mods/official`, mirroring
    `assets/mods/examples` (already a submodule, `.gitmodules`).
  - Enabled by default: no entry in `[mods] disabled` means on (ADR-0078 decision 1).
    Can be turned off on the MODS page.
  - **LAN fingerprint: count only mods that change a race** (a simulation module,
    a track, a campaign). Page-only and string-only mods are left out of
    `content_fingerprint`. Without this, Remix on and off cannot play together.
  - What the engine keeps: the source pair, the custom-track scans, the speed-class
    union data. A missing mod removes its page, its 2048 tile and its CUSTOM row,
    and nothing else.
  - **Delivery per target, decided explicitly.** Two options. (i) Folder on disk,
    found by the search path, copied beside the executable for packaged builds.
    (ii) Embed the folder in the binary with `include_bytes!`, as `oag-extras` is
    (`crates/mod/src/extras.rs:11-21`), which makes the submodule a compile-time
    requirement and removes the on-disk search for this mod. Targets to decide for:
    Linux AppImage (`packaging/appimage/`, no `mods` reference today), AUR
    (`packaging/aur/`), Windows and macOS builds, the Steam Deck build, the web
    build (`scripts/build-web.sh`, `crates/game` `wasm32` cfgs in `gpu.rs` and
    `race_build.rs`), and Android/iOS (`crates/mobile`). Not read in this scout:
    whether the web and mobile builds have any mod search at all. If they do not,
    the Remix mod there is either embedded or absent, and that is a choice to make
    here, not a side effect.
A single ADR is possible, but B is product policy and A is a mechanism the ordinary
RACE page will reuse. Two keeps each reviewable.

### 4. Custom tracks when Remix can be turned off

Three sources of custom tracks, and only one depends on the page:
- **Scanned files** (`oag_source::tracks::all_in`, `--ballisticng-track`): engine
  plumbing; stays.
- **Mod tracks** (`assets/mods/examples/hillside`, `checkpoint-circuit`, with
  `[extension.track]`): carried by their own mod; reach CUSTOM through
  `set_mod_files` (`crates/game/src/custom_tracks.rs:94-118`) and campaign events.
  They do not need the Remix mod on. ADR-0078 decision 5 already covers a campaign
  that races a turned-off mod's track.
- **The CUSTOM title on RACE REMIX**: the page's row. It goes with the page, as a
  `CustomTracks` query.
Consequence: with Remix off, Hillside still races from campaign events and the
command line; the MODS page still lists it; only the RACE REMIX CUSTOM title is
gone.

### 5. How official-mods gets into a build and a release

Read today:
- The default search path is `data/mods`, `assets/mods/examples` (relative to the
  cwd), `mods` beside the executable (`APPIMAGE` first), and `<data dir>/oag/mods`
  (`crates/mod/src/discover.rs:14-47`). `OAG_MODS` replaces all of it.
- `assets/mods/examples` is a submodule (`.gitmodules`); CI checks it out
  (`.github/workflows/ci.yml:38-42`, `submodules: true`).
- `oag-extras` is embedded (`crates/mod/src/extras.rs`).
- `packaging/appimage/` has no `mods` reference. `just play` runs from a checkout.
  A shipped build finds the examples only if the cwd is a checkout.

**The repo is private** (`OpenAntiGrav/official-mods`, created 2026-11-02, empty,
no default branch yet; maintainer). Consequences, read from `.github/workflows/`:
- `ci.yml:38-42` is the only job that sets `submodules: true` (and `lfs: true`).
  `actions/checkout` with a submodule the default `GITHUB_TOKEN` cannot read fails the
  whole job. The same goes for forks and for any clone without access. Today the
  examples repo is public, so this passes. A private submodule would turn that job red.
- `release.yml` (checkout at :90, :146, :193, :249, :309) and `nightly.yml` do not
  set `submodules`, so they already ship without the examples. They would build
  without Remix and without a token, which is what the maintainer wants.
- So the game must build and run with the mod absent. Remix simply does not appear:
  no REMIX row, no 2048 tile, no CUSTOM title. The engine plumbing stays. Engine code
  must not reference a mod that is not there. ADR-0066's "a broken mod never stops
  the game" already covers a missing one.
- Options for the mod's source, for ADR-B to pick:
  (i) Keep the submodule but make CI fetch it with a token secret
  (`actions/checkout` `token:`), and let fork PRs skip it. A private repo then
  needs a secret in every consuming job, and forks never see the mod.
  (ii) Keep the submodule and do not set `submodules: true` on the gate job.
  The gate and tests then run with the mod absent, and a separate job with the token
  checks the mod. Tests that need Remix skip when the folder is missing, as the
  disc-backed tests do.
  (iii) Not a submodule: the packaging step downloads a pinned release asset of
  `official-mods` with a token, and the repo stays a plain repo. No `.gitmodules`
  entry, no checkout cost.
  (iv) Make `official-mods` public. The simplest, and the only one that needs no
  token. A maintainer decision.
- **Recommendation:** (ii) for the gate and tests, plus (iii) for release and
  nightly. (iv) removes the token problem entirely and is the cheapest if the content
  can be public. Either way, the lead's gate and `ci.yml`'s first job must not require
  the private repo.

Proposal, pending ADR-B's delivery choice: submodule `assets/mods/official` from
`OpenAntiGrav/official-mods`, added to the default search list next to `EXAMPLES`
(`pub const OFFICIAL` in `discover.rs`); the packaged builds copy it beside the
executable; enabled by default; id `oag.remix`. The lead's gate must have the
submodule, as it does for examples. A test that drives the menus must pass with
and without it (the 2026-10-29 incident).

### Measurement and risk

- **No sim crate is touched.** `oag-core`, `oag-physics`, `oag-gameplay`,
  `oag-ai`, `oag-race`, `oag-formats` are in no lane. A remix race still builds the
  same `race::Options`. Determinism is not at risk.
- **Risk areas**: the LAN fingerprint (silently refuses joins), the 2048 tile (boot
  path), `custom_tracks.rs:103` (a page-id hook that stops working if the id changes),
  the `[remix]` saved picks, and the tests that name `remix`:
  `crates/ui/src/menu/tests/{strip,rows,reveal}.rs`,
  `crates/ui-screens/tests/menu_remix_ai_ground_truth.rs`,
  `crates/raceplay/tests/vita_2048_original_ai_ground_truth.rs`,
  `crates/game/tests/custom_campaign_ground_truth.rs`.
- **Measure**: `just check-strings`, `just check-names`, `just check-schemas`, and
  a headless `--menu-page racebox` and `--menu-page remix` still per title before
  and after (`oag-game --menu-page`). Judge by picture, not by a green test.
- `has_player_mods` has no production caller (only `crates/game/tests/mods_switch.rs:80`),
  so no MODDED mark or replay flag reads mod presence today. Checked, not a risk.

## Lane plan

Ordered. Size and difficulty use the frontmatter vocabulary, each with the
scout's confidence (0-100; scouts stay at 20-60). Size: XS under an hour, S a few
hours, M half a day, L a day, XL more than a day (split it).

1. **ADR-0082 and ADR-0083 (done)** (docs only, maintainer decides). Claim: both are written
   and accepted, including the fingerprint rule and the delivery choice per target.
   Acceptance: `just check-docs`, ADR index updated, ADR-0066 decision 4 cross-referenced.
   Crates: none. Size: M (50). Difficulty: easy (55). **Blocks everything.** Needs
   the maintainer on the launch option (1 or 2) and the delivery per target.
2. **2a: mod `[[page]]` parse and merge (done).** Claim: a mod `menu.toml` may carry
   `[[page]]`; the shipped vanilla file still parses with `parse_vanilla`.
   Acceptance: a unit test in `crates/ui/src/menu/definition/build/tests.rs`
   with an example mod that has a page. Crates: `oag-ui`. Size: L (50). Difficulty:
   medium (45).
3. **2b: page reachability and namespacing (done: oag-ui build tests, `crates/game/tests/mods_pages.rs` load test good and bad, `check-strings` on mod pages, template row on a mod page).** Claim: a mod page is reachable from
   an `entry` of the same mod; a bad id is a load error naming the mod. Acceptance:
   the build tests plus a `mods.rs` load test. Crates: `oag-ui`, `oag-game`
   (`mods.rs`). Size: M (40). Difficulty: medium (45). Updates the mod-author skill
   (`assets/mods/examples/.claude/skills/oag-mod-author/SKILL.md`, submodule branch)
   and `docs/tools/mods.md`.
4. **LAN fingerprint: only race-changing mods (done).** Claim: `Mods::content_fingerprint`
   (`crates/game/src/mods.rs:503`) leaves out mods with no simulation module, track
   or campaign, so a page-only mod on one machine does not change the hash.
   Acceptance: tests in `crates/game/src/mods.rs` (or `crates/game/tests/`) that a
   page-only mod leaves the value at `oag_net`'s `NO_MODS`, and that a
   simulation mod still changes it. The filter lives in `oag-game` only: the net
   function takes a list of strings and knows no mods. Crates: `oag-game`. Size:
   S (60). Difficulty: easy (55). Can run as soon as ADR-B is accepted.
5. **Mod settings namespace (done; the row-side refusal `Definition::check_mod_setting` is called for every mod page row, a foreign key drops its mod alone).** Claim: `setting = "mod.<id>.<key>"` saves to
   `[mod_settings."<id>"]` and survives a restart; a turned-off mod's values are kept.
   Acceptance: a round-trip test in `crates/game/src/settings/tests.rs`. Crates:
   `oag-game`. Size: L (45). Difficulty: easy (50). Skill update as lane 3.
6. **Queries.** Split. **6a DONE** (`oag_ui::menu::Query`/`QueryRef`, the table form
   parsed and schema'd; `Entry::Choice.query` carries a non-empty `of`; the remix_*
   names stay their own supply keys, so the Remix page still gets its own lists; an
   empty `of` maps to the booted title's fixed name; `custom_tracks` has no supplier
   yet; `of` must name a choice row's setting in the same file; nothing resolves a
   `QueryRef` yet). **6b DONE**: `Menu::supply_query`/`query_refs`/`value_of` (`oag-ui`),
   `oag_game::remix::query_choices`/`resolve_queries`; the session re-lists on menu open
   (`menu_model`, after seeding) and on any setting change (`apply_setting`). The value of
   `of` is read off the row that edits it, an engine or a `mod.<id>.<key>` setting alike.
   `tracks_of`/`teams_of`/`variants_of` answer; `variants_of` takes a title (not a team)
   and lists the variants of that title's first team. Open: `speed_classes_union` and
   `titles_available` with a non-empty `of` have no argument to take. Was: the closed `Query` enum and the parser; the seven
   fixed names become aliases with empty `of`. Acceptance: all existing menu
   tests pass unchanged. Crates: `oag-ui`. Size: M (45). Difficulty: medium (40).
   **6b:** `of` takes a setting key; the ordinary RACE `tracks` is `tracks_of("")`.
   Acceptance: a test that `tracks_of("")` equals today's RACE list on Pulse and
   Pure. Crates: `oag-ui`, `oag-game`. Size: L (45). Difficulty: medium (40).
   Skill update.
7. **Launch by keys (`launch_race`, landed).** The maintainer chose option 2: one
   generic `launch_race` whose page entry names its inputs by setting key (no
   `launch_two_source`). Claim: the launch builds the same `race::Options`
   `launch_remix` builds now. Done: `session/launch_keys.rs`; open: mod `[[entry]]`
   rows cannot carry `launch`, and mod key validation waits on `mod-setting-check`. Acceptance: the existing remix launch tests pass, and
   `just gate-affected`. Crates: `oag-ui`, `oag-game`. Size: L (45). Difficulty:
   medium (35). Skill update.
8. **Move RACE REMIX to `oag.remix` (done, 8a and 8b).** `OpenAntiGrav/official-mods` holds
   `remix/` (manifest, `menu.toml`, `strings/` in four languages), mounted at
   `assets/mods/official` and on the default search path (`oag_mod::OFFICIAL`). The engine lost
   the page, the extras' REMIX row, `settings::Remix`/`[remix]`, `Action::LaunchRemix`,
   `LaunchInputs::remix`, the `remix_tracks`/`remix_teams`/`remix_variant` sources, the per-key
   resupply and the page-id hooks; `remix_speed_classes`/`remix_ai_difficulty` are
   `speed_classes_union`/`ai_difficulties`. What the page needed became generic: a query's
   companion setting (`tracks_of` `craft_title`, `teams_of` `class`, `variants_of` `team`), a
   choice row's `default`, `oag_game::remix::settle_queries` (a mod pick follows its row, a change
   re-lists down the chain), CUSTOM and `--menu-page` following `titles_available` rows, 2048's
   tiles as `Launch::Page` for RACEBOX, the rows mods attach under it, and MULTIPLAYER. The strings
   Rust reads are `OAG_RACE_NO_DRIVER`, `OAG_CUSTOM_TRACKS`, `OAG_CUSTOM_TRACKS_CHECKING`.
   Differences from before, by design: the page's MODE, SPEED CLASS and AI DIFFICULTY are its
   own (`mod.oag.remix.*`), no longer the RACE page's; a TEAM pick is kept across a CRAFT TITLE
   change when the new roster has it. Tests: `crates/game/tests/official_remix.rs`,
   `query_resolve_ground_truth.rs`, `boot::tests` (2048 tiles), `launch_keys::tests`. The
   Hillside cell of NIGHT RUN was not driven to a race with the mod off: from a fresh profile a
   `--drive` raced the first cell, but pressing cross on any other cell left the cell screen
   where it was until the wait timed out (cause not investigated);
   `custom_campaign_ground_truth` resolves Hillside from the two example mods alone, and no
   campaign launch code changed. Open: a drive test that races a mod campaign's Hillside event.
9. **Bundle and enable.** Claim: the delivery chosen in ADR-B is in place for
   each target the ADR names. Acceptance: `just mods-list` shows `oag.remix` from a
   fresh clone; the packaged build's file list is checked by reading the recipe
   (the packaged build runs on CI and is verified locally only). Crates: `oag-mod`
   (none: `OFFICIAL` landed in lane 8), `packaging/`. Size: M (45). Difficulty: medium (40).
   What it needs, read in lane 8: `ci.yml`'s `submodules: true` already checks out the public
   `assets/mods/official` (no token); `release.yml` (checkouts at about :90, :146, :193, :249,
   :309) and `nightly.yml` set no `submodules`, so a release ships without RACE REMIX until they
   check it out and copy `assets/mods/official/remix` to `mods/remix` beside the executable
   (AppImage: beside the `.AppImage` file, or inside the image with a new search entry; AUR:
   beside `/usr/bin/oag-game`; macOS: beside the bundle's executable). Web and phones: none
   (ADR-0083 decision 5). `oag-extras` no longer carries REMIX, so a release without the
   folder has no RACE REMIX at all: ADR-0083's consequence says not to ship lane 8 to players
   before the desktop recipes copy the folder.

Lane 9, desktop part, done: `scripts/stage-official-mods.sh` copies `assets/mods/official/*`
into `mods` in the AppImage (inside the image, `usr/bin/mods`, found through a second
search entry beside the program), the Linux tarball, the Windows zip and the macOS app
(`Contents/MacOS/mods`); `release.yml` checks out with `submodules: true` (nightly.yml calls
it); `docs/tools/packaging.md`, "Official mods". Verified: a container-built AppImage run
from outside the checkout, plus `act` on the Linux jobs; the macOS recipe by reading only.
Android and iOS done: iOS stages into `OpenAntiGrav.app/mods`; Android ships the mods as APK
assets with an index and unpacks them on first launch to the files directory's
`assets/mods/official` (`docs/tools/packaging.md`, "Official mods"). Built and inspected with an
APK built locally; not run on a device.
Still open: the AUR PKGBUILDs (`mods` beside `/usr/bin/oag-game`), web.

Lane 8 confirmed before it started: the repo exists (public), launch option 2, folder delivery.

## Next Steps

1. Done: ADR-0082 and ADR-0083 accepted. Open: read whether the web and mobile builds can
   search for mods at all, and pick their delivery (ADR-0083 decision 7).
2. Lane 9 left: AUR PKGBUILDs install the mods to `mods` beside `/usr/bin/oag-game`; web
   delivery (see item 9). Android and iOS are done, an Android device run is not.
