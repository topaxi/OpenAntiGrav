# In-game benchmark

One fixed race per title, flown in the window with the player's own settings (or a named preset and the run's own vsync and frame limit),
ending on a stats screen and a result file that `just bench-compare` reads. It is the player-facing
half of the [whole-race render bench](render-bench.md): that one draws offscreen and measures the
renderer; this one draws through the window, so a frame is the interval between two presents under
the player's present mode, and the same figures come out of a desktop, a Steam Deck and a phone.

## Starting it

| From | How | BACK on the stats screen |
| --- | --- | --- |
| The command line | `oag-game --benchmark [title] [--preset NAME] [--benchmark-mods on\|off] [--benchmark-vsync MODE] [--benchmark-frame-limit LIMIT]` | quits |
| The menus (both rows open BENCHMARK OPTIONS first) | Pulse: EXTRAS (its disc's own `FE_EXTRAS` row, last in the disc's order) opens an EXTRAS page of ours holding BENCHMARK. Every other title: BENCHMARK, the row the built-in `oag-extras` mod adds after OPTIONS (`assets/mods/oag-extras/menu.toml`; on a flat main menu it draws inline after OPTIONS) | the page it was fired from |
| The graphics page | RUN BENCHMARK, before ADVANCED (`assets/ui/menu.toml`): change a setting, run, change it again | the graphics page |

## Benchmark options

BENCHMARK and RUN BENCHMARK open a BENCHMARK OPTIONS page (`assets/ui/menu.toml`, page
`benchmark_options`; pad, pointer and touch like every page) before the run, with START BENCHMARK
and BACK. Its rows are **this run's own**: they start from the player's current settings every
time the page opens, are held by the session (`Session::apply_setting`'s `benchmark.` keys) and are
never written, so `settings.toml`, which holds the `[mods]` switches and the `[mod_settings]`
tables, is byte for byte what it was after any run
(`render_bench_ground_truth::the_options_page_reaches_the_run_and_saves_nothing`).

| Row | Values | Default | Reaches the run as |
| --- | --- | --- | --- |
| MODS | off, on | off; the row is left out while no mod of the player's is enabled (`Menu::offer_choice`) | `--benchmark-mods` |
| VSYNC | the display page's: off, on, smooth; left out on the web, whose canvas has one present mode (`settings::web::HIDDEN_ROWS`) | the player's | `--benchmark-vsync`; the surface is reconfigured for the run and again when it ends |
| FRAME LIMIT | the display page's list (greyed under vsync on, as there) | the player's | `--benchmark-frame-limit` |
| PRESET | YOUR SETTINGS, low, medium, high | YOUR SETTINGS | `--preset` |

On the command line the options page is skipped and the flags are the rows; `--benchmark-vsync`
and `--benchmark-frame-limit` are benchmark-scoped (the game has no persisting `--vsync`). On
Android the launch file takes them one argument per line like `--preset`. BACK on the stats screen
returns to the page the BENCHMARK row was on, not the options page; firing the row again reopens
it, and RUN AGAIN repeats the run with the same options.

**MODS OFF is the game as shipped.** `plan::without_mods` clears every door a mod comes in by:
the simulation modules and the mods' effects, shaders and sounds (`find_extensions`), the texture
packs (`find_texture_packs`) and the player's override directory
(`Options::no_overrides`, which mounts only this project's own fallback layer). The mods still
list on the MODS page and the built-in `oag-extras` is not a mod for this purpose (it adds menu
rows, not race content); the official `oag.*` mods are mods and load under MODS ON.

The wingtip contrails' lender disc opens through the same override rule
(`Options::override_bases`), and the rumble library is read out of the race's own archives, so
neither lets an override of the player's in (the rumble override in the test's override directory
answers with mods on and stays silent with them off). The rumble probe in
`crates/game/src/rumble.rs` is a development tool and not a race.

The result's `run` object records `mods` (`"off"` or `[{id, version, hash}]`; `hash` is the
SHA-256 of the mod's simulation module, empty for a mod with none), `vsync_requested`,
`present_mode` (the surface's `wgpu` present mode) and `frame_limit` (as configured, a rate,
`unlimited` or `refresh_rate`), and `command` repeats the flags. `just bench-compare` warns when
two results differ in any of them (`mods differ`, `vsync differs`, `present mode differs`,
`frame limit differs`).

`title` is a disc image or folder like any first argument, or a title's short name (`pulse`,
`pure`, `hd`, `2048`, `omega`) for the first playable disc of that title on the search path. On
Android the launch file takes the same flags, **one argument per line** (`files/oag-args.txt`,
[Android](android.md)): the image path on one line, `--benchmark` on the next, and `--preset` and
its name on two more. The result is in the app's `files/data/oag/bench/` (`adb pull`). `--ticks N`
stops the race at tick N and the result says `finished: false`; it is for a short test run, not
for results (`--ticks` under 272 ends inside the countdown and has no figures).

Back during the run abandons it: Escape, the pad's back button (CIRCLE by default) and the
Android system Back all end it the same way. The player's settings come back, **no result file is
written**, and a menu launch returns to the page it was started from; a run started on the command
line has nothing behind it and ends the game. A benchmark's race is never held, finished or not:
it is not parked behind the menus (`Session::open_menus`) and they are not built as a pause menu
over it (`Session::pausing_race`), with no PHOTO MODE row (`Session::photo_available`). RUN BENCHMARK is not offered over a race parked
behind the menus: it would replace that race.

## Input

**Nothing the player does changes what a benchmark runs, except Back.** The flyby plays its whole
length and the race flies on the autopilot, so two runs on one title do the same thing whoever is
holding the pad. While the run is up (the flyby, the countdown and the race, not the stats screen):

| Input | What it does |
| --- | --- |
| A held CROSS, a click or a tap on the flyby | nothing: the flyby ignores them (`Race::tick_intro_unattended`) |
| START | nothing: no pause, no pause menu, no photo mode |
| SELECT (the camera cycle) | nothing: the camera does not change and `settings.toml` is not written |
| Sticks, triggers, face buttons | nothing: the autopilot replaces the player's snapshot outright (`RaceSim::set_autopilot`), so no pad reaches the craft |
| F11 (fullscreen) | nothing while the run loads or flies: a new output size would move every figure after it (`window::fullscreen_may_change`); the stats screen takes it again |
| Touch on the racing overlay | nothing: the overlay is not drawn or read in a benchmark (`touch::overlay_reads_fingers`) |
| Losing focus | nothing: it neither pauses nor abandons the run (`Session::window_away`) |
| Back | abandons the run, as above |

The music buttons are not in the table: they move the soundtrack, which is not what the run
simulates or draws.

**The flyby.** Pulse's flyby ends by itself after its authored length (17.7 to 27 s), and a
benchmark plays all of it. HD's flyby (the only other one wired) loops its 173 to 188 s tour until a
press (`docs/gameplay/race-intro.md`), so waiting for the end is not possible and a full pass
would put three minutes of unmeasured flyby in front of every run. The run presses for it at a
fixed tick, `plan::SKIP_ONLY_FLYBY_TICKS` (27 s, the longest flyby Pulse's own circuits end on,
so a benchmark's flyby is about as long on every title; **chosen, not measured**). The same
press lands on the same tick every run: an HD result says `flyby_ticks: 1621`. The flyby's
frames carry no world tick and are in no figure (the world has not started), so the choice moves
no result, only how long the run takes before the countdown. `--no-intro` is a launch flag, not a
player's input, and still removes the flyby for a test run (`flyby_ticks: 0`).

The result file's `flyby_ticks` is the flyby's length in ticks, so a skipped flyby shows.

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
  auto-replay), and the pre-race flyby has no tick, so it is not recorded.

**Same race as the offscreen bench, checked.** 240 ticks on every title end on the offscreen
bench's state hash (Pulse `e90e3913bf520931`, Pure `9cd28090d7b085f6`, HD `9f831f9e0e9c728e`, 2048
`c5e93aae56eef808`, Omega `4113d6ecf6caa052`), a menu launch from either row ends on the same hash as
the command line, and a full three-lap Pulse race finishes in the window at tick 6,874 on
`8c5dcddcba9e3618`, the offscreen bench's own. The end is taken on the tick the player crosses the
line, inside the frame loop, not on the frame after, which is what keeps the hash equal.

## What it measures

A frame is recorded when the performance meter records it (`Session::frame`'s `elapsed`, under the
same rule): a frame that carried a load is not a frame time. The frames drawn while the world is
still in its 272-tick start-line countdown (`COUNTDOWN_TICKS`: the grid, the lights, nothing
moving, and the first frames' pipeline and buffer first use) are left out of every figure, the
per-lap rows and `warmup_frames_excluded`'s count, not out of the CSV or `frames`. The offscreen
bench counts them after its first 60 frames, so the two differ by that grid time.

| Figure | Definition |
| --- | --- |
| Average FPS | frames counted over their summed time (`1e6 / mean frame_us`) |
| 1 % low, 0.1 % low | the rate of the slowest 1 % (0.1 %) of frames: 1,000 ms over the **mean** frame time of the slowest `ceil(n x 1 %)` frames, at least one (`stats::low_fps`). Under 1,000 counted frames the 0.1 % low would be the one worst frame, so the screen says `N/A UNDER 1000 FRAMES` and the file has `null` |
| Median, P95, P99, worst | nearest-rank percentiles of the frame time, so each is a frame that happened (`stats::percentile`) |
| Over the budget | frames that took more than 1.05 times the frame budget ("The frame budget" below) |

The screen also lists the settings that change what a frame costs (the vsync mode, the frame limit
and the dynamic-resolution target among them), the adapter and driver, and the machine (operating
system, architecture, processor).

**The frame limit.** A fresh install runs vsync off with a 240 fps limit, so any machine faster
than that reports about 240 fps: the number is the limiter's, not the GPU's. When the average sits
within 10 % of the limit in force (the limit is not in force under vsync on, where the display
paces), the screen says so on its own line and the file has `limiter_binds: true`; set
`frame_limit` to unlimited to measure the machine.

## The frame budget

The verdict, the stutter line and the FRAMES OVER row all judge one budget, the frame rate the
player asked for (`report::Context::budget`; **chosen, not measured**): the longest period, so the
lowest rate, of these three.

| Term | In force when | Rate |
| --- | --- | --- |
| TARGET FPS | it names a rate. `off` names none; `original` (HD's width throttle) names none of its own and paces at the limit or the screen, which the other two terms carry | the rate, `default` being the screen's (60 while none is known) |
| FRAME LIMIT | the loop is held to it: not `unlimited`, and not under vsync on | the rate, `refresh_rate` resolved the same way |
| Screen refresh | always | the display's, reported or measured, else 60 Hz and the result says `assumed` |

With neither a target nor a limit the screen alone judges the run. A target above the refresh does
not count, because the refresh is longer. An assumed 60 Hz still counts: a browser names no rate
but holds frames to its screen, so judging it against a higher target would call every frame late.
The cost is the other way round: a faster screen that names no rate is judged at 60. SCREEN REFRESH
stays a DETAILS row of its own, and the result file names the winning term as `budget_from`
(`target_fps`, `frame_limit` or `refresh`; equal rates are named in that order).

The 1.05 slack is because a loop held by vsync, the limit or dynamic resolution lands within a few
percent of the period on every frame, so the literal period would count half of them.

## Reading the screen

The stats screen is for a player, not an engineer. It leads with a **verdict**, then plain lines,
and keeps the engineer's numbers behind DETAILS (`benchmark::text` holds every word, each through
a string id; `benchmark::screen` draws them):

| Line | Says |
| --- | --- |
| SMOOTH, MOSTLY SMOOTH or CHOPPY, with the average FPS and the FPS it is judged against | the share of counted frames over the frame budget: up to 1 % is SMOOTH (at 60 FPS one late frame every 1.7 seconds), up to 5 % MOSTLY SMOOTH (three a second), more CHOPPY (`report::SMOOTH_SHARE`, `MOSTLY_SMOOTH_SHARE`; **chosen, not measured**) |
| Slowest moments: N FPS (worst 1 in 100 frames) | the 1 % low |
| Stutters: N of M frames missed the T ms frame budget | the frames over the budget |
| Your frame limit (N FPS) capped this result. Turn FRAME LIMIT off in BENCHMARK OPTIONS ... | only when the average sits on the limit in force (`limiter_binds`) |
| Mods: none, or Mods: N loaded | the run's MODS row |

The heading line names the game, the circuit as the track select names it (never the archive's
`16_Track`) and the preset. The race holds its circuit as a path, and the row is the one that
addresses the same file with separators, capitals and a leading slash folded
(`Track::addresses`, `benchmark::circuit_name`): HD's default circuit is spelled
`/data/environments/talons_junction/track.vex` where its definition names `Talons_Junction`, and
Omega's `environments` differs in case the same way, so an exact comparison names no circuit on
either (run on both, 2026-11-27, confidence 95; **ported** to Omega, each with its own ground
truth). The buttons are DETAILS (and SUMMARY to come back), COPY TO CLIPBOARD,
BACK and RUN AGAIN; the pad walks them with left, right, up and down, and a click or a tap presses
one. DETAILS lists the numbers of the table below, the screen refresh and where it came from, the
settings under the menu's own row names and values (read out of the menu tree, so a name edited
there is edited here), then the mods, the device, the GPU with its driver and the build version.
The GPU line says what the platform names: it is left out where neither adapter nor driver has a
name (a browser), and a driver that repeats the adapter's name (Mali) is not said twice.

**COPY TO CLIPBOARD** puts the same text on the clipboard in the same order: the game, circuit and
preset, the plain lines, a blank line, then DETAILS (`text::copy_text`, pinned line for line by
`the_copied_text_is_pinned_line_for_line`). The button shows COPIED for two seconds, or COULD NOT
COPY, once the platform has answered: until then it keeps its own name. It is on the screen where
the platform can write a clipboard and left off where it cannot:

| Platform | Write | How |
| --- | --- | --- |
| Linux, Windows, macOS | yes | `arboard` (`app::clipboard::write`); the clipboard object is kept for the life of the process because X11 and Wayland serve the text from the process that owns it |
| Android | yes | `oag_mobile::clipboard`, the activity's `ClipboardManager` through JNI, on any thread |
| Web | yes | `navigator.clipboard.writeText` through `js_sys`, which needs a secure page; the promise is awaited, and a refusal shows COULD NOT COPY and logs a warning |
| iOS | no | no JNI-like shim; the button is left off (`clipboard::CAN_WRITE`) |

Run on Linux under Xvfb, the clipboard held the text above after the press (`xclip -o`).

## Fitting the stats screen

The screen, like the finished race's results table (`crate::scoreboard`, which shares its overlay),
is laid out in the 480x272 grid and letterboxed into the window's aspect rectangle, so it takes the
same share of the picture on every screen. **Pixel size and density do not enter it**: a phone's
density changes nothing here.

**What overflowed (2026-11-26).** The text is set in the title's own HUD caption face, and each face
is authored in its title's grid:

| Title | Grid | Line height | Mean capital advance |
| --- | --- | --- | --- |
| Pulse (PSP and PS2) | 480x272, 640x448 | 10 | 6.3, 7.4 |
| Pure | 480x272 | 12 | 7.0 |
| HD, Omega | 1920x1080 | 34 | 21 |
| 2048 (its value face; it names no caption face) | 960x544 | 34 | 25 |

Drawn as they stand into the 480x272 grid, the last two rows are about three times the panel's text:
on HD, Omega and 2048 the button labels ran over each other and off the panel and the summary ran
into the saved path, on a desktop at 1440x816 and on a Galaxy S21 at 2400x1080 alike. Pulse and Pure
fitted, which is why desktop checks on Pulse did not show it.

**The rule**, the same on every platform:

- `scoreboard::panel_face` brings the face into the grid: a line taller than 12 units (Pure's, the
  tallest of the PSP-grid faces) is scaled down to 12, a smaller one is left alone. The results
  table draws through the same overlay and had the same defect.
- The stats screen fits every string to its room by measured width: lines wrap by width, the body
  shrinks until it ends above the saved path, and a heading, a settings row, the saved path or a
  button label that is still too wide (a long translation) shrinks on its own. The buttons'
  rectangles, which a click or a tap is tested against, do not move.

`screen::misfits` is the check: a string outside the panel, a label outside its button, text over
the button rows, or two strings drawn over each other. The results table was checked by
`--race --autopilot --screenshot` captures: HD before (columns running into each other, four rows
off the screen) and after (all eight rows in the panel), and Omega after, whose finished race keeps
this table because it has no EndRace screens wired ([endrace-screens.md](../ui/endrace-screens.md)).

## The result file

`<data dir>/oag/bench/run-<unix seconds>/<title key>.json` and `.csv` (`pulse`, `pure`, `hd`,
`2048`, `omega`: the names `bench-render.py` files its results under, so two run directories line
up by file name). The JSON is `bench-render`'s format: `title`, `git_hash`, `build`, `adapter`,
`settings`, `mode`, `laps_target`, `tick_cap`, `finished`, `final_tick`, `final_state_hash`,
`frames`, `metrics.frame_us` (`count, min, mean, p50, p95, p99, max`), `per_lap`, the load average
at the start and the end, and `run` (`kind: "in_game"`, the circuit as the loader names it, the
preset, where it started from, the flags). `in_game` carries what is new: the average and the two
lows (`low_0_1pct_fps` is `null` under 1,000 counted frames), the frames over the budget and the
budget, the refresh rate and where it came from, the frame limit in force (`frame_limit_hz`) and
whether the average sits on it (`limiter_binds`), what set the budget (`budget_from`), the device and the driver. The CPU and GPU columns of the offscreen bench are not timed through a
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
- `render_bench_ground_truth::no_input_changes_a_pulse_benchmark` and `..._an_hd_benchmark`: a run
  with every input spent (held CROSS, click, SELECT, START, a stick, focus loss, a key) has the
  untouched run's flyby length, final tick, state hash, frame count and `settings.toml`. The HD
  one also reads the stats heading, circuit name included, out of the log;
  `hd_boot_ground_truth` and `omega_track_model_ground_truth` hold the name lookup against each
  disc's own track list.
  `a_benchmark_with_its_mods_off_loads_no_extension_and_no_override`,
  `the_options_page_reaches_the_run_and_saves_nothing` and
  `the_mods_row_is_left_out_while_no_mod_is_enabled`: BENCHMARK OPTIONS. Unit tests hold the page in
  every title's menu tree (`every_titles_tree_holds_the_benchmark_options_page`) and the PRESET,
  VSYNC and FRAME LIMIT rows to the shipped preset names and the display page's values.
  The frame budget: `benchmark::report` replays the S24 run, a 120 Hz run with nothing asked
  for, a vsynced 60 Hz run and a table of which term wins; `the_in_game_benchmark_flies_the_race_the_offscreen_bench_does`
  checks that a frame limit under vsync on is not a term; `the_options_page_reaches_the_run_and_saves_nothing`
  checks that the player's TARGET FPS reaches the result.
  `the_stats_screen_swaps_to_details_and_answers_a_copy`: DETAILS and COPY through the real
  `Session`, by screenshots. The unit tests in `benchmark::text` hold the verdict thresholds, the
  frame-limit line, the copied text and that no internal name (`_Track`, `track.vex`) is shown.
  `back_abandons_a_benchmark_and_writes_nothing`: Back by the pad and the system Back from the EXTRAS page, with nothing left parked behind the menus and the main menu, not the pause menu, opened; Escape from the command line.
- Unit tests in `crates/game/src/benchmark/` for the lows, the figures, the JSON keys `compare`
  reads, the screen's hit rectangles and layout, the presets, and the command line.
  `every_title_face_fits_both_pages_even_in_a_long_language` runs `screen::misfits` over both pages
  with each face's measured size and every string doubled.
- `benchmark_stats_ground_truth` (one test per title): both pages drawn at 2340x1080 through the
  title's own caption face and the window's overlay, with no misfit and no lit pixel outside the
  panel.
- `scripts/bench-render.py selftest` covers the in-game/offscreen warning.

## Run on a phone and in a browser

2026-11-26, Wipeout Pulse (PSP EU), Talon's Junction White, driven by touch from the EXTRAS page.

| Target | Run | COPY read back |
| --- | --- | --- |
| Galaxy S21 (Mali-G78, Android 14, `dist` APK, `--no-audio --ticks 600`) | every option row answered a tap (MODS on, VSYNC off, FRAME LIMIT 360, PRESET low) and the result file recorded those; SUMMARY, DETAILS, BACK and RUN AGAIN answered, the screen survived a switch to another app | pasted into a page served over `adb reverse`, which sent the bytes back: the whole summary and DETAILS text |
| Headless Chromium, 844x390 at ratio 3, `just web` | full three-lap runs by touch (FRAME LIMIT, PRESET, START, COPY, DETAILS, BACK); by mouse at 960x544 the options rows and DETAILS, SUMMARY, COPY, RUN AGAIN and BACK | `navigator.clipboard.readText` after granting the read, the write having had no permission; with `writeText` replaced by a rejecting one the button said COULD NOT COPY |

The walk is `scripts/web-screenshot.py` with `--tap`, `--grant clipboard-read@T` and
`--eval 'navigator.clipboard.readText()@T'`. Grant after the COPY press: Chromium denies the
write to a page that has been granted the read alone. On the S21 `adb shell cmd clipboard` has no
implementation, hence the paste.

**The refresh rate.** Both runs above said 60 Hz, `assumed` in DETAILS, because winit reports
none on Android or the web; the S21's panel ran at 120 Hz during its run. Android now reads the
display itself ([android.md](android.md), "The display's refresh rate"), so a phone's SCREEN
REFRESH row and the screen's term of the frame budget are its own rate. The web still reports none
and says 60 Hz, `assumed`.

**The stats screen's fit.** 2026-11-26, the same S21, `--benchmark` from the launch file, screenshots
of both pages (`adb exec-out screencap`): before the fix HD's button labels covered each other and
ran off the panel and its summary ran into the saved path; after it HD and Pulse fit both pages, and
DETAILS, SUMMARY and BACK answer a tap. Headless on the desktop at 2340x1080, both pages of all six
titles fit.

## First phone result

Reported by the maintainer from a Galaxy S24 (Xclipse 940), build `aded31f`, Wipeout HD: the
stats screen's own figures; no result file of the run is in the record.

| Setting or figure | Value |
| --- | --- |
| SCREEN REFRESH | 120 Hz |
| VSYNC, FRAME LIMIT | off, unlimited |
| TARGET FPS, RENDER SCALE | 60, 100 |
| Average FPS, 1 % low | 81.5, 50.4 |
| Frame time median, P95, P99, worst | 11.82, 16.90, 18.27, 25.54 ms |
| FRAMES OVER 8.3 MS | 8476 OF 9165 |
| Verdict | "CHOPPY: 81 FPS ON AVERAGE, ON A 120 HZ SCREEN" |

That build judged every frame against the screen's 8.3 ms period, so a run that mostly held the 60
the player asked for read CHOPPY. Against the budget above (16.7 ms, late past 17.5 ms) the same
numbers give MOSTLY SMOOTH, whatever lay between the percentiles: P95 (16.90 ms) is under 17.5 ms,
so at most 458 of the 9165 frames are late (4.997 %), and P99 (18.27 ms) is over it, so at least 92
are (1.004 %). This is computed from the reported figures
(`benchmark::report::tests::a_run_that_asked_for_60_is_not_judged_against_a_120_hz_screen`
replays them); the phone has not been re-run on the new rule.

## Not checked

The Steam Deck was not run for this page: the code path is the window's on every platform (the
launch file hands `--benchmark` to the same parser), but no result from it is in the record.
Safari and Firefox were not tried with COPY.

`FE_EXTRAS` outside Pulse: HD's, Omega's and Pure's main menus author no such row
(`crates/raceplay/tests/main_menu_ground_truth.rs` reads each list back), so they keep BENCHMARK
after OPTIONS. 2048's touch grid has an `FE_EXTRAS` tile (manual, credits) that is not wired to
BENCHMARK. The EXTRAS page is ours: the tree drops it on a title whose main menu has no EXTRAS row
(`Definition::build`), and the Pulse and Pure main-menu compare pairs re-render byte-identical
because they draw the disc's own rows.
