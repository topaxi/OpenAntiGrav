---
status: measure
titles: [all]
updated: 2026-11-25
summary: "In-game benchmark: run it on the Deck and the S21 and compare; give Pulse's EXTRAS row a BENCHMARK page; try a mod preset file"
---

# The in-game benchmark is wired on every title; the Deck, the phone and Pulse's EXTRAS are not done

`oag-game --benchmark`, the BENCHMARK row and the graphics page's RUN BENCHMARK row fly one fixed
race per title and end on a stats screen and a result file (`docs/tools/in-game-benchmark.md`).
On the desktop all five titles end on the offscreen bench's state hash, and a full Pulse race
finishes in the window on it too.

## Open

- **No result from the Steam Deck or the S21.** The code path is the window's and the launch file
  hands `--benchmark` to the same parser, but nobody ran it there. A phone's `--race` stays in the
  pre-race flyby (`docs/tools/android.md`); `--benchmark` goes through the menus, which is the
  route that works there.
- **Pulse's `FE_EXTRAS` row still holds nothing** (`crates/pulse/src/main_menu.rs`): the brief
  asked for EXTRAS > BENCHMARK, and only Pulse (and 2048's touch tile) author an EXTRAS row. BENCHMARK
  is on every title after OPTIONS instead (the built-in `oag-extras` mod). Pointing the row at an
  `extras` page that holds BENCHMARK is one `MainMenu` target and one page in `assets/ui/menu.toml`.
- **A mod-supplied or player-supplied preset file** (`overrides/<title>/Data/XML/BenchmarkPresets_<Title>.toml`)
  is read through the asset layers like every other file of ours, but no test puts one there.
- **The refresh budget is 60 Hz where the loop has not measured the display** (a `--drive` run, the
  first seconds of a window): the result says `refresh_source: assumed`.

## Next Steps

1. `oag-game --benchmark pulse` on the Deck and `<image> --benchmark` in the S21's launch file,
   then `just bench-compare` the two result directories and the desktop's (read, 30 minutes with
   the devices free).
2. The EXTRAS page for Pulse (wire, 1 hour: the `MainMenu` target, the page, strings, the
   `main_menu_ground_truth` list).
