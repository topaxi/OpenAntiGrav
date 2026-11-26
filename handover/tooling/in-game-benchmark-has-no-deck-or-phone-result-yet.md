---
status: measure
titles: [all]
updated: 2026-11-26
summary: "Benchmark: compare Deck and full S21 result files; S24 figures only; mod preset file untried; 2048 tile unwired; HD/Omega heading no circuit"
---

# The in-game benchmark is wired on every title; no compared Deck or phone result files

`oag-game --benchmark`, the BENCHMARK row and the graphics page's RUN BENCHMARK row fly one fixed
race per title and end on a stats screen and a result file (`docs/tools/in-game-benchmark.md`).
On the desktop all five titles end on the offscreen bench's state hash, and a full Pulse race
finishes in the window on it too.

## Open

- **No result from the Steam Deck, and no full-length one from the S21.** The S21 flow runs by
  touch from the EXTRAS page, capped at 600 ticks (`docs/tools/in-game-benchmark.md`, "Run on a
  phone and in a browser"), so its numbers compare with nothing.
- **The Galaxy S24's first report is screen figures only** (`docs/tools/in-game-benchmark.md`,
  "First phone result"): no result file is in the record, so `just bench-compare` has nothing to read, and the
  phone has not shown the verdict of the frame budget.
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
- **HD's and Omega's stats heading names no circuit** (2026-11-26, S21 and desktop, a command-line
  run): the heading reads `WIPEOUT HD  YOUR SETTINGS`, where Pulse, Pure and 2048 name one. The
  result file has the circuit (`/data/environments/talons_junction/track.vex`), so
  `benchmark_report`'s lookup of it in `shell.tracks` finds nothing. Cause not read.
- **The screen's term of the frame budget is 60 Hz where the loop has not measured the display** (a
  `--drive` run, the web, the first seconds of a window): the result says `refresh_source:
  assumed`, and a faster screen that names no rate is judged at 60.

## Next Steps

1. `oag-game --benchmark pulse` on the Deck and a full-length run from the S21's BENCHMARK row,
   then `just bench-compare` the two result directories and the desktop's (read, 30 minutes with
   the devices free).
