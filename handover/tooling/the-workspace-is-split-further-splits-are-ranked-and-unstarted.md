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

## Open

- **Split `oag-raceplay`'s `load` and `scene`** (4.2k and 4.8k lines, plus
  `weapons` visuals 2.4k, `gantry`, `field`, `scenery_fx`). Unmeasured: how many
  `Race` fields each reads. If `load` only produces plain data the tick consumes,
  it can sit below the tick; if it needs `Race`, leave it.
- **A profile/progress crate** from `oag-game`: `records` (1k lines), `unlock`,
  `ghosts`, campaign state, probably `settings` (1.7k). This is where "modern
  saves" persistence lives. Check what `main/` reads out of them first.
- **The boot and media cluster** in `oag-game`: `movie` (2.5k), `loading` (2.6k),
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
