---
status: ready-to-wire
titles: [all]
updated: 2026-11-14
summary: "RACE REMIX is already a mod; the engine still names Remix in modules, a title field, a source type and comments. Rename, nothing left to move."
size: M (35)
difficulty: easy (25)
kind: debt
---

# Remix vocabulary out of the engine

**Rechecked 2026-11-14 on `main` at 98f8985c8: every cluster below is still open**
(`crates/game/src/remix.rs`, `session/remix.rs`, `crates/source/src/remix.rs` exist; `remix_only` and
`is_offered_outside_remix` have 29 hits in `crates`; "Race Remix" is still in
`crates/game/src/custom_tracks.rs:375` and `crates/raceplay/src/foreign.rs:136`; the two test files
and the `[remix]` comment at `crates/game/src/settings/race.rs:1` are unchanged; `rg -il remix crates`
lists 82 files). The only item that moved is `docs/tools/modding.md`'s false "a mod cannot add a
page" sentence, fixed 2026-11-14 (docs half of C5). The rest is a rename lane of about half a day, not
a one-hour wire.

Scout census, 2026-11-04, `main` at f9bb72ea2, branch `remix-scout`. Confidences
in brackets are the scout's own estimates of size and difficulty, not measurements.
Line numbers are current on `main`.

## Plain answer

- **No setting key, menu row id or string id named remix is left in the engine.**
  `rg -i remix assets/ui` returns nothing. `settings::Remix` and `[remix]` are gone
  (`rg -i remix crates/game/src/settings.rs crates/game/src/main/session/apply.rs` is empty).
  `Action::LaunchRemix` and the `remix_*` value sources are gone
  (`crates/ui/src/menu/vocabulary.rs`). `assets/mods/oag-extras` has no REMIX row.
- What is left is **Rust identifiers** (two modules, one type, one title field, one method),
  **user-visible text** (two strings), and **comments and docs** that describe Remix.

## Decided, not reopened

- **ADR-0082** (accepted 2026-11-02): a mod owns pages, settings, queries and one launch
  action. Its "Alternatives considered" states the goal: "the game runs without Remix and
  the engine carries no Remix names". Renames are inside that goal.
- **ADR-0082 decision 5**: one `launch_race` action, two sources when track and craft
  titles differ. The picker and launch rules stay in the engine (ADR-0082 "Consequences";
  ADR-0083 decision 3 keeps the two-source plumbing).
- **ADR-0083 decision 4**: the fingerprint counts only race-changing mods.
- **Old thread** `handover/tooling/race-remix-as-an-official-mod.md`: its lane plan marks
  lanes 1-2a, 7 and 8 done (its "Lane 8 ... (done, 8a and 8b)" at about line 392). Its
  "Open" section still lists the pre-landing state. It rules on the names in three places,
  cited per cluster below. It plans **no rename lane**: the one rename it asks for is
  "rename later" for `is_offered_outside_remix` (line 102).

## Already landed on main (the old thread's Open items 1-5 are done)

| Item | Evidence on main |
| --- | --- |
| Mod menu files take `[[page]]` | `assets/mods/official/remix/menu.toml` has `[[page]]` with `id = "remix"`; `ModFile` grammar in `crates/ui/src/menu/definition/build.rs` |
| The REMIX page left the engine | `rg -i remix assets/ui` returns nothing; `assets/ui/menu.toml` has no remix row |
| Remix strings left the engine | no `OAG_REMIX`/`OAG_MENU_REMIX` in `assets/ui/strings/`; the mod ships `remix/strings/{english,french,german,portuguesebr}.toml` |
| `Action::LaunchRemix`, `ValueSource::Remix*` gone | `crates/ui/src/menu/vocabulary.rs`: `Action::LaunchRace` at :15, `Query` enum at :435 |
| `[remix]` settings section gone | commit 6053df85b; the grep above is empty |
| LAN fingerprint counts race-changers | `crates/game/src/mods.rs:515-523` filters with `changes_a_race` (`:433-439`) |
| 2048 tile list is mod-driven | `crates/game/src/boot/campaign2048.rs:172` is a comment only; no hard-coded `["racebox","remix","multiplayer"]` left |
| Page-id hooks | no `"remix"` page-id check in engine code; `crates/game/src/main/session/custom_tracks.rs` is clean |

Commits: 87093bf3c (the mod lands), 6053df85b (engine names removed), c783e02b2 (mod README bump).

## Census

Counts, measured with `rg -in remix crates` split by path: 52 non-test files under
`crates/` mention remix, on 157 lines (about 110 are comments, about 48 are code).
25 test files mention it.

### Docs census (`docs/`, excluding the mod's own page)

| File | Mentions | Verdict |
| --- | --- | --- |
| `docs/architecture/menus.md` | 23 | **fix** with C4 and C5: headings "SPEED CLASS: `VECTOR` is confined to RACE REMIX" (:183), the mechanism paragraph (:200-203, names `is_offered_outside_remix` and `remix_only`), the CUSTOM section (:273-287) which names the `remix` page. Most other lines describe the present correctly (`mod.oag.remix.track` at :330). |
| `docs/architecture/adr/0082-...md` | 18 | **keep**: accepted ADR, immutable. Cites `Remix::Split`, `LaunchRemix` as the old state. |
| `docs/architecture/adr/0034-...md` | 12 | **keep**: immutable. Cites `oag_source::remix::Remix` as it was; the name goes stale after C3. |
| `docs/architecture/adr/0035-...md` | 11 | **keep**: immutable, same as 0034. |
| `docs/architecture/adr/0076-...md` | 4 | **keep**: decision history about track art and remix. |
| `docs/architecture/adr/0066-...md` | 4 | **keep**: immutable; superseded for mods by ADR-0082 decision 4. |
| `docs/architecture/adr/0078`, `0058`, `0077`, `0057`, `0022`, `0083`, `adr/README.md` | 2, 2, 1, 1, 1, 1, 1 | **keep**: immutable, or the index line. |
| `docs/architecture/workspace-layout.md:78` | 1 | **fix** with C3: "a race's track and craft sources (`remix::Remix`)". |
| `docs/architecture/wasm-extensions/README.md:682, 1478` | 2 | **keep**: a design note that uses "Race Remix" as an example race; not an engine name. |
| `docs/tools/modding.md:125` | 1 | fixed 2026-11-14. |

### C1. Session module named `remix_menu` holds the generic query answers (rename)

- `crates/game/src/main/session/remix.rs` (344 lines; header line 1 "Race Remix's own
  CRAFT TITLE/TRACK TITLE/TEAM rows, and its launch" is stale: the launch is in
  `launch_keys.rs`).
- Crate: `oag-game`. Non-test files: `session/remix.rs`, `session.rs` (the module
  declaration at :117-121), `session/menus.rs`, `session/launch_keys.rs`.
- Its functions are generic query answers, not Remix policy: `settle` (:26) is called by
  the **ordinary RACE page** at `crates/game/src/main/session/menus.rs:466` and `:970`;
  `speed_classes_union` (:251), `ai_difficulties` (:271), `craft_title_choices` (:44),
  `query_lists` (:114), `custom_track_choices` (:162), `resupply_queries` (:191),
  `resolve_menu_queries` (:203), `craft_catalogue` (:292), `is_guest_team` (:338).
- The module name exists only because of a collision: `crates/game/src/main/session.rs:117-121`
  says "Named `remix_menu`, not `remix` - `oag_game::remix` is already imported unaliased".
  Fixing C2 removes the collision, so C1 must follow C2.
- Collision check for `queries`: no `queries` module in `crates/game/src/main/session*`.
  The name appears in `crates/ui/src/menu/tests.rs:21` and `crates/raceplay/src/scene.rs:27`,
  different crates, so no clash.
- Old thread: line 96 says "Remix policy; moves into the mod's vocabulary". That was
  partly done (lane 8: the page and policy rows moved; the queries stayed as generic
  Query answers). Its verdict is superseded for the file's functions, not for the names.
- **Verdict: rename** to `session/queries.rs` (`mod queries`); move `settle` next to the
  generic supply code in `menus.rs`, since the RACE page uses it. Call sites: `session.rs`,
  `menus.rs` (2), `launch_keys.rs` (2 imports, 2 uses), and the module's own test file
  `session/remix/tests.rs` (6 mentions).
- Move into the mod: **no**. ADR-0082 "Consequences": a declarative mod cannot define a
  query; moving this needs ADR-0080's wasm screen, which is unbuilt.

### C2. `oag_game::remix` is the title catalogue and query answering (rename)

- `crates/game/src/remix.rs` (290 lines; doc line 1 "What the RACE REMIX page offers").
  Exports `Catalogue` (:21), `catalogue` (:68), `QueryArgs` (:148), `query_choices` (:161),
  `resolve_queries` (:194), `settle_queries` (:242). The body is generic: circuits and
  roster of any title, answered per query.
- Crate: `oag-game`. Declared at `crates/game/src/lib.rs:82` (`pub mod remix;`).
- Call sites, non-test: `session/remix.rs`, `session/launch_keys.rs` (:189, :225),
  `session/campaign.rs` (:370, :716, :781), `capture/menu_page/query_sources.rs:46`, and
  comment-only mentions in `session.rs` and `boot/roster.rs`. Six non-test files.
  Test callers: `tests/query_resolve_ground_truth.rs`, `tests/race_remix_ground_truth.rs`,
  `tests/custom_campaign_ground_truth.rs:37`, `tests/ai_speed_plan_sweep.rs:107`,
  `tests/omega_campaign_launch_ground_truth.rs` (comment), `session/launch_keys/tests.rs`,
  `session/remix/tests.rs:500`. Seven test files.
- Old thread: line 99 "Catalogue ... stays (lane 8b)". It rules that the code stays. It
  does not rule on the name.
- Collision check for `title_lists`: no hit in `crates/` (`rg '\btitle_lists\b'`).
  `catalogue` is not used: `oag_raceplay::catalogue` is already a module path in the same
  files.
- **Verdict: rename** to `oag_game::title_lists`. Keep the code.

### C3. `oag_source::remix` and the `Remix` enum (rename)

- `crates/source/src/remix.rs` (107 lines): `pub enum Remix { Single(Opened), Split { track, craft } }`
  (:32), `open` (:46), `open_server` (:68), `into_parts` (:83), `craft_of` (:102).
  Module declared at `crates/source/src/lib.rs:25`.
- Crate: `oag-source` (type and module). Callers, non-test: `crates/raceplay/src/load.rs:61,
  75-76, 114-124` (local variable also named `remix`), `crates/game/src/main/cli.rs:460`
  (doc). Tests: `crates/source/tests/remix_2048_sources_ground_truth.rs` (+ `tests/it.rs:14`),
  `crates/game/tests/race_remix_ground_truth.rs`.
- Old thread: its table at line 100 says "**Stays** (plumbing)" for the source pair, and
  line 101 "**Stays**" for the two-source load. The thread rules that the logic stays. Its
  table is silent on the names.
- The enum is plumbing (ADR-0034, ADR-0035 and ADR-0082 decision 5 all name
  `oag_source::remix::Remix::Split`). A rename keeps the logic and changes the spelling
  those ADRs use. ADRs are immutable: note the new name in `docs/architecture/workspace-layout.md`
  and leave the ADRs.
- Collision check: no `SourcePair` or `mod pair` in `oag-source`. `physics` has `mod pair`,
  a different crate.
- **Verdict: rename** to module `oag_source::pair` with type `SourcePair` (variants `Single`
  and `Split` unchanged). The module name `sources` was considered and dropped: it repeats
  the crate name's job. `pair::SourcePair` stutters slightly; accepted.
- **Git overlap:** `crates/raceplay/src/load.rs` is edited on both `main` and `slipstream`
  since the merge-base. C3 there is a three-way merge.

### C4. Title speed-class field and method named for Remix (rename)

- `crates/title/src/speed.rs:48` `pub remix_only: &'static [&'static str]`, doc
  "The names ... only RACE REMIX offers" (:40-47). :177 `pub fn is_offered_outside_remix`,
  :180 uses `remix_only`, :188 and :193 call it.
- Crate: `oag-title` (field and method). Authored data, one line each in four title crates:
  `crates/pulse/src/race.rs:93` (`&[]`), `crates/hd/src/race.rs:115` (`&[]`),
  `crates/pure/src/race.rs:144` (`&[oag_title::SpeedClasses::VECTOR]`),
  `crates/2048/src/race.rs:127` (`&[... SUPER_PHANTOM]`).
- Read: `crates/net/src/extract.rs:127` (`oag-net`) filters the lobby's class list with
  `classes.remix_only`.
- Callers: `crates/game/src/main/session/menus.rs:31, 53, 806` (comments and a test
  message at :817 "VECTOR is a RACE REMIX offering"), `crates/game/src/scoreboard.rs:188-190`
  (comment), `crates/game/src/main/session/remix.rs:245` (doc naming the function).
- Tests: `crates/title/src/speed/tests.rs` (about 14 mentions, `remix_only:` fixtures at
  :10, :16, :22; test name at :132).
- Old thread: line 102, "Title data (ADR-0058); name says Remix, rename later". **This is
  the thread's own ruling: rename later.**
- The concept is title data (ADR-0058): rungs the ordinary RACE page hides. Its doc
  labels the values **chosen, not measured**; keep that label.
- Collision check: none found for the new names (`union_only`, `is_offered_on_race_page`).
- **Verdict: rename**: `remix_only` to `union_only` (what the union adds that the RACE page
  does not show), `is_offered_outside_remix` to `is_offered_on_race_page`. Nine non-test
  files and one test file, plus the `docs/architecture/menus.md` section and its citations.
  Rename the doc heading and its links in the same commit.

### C5. Doc comments and stale claims that name Remix (fix now; no code)

- About 110 comment lines in non-test engine code name Remix. Mechanical rewrite to "a
  two-title race", "the mod", or the new names, in the same commits as C1-C4.
- Stale, and wrong on main today (fix first, cheapest):
  - `crates/game/src/settings/race.rs:1` "The `[race]` and `[remix]` sections" (no
    `[remix]` section exists).
  - `crates/game/src/main/session/remix.rs:1` "and its launch" (launch is `launch_keys.rs`).
  - `crates/ui/src/menu/definition/build.rs:9` "The project's own rows (REMIX, RECORDS,
    QUIT ...)" (REMIX is the mod's row).
  - `crates/ui/src/menu/definition.rs:834-835` "RACE REMIX and RECORDS keep their own
    `race.track`/`remix.*` rows" (the keys are `mod.oag.remix.*`).
  - `crates/game/src/custom_tracks.rs:256` "measured in `--menu-page remix`" (the page is
    `oag.remix/remix`).
  - `docs/tools/modding.md:125`: "a mod cannot add a page, a setting or a ..." (false since
    ADR-0082; the official mod has `[[page]]` and settings).
- **Verdict: fix now.** The stale comments and the modding doc are a few lines.
  The rest of the comment lines change with the renames in C1-C4.

### C6. User-visible text in Rust that names "Race Remix" (rename text)

- `crates/raceplay/src/foreign.rs:136` (`oag-raceplay`): `anyhow::bail!` text "on {}'s craft
  that is a Race Remix (the {} disc as the source with --craft-source <{} disc>)". Printed to
  a player or CLI user, even with the mod off.
- `crates/game/src/custom_tracks.rs:375` (`oag-game`): log line "racing it as a Race Remix,
  track from ...".
- `crates/raceplay/tests/trackfile_ground_truth.rs:662` asserts the refusal contains the
  literal "Race Remix" (`for needle in [...]`); change the test with the text.
- **Verdict: rename text**, "a two-title race". XS. `foreign.rs` is not edited on
  slipstream.

### C7. The CUSTOM track title (keep; rename docs only)

- `crates/game/src/custom_tracks.rs:1, 13` ("RACE REMIX's CUSTOM track title"),
  `pub const CUSTOM` at :39, the entry label fallback `unwrap_or(CUSTOM)` at :186 (doc
  "What TRACK TITLE calls the CUSTOM entry", :181). Crate: `oag-game`.
- `crates/game/src/main/session/remix.rs:122` matches `Query::TracksOf` against
  `oag_game::custom_tracks::CUSTOM`: the engine decides when CUSTOM is answered.
- The scan (`Scan::begin`), the custom track files and the `--ballisticng-track` path are
  engine plumbing. `docs/architecture/menus.md:287` says that with the mod off there is no
  CUSTOM title, while campaign events and `--ballisticng-track` still race custom tracks.
- Old thread: not ruled on.
- **Verdict: keep the code; fix the doc comments.** Unclassified: `CUSTOM` is an English
  literal used as a fallback label. Whether it reaches a screen untranslated when the
  mod's `strings/` lack the entry is not read. Check with `--menu-page` before naming it.

### C8. Test files and fixtures named for Remix (rename, low value)

- Files to rename: `crates/game/tests/race_remix_ground_truth.rs` (23 mentions, in
  `it_p_r.rs:38`), `crates/source/tests/remix_2048_sources_ground_truth.rs`, and the test
  name `crates/game/src/custom_tracks/tests.rs:454` ("...becomes_a_remix_on_its_hosts_disc").
- Keep, they test the official mod: `crates/game/tests/official_remix.rs` (`it_m_o.rs:38`),
  `crates/game/tests/drive_ground_truth.rs` (scripts `select launch_remix`,
  `mod.oag.remix.*`), `crates/game/src/boot/tests.rs:220-226` (the mod's folder),
  `crates/ui/src/frontend/tests/wipeout2048.rs:621-653` (an `oag.remix/remix` fixture).
- Fixtures in `crates/ui/src/menu/tests.rs:92-103` and `tests/reveal.rs`, `tests/strip.rs`
  use the labels "REMIX" and `parent = "remix"` as stand-ins for the mod's row. Keep: they
  mirror a real mod.
- **Verdict: rename two test files and one test name** when C2 and C3 land. XS.

## Ranked by value for effort

| Rank | Cluster | Crate | Verdict | Files (non-test + test) | Size | Difficulty | Slipstream overlap |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | C5 stale comments and docs | `oag-game`, `oag-ui`, `docs/` | fix | 4 stale-comment files + 2 docs; the other comment lines ride with the renames | S (25) | easy (20) | none |
| 2 | C1 `remix_menu` module and `settle` | `oag-game` | rename | 4 non-test + 1 test | S (30) | easy (25) | none (`session.rs`, `menus.rs`, `launch_keys.rs` not edited on slipstream) |
| 3 | C4 `remix_only`, `is_offered_outside_remix` | `oag-title` (+ `pulse`, `hd`, `pure`, `2048`, `net`, `game`) | rename (old thread: "rename later") | 9 non-test + 1 test, plus `menus.rs` doc and `docs/architecture/menus.md` | S (30) | easy (25) | none |
| 4 | C2 `oag_game::remix` to `title_lists` | `oag-game` | rename | 6 non-test + 7 test | S (35) | easy (25) | none (`launch_keys.rs`, `campaign.rs` not edited on slipstream) |
| 5 | C6 user-visible "Race Remix" text | `oag-raceplay`, `oag-game` | rename text | 2 non-test + 1 test | XS (10) | easy (15) | none |
| 6 | C3 `oag_source::remix::Remix` to `pair::SourcePair` | `oag-source`, `oag-raceplay` | rename | 4 non-test + 3 test | S (30) | easy (25) | `raceplay/src/load.rs` edited on both branches since merge-base (three-way) |
| 7 | C8 test files and names | `oag-game`, `oag-source` | rename | 2 test files | XS (15) | easy (20) | none on the remix lines |
| - | C7 CUSTOM title | `oag-game` | keep, docs | 1 non-test (+ `menus.rs` doc) | XS (10) | medium (40) | none |

Scout's confidence on the sizes is 20-40; no quoted estimate exists for any of these.
One lane for C1 through C6 is about half a day for one member, with the gate. The
renames are `sed`-level changes plus call-site fixes. No behaviour changes. The
simulation crates are not touched: C4 edits the four title crates, which are data, and
`oag-net` (the lobby filter), neither of which is on the determinism list.

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
  `crates/raceplay/src/load.rs` (see C3) and `crates/game/src/launcher.rs` (commit
  4ea92384d, launcher build date). Not read in detail: whether either edit touches a remix line.

## Open

1. The old thread `handover/tooling/race-remix-as-an-official-mod.md` lists landed work as
   open (its Open items 1-5, its summary, and the "Remix specifics" table). Its lane plan
   is right. Strike the Open section in that thread's next edit, or delete the thread once
   C1-C8 land.
2. Whether the engine's `CUSTOM` fallback label reaches a screen untranslated (C7).
3. The ADR citations of `oag_source::remix::Remix::Split` and `is_offered_outside_remix`
   (ADR-0034, ADR-0035, ADR-0082) go stale with C3 and C4. ADRs are immutable: note the
   new names in `docs/architecture/menus.md` and `workspace-layout.md`, do not edit the ADRs.

## Next Steps

1. Fix C5's stale comments (the `modding.md` half is done). One commit.
2. Land C1 and C2 together: rename `oag_game::remix` to `title_lists`, then `remix_menu` to
   `queries`, moving `settle` into `menus.rs`. Gate with `just gate-affected`.
3. Land C4: `remix_only` to `union_only`, `is_offered_outside_remix` to
   `is_offered_on_race_page`; update `docs/architecture/menus.md` headings and links.
4. After slipstream merges (or rebased there): C3 to `oag_source::pair::SourcePair`, then C6 and C8.
5. Then strike the old thread's open items and delete it when nothing is left.
