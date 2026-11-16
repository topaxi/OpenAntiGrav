---
status: blocked-maintainer
titles: [all]
updated: 2026-11-10
summary: "Affected gate floor is raceplay+game+net tests (71 % of suite seconds); open: assets->title edge (new ADR), raceplay view-gated tests"
size: L (50)
difficulty: medium (50)
---

# The affected gate's floor is the raceplay, game and net tests; two measured cuts are open

`scripts/affected.py` now selects by what a test links, down to the test
module ([`docs/architecture/affected-gate.md`](../../docs/architecture/affected-gate.md)).
Replaying the last 50 merges, the median selection fell 12 % in summed test
seconds (6,795 s to 5,949 s on a 2026-11-10 full log) and 22 % in tests, not
the 40 % the lane aimed for. The remaining cost is not over-selection:
`oag-raceplay` (3,306 s), `oag-game` (1,662 s) and `oag-net` (326 s) hold
5,294 of the suite's 7,431 summed seconds and link nearly every crate, so any
rule that follows the link graph stops there.

Measure anything below with a full log:

```sh
python3 scripts/affected-replay.py target/test-data.log -n 50
python3 scripts/affected-replay.py target/test-data.log --crates
```

## Open

1. **`oag-assets` links `oag-title`, so a `Title` field change selects every
   format and texture test.** `oag-title` changed 468 times in three months
   and 20 of the last 50 merges touched it or a title crate. Those merges
   select about 7,191 s; with the `oag-assets` to `oag-title` edge removed
   they would select about 5,681 s (`oag-texture`, `oag-rcs`, `oag-mesh` and
   most of `oag-vex` drop out). `oag_assets::source` reads
   three things off a `Title`: `archives` (`ArchiveCandidates`),
   `foreign_serials` and the `AddOn` list (which carries `HudLayouts` and
   `NodeMap`). Cutting the edge means `Source::open` takes those as its own
   narrow type and the archive vocabulary moves below `oag-title` (it already
   depends on `oag-disc`, which holds `Platform`). ADR-0022 says `oag-assets`
   knows which title a file came from, so this is a new ADR and about 260
   call sites in 138 files. **Waits on the maintainer's decision.**
2. **`oag-raceplay`'s simulation tests run for a renderer-only change.** A
   first `cargo check -p oag-raceplay --no-default-features --tests` fails in
   140 of the 220 integration test modules; the other 80 hold at most 921 s
   of the crate's roughly 3,000 integration seconds (an upper bound: gating the 140
   may surface more). Marking the view-using modules
   `#[cfg(feature = "view")]` in `tests/it_*.rs`, adding a gate check that the
   rest compile without `view`, and teaching `scripts/affected.py` that a
   change reached only through a default feature's optional dependencies
   selects only the gated modules would take those seconds off every
   `oag-render`, `oag-mesh`, `oag-fx` and `oag-post` lane (about 16 % of what
   such a lane selects). The miss to name: a `#[cfg(feature = "view")]`
   statement inside a simulation function, which runs in a unified build.
3. **32 test modules stay in `crates/render/tests/` that never name
   `oag_render`**, because they name `oag_pulse`, `oag_pure`, `oag_tables` or
   `oag_post`, which `oag-mesh` must not depend on. They cost 35 s together.
   The selector already skips them for a render-only change.

## Next Steps

- Decide item 1 (an ADR superseding the relevant part of ADR-0022), then a
  mechanical lane moves the three types and fixes the call sites.
- Item 2 is a half-day lane on its own: the module list is the set of files
  the failing `cargo check` above names.
- The larger lever is the suite's own cost: every selection is a subset of
  the same 7,431 s, and 5,294 s of it runs for almost every change. Re-profile
  the tail of `oag-raceplay` and `oag-game` by the recipe in `CLAUDE.md`
  ("The test-data suite").
