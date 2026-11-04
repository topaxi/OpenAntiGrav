---
status: ready-to-wire
titles: [all]
updated: 2026-11-04
summary: "RACE REMIX is already a mod; the engine still names Remix in modules, a title field, a source type and comments. Rename, nothing left to move."
size: S (30)
difficulty: easy (25)
kind: debt
---

# Remix vocabulary out of the engine

Scout census, 2026-11-04, `main` at f9bb72ea2, branch `remix-scout`. Confidences
in brackets are the scout's own estimates of size and difficulty, not measurements.
Line numbers are current on `main`.

## Decided, not reopened

- **ADR-0082** (accepted 2026-11-02): a mod owns pages, settings, queries and one
  launch action. Its "Alternatives considered" states the goal: "the game runs without
  Remix and the engine carries no Remix names". Renames are inside that goal.
- **ADR-0082 decision 5**: one `launch_race` action, two sources when track and craft
  titles differ. The picker and launch rules stay in the engine (ADR-0082
  "Consequences"; ADR-0083 decision 3 keeps the two-source plumbing).
- **ADR-0083 decision 4**: the fingerprint counts only race-changing mods.

## Already landed on main (the old thread's Open items 1-5 are done)

The earlier thread `handover/tooling/race-remix-as-an-official-mod.md` still lists
these as open. They are not:

| Item | Evidence on main |
| --- | --- |
| Mod menu files take `[[page]]` | `assets/mods/official/remix/menu.toml` has `[[page]]` with `id = "remix"`; `ModFile` grammar in `crates/ui/src/menu/definition/build.rs` |
| The REMIX page left the engine | `rg -i remix assets/ui` returns nothing; `assets/ui/menu.toml` has no remix row |
| Remix strings left the engine | no `OAG_REMIX`/`OAG_MENU_REMIX` in `assets/ui/strings/`; the mod ships `remix/strings/{english,french,german,portuguesebr}.toml` |
| `Action::LaunchRemix`, `ValueSource::Remix*` gone | `crates/ui/src/menu/vocabulary.rs`: `Action::LaunchRace` at :15, `Query` enum at :435 |
| `[remix]` settings section gone | `rg -i remix crates/game/src/settings.rs crates/game/src/main/session/apply.rs` returns nothing (commit 6053df85b) |
| LAN fingerprint counts race-changers | `crates/game/src/mods.rs:515-523` filters with `changes_a_race` (`:433-439`) |
| 2048 tile list is mod-driven | `crates/game/src/boot/campaign2048.rs:172` is a comment only; no hard-coded `["racebox","remix","multiplayer"]` left |
| Page-id hooks | `crates/game/src/main/session/custom_tracks.rs` and `capture/menu_page` no longer name `"remix"` in code |

Commits: 87093bf3c (the mod lands), 6053df85b (engine names removed), c783e02b2 (mod README bump).

## What still names Remix, by cluster

Counts, measured with `rg -in remix crates` split by path: 52 non-test files under
`crates/` mention remix, on 157 lines (about 110 are comments, about 48 are code).
25 test files mention it. Nothing in `assets/ui/` is left, and `schemas/` has one
comment (`schemas/v1/track.tosd:38`). The mod itself is not an engine cluster.

### C1. Session module named `remix_menu` holds the generic query answers (rename)

- `crates/game/src/main/session/remix.rs` (344 lines, header line 1: "Race Remix's own
  CRAFT TITLE/TRACK TITLE/TEAM rows, and its launch" - the launch moved to
  `launch_keys.rs`, so the header is stale).
- Its functions are generic query answers, not Remix policy: `settle` (:26) is called by
  the **ordinary RACE page** at `crates/game/src/main/session/menus.rs:466` and `:970`;
  `speed_classes_union` (:251), `ai_difficulties` (:271), `craft_title_choices` (:44),
  `query_lists` (:114), `custom_track_choices` (:162), `resupply_queries` (:191),
  `resolve_menu_queries` (:203), `craft_catalogue` (:292), `is_guest_team` (:338).
- The module name exists only because of a collision: `crates/game/src/main/session.rs:117-121`
  says "Named `remix_menu`, not `remix` - `oag_game::remix` is already imported unaliased".
  Fixing C2 removes the collision, so C1 must follow C2.
- **Verdict: rename** to `session/queries.rs` (`mod queries`); move `settle` next to the
  generic supply code (`menus.rs`) since the RACE page uses it. Call sites: `session.rs`,
  `menus.rs` (2), `launch_keys.rs` (2 imports, 2 uses), plus the module's own
  `remix/tests.rs` (`session/remix/tests.rs`, 6 mentions).
- Move into the mod: **no**. ADR-0082 "Consequences": a declarative mod cannot define a
  query; moving this needs ADR-0080's wasm screen, which is unbuilt.

### C2. `oag_game::remix` is the title catalogue and query answering (rename)

- `crates/game/src/remix.rs` (290 lines, doc line 1: "What the RACE REMIX page offers").
  Exports `Catalogue` (:21), `catalogue` (:68), `QueryArgs` (:148), `query_choices` (:161),
  `resolve_queries` (:194), `settle_queries` (:242). The body is generic: circuits and
  roster of any title, answered per query.
- Re-exported through `crates/game/src/lib.rs:82` (`pub mod remix;`).
- Call sites, non-test: `session/remix.rs`, `session/launch_keys.rs` (:189, :225),
  `session/campaign.rs` (:370, :716, :781), `capture/menu_page/query_sources.rs:46`, and
  comment-only mentions in `session.rs` and `boot/roster.rs`. Test callers:
  `tests/query_resolve_ground_truth.rs`, `tests/race_remix_ground_truth.rs`,
  `tests/custom_campaign_ground_truth.rs:37`, `tests/ai_speed_plan_sweep.rs:107`,
  `tests/omega_campaign_launch_ground_truth.rs` (comment), `session/launch_keys/tests.rs`,
  `session/remix/tests.rs:500`. Six non-test files plus seven test files.
- **Verdict: rename** to `oag_game::title_lists` (not `catalogue`: `oag_raceplay::catalogue`
  is already a module path in the same files, and a second `catalogue` would shadow or
  clash). Keep the code.

### C3. `oag_source::remix` and the `Remix` enum (rename)

- `crates/source/src/remix.rs` (107 lines): `pub enum Remix { Single(Opened), Split { track, craft } }`
  (:32), `open` (:46), `open_server` (:68), `into_parts` (:83), `craft_of` (:102).
  Module declared at `crates/source/src/lib.rs:25`.
- Callers, non-test: `crates/raceplay/src/load.rs:61, 75-76, 114-124` (local variable also
  named `remix`), `crates/game/src/main/cli.rs:460` (doc). Tests:
  `crates/source/tests/remix_2048_sources_ground_truth.rs` (+ `tests/it.rs:14`),
  `crates/game/tests/race_remix_ground_truth.rs`.
- The enum is plumbing (ADR-0034, ADR-0035 and ADR-0082 decision 5 all name
  `oag_source::remix::Remix::Split`). A rename keeps the logic and changes four ADR
  citations' spelling. ADRs are immutable, so those names go stale; state that in the
  thread's Next Steps rather than edit them.
- **Verdict: rename** to `oag_source::sources` with type `SourcePair` (variants `Single`
  and `Split` unchanged). **Slipstream overlap:** `crates/raceplay/src/load.rs` is edited
  on `slipstream` since merge-base 10d3c2ca (see below). Land C3 after slipstream or
  rebase it there.

### C4. Title speed-class field and method named for Remix (rename)

- `crates/title/src/speed.rs:48` `pub remix_only: &'static [&'static str]`, doc
  "The names ... only RACE REMIX offers" (:40-47). :177 `pub fn is_offered_outside_remix`,
  :180 uses `remix_only`, :188 and :193 call it.
- Authored data, one line each: `crates/pulse/src/race.rs:93` (`&[]`),
  `crates/hd/src/race.rs:115` (`&[]`), `crates/pure/src/race.rs:144`
  (`&[oag_title::SpeedClasses::VECTOR]`), `crates/2048/src/race.rs:127`
  (`&[... SUPER_PHANTOM]`).
- Read: `crates/net/src/extract.rs:127` filters the lobby's class list with
  `classes.remix_only`.
- Callers: `crates/game/src/main/session/menus.rs:31, 53, 806` (comments and a test
  message at :817 "VECTOR is a RACE REMIX offering"), `crates/game/src/scoreboard.rs:188-190`
  (comment), `crates/game/src/main/session/remix.rs:245` (doc naming the function).
- Tests: `crates/title/src/speed/tests.rs` (about 14 mentions, `remix_only:` fixtures at
  :10, :16, :22; test name at :132).
- The concept is title data (ADR-0058): rungs the ordinary RACE page hides. Its doc
  labels the values **chosen, not measured**; keep that label.
- **Verdict: rename**: `remix_only` to `race_page_hidden` (or `union_only`; the scout
  prefers `union_only` because it says what the union adds), `is_offered_outside_remix`
  to `is_offered_on_race_page`. Nine non-test files and one test file, plus the doc page
  `docs/architecture/menus.md` (section "SPEED CLASS: `VECTOR` is confined to RACE REMIX",
  :183 and :200-203, which cites the names). The docs are a link-checked section heading,
  so rename both together.

### C5. Doc comments and stale claims that name Remix (rename or delete; no code)

- About 110 comment lines in non-test engine code name Remix. Mechanical rewrite to "a
  two-title race", "the mod", or the new names.
- Stale, and wrong on main today (fix first, cheapest):
  - `crates/game/src/settings/race.rs:1` "The `[race]` and `[remix]` sections" (no
    `[remix]` section exists).
  - `crates/game/src/main/session/remix.rs:1` "and its launch" (launch is `launch_keys.rs`).
  - `crates/ui/src/menu/definition/build.rs:9` "The project's own rows (REMIX, RECORDS,
    QUIT ...)" (REMIX is the mod's row).
  - `crates/ui/src/menu/definition.rs:834-835` "RACE REMIX and RECORDS keep their own
    `race.track`/`remix.*` rows" (the keys are `mod.oag.remix.*`).
  - `docs/tools/modding.md:125`: "a mod cannot add a page, a setting or a ..." (false since
    ADR-0082; the official mod has `[[page]]` and settings).
  - `docs/architecture/menus.md` has 23 mentions; most describe the present correctly
    (the `mod.oag.remix.track` path at :330), but :200-203 and :287 name the old
    `is_offered_outside_remix` and the `remix` page.
- **Verdict: fix now.** The four stale comments and the modding doc are a few lines.
  The rest of the comment lines change with the renames in C1-C4. No logic.

### C6. User-visible text in Rust that names "Race Remix" (rename text)

- `crates/raceplay/src/foreign.rs:136`: `anyhow::bail!` text "on {}'s craft that is a Race
  Remix (the {} disc as the source with --craft-source <{} disc>)". Printed to a player or CLI
  user, even with the mod off.
- `crates/game/src/custom_tracks.rs:375`: log line "racing it as a Race Remix, track from ...".
- `crates/raceplay/tests/trackfile_ground_truth.rs:662` asserts the refusal contains the
  literal "Race Remix" (`for needle in [...]`); change the test with the text.
- **Verdict: rename text**, "a two-title race". XS. `foreign.rs` is not edited on
  slipstream.

### C7. The CUSTOM track title (keep; rename docs only)

- `crates/game/src/custom_tracks.rs:1, 13` ("RACE REMIX's CUSTOM track title"),
  `pub const CUSTOM` at :39, the entry label fallback `unwrap_or(CUSTOM)` at :186 (doc
  "What TRACK TITLE calls the CUSTOM entry", :181).
- The scan (`Scan::begin`), the custom track files and the `--ballisticng-track` path are
  engine plumbing. `docs/architecture/menus.md:287` says that with the mod off there is no
  CUSTOM title, while campaign events and `--ballisticng-track` still race custom tracks.
- **Verdict: keep the code; fix the doc comments.** Unclassified: `CUSTOM` is an English
  literal used as a fallback label. Whether it reaches a screen untranslated when the
  mod's `strings/` lack the entry is not read. Check with `--menu-page` before naming it.

### C8. Test files and fixtures named for Remix (rename, low value)

- Files: `crates/game/tests/race_remix_ground_truth.rs` (23 mentions, in `it_p_r.rs:38`),
  `crates/source/tests/remix_2048_sources_ground_truth.rs`, `crates/game/src/custom_tracks/tests.rs:454`
  (test name "...becomes_a_remix_on_its_hosts_disc").
- Keep, they test the official mod: `crates/game/tests/official_remix.rs` (`it_m_o.rs:38`),
  `crates/game/tests/drive_ground_truth.rs` (scripts `select launch_remix`,
  `mod.oag.remix.*`), `crates/game/src/boot/tests.rs:220-226` (the mod's folder),
  `crates/ui/src/frontend/tests/wipeout2048.rs:621-653` (an `oag.remix/remix` fixture).
- Fixtures in `crates/ui/src/menu/tests.rs:92-103` and `tests/reveal.rs`, `tests/strip.rs`
  use the labels "REMIX" and `parent = "remix"` as stand-ins for the mod's row. Keep or
  rename to `test.mod`; the scout's preference is keep (they mirror a real mod).
- **Verdict: rename two test files and one test name** when C2 and C3 land. XS.

## Ranked by value for effort

| Rank | Cluster | Verdict | Files (non-test + test) | Size | Difficulty | Slipstream overlap |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | C5 stale comments and docs | fix | 4 stale-comment files + 2 docs; the other comment lines ride with the renames | S (25) | easy (20) | none |
| 2 | C1 `remix_menu` module and `settle` | rename | 4 non-test + 1 test | S (30) | easy (25) | none (`session.rs`, `menus.rs`, `launch_keys.rs` not edited on slipstream) |
| 3 | C4 `remix_only`, `is_offered_outside_remix` | rename | 9 non-test + 1 test, plus `menus.rs` doc | S (30) | easy (25) | none |
| 4 | C2 `oag_game::remix` to `title_lists` | rename | 6 non-test + 7 test | S (35) | easy (25) | none (`launch_keys.rs`, `campaign.rs` not edited on slipstream) |
| 5 | C6 user-visible "Race Remix" text | rename text | 1 non-test + 1 test | XS (10) | easy (15) | none |
| 6 | C3 `oag_source::remix::Remix` to `SourcePair` | rename | 4 non-test + 3 test | S (30) | easy (25) | `raceplay/src/load.rs` edited on slipstream (not on its remix lines) |
| 7 | C8 test files and names | rename | 2 test files | XS (15) | easy (20) | `official_remix.rs` edited on slipstream (not on its remix lines) |
| - | C7 CUSTOM title | keep, docs | 1 non-test (+ menus.rs doc) | XS (10) | medium (40) | none |

Scout's confidence on the sizes is 20-40; no quoted estimate exists for any of these.
A rename pass of C1 through C6 is one lane of about half a day for one member, with the
gate. Each rename is a `sed`-level change plus call-site fixes; no behaviour changes, so
the simulation determinism rules are not in play (none of these crates' arithmetic moves).
Note that `oag-game`, `oag-source`, `oag-title` are not simulation crates for C5 but
`oag-title` is consumed by `oag-pulse`, `oag-hd` and `oag-2048` (C4 touches four title crates).

## Move into the mod: nothing left

Every remaining engine name is plumbing, a title's data, or a comment. A declarative mod
cannot take the picker logic (ADR-0082 "Consequences"; a mod cannot define a query).
Moving it needs ADR-0080's wasm screen, which is proposed, not built. The maintainer
decision that would reopen this is unchanged: "remove Remix's logic" means its names,
page, settings and policy rows, not the two-source rules (the old thread's Open item 6).

## Slipstream

`slipstream` is 68 commits ahead of and 10 behind `main` (merge-base 10d3c2ca, 2026-11-03).

- **No cluster exists only on slipstream.** `git grep -i remix slipstream -- crates`
  returns the same names as main, file for file.
- **No remix-named line changed on slipstream since the merge-base.** `git diff
  10d3c2ca slipstream` has no `+`/`-` line containing "remix".
- **Eight remix-named files are edited on slipstream since the merge-base** (so a rename
  there may need a rebase): `crates/game/Cargo.toml`, `crates/game/src/boot/tests.rs`,
  `crates/game/src/main/prepare.rs`, `crates/game/src/main/session/load.rs`,
  `crates/game/src/main/session/mods_page.rs`, `crates/game/tests/it_m_o.rs`,
  `crates/game/tests/official_remix.rs`, `crates/raceplay/src/load.rs`. Their edits are not
  on remix lines.
- Main has 10 commits slipstream lacks. Two of them edit remix-named files:
  `crates/raceplay/src/load.rs` (edited on both branches since the merge-base, so C3 there
  is a three-way merge) and `crates/game/src/launcher.rs` (commit 4ea92384d, launcher build
  date). Not read in detail: whether either edit touches a remix line.

## Open

1. The old thread `handover/tooling/race-remix-as-an-official-mod.md` lists landed work as
   open (its Open items 1-5, and the "Remix specifics" table). Strike them in that
   thread's next edit, or delete the thread once C1-C8 land.
2. Whether the engine's `CUSTOM` fallback label reaches a screen untranslated (C7).
3. The ADR citations of `oag_source::remix::Remix::Split` and `is_offered_outside_remix`
   (ADR-0034, ADR-0035, ADR-0082) go stale with C3 and C4. ADRs are immutable: note the
   new names in `docs/architecture/menus.md`, do not edit the ADRs.
4. `docs/tools/modding.md:125` is false today (C5). Fix with the first lane.

## Next Steps

1. Fix C5's four stale comments and `docs/tools/modding.md:125`. One commit, `just check-docs`.
2. Land C1 and C2 together: rename `oag_game::remix` to `title_lists`, then `remix_menu` to
   `queries`, moving `settle` into `menus.rs`. Gate with `just gate-affected`.
3. Land C4: `remix_only` to `union_only`, `is_offered_outside_remix` to
   `is_offered_on_race_page`; update `docs/architecture/menus.md` headings and links.
4. After slipstream merges (or rebased there): C3 to `oag_source::sources::SourcePair`, then C6 and C8.
5. Then strike the old thread's open items and delete it when nothing is left.
