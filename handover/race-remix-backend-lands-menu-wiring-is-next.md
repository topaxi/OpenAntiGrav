# Race Remix's backend lands; the menu is what's left

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
  paraphrase - see the plan file this thread's work followed,
  `/home/topaxi/.claude/plans/stateless-frolicking-floyd.md` on the machine
  that ran it (not committed; reconstructable from this file and the diff).

## What landed (Phase 1, backend)

**Scope decision, load-bearing for everything below:** a race gets **two**
sources, not eight - one for the track (collision, environment, pads, weapon
tuning) and one for the craft (livery, HUD, exhaust/flare, boost plume,
handling stats, the whole grid's roster). Every AI opponent's team comes from
the craft title, same as today's single "TEAM" picker. Per-opponent title
mixing (8 independently-sourced ships) was not asked for and would need a
per-`Ship` provenance field reconciled against `crates/gameplay/src/hash.rs`'s
exhaustive `write_ship` destructure - real work with no requester yet.

- `crates/game/src/remix.rs` - new. `Remix::Single(Opened)` /
  `Remix::Split { track, craft }`, `Remix::open(source, craft_source, packs)`,
  `Remix::into_parts()` (returns owned values rather than `&mut self`
  accessors, deliberately - see its doc comment for the borrow-checker reason:
  `race::load` reads its `Archives` on and off across the whole function, so a
  borrow taken from a method call would have to stay live for the entire span
  and would block ever reading the other side). `craft_of(&mut Option<Archives>,
  &mut Archives) -> &mut Archives` is the one-line wrapper every craft-governed
  read in `load.rs` uses.
- `crates/game/src/race/options.rs` - `Options.craft_source: Option<String>`,
  `None` meaning "same as `source`", today's behaviour, byte-identical.
- `crates/game/src/race/load.rs` - `load()` opens a `Remix` instead of one
  `Opened`; every call site now reads from `archives`/`title` (track) or
  `craft_of(&mut craft, &mut archives)`/`craft_title` (craft), decided per
  what it actually loads. Also moved the roster-resolution block into
  `crates/game/src/race/load/roster.rs` (new) to stay under the 1,000-line
  cap - a pure move, no behaviour change, matching the existing
  `audio`/`cameras`/`environment`/`geometry`/`surfaces` submodule pattern.
- `crates/game/src/main/cli.rs` - `--craft-source` flag.
- `crates/game/src/main/prepare.rs` - wires it into `race::Options`.

**Verified against real data, not just believed:**
- Full `just` gate (fmt, lint, check-size, check-deps, check-determinism,
  check-docs) passes.
- Every existing ground-truth test across all four titles still passes
  unmodified (`race_ground_truth`, `pure_race_ground_truth`,
  `hd_livery_ground_truth`, `hd_hud_ground_truth`, `livery_ground_truth`,
  `hd_engine_flare_ground_truth`, `hd_trackwall_ground_truth`,
  `ps2_source_ground_truth`, `sfx_ground_truth`, `zone_ground_truth`,
  `zone_grade_ground_truth`, `zone_sky_ground_truth` - 115/115) - the
  regression property held under real load, not merely under a same-shape
  argument.
- New `crates/game/tests/race_remix_ground_truth.rs`:
  `naming_the_same_source_as_craft_source_changes_nothing` (Pulse and HD, the
  no-op case) and `a_2048_track_races_with_a_pure_craft` (the spec's own
  example - a genuine remix, verified end to end: `[INFO] racing on Wipeout
  2048` / `[INFO] craft from Wipeout Pure`, Pure's own team roster, Pure's own
  `TimeTrial_HUD.xml`). Both pass with real data
  (`OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all
  -E 'binary(race_remix_ground_truth)'`).
- Manual `cargo run -p oag-game --bin oag-game -- --race --dry-run
  data/extracted/vita/PCSF00007 --craft-source data/images/pure-psp-eu.chd`
  is the same evidence, read by eye.

Pre-existing, unrelated failure noticed along the way and confirmed to
predate this work (reproduces identically on the main checkout with no
changes): `dlc_ground_truth::the_four_pulse_packs_declare_the_four_teams_they_add`
and `::european_packs_mount_against_the_american_disc` both fail because
`data/dlc/` on this machine carries more packs (12 teams) than the tests'
hardcoded expectation (4 teams) - an environment/test-data mismatch, not a
regression from this thread. Not investigated further; flagging so nobody
chases it as a Race Remix bug.

## Open

Phase 2 (menu wiring) and Phase 3 (docs - an ADR recording "plurality of open
titles is fine, dispatch through one still is not" as an explicit precedent,
plus a roadmap line) have not started. The full design for Phase 2 is in the
plan file cited above; the two things it flags as needing an implementation-
time read rather than an upfront assumption:

1. **Whether `disabled_by` (`crates/game/src/menu.rs`, `Entry::Choice`
   handling near `menu.rs:453-528`) reads another `Choice` row's own
   `current`, or an externally-seeded setting.** Both existing uses reference
   `race.mode`, a `Choice` row on the same page. If it only ever reads
   another `Choice`, a synthetic `remix.available` key with no `Choice` row
   silently never matches and the `REMIX` menu entry stays enabled always -
   check this before wiring the entry's gate, not after.
2. **`Session::handle_menu`'s `LaunchRace` branch**
   (`crates/game/src/main/session/menus.rs:374-486`) reads
   `self.settings.race.{mode,track,team,class}` against the single boot-time
   `self.shell` and does not touch `source` at all - a remix menu page needs
   either to write the same `race.*` keys plus two new source keys (extending
   this branch), or its own `Action::LaunchRemix` (which also needs adding to
   `menu.toml`'s loader-enforced action allow-list, currently
   `{launch_race, quit}`). The picker yields a **title**; `Options.source`/
   `craft_source` need a **path string** - `launcher::Candidate::source`/
   `.title()` already carries that mapping, so hold the surveyed
   `Vec<Candidate>` (or equivalent) somewhere reachable at launch time.

## Next Steps

1. Read `menu.rs`'s `disabled_by` resolution (item 1 above) - the
   discriminating fact the rest of the gating design depends on.
2. New `remix` page in `assets/ui/menu.toml`: MODE/SPEED CLASS/AI DIFFICULTY
   (unchanged, reused) then TRACK TITLE -> TRACK (scoped) -> CRAFT TITLE ->
   TEAM (scoped) -> START. Extend `menu.rs`'s closed `ValueSource` enum with
   `Titles`, `RemixTrack`, `RemixTeam`.
3. `crates/game/src/main/session/menus.rs`: reactive resupply - when
   `remix.track_title`/`remix.craft_title` changes, open (or reuse from a
   small `HashMap<&'static str, Shell>` cache - fine here, this is UI-layer
   `crates/game`, not a determinism-bound crate) the newly picked title's
   `Shell` and re-`supply` the scoped lists. `Titles` supplied once from
   `launcher::survey` deduped by `Candidate::title()`.
4. Wire item 2 above (`LaunchRace` extension or `Action::LaunchRemix`).
5. HUD-follows-craft is "by default" in the spec, which implies overridable;
   Phase 1/2 as scoped hardwire it (no separate HUD-source picker) -
   confirm that's still an acceptable cut before closing this thread, or add
   the override.
6. ADR + roadmap line (Phase 3), once the menu half is real.
