---
status: ready-to-wire
titles: [all]
updated: 2026-11-16
summary: "22 game/raceplay test files name no host crate; 10 have a lower host, but their gate selection is unchanged, so this is tidying not a gate win"
size: XS (30)
difficulty: easy (30)
---

# Tests that could move down the dependency tree

CLAUDE.md: a test goes in the lowest crate whose dependencies reach everything
it reads. Of the 22 integration test files in `crates/game/tests/` and
`crates/raceplay/tests/` (5 and 17) that name no `oag_game` or `oag_raceplay`
item in code, 10 have a lower host crate. The other 12 stay: no lower host
reaches what they name (`oag-render`, `oag-mesh`, `oag-sound` and
`oag-source` together, for instance).

**Do not draw this for gate time.** The moves change no test selection, so the
gate saves nothing. Only one move (`web_picker_table`) saves a build, and that
saving is a binary link, not test seconds.

## Why the moves save nothing

A test module can compile only in a package that links every crate it names,
and a package's link set is closed under normal and build dependencies. So the
host's link set contains the library closure of every crate the module names.
`scripts/affected.py` selects a module when a changed crate lies in the library
closure of a crate the module names (`reach`, around line 390; `dependents`,
around line 745). That closure is the same in every valid host, so the trigger
set does not change with the host. The one exception is a path-only trigger
(next section). Feature unification between hosts is the other way a set could
differ; none of the moves here changes a feature.

Measured trigger sets (workspace crates whose change selects the module), now
and in the lowest host:

| file | now | lowest host | triggers now / host |
| --- | --- | --- | --- |
| `crates/game/tests/player_rows.rs` | oag-game | oag-ui | 21 / 21 |
| `crates/game/tests/frontend_grid_ground_truth.rs` | oag-game | oag-ui | 11 / 11 |
| `crates/game/tests/pure_dlc_ground_truth.rs` | oag-game | oag-source | 14 / 14 |
| `crates/game/tests/movie_access_units_ground_truth.rs` | oag-game | oag-assets | 10 / 10 |
| `crates/game/tests/font_roles_ground_truth.rs` | oag-game | oag-raceplay | 22 / 22 |
| `crates/raceplay/tests/hd_texture_spelling_ground_truth.rs` | oag-raceplay | oag-omega | 11 / 11 |
| `crates/raceplay/tests/omega_font_scale_ground_truth.rs` | oag-raceplay | oag-omega | 10 / 10 |
| `crates/raceplay/tests/magstrip_anchor_ground_truth.rs` | oag-raceplay | oag-texture | 11 / 11 |
| `crates/raceplay/tests/plasma_blast_models_ground_truth.rs` | oag-raceplay | oag-fx | 18 / 18 |
| `crates/raceplay/tests/web_picker_table.rs` | oag-raceplay | oag-bytes | path trigger, see below |

Runtime is the logged `test-data` seconds in `target/test-data.log`, from the
main checkout's 2026-11-15 run (the lead-gate log is partial, 928 of 12,692
lines): `font_roles` 1.0 s, `omega_font_scale` 0.5 s, `plasma_blast_models`
0.3 s, `magstrip_anchor` 0.2 s, `hd_texture_spelling` 0.2 s,
`movie_access_units` 0.2 s, `frontend_grid` 0.2 s, `player_rows` 0.1 s,
`pure_dlc` 0.1 s. Frequency times runtime ranks nothing here, because
every move has zero selection change.

## The one path-only trigger

`crates/raceplay/tests/web_picker_table.rs` names no crate. It reads
`web/index.html` and `docs/overview/status.md` (from the repo root). The
selector's files-outside-crates rule (`affected.py`, "Files outside crates") selects
its module for a change to `web/index.html`, and `docs/` is inert, so a
change to `status.md` alone selects nothing. Today a change to the picker page
builds and links the whole raceplay test binary (49 link set) to run a
test of about 0 s. Moved to `oag-bytes` (1 link, no workspace dependencies), the
same change links one tiny binary. `oag-tools` (18 links) is the topical
alternative if the test should sit with the tools. This is a link-time saving
on web changes, measured by reading the rule, not by a timed run.

## Recipe, for a lane that wants the tidying

1. Check the host reaches every crate the module names (`oag_x` or `oag-x` in
   code). Dev-dependencies of the host count: a test binary links them.
2. `git mv` the file into the host's `tests/`. Add `mod <stem>;` to the host's
   `tests/it*.rs` (or the crate's one binary); `every_test_file_is_listed` fails
   until you do.
3. Check the file's `CARGO_MANIFEST_DIR` or relative path. `web_picker_table`
   reads repo files through `../..`, so its root must stay the repo root after
   the move. Grep the other nine for `CARGO_MANIFEST_DIR` before moving them.
4. `cargo nextest list -p <host>` before and after; the count rises by the
   moved file's tests.
5. `just check-title-reach` skips `tests/` (`SKIPPED_DIRS` in
   `scripts/check-title-reach.py`), so moves into `oag-ui`, `oag-fx` or
   `oag-texture` do not trip it. The moves add no normal dependency edge, so
   `just check-deps` should stay green; run it to confirm.
6. `gpu-vram`: none of the ten spawns `oag-game` or captures a frame in code
   (`font_roles` mentions `oag-game` only in a doc comment), so the group does
   not change. `just check-test-groups` should stay green.
7. `movie_access_units` reads a PMF cut and `oag-assets` already runs
   `pmf_ground_truth.rs`. Per CLAUDE.md ("find what already walks that data"),
   fold its check into that pass, not a second disc read.

Gate: `just gate-affected` (quote its `affected:` line before and after), then
`test-data-affected` on the moved files.

## Open

- Whether any lane wants the tidying. Gate time is unchanged.
- The bigger gate cost is raceplay and game as hosts of their own modules
  (2,301 and 1,877 tests, `docs/architecture/affected-gate.md`), not the
  placement of these ten.

## Next Steps

1. Skip unless a lane is already in the file. Then run the recipe for
   `web_picker_table` to `oag-bytes` first, since it is the only move with a
   build saving.
2. Revisit if `oag-omega` or `oag-texture` tests grow near their own ratchets,
   which would make a move cost something.

## Method

The workspace test-link and library closures come from `cargo metadata --offline`:
a package's test binary links its normal and build closure, plus each
dev-dependency with its closure. The module list is a `grep` over each file with
`//` comments removed. Re-run the trigger count with the same closures after
any dependency change before moving a file.
