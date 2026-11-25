---
status: measure
titles: [all]
updated: 2026-11-25
summary: "In-game benchmark: run it on the Deck and the S21 and compare; try a mod-supplied preset file; 2048 EXTRAS tile unwired"
---

# The in-game benchmark is wired on every title; the Deck and the phone are not run

`oag-game --benchmark`, the BENCHMARK row and the graphics page's RUN BENCHMARK row fly one fixed
race per title and end on a stats screen and a result file (`docs/tools/in-game-benchmark.md`).
On the desktop all five titles end on the offscreen bench's state hash, and a full Pulse race
finishes in the window on it too.

## Open

- **No result from the Steam Deck or the S21.** The code path is the window's and the launch file
  hands `--benchmark` to the same parser, but nobody ran it there. A phone's `--race` stays in the
  pre-race flyby (`docs/tools/android.md`); `--benchmark` goes through the menus, which is the
  route that works there.
- **2048's `FE_EXTRAS` tile** (manual, credits) holds no BENCHMARK: 2048 shows it after OPTIONS
  like HD, Omega and Pure, whose disc menus author no EXTRAS row.
- **A mod-supplied or player-supplied preset file** (`overrides/<title>/Data/XML/BenchmarkPresets_<Title>.toml`)
  is read through the asset layers like every other file of ours, but no test puts one there.
- **Two restore paths have no automated test.** A benchmark whose race fails to build
  (`Session::abandon_benchmark_launch`, called from `finish_race_loading`'s error arm) and the
  anisotropy put back after a preset (`restore_benchmark_player`) need a `Session` with a window,
  which no unit test has; a drive script cannot make a load fail or read the runtime field after
  the CLI run quits. Focus loss, the system Back, a preset's settings file and RUN BENCHMARK over
  a parked race do have `render_bench_ground_truth` tests.
- **The refresh budget is 60 Hz where the loop has not measured the display** (a `--drive` run, the
  first seconds of a window): the result says `refresh_source: assumed`.

## Next Steps

1. `oag-game --benchmark pulse` on the Deck and `<image> --benchmark` in the S21's launch file,
   then `just bench-compare` the two result directories and the desktop's (read, 30 minutes with
   the devices free).
