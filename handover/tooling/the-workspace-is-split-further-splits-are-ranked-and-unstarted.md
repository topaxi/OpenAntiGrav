---
status: parked
titles: [all]
updated: 2026-11-11
summary: "Measured 2026-11-11: raceplay load and game profile/boot splits save 0 percent in the moved code, 6-10 percent outside it; parked"
size: M (35)
difficulty: medium (30)
---

# The workspace is split into 41 crates; the next three splits are ranked and unstarted

2026-10-05. Fourteen crates were extracted from `oag-game`, `oag-render`, `oag-ui`,
`oag-vex` and `oag-gameplay` in one day; the decision and layering are in
[ADR-0057](../../docs/architecture/adr/0057-the-workspace-after-the-2026-10-05-splits.md)
and the crate table in
[`workspace-layout.md`](../../docs/architecture/workspace-layout.md). Everything is
on `main`, gated: `just` green (5,109 tests), `just test-data` 6,559 of 6,559 in 297 s.
`oag-game` is still about 51k non-test lines and `oag-raceplay` about 51k, so the
job is not finished, only the cheap seams are cut. Method for each: a member in
its own worktree, no re-exports, test count identical before and after, gate plus
the ground-truth tests that touch moved code.

## Measured: the three splits do not pay (2026-11-11)

`cargo build -p oag-game` (dev profile: `oag-raceplay` and `oag-game` at opt-level 2,
`incremental = false`, kache as rustc wrapper, `jobs = 8`), one unique function-body
edit per run, three runs each, under the gate lock, load 6-17:

| edit in | wall (s), 3 runs | critical path |
| --- | --- | --- |
| `raceplay/src/load/` | 43.6, 42.2, 36.4 | raceplay 13.6-15.7, game lib 12-17, game bin 12-16 |
| `raceplay/src/zoom.rs` (outside load/scene) | 45.2, 43.9, 44.7 | raceplay 17.9-20.7, game lib 14-17.5, game bin 13-14 |
| `game/src/boot/` | 29.3, 37.6, 37.5 | game lib 13.4-22.4, game bin 14.7-17.8 |
| `game/src/cursor.rs` (outside the cluster) | 29.1, 34.3, 39.3 | game lib 14-20, game bin 14.6-18.4 |

Why a split cannot cut the first and third rows: cargo has no early cutoff, so an edit
in a crate rebuilds every crate above it, and with `incremental = false` each is a full
recompile. After a split an edit in `load` rebuilds the new crate, then all of raceplay,
then all of game: the same work plus a crate boundary. A split only saves the moved
code's compile time for an edit *outside* it. By non-test code lines, `load` is 12.8 %
of raceplay (about 2.5 s of a 43 s rebuild, 6 %); boot, movie and loading together are
17.5 % of the game lib, the profile set (records, unlock, ghosts, campaign, settings,
profile) 13 %, together 30 % (about 5 s of 34 s, 15 % best case, about 10 % weighted
by where edits land). The game binary unit (`main/`, 12-18 s, a third of the wall) does
not move. Line share is the only proxy (`cargo llvm-lines` is not installed here).

Coupling, which is the other reason to stop: `load` is `use super::*` over lib.rs,
reaches about 30 sibling raceplay modules (`foreign`, `stages`, `gantry`, `zone_grade`,
`scenery_fx`, `extensions`, `catalogue`, `hud`, ...), and 40 files outside it name
`load::` or `Loaded`; it is not plain data below the tick. In `oag-game`, `records`,
`campaign` and `settings` are named by 44, 45 and 61 other files and reach `flyer`,
`mods`, `preview`, `capture` and `main`.

Reopen only if the dev profile gains incremental compilation (then a crate boundary
changes what is reused) or a concrete need other than build time appears (a profile
crate for persistence).

## Open

- **Measured, not worth it (see above): split `oag-raceplay`'s `load` and `scene`** (4.2k and 4.8k lines, plus
  `weapons` visuals 2.4k, `gantry`, `field`, `scenery_fx`). Unmeasured: how many
  `Race` fields each reads. If `load` only produces plain data the tick consumes,
  it can sit below the tick; if it needs `Race`, leave it.
- **Measured, not worth it for build time (see above): a profile/progress crate** from `oag-game`: `records` (1k lines), `unlock`,
  `ghosts`, campaign state, probably `settings` (1.7k). This is where "modern
  saves" persistence lives. Check what `main/` reads out of them first.
- **Measured, not worth it (see above): the boot and media cluster** in `oag-game`: `movie` (2.5k), `loading` (2.6k),
  `boot` (3.6k). `capture` and `race_capture` composite the front end's overlays
  and probably stay.
- **`oag-ui` core** (24k: `frontend` 9k, `menu` 7k) could be cut again, but the
  screens split only just landed; wait for a concrete reason.
- **`oag-mesh`** (16k): check whether `mesh` (pipeline) and `mesh_render` (scene
  drawing) separate cleanly.
- **Loose ends from the splits**, none gated:
  - `crates/pob/src/lib.rs` is exactly 1,000 lines; its next addition trips
    `just check-size`.
  - About 40 items in `oag-ui` and a few in `oag-mesh` went `pub(crate)` to `pub`.
  - Rustdoc unresolved-link warnings rose (93 to 105 across the render crates at
    the render split, and more since); no gate covers them. `just docs` was not
    run on the merged tree.
  - `perf-probe` builds (`cargo check -p oag-game --features perf-probe`, and for
    `oag-raceplay`) but no gate covers it.
  - The four early member worktrees (`../OpenAntiGrav-{audio,particles,render,weapons}-split`)
    and later ones under `../OpenAntiGrav-worktrees/` still exist; their reports
    are in each worktree's `data/scratch/`. Remove with `git worktree remove`
    once read.

## Next Steps

1. `just docs 2>&1 | rg -c warning` on `main` for a baseline, then fix the broken
   cross-crate links in the new crates (about an hour).
2. Measure the `load`/`scene` coupling: `rg -c 'race\.|&Race|&mut Race'
   crates/raceplay/src/load crates/raceplay/src/scene` decides the first split.
3. Draw the profile crate as one member lane (an afternoon), brief: no re-exports,
   host trait only where `main/` needs it.
