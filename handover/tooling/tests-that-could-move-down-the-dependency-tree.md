---
status: ready-to-wire
titles: [all]
updated: 2026-11-16
summary: "Nine tests in oag-game and oag-raceplay name no game crate; moving them saves ~0 gate seconds (same selection), so the move is for locality only"
size: XS (10)
difficulty: easy (30)
---

# Tests that could move down the dependency tree

CLAUDE.md: a test goes in the lowest crate whose dependencies reach everything
it reads. Nine integration tests in `crates/game/tests/` and
`crates/raceplay/tests/` name no `oag_game`/`oag_raceplay` item in code and
spawn no `oag-game` binary, so they could live lower. A measured check says
the move saves no test-seconds under `scripts/affected.py`. Do this only as
tidying, not as a gate win.

## Why it saves nothing

`scripts/affected.py` (`reach`, `dependents`, around lines 390 and 745) selects
a test module when a crate it names (`oag_x`/`oag-x` in code) has the changed
crate in its library closure (normal and build dependencies). The module's
trigger set is the same in its current host and in the candidate host, for all
nine (scored with the library closures in `lib.json`, method below).
Also, the host package is selected anyway: `lib(oag-game)` holds 52 of its
53-package link set, so every changed crate that would trigger a candidate
also selects other `oag_game`-naming modules of the same binary.

## Candidates

Trigger count = workspace crates whose change selects the module, before and after.
Runtime = logged `test-data` seconds in `target/test-data.log` (2026-11-15).

| path | now | lowest host with the same trigger set | triggers now / host | runtime |
| --- | --- | --- | --- | --- |
| `crates/game/tests/player_rows.rs` | oag-game | oag-ui | 21 / 21 | 0.1 s |
| `crates/game/tests/frontend_grid_ground_truth.rs` | oag-game | oag-ui | 11 / 11 | 0.2 s |
| `crates/game/tests/pure_dlc_ground_truth.rs` | oag-game | oag-source | 14 / 14 | 0.1 s |
| `crates/game/tests/movie_access_units_ground_truth.rs` | oag-game | oag-assets | 10 / 10 | 0.2 s |
| `crates/game/tests/font_roles_ground_truth.rs` | oag-game | none lower than oag-raceplay (names ui, source, assets) | 22 | 1.0 s |
| `crates/raceplay/tests/hd_texture_spelling_ground_truth.rs` | oag-raceplay | oag-omega | 11 / 11 | 0.2 s |
| `crates/raceplay/tests/omega_font_scale_ground_truth.rs` | oag-raceplay | oag-omega | 10 / 10 | 0.5 s |
| `crates/raceplay/tests/magstrip_anchor_ground_truth.rs` | oag-raceplay | oag-texture | 11 / 11 | 0.2 s |
| `crates/raceplay/tests/plasma_blast_models_ground_truth.rs` | oag-raceplay | oag-fx | 18 / 18 | 0.3 s |

Stay where they are (no lower host reaches what they name): `main_menu_ground_truth`
(needs `oag_source`, which `oag-ui` does not link), `german_ground_truth`,
`language_offered_ground_truth`, `hd_mix_ground_truth`, `hd_flare_spike_names_ground_truth`,
`hd_shield_colour_ground_truth`, `omega_2048_shield_colour_ground_truth`,
`sfx_xfade_ground_truth`, `sfx_xfade_2048_ground_truth`, `track_audio_hd_ground_truth`,
`zone_announcer_ground_truth`, `gantry_mount_hd_circuits_ground_truth` (37.9 s,
needs `oag_render` and `oag_mesh`, which only raceplay reaches among the
existing hosts), and `web_picker_table.rs` (names no crate, reads a file;
it stays until someone checks what path it reads).

`hud` (25 links) reaches the same modules for german, language_offered and
main_menu, but it is a HUD crate and those tests are menu and language tests.
Rejected on topic.

## Recipe, if a lane wants the tidying anyway

1. Confirm the host reaches every crate the module names (`oag-x` in
   `scripts/affected.py`'s `named`). Dev-dependencies of the host are fine:
   a test binary links them.
2. `git mv` the file into the host's `tests/`, then add `mod <stem>;` to
   the host's `tests/it*.rs` (one `it*.rs` binary per crate, or the listed
   range), keeping `every_test_file_is_listed` green.
3. Check `crates/*/tests` for a path into the old crate (`CARGO_MANIFEST_DIR`,
   `../../crates/raceplay`); none of the nine reads one.
4. `cargo nextest list -p <host>` before and after: the count should rise by
   exactly the moved module's tests.
5. Gate-affected plus test-data-affected on the moved file. No test here spawns
   `oag-game` or captures a frame, so the `gpu-vram` group does not change.
   `just check-test-groups` should stay green.

## Open

- Gate saving: none measured or expected. The move only shortens the link of
  the host binary for a developer working on that host.
- The bigger cost in the gate is raceplay and game as hosts of their own
  modules (2,301 and 1,877 tests, `docs/architecture/affected-gate.md`), not
  the placement of these nine.
- Measure with `just gate-affected` (its `affected:` line) before and after a
  move; the expected change is zero.

## Next Steps

1. Do nothing unless the lead wants tidying: then run the recipe above for the
   five game and raceplay candidates whose host is `oag-ui`, `oag-source`,
   `oag-assets`, `oag-omega`, `oag-texture` or `oag-fx`.
2. Revisit when `oag-omega` or `oag-texture` tests grow past their own
   ratchets, which would make the move cost something.

## Method

The test-link and library closures come from `cargo metadata --offline` over
the workspace: a package's test binary links its normal and build closure, its
dev-dependencies with their closures, and a crate named in code selects a
module through its library closure. The module list is a `grep` over each file
with `//` comments removed. Commits per crate are the table in
`docs/architecture/affected-gate.md`. Rerun the trigger count with the same
closures after any dependency change before moving a test.
