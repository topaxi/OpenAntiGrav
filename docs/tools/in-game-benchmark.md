# In-game benchmark

One fixed race per title, flown in the window with the player's own settings (or a named preset),
ending on a stats screen and a result file that `just bench-compare` reads. It is the player-facing
half of the [whole-race render bench](render-bench.md): that one draws offscreen and measures the
renderer; this one draws through the window, so a frame is the interval between two presents under
the player's present mode, and the same figures come out of a desktop, a Steam Deck and a phone.

## Starting it

| From | How | BACK on the stats screen |
| --- | --- | --- |
| The command line | `oag-game --benchmark [title] [--preset NAME]` | quits |
| The menus | Pulse: EXTRAS (its disc's own `FE_EXTRAS` row, last in the disc's order) opens an EXTRAS page of ours holding BENCHMARK. Every other title: BENCHMARK, the row the built-in `oag-extras` mod adds after OPTIONS (`assets/mods/oag-extras/menu.toml`; on a flat main menu it draws inline after OPTIONS) | the page it was fired from |
| The graphics page | RUN BENCHMARK, before ADVANCED (`assets/ui/menu.toml`): change a setting, run, change it again | the graphics page |

`title` is a disc image or folder like any first argument, or a title's short name (`pulse`,
`pure`, `hd`, `2048`, `omega`) for the first playable disc of that title on the search path. On
Android the launch file takes the same flags (`files/oag-args.txt`, [Android](android.md)):
`<image> --benchmark`; the result is in the app's `files/data/oag/bench/` (`adb pull`). `--ticks N` stops the race at tick N and the result says `finished: false`;
it is for a short test run, not for results.

Escape during the race abandons it with no result. The race cannot be paused: a pause would be a
frame time.

## The race

Defined once, in `crates/game/src/benchmark/plan.rs`; `scripts/bench-render.py`'s `COMMON` table is
the same race as flags, and `the_offscreen_bench_flies_the_same_race` holds the two together.

- Single Race, seed 1, the title's default circuit and team, the default speed class, the full AI
  field with the weapons the mode ships with, the title's laps, `[ai] difficulty` at its default.
- The player's craft is flown by the autopilot (`Session::autopilot`, set for the launch and put
  back after).
- Everything the simulation reads is pinned (`plan::pin`) over what the front end had chosen, so a
  player's team, circuit or difficulty never moves the race. Graphics, the HUD and the preset are
  the player's.
- Nothing it does is a record, a ghost or a recording (`result_saved`, the ghost hidden, no
  auto-replay), and the pre-race flyby and the countdown before tick zero are not timed.

**Same race as the offscreen bench, checked.** 240 ticks on every title end on the offscreen
bench's state hash (Pulse `e90e3913bf520931`, Pure `9cd28090d7b085f6`, HD `9f831f9e0e9c728e`, 2048
`c5e93aae56eef808`, Omega `4113d6ecf6caa052`), a menu launch from either row ends on the same hash as
the command line, and a full three-lap Pulse race finishes in the window at tick 6,874 on
`8c5dcddcba9e3618`, the offscreen bench's own. The end is taken on the tick the player crosses the
line, inside the frame loop, not on the frame after, which is what keeps the hash equal.

## What it measures

A frame is recorded when the performance meter records it (`Session::frame`'s `elapsed`, under the
same rule): a frame that carried a load is not a frame time. The first 60 raced frames are left
out of every figure (they pay pipeline and buffer first use), not out of the CSV.

| Figure | Definition |
| --- | --- |
| Average FPS | frames counted over their summed time (`1e6 / mean frame_us`) |
| 1 % low, 0.1 % low | the rate of the slowest 1 % (0.1 %) of frames: 1,000 ms over the **mean** frame time of the slowest `ceil(n x 1 %)` frames, at least one (`stats::low_fps`) |
| Median, P95, P99, worst | nearest-rank percentiles of the frame time, so each is a frame that happened (`stats::percentile`) |
| Over the budget | frames that took more than 1.05 times `1000 / refresh Hz`; the refresh rate is the display's when the loop measured it, else 60 Hz and the result says `assumed` |

The screen also lists the settings that change what a frame costs, the adapter and driver, and the
machine (operating system, architecture, processor).

## The result file

`<data dir>/oag/bench/run-<unix seconds>/<title key>.json` and `.csv` (`pulse`, `pure`, `hd`,
`2048`, `omega`: the names `bench-render.py` files its results under, so two run directories line
up by file name). The JSON is `bench-render`'s format: `title`, `git_hash`, `build`, `adapter`,
`settings`, `mode`, `laps_target`, `tick_cap`, `finished`, `final_tick`, `final_state_hash`,
`frames`, `metrics.frame_us` (`count, min, mean, p50, p95, p99, max`), `per_lap`, the load average
at the start and the end, and `run` (`kind: "in_game"`, the circuit as the loader names it, the
preset, where it started from, the flags). `in_game` carries what is new: the average and the two
lows, the frames over the budget and the budget, the refresh rate and where it came from, the
device and the driver. The CPU and GPU columns of the offscreen bench are not timed through a
window and are absent: the comparison says so.

```sh
just bench-compare data/bench/before ~/.local/share/oag/bench/run-1795576866     # a directory each
python3 scripts/bench-render.py compare a/pulse.json b/pulse.json                # or two files
```

`compare` warns when the two are not the same workload, and now also when one is an in-game result
and the other an offscreen one (`run kind differs`): an in-game frame includes the wait for the
present, so only two in-game results on the same present mode are a fair pair.

## Presets

`Data\XML\BenchmarkPresets_<Title>.toml`, one per title, read through the asset layers
(`oag_source::layers`): the embedded file in `assets/fallback/Data/XML/` is the fallback, and a file
of the same name under a mod's or the player's `overrides/<title>/Data/XML/` replaces it. The shape
is `schemas/v1/benchmark-presets.tosd` (`just check-schemas`):

```toml
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/benchmark-presets.tosd", version = "1.0.0" }

[[preset]]
name = "low"
summary = "Half resolution, blob shadows, no anisotropic filtering"
render_scale = "50"
msaa = "off"
shadows = "blob"
```

A preset names graphics settings by the key the graphics page uses (`render_scale`, `msaa`,
`shadows`, `motion_blur`, `motion_blur_resolution`, `anisotropy`, `model_detail`,
`texture_detail`, `trail_order`, `scenery_sort`, `trail_length`, `weather`); a key outside that
list is refused when the file is read. The run applies them through the graphics page's own
`apply_setting`, in memory: `settings.toml` is not written while a preset is in effect, and the
player's settings are put back when the stats screen is left. `--preset` with a name the title's
file lacks stops the run and says which names there are. The shipped files hold `low`, `medium` and
`high` on every title, **chosen, not measured**; `the_shipped_presets_set_values_the_graphics_page_offers`
checks each value against the page.

## Tests

- `render_bench_ground_truth::the_in_game_benchmark_flies_the_race_the_offscreen_bench_does`:
  `--benchmark --drive` (the real `Session`, no window) ends on the offscreen bench's hash, writes
  the result file and draws the stats screen.
- `render_bench_ground_truth::a_preset_changes_the_picture_and_not_the_race_or_the_players_settings`.
- Unit tests in `crates/game/src/benchmark/` for the lows, the figures, the JSON keys `compare`
  reads, the screen's hit rectangles and layout, the presets, and the command line.
- `scripts/bench-render.py selftest` covers the in-game/offscreen warning.

## Not checked

The Steam Deck and a phone were not run for this page: the code path is the window's on every
platform (the launch file hands `--benchmark` to the same parser), but no result from either is
in the record.

`FE_EXTRAS` outside Pulse: HD's, Omega's and Pure's main menus author no such row
(`crates/raceplay/tests/main_menu_ground_truth.rs` reads each list back), so they keep BENCHMARK
after OPTIONS. 2048's touch grid has an `FE_EXTRAS` tile (manual, credits) that is not wired to
BENCHMARK. The EXTRAS page is ours: the tree drops it on a title whose main menu has no EXTRAS row
(`Definition::build`), and the Pulse and Pure main-menu compare pairs re-render byte-identical
because they draw the disc's own rows.
