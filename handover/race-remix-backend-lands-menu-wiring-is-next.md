# Race Remix's backend and menu land; interactive verification is what's left

2026-09-01. Scoped from a pasted feature request (`race_remix_prompt.md`, not
committed - see it in the session that opened this thread if it still
exists): a "Race Remix" menu entry, the existing custom-race box except track
and craft can come from different titles, HUD following the craft by
default, appearing once more than one title is available.

Two premises the request's own framing needed correcting first, both now
fixed:

- **All four titles already race, textured**, not just Pulse -
  `docs/overview/roadmap.md`'s M8 checklist and `crates/game/src/title.rs`'s
  module doc both understated HD/Fury (2026-08-18) and 2048 (2026-08-31).
  Fixed in this session, along with two similarly stale `justfile` `play`
  comments claiming 2048/HD "ships do not draw".
- **ADR-0022 forbids dispatch through a title, not plurality of open
  titles.** `Title`/`Archives` are plain, non-global structs; two `Opened`
  values coexisting is unremarkable (`crate::prefetch`'s `Vec<Archive>` is
  precedent). Confirmed by reading ADR-0022 in full rather than trusting a
  paraphrase.

## What landed (Phase 1, backend - commit 869f907d)

**Scope decision, load-bearing for everything below:** a race gets **two**
sources, not eight - one for the track (collision, environment, pads, weapon
tuning) and one for the craft (livery, HUD, exhaust/flare, boost plume,
handling stats, the whole grid's roster). Every AI opponent's team comes from
the craft title, same as today's single "TEAM" picker. Per-opponent title
mixing (8 independently-sourced ships) was not asked for and would need a
per-`Ship` provenance field reconciled against `crates/gameplay/src/hash.rs`'s
exhaustive `write_ship` destructure - real work with no requester yet.

- `crates/game/src/remix.rs` - `Remix::Single(Opened)` /
  `Remix::Split { track, craft }`, `Remix::open(source, craft_source, packs)`,
  `Remix::into_parts()` (returns owned values rather than `&mut self`
  accessors, deliberately - see its doc comment for the borrow-checker
  reason). `craft_of(&mut Option<Archives>, &mut Archives) -> &mut Archives`
  is the one-line wrapper every craft-governed read in `race/load.rs` uses.
- `crates/game/src/race/options.rs` - `Options.craft_source: Option<String>`,
  `None` meaning "same as `source`", today's behaviour, byte-identical.
- `crates/game/src/race/load.rs` - `load()` opens a `Remix`; every call site
  reads from `archives`/`title` (track) or `craft_of(...)`/`craft_title`
  (craft), decided per what it actually loads. Roster resolution moved to
  `crates/game/src/race/load/roster.rs` to stay under the 1,000-line cap.
- `crates/game/src/main/cli.rs` - `--craft-source` flag.
- `crates/game/src/main/prepare.rs` - wires it into `race::Options`.

## What landed (Phase 2, menu - uncommitted, this session)

- `crates/game/src/menu/vocabulary.rs` - new. `Action`/`ValueSource` moved
  out of `menu.rs` verbatim (a pure move) to make room under `menu.rs`'s own
  baselined ceiling in `scripts/check-file-size.py`, which is now **lowered**
  to the post-move size (1824, was 1975) rather than left slack - the ratchet
  the file's own doc comment asks for. `Action::LaunchRemix` and
  `ValueSource::{Titles, RemixTracks, RemixTeams}` are the new variants.
- `assets/ui/menu.toml` - new `remix` page: MODE/SPEED CLASS/AI DIFFICULTY
  reuse `race.mode`/`race.class`/`ai.difficulty` verbatim (same settings, same
  values - nothing about a mixed grid changes what a mode means); TRACK
  TITLE -> TRACK and CRAFT TITLE -> TEAM are title-then-scoped-list, each
  pair on its own settings keys (`remix.track_title`/`remix.track`,
  `remix.craft_title`/`remix.team` - deliberately not reusing
  `race.track`/`race.team`, which would let the two pages clobber each
  other's last pick). `REMIX` sits on `main`, unconditionally - see the
  resolved gating question below.
- `crates/game/src/settings.rs` - `Settings.remix: Remix` (new struct:
  `track_title`, `track`, `craft_title`, `team`, all `String`, empty
  default), seeded in `menu_seeds` alongside `race.*`. Required by
  `every_settings_row_is_one_the_game_seeds`, which sweeps every page, not
  just the ones somebody happened to be working on.
- `crates/game/src/launcher.rs` - `distinct_titles(rows: &[Candidate]) ->
  Vec<Candidate>`: every playable candidate, one per distinct title, first
  pressing found wins. Feeds both the `REMIX` menu's title lists and, in
  principle, a future real gate (see below).
- `crates/game/src/remix.rs` - `Catalogue { tracks: Vec<(catalogue::Track,
  String)>, teams: Vec<menu::Choice> }` and `pub fn catalogue(source: &str)
  -> anyhow::Result<Catalogue>`: opens a source only as far as "what can this
  title race" needs - its plugin definition and one language's string table
  for labels - **not** a full menu shell (font atlas, sprite sheet, menu
  frame), which a remix picker never draws in since it stays in the booted
  title's own chrome. `Catalogue::track`/`Catalogue::team` mirror
  `main::session::Shell::track`/`::team`'s lookup-by-id rule, scoped to
  whichever title the catalogue was built from.
- `crates/game/src/main/session.rs` - `Session.titles: Vec<Candidate>`, a
  survey run once at `Session` construction (`main/app.rs`,
  `launcher::distinct_titles(&launcher::survey(&source::candidates()))`) -
  a **second** survey, deliberately not threaded through from whichever one
  decided the disc chooser in `main.rs`; see the field's own doc comment for
  why (different process-state shape, and the cost is once per windowed
  boot, not per frame).
- `crates/game/src/main/session/menus.rs` - `Session::open_menus` supplies
  `Titles` and, when `remix.track_title`/`remix.craft_title` are already
  saved, the scoped lists too, so a reopened menu shows a saved pick from
  the first frame - the same reasoning `resupply_tracks_for_mode` already
  established for the ordinary RACE page.
  `Session::resupply_remix_tracks`/`resupply_remix_teams` are the
  `remix.track_title`/`remix.craft_title`-changed reactive counterparts,
  wired from `Session::apply_setting`. `Action::LaunchRemix` in
  `Session::handle_menu` builds `race::Options` the way `Action::LaunchRace`
  does, plus resolving `source`/`craft_source` from the picked titles' own
  `Candidate::source` and validating `remix.track`/`remix.team` against a
  freshly fetched `remix::catalogue` for each side - an unpicked CRAFT TITLE
  (empty setting) means "same as the track", matching
  `Options::craft_source: None`'s own meaning, not a redundant second open.

**The `disabled_by` gating question is resolved, and the answer is sharper
than the original framing.** `Menu::held` (`crates/game/src/menu.rs:1537`)
only ever reads another entry's own `Entry::chosen()` - there is no external
fact injection anywhere in the menu engine. Worse than the synthetic-key risk
first flagged: `resolve()` (`menu.rs:968-980`, now inside `vocabulary.rs`'s
sibling logic - the check itself is still in `menu.rs`) refuses `disabled_by`
on anything but `choice`/`toggle`, and `REMIX` is a `Submenu` entry, so it
cannot carry one at all, synthetic key or not. **Decision, applied:** `REMIX`
sits on `main` unconditionally. With one title available, TRACK TITLE and
CRAFT TITLE each offer exactly one row and the page degrades to today's
ordinary race box - nothing broken, nothing misleading, the same page doing
less. If real gating is wanted later, copy `launcher.rs`'s shape: the
composition root decides before any menu exists (`rows.len() > 1` there),
not a `menu.toml` condition - extending `Condition` to read an injected fact
map would mean loosening `resolve()`'s kind check and exempting that key from
`check_condition`'s validation (`menu.rs:873-905`), three coupled changes to
the one module whose own doc comment calls cross-row logic "deliberately the
*only*" logic it has.

**Verified against real data, and by the gate, but not by eye on screen:**
- Full `just` gate (fmt, lint, check-size, check-deps, check-determinism,
  check-docs, check-handover) passes.
- `cargo nextest run --workspace`: 2692/2692 non-ignored tests pass, 0
  regressions - includes `crates/game/src/menu/tests/strip.rs`'s six tests
  that hardcode the `main` page's row count/order, updated for the new
  `REMIX` row (row count 3→4, wrap index 2→3, one extra `Down` press to
  reach OPTIONS).
- Every existing ground-truth test across all four titles still passes
  unmodified (`race_ground_truth`, `pure_race_ground_truth`,
  `hd_livery_ground_truth`, `hd_hud_ground_truth`, `livery_ground_truth`,
  `hd_engine_flare_ground_truth`, `hd_trackwall_ground_truth`,
  `ps2_source_ground_truth`, `sfx_ground_truth`, `zone_ground_truth`,
  `zone_grade_ground_truth`, `zone_sky_ground_truth`, `loading_screen_ground_truth` -
  141/141 across both sessions) - the regression property held under real
  load.
- `crates/game/tests/race_remix_ground_truth.rs`, three tests, all passing
  with real data: the no-op regression (Pulse and HD), the 2048-track/Pure-
  craft remix end to end (`[INFO] racing on Wipeout 2048` /
  `[INFO] craft from Wipeout Pure`, Pure's own roster and HUD entry name),
  and `the_catalogue_offers_a_titles_real_tracks_and_roster` - `remix::catalogue`
  exercised directly against a real HD disc, independent of the menu
  machinery around it.
- **Not verified: the menu on screen.** This environment has no GUI (see the
  `no-gui-windows-on-desktop` memory) - screenshots come back black even when
  the sim runs headlessly, so the `remix` page's actual layout, the title
  pickers' scrolling, and `Action::LaunchRemix` firing from a real button
  press have none of that verified interactively. Everything above proves
  the *data path* is correct; a human at a keyboard (or a machine with a
  real display) is the next check the menu itself needs.

Pre-existing, unrelated failure noticed along the way and confirmed to
predate this work (reproduces identically on the main checkout with no
changes): `dlc_ground_truth::the_four_pulse_packs_declare_the_four_teams_they_add`
and `::european_packs_mount_against_the_american_disc` both fail because
`data/dlc/` on this machine carries more packs (12 teams) than the tests'
hardcoded expectation (4 teams) - an environment/test-data mismatch, not a
regression from this thread.

## 2026-09-01, later: a human tested it interactively, and found two real bugs

Item 1 below is closed by this. Two findings, both from an actual player on
an actual display - exactly what this thread flagged as unverifiable from
inside the session:

- **`just play remix`'s first version never opened a menu at all.** It kept
  `--race`, which skips the front end entirely, so the recipe raced 2048 and
  Pure and exited without ever giving anyone a chance to see the RACE REMIX
  page. Fixed: it now boots normally and prints the walk to it (START, Down,
  confirm), since no flag seeds a *live* window onto one interactive menu
  page - `--menu-page` only does that for the non-interactive `--screenshot`
  capture path.
- **HD's track labels on the REMIX page read as raw ids (`01_Track`), not
  names.** Item 3 below undersold this - it isn't only reversed-circuit
  disambiguation that was missing, it's that `strings.get(&track.id)`
  resolves *nothing* on HD (confirmed: 1,602-entry string table, direct
  lookup by track id returns `None` for all 28 circuits) because a circuit's
  real name lives at a string-table key the id does not predict. Fixed by
  wiring `crate::boot::load_circuit_names` (made `pub(crate)`, was
  `pub(super)`) and `crate::catalogue::label` into `remix::catalogue`
  properly, the same machinery the ordinary RACE page's `Shell` already
  uses. Verified: all 28 HD circuits now show real names
  (`TALON'S JUNCTION`, `TALON'S JUNCTION REVERSE`, ...), reproducer
  `crates/game/examples/remix_hd_track_labels_probe.rs`, regression pinned in
  `race_remix_ground_truth.rs`.
- **2048 as the CRAFT title fails for every team the picker offers** -
  discovered testing the same session, a third real finding, but not a Race
  Remix bug and not fixed here: it needs actual RE/type-design work. See
  [the 2048 handover thread](2048s-track-vex-parses-but-two-binary-formats-changed.md#2026-09-01-race-remixs-live-team-picker-is-the-first-thing-to-ever-ask-for-a-2048-team-by-name-and-none-of-the-17-it-offers-resolve)
  for the full finding. **Workaround: use 2048 only as the TRACK title in
  Race Remix for now** - proven working end to end at the loader - and pick
  Pulse, Pure or HD as the craft title.

## 2026-09-01, later still: picking 2048 as TRACK TITLE showed no circuits

A fourth real finding from the same round of interactive testing, and the
"proven working end to end" note directly above was true of `race::load`
alone, not of the menu a player actually sees - the gap this closes.

`remix::catalogue` (the RACE REMIX page's own reader, see the section above)
read `PI_Track` nodes off `oag_title::Title::plugin_definition` - the file
every other title's roster *and* circuit list share. Wipeout 2048 splits
into three plugin files where every other title ships one (see
`oag_2048::names`'s own module doc), and `plugin_definition` was pointed at
the *teams* one, by design: a race needs a roster before it needs a circuit
list. Nothing named where the circuits one was, so the catalogue's `PI_Track`
read found nothing on 2048's own plugin file and the TRACK row came back
empty - not a missing feature, a title fact with nowhere to be recorded.

Fixed with a new axis, `oag_title::Title::track_plugin_definition: Option<&str>` -
`None` on Pulse/Pure/HD, where `plugin_definition` already answers for
tracks too; `Some(oag_2048::names::TRACK_PLUGIN_DEFINITION)` on 2048.
`remix::catalogue` reads teams off `plugin_definition` and tracks off
`track_plugin_definition.unwrap_or(plugin_definition)`, so it is one extra
archive read only on the one title that needs it. Confirmed against the
extracted package: the base game declares ten circuits (`square`, `park`,
`tower`, `mall`, `bridge`, `arena`, `subway`, `cathedral`, `sol`, `altima`),
all found by id; pinned in
`race_remix_ground_truth::the_catalogue_offers_2048s_own_track_list`. The
DLC-added twelve (HD circuits reshipped, see `oag_2048::lib`'s own doc) are
not checked by this test - base alone was sufficient to prove the axis.

## Open

1. ~~Interactive verification.~~ **Done** - see the section above. Two real
   bugs found and fixed; a third (2048-as-craft) found, diagnosed and
   tracked in the other thread, not fixed here.
2. **HUD-follows-craft is "by default" in the spec**, which implies
   overridable. This build hardwires it (HUD always follows the craft title,
   no separate picker) - an explicit scope cut carried from Phase 1, not
   revisited since. Confirm that is still acceptable, or add the override as
   its own small page entry.
3. ~~Track/team labels are string-table-only, with no `crate::catalogue::label`
   disambiguation.~~ **Fixed** - see the section above; this undersold the
   actual bug (no label resolved at all on HD, not merely un-disambiguated
   reversed circuits).
4. **The session-construction survey is a second one**, not threaded through
   from whichever decided the disc chooser in `main.rs` - see
   `Session.titles`'s doc comment for the reasoning and the cost (one extra
   survey per windowed boot, not per frame). Worth revisiting if it turns out
   to matter in practice.
5. ~~Docs (Phase 3): a roadmap line, and a short ADR.~~ **Done.**
   [ADR-0034](../docs/architecture/adr/0034-a-race-may-open-two-titles-at-once.md)
   records "more than one `Title`/`Archives` alive at once is fine, dispatch
   through one is still not" as an explicit precedent (ADR-0022 never ruled
   on plurality, so this clarifies rather than supersedes it) and
   `docs/overview/roadmap.md`'s M8 section carries a "What Race Remix does
   today" line and subsection alongside Pure/HD/2048's own.
6. ~~2048 as CRAFT TITLE.~~ **Partly fixed, through the menu only.**
   `oag_title::RaceDefaults::team_variants` plus a VARIANT row on both the
   RACE and RACE REMIX pages (menu.toml, `session/menus.rs`) fixes the
   native 5-team roster from the menu - a full race loads on a combined id
   neither title's `DEFAULT_TEAM` reaches, pinned in
   `race_remix_ground_truth::a_team_variant_races_on_both_join_shapes`.
   `race::load` itself still knows nothing of `team_variants` - `--race
   --team Auricom2048` fails exactly as before; a CLI user must spell the
   combined id (`Auricom2048\1`) directly. The 12 HD-derived teams still
   fail either way (a different tree, `HD_SHIP_DIR` vs `HANDLING_DIR`, that
   `team_variants` does not address); tracked in the other thread.

Items 2 and 4 are documented, deliberate scope cuts, not defects. Nothing
in this thread is blocking; it can close once item 2 is either confirmed as
staying a cut or given its own follow-up thread.

## Next Steps

1. Decide item 2 (HUD override) - confirm the cut or scope the override.
2. If item 4 (double survey) ever shows up in a profile or a bug report,
   revisit threading the boot-time survey through instead of a second one.
