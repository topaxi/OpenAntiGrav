# Compare page: the original and ours, side by side, for a human to judge

`just compare-page` builds one static web page of review pairs: a screenshot of
the original running in its emulator and one of this engine, same ship, same
circuit, same state. Views per pair: a **split** slider (drag the handle, works by
touch), an **onion skin** opacity slider, **side by side**, and a precomputed
**difference** (twice the absolute difference). Keys: up/down or j/k change pair,
1-4 change view, left/right move the split. A pair that has an original, a
"before" and an "ours" offers each combination.

```sh
just compare-page                 # builds data/compare/index.html from every manifest in data/reference/compare-page/
just compare-page --serve         # the same, then serves it on all interfaces, port 8097
just compare-page data/reference/compare-page/live.toml --out /tmp/page --serve --port 8123
```

`--serve` prints every LAN address, so the desktop, a Steam Deck or a phone opens
`http://<address>:8097/`. The page and its images stay under `data/` (gitignored):
**nothing captured or rendered is ever committed.** Needs ImageMagick (`magick`),
the same as [frame comparison](frame-compare.md).

## The manifest

TOML, one `[page]` table (title, intro) and an `[[entry]]` per pair; paths are
relative to the manifest. A directory argument reads every `*.toml` in it, so a
lane drops its own file next to the others. This is a developer file under `data/`
that the game never loads, so it carries no `[toml-schema]` header
([ADR-0064](../architecture/adr/0064-project-toml-files-declare-a-toml-schema.md) is for
files a player or modder edits); a wrong shape is refused by name instead
(missing `id`, an image that does not exist, fewer than two images, an unknown key).

| Key | Meaning |
| --- | --- |
| `id`, `game`, `title` | required; `game` is the title and region (`Wipeout HD (EU)`) and groups the list |
| `platform` | the console the original ran on (`PS3`, `PSP`, `PS4`, `Vita`); required to publish, a warning otherwise (see Credits) |
| `original`, `before`, `ours` | image paths; at least two |
| `track`, `craft`, `state`, `setting`, `environment` | the caption: circuit, ship, what moment and how the pose was matched, emulator and size, and the `[entry.environment]` table `scripts/compare_env.py manifest` writes from `pair.json` (profile, output size, every setting that changes the picture, and the build that drew ours: the commit `oag-game --version` reports, else the tree's `HEAD` at render time marked `(tree at render)` and `+dirty` when tracked files had changes) |
| `note` | free text, shown under the caption (what is not matched) |
| `not_captured` | why there is no original; shown as a warning |

Build time normalises each pair with ImageMagick: a black letterbox is trimmed from
the original, and every image is brought to one size. The aspect is never stretched
(a different aspect is fitted and padded, so a mismatch stays visible). **An entry
with no `original` is shown as before and after, and `not_captured` says why:** one
of ours is never put in the original slot.

`just compare-seed` rebuilds `seed.toml` from the HD lanes' reference sheets
(`scripts/compare-seed.py` splits the 3x3 composites into one entry per column).
`live.toml` is hand-written from the captures below.

The page lists titles in release order (Pure, Pulse, HD/Fury, 2048, Omega; `compare-page.py`'s `GENERATIONS`), a
title's handheld original before its port (PSP, PS2, PS3, Vita, PS4), and keeps the manifest order inside one title.

## Credits and the rights-holder statement

ADR-0074 requires both on every published page, so every page carries them:

- **A credit under each image** on view: `<game> - <platform> - Original game` or
  `... - OpenAntiGrav engine` (`before` reads `OpenAntiGrav engine, before`), built from
  the entry's `game` and `platform`; in side by side it is the figure caption. An entry
  with no `platform` is **warned about by file and entry** in a local build and shows
  "platform not recorded"; `--strict`, and `--publish` always, **refuse the build naming
  the manifest** instead of guessing.
- **The rights-holder statement** in the page footer. Its text is read at build time from
  `docs/overview/legal.md`, "Screenshots and rights holders", the one source, and the
  build refuses if that no longer equals the statement quoted in ADR-0074 word for word
  (markdown emphasis and line breaks ignored). `python3 scripts/compare-page.py
  --check-statement` runs only that check; `just check-docs` runs it, so the gate fails on
  drift. The ADR is immutable, so a mismatch is fixed in `legal.md`.

## Autopilot timer in a race state (2026-10-21)

The rail and the hunts arm the player's Autopilot pickup with a huge timer (PSP `record+0x148 = 1.0e9`, HD slot `+0x158 = 10000`)
so a state keeps flying, and the HUD **prints that timer**: the PSP race and both Zone pairs showed `1000000000.00`, the HD race pair
`9945.79`. The pair scripts now put the pickup's authored timer back after the load (`<Weapon type="Autopilot"><Stats time>`,
5 s on Pulse and HD): `psp-pulse-pair.py --field|--zone` writes it with the CPU stopped (`restore_autopilot_timer`, at the frame's own moment, after any `--target-clock`/`--run-ticks`), `rpcs3-hd-pair.py --field` through `rpcs3-drive.py capture --autopilot-timer 5` (the game then runs
0.3 s so the HUD redraws, the frame, pose, field and HUD record are from that moment). The HUD shows a pickup's own countdown
(`5.00`, `4.95`), the manifest carries a caveat, `pair.json` records the timer before and after, and a pair whose timer at the stop
is still above 600 s is refused. A run to let the pickup expire instead was tried on PSP and rejected: after 0.8 s the craft had
left its line and the pack was gone. (The HD frames carried RPCS3's "Press and hold the START button to
resume" toast until 2026-10-21, `hd-capture-profile`: the profile now turns the native interface off.) A PSP state loads a different number of emulated frames before the stop on
a slower instance (a Zone 8 frame moved from zone second 1.5 to 2.1, where its HUD is mid-fade).

### The pickup row of a race shot (2026-10-26, `talons-pickup-row`)

The Autopilot's icon with its `5.00` under it (the hexagon at the top, about x 210 to 270, y 5 to 62 at 480x272) was not drawn in
ours. `psp-pulse-pair.py --field` now records the player's held slot beside the timer (`original.held_pickup`: `+0x1bc` read **-1**,
empty; `+0x1b8` read `0x800`, bit `0x1000` clear; `+0x148` the timer) and passes `--hud-autopilot SECONDS` when the timer is above zero
and below the sane bound. The frame is the evidence for what draws the row: held empty, no `0x1000`, timer 5.0, and the icon and `5.00`
are in `theirs.png`, so the row follows the timer, as ours' readout does (`Race::sub_weapon` and the icon fallback).
**`--give` was not the lever:** it acts inside a tick (`race_capture/tick.rs`) and a race shot runs none, and a held pickup shows no countdown until it
runs. `--hud-autopilot` (a `CaptureHud` field, like `--hud-boost-phase`) draws the icon and countdown on the readout alone and leaves the
race's own `autopilot_timer` at 0, so the craft is not handed to the driver and nothing in the simulation moves. `compare-rerender-ours.py`
adds it to a stored Pulse PSP race pair from `autopilot_timer_at_stop_s` when the pair's provenance says the Autopilot was armed (the stored
pair has no `held_pickup`; a fresh capture of the same state at `--target-clock 38.4082` gave a `theirs.png` bit-identical to the stored one and the
slot values above).

| `pulse-psp-race/talons-alongside` | before | after |
| --- | --- | --- |
| pickup box (205, 0, 76x64): RMSE against `theirs.png` | 0.208 | 0.122 |
| pixels that changed in ours | none outside the box; the row is 60x53 at (210, 8) | |
| whole frame RMSE | 0.0486 | 0.0361 |

Open: the original's hexagon is translucent (the scene shows through it) where ours' is a flat green; the HD race pair (`hd-race/talons-pack`) and the
Zone pairs are unchanged (the flag is Pulse PSP's, from `restore_autopilot_timer`'s own pair types; the Zone pair's frame
shows the same row and has not been given it).

## Environment parity: the profile and the checks (2026-10-20)

**The set from before 2026-10-20 is archived** (the maintainer's request: start fresh, make the original's and
our environment as close as possible *before* anything renders). The old pairs were taken at whatever size the
emulator window had (PPSSPP 960x544, RPCS3 1760x990 with black bars, PCSX2 512 lines), with the original's race
clock running on from the state and ours at its own tick 300, so the GO board, the record panel and the HUD clock
differed by construction. None of them is a reference now.

`scripts/compare-environment.toml` is the one **per-title environment profile**; `scripts/compare_env.py` reads
it. A pair script builds a `compare_env.Pair`, records what each side actually ran with, and `Pair.finish` writes
`pair.json` and **refuses** (exit 3, every mismatch named) when a check fails. `python3 scripts/compare_env.py
manifest OUT.toml DIR...` turns the non-refused `pair.json` files into a page manifest whose `[entry.environment]`
table the page shows under each pair as a definition list (2026-10-21).

The fields come from what the pair **recorded**, never from the live profile: profile and its hash, output, the original's
emulator and renderer, ours' settings as read back, the three overlap metrics, build and binary, and two times. Each
metric label carries an `<abbr title>` (`compare_env.METRICS`): **RMSE** is the average per-pixel colour error, 0 is
identical, lower is better; **edge correlation** compares where edges are, 1 is identical, higher is better, and shows
whether shapes line up independent of colour; **mean grey difference** is the average brightness difference on a 0-255
scale, lower is better. Each metric's value is coloured green (close), amber (partly) or red (off) by
`compare_env.METRIC_BANDS`, **chosen, not measured** (2026-10-21, against the spread of the 42 pairs then): RMSE green
at or under 0.06 and amber at or under 0.12; edge correlation green at or over 0.40 and amber at or over 0.25; mean grey
difference green at or under 5 and amber at or under 12; the value's tooltip names its band. `original.captured_at` and `ours.rendered_at` (ISO 8601, local offset) are stamped by `Pair.finish`
from the clock when the pair is made; **there is no fallback to file times**, so a pair made before they existed shows
none until it is re-made. A pair needs `meta` (`id`, `title`) in its `pair.json`: `manifest` refuses one without, so
the page never shows a directory name. `manifest` no longer rewrites `pair.json`. `scripts/check-compare-environment.py`
(in `just check-docs`) proves the fields round-trip.

| Platform | Output | Original's emulator settings (in the profile) | Ours |
| --- | --- | --- | --- |
| Pulse PSP (and Pure PSP, same values) | 480x272, the PSP's own | PPSSPP v1.20.4 software renderer, `InternalResolution = 1`, `AnisotropyLevel = 0`, no post shader | `--size 480x272`, aspect `psp` |
| HD/Fury | 1920x1080 | RPCS3 Vulkan, `Resolution 1920x1080`, `Resolution Scale 100` (2026-10-21: 150 corrupts the exposure readback, see `rendering/hd-track-tone.md`), `Use native user interface` off and both compile hints off (no pause toast, no spinner in a frame), `Output Scaling Mode Nearest`, `MSAA Disabled`, anisotropic override 0, `Write Color Buffers: true` in `config.yml` **and** the game's custom config | `--size 1920x1080`, aspect `wide` |
| Pulse PS2 | **1280x720, a chosen pick** (the maintainer asked for 720p; the game's own `Aspect Ratio` option is on `16:9` in the state and PCSX2 shows it at 16:9, so 1280x720 is the whole frame; adjustable in the profile) | PCSX2 OpenGL hardware renderer (the software renderer needs a Vulkan present queue Xvfb does not give), `upscale_multiplier 2`, `ScreenshotSize 1` (internal, aspect corrected), `AspectRatio 16:9` (1820x1024), then ONE Lanczos resize to 1280x720 | `--size 1280x720`, aspect **`wide`** (it was `ps2`, which is 4:3 and pillarboxes ours inside the 16:9 frame; 2026-10-21), rendered natively |
| Omega | 1920x1080 | shadPS4 0.20, full screen at a 1920x1080 internal resolution, RCAS off, `readbacks_mode 1` and `readback_linear_images_enabled true` (without them the 3D scene is black) | `--size 1920x1080`, aspect `wide` |
| 2048 | 960x544, the Vita's own | Vita3K v0.2.1 4095-84184a36, Vulkan, `resolution-multiplier: 1` (the game window is exactly 960x544), no frame filter | `--size 960x544`, aspect `psp` |

**Ours is the same on every platform** (`[ours_defaults]`): every render setting with an `original` value is set
to it (shadows `original`, model and texture detail `original`, trail order `original`), upscaler, FSR, SMAA, MSAA, motion blur and screen
filter off, render scale 100, anisotropic filtering off (no original forces it), the boost fov kick off (our own
effect), fov 100 %, an ungraded capture at exactly `--size` (no `--presented`), the HUD on (the original cannot
switch it off), and the camera is the original's own pose and fov. The profile is written into the pair's private
`settings.toml` **and** passed as flags where a flag exists, and the file the run rewrites is **read back** and
compared with the profile (`read_back`).

**State parity, recorded in `pair.json` on both sides:** circuit and layout file (our own start on each layout
against the original's grid pose, `--layout auto`; refused beyond 25 units or 5 degrees), game mode and speed
class (declared by the state's file name or sidecar; **not read from RAM**), team, **race clock** (below), ship
position and heading **at the rendered tick** (read from a second, `--trace-out` run of the same command, within
0.5 units and 1 degree; with `--pose-hold`, below, this is an import check of the basis, convention and pose, not a
measurement of the simulation, which `scripts/compare-resting-yaw.py` makes), and the field of view. The
binary that rendered ours is recorded by hash.

**The race clock** is the state difference that mattered most: the original's HUD clock runs on from the state
(PPSSPP 40 s after GO on the grid states) and its GO board fades and its record panel counts down with it, so ours
runs `GO_TICK` ticks (our countdown, `oag_race::COUNTDOWN_TICKS`, 272: every pair script and `compare-resting-yaw.py` reads it from the source with `compare_env.go_tick()` since 2026-10-21, `rerender-all`, where HD's, PS2's and the resting-yaw script's own copies still said 276; ours has one countdown for every title, so there is one number, and a platform table may carry a `go_tick` of its own once a title's is measured to differ) plus the original's clock times 60 (`--ours-ticks`). The original's clock
is read out of its RAM: PPSSPP f32 seconds at `0x08B344B0` (the CPU is stopped straight after the load so it stands
still), HD f32 at craft `+0x7014` (read in the same pause as the pose), PS2 f32 at EE `0x70EE9C` of the state file
(the HUD reads it **less 4.25 s**, not 0.19: see "The presented frame" below). With it the GO board, the record panel and the HUD clock match on the Pulse PSP and HD
test pairs. **A craft resting after GO turns, in the original as in ours** (bank-to-yaw; ours unposed lands within 0.101
degrees of the original's heading on all 24 Pulse PSP pairs and 0.694 on all 13 HD pairs, see
[grid state](../physics/grid-state.md#a-resting-craft-after-go-turns-in-the-original-too-2026-10-20-resting-yaw)).
Posing ours on tick 0 at the original's already-turned pose and then simulating the clock again turned it twice, so
the pair scripts pass `--pose-hold` (2026-10-20): the countdown and the clock run as before (GO board, record panel,
HUD clock, exhaust), and the craft is put back on the fed pose for the frame and the last trace row (`pair.json`
`ours.pose_hold`). It replaced a heading-correction loop (`ours.yaw_corrections`, `ours.heading_corrections` in a
pair made before it): Pulse PSP Talon's Junction White, refused at 0.777 units, passes at 0.000 units and 0.000
degrees (edge correlation 0.34 before, 0.36 after); HD Talon's after `Restart Race` at 17.1 s is 0.000 units and
0.088 degrees in one run, against 4.085 degrees without the hold (edge correlation 0.24 on the old pair, 0.25).
`--ticks 0` with an imposed `--hud-state`, the race shots' route, was tried for the grid pairs and dropped: the
trackside GO board renders dark and the exhaust glow is missing at tick 0.

**The camera view and its fov (Pulse PSP, 2026-10-21, `pulse-outliers`).** The pair script no longer assumes the close chase
view at 60 degrees. It classifies the view by the camera's distance from the craft in the same trace row: **11.64 units** is
`<ExternalCameraClose>` (`--camera-view close`), **3.0 units at the craft's own height** is `<InternalCamera>` (`--camera-view
internal`, the only view that hides the hull), and any other distance **refuses the pair** (`pair.json` `original.camera_view`,
`ours.camera_view`). The fov is the view's authored fov plus `0.075 * dot(fwd, vel)`: **60** for the close view, **65** for the
internal one (`<InternalCamera fov>` in the Assegai file our default team reads; the load report prints it, and the RAM check above refuses a ship that authors another), checked against `g_camera_fov_degrees`
(`0x08B34310`) read at the clock's stop (`original.fov_ram_at_stop`, within 1 degree). Moa Therma Black's state was saved in the
cockpit view: the old pair drew ours in the chase view at 60 degrees against an original with no hull, mean grey difference
**50.7**, and **9.3** with the view and fov matched (an A/B on the same state, `--camera-fov 65` alone gives 10.4).

**Moa Therma Black was re-made (2026-10-21, `psp-moa-black-recapture`): a state's camera view and clock are part of the state, and the
grid states do not agree on either.** `time-trial-moa-therma-black-venom-grid.ppst` was saved with the camera in the **cockpit view**
(3.0 units from the craft, the HUD clock at 11.5 s) while the other 23 grid states sit in the chase view (11.64 units). The craft was
not wedged: its pose `(-385.54, 4.015, 126.07)` and speed 0.018 are the grid's, and a fresh `psp-drive.py menu --unlock --track-down 3
--any-track` from an empty profile reproduces that pose to 0.01 units. The cockpit view hides the hull, so the original shows only the
bright streak and ours (which copies the pose) drew wings around the camera. *Why* that walk ended in the cockpit view is not
established (a view-toggle press during the walk is the candidate); the fresh walk came up in the chase view at HUD 35.0 s. A re-make
is `time-trial-moa-therma-black-venom-grid-r2.ppst` (same recipe, `psp-state.py save 1 --keep`). Check a new state before pairing it:
`original.camera_view.view` must read `close` (the pair script records it), and look at `theirs.png`. The state's clock is whatever
the walk ended at (11.5, 18.6, 27.2, 35.0, 41.2 s across the grid states): compare ours at the original's own clock, which the pair
script already does. The re-made pair: mean grey difference 0.042 RMSE (the old pair 0.179), `theirs.png` bit-identical across two
captures of the state (`--target-clock 35.0346` on the second).

**The world meshes' animation clock (Pulse PSP, same day).** Scrolling surfaces, flip-book signs and the neon strips run on a
clock the original feeds every world mesh (`Mesh_UpdateTextureTransforms`, `0x0890e160`, `mesh+0x40`) that is **not the HUD
clock**: it stands at the HUD clock plus a constant per state. Read live by `scripts/psp-world-anim-clock.py` and by the pair
script (the most common non-zero `mesh+0x40` less the clock at the same stop; `pair.json` `original.world_clock_offset_s`,
with the hit counts), over the 24 Pulse PSP states the offset is **13.6 to 24.3 s** (Metropia Black 13.64 to Moa Therma White
24.35; `scripts/psp-world-anim-clock.py` on one state: 16.205 s on Fort Gale Black, identical across two loads). It differs per
state, so it is not a fixed epoch; **inferred, not measured**: it is the time since the circuit loaded (the intro flyby, which a
press skips, plus the 276-tick countdown), which is what makes it vary. Ours ran the animation on `tick / 60`, which already
contains the 4.6 s countdown, so the pair passes `--world-clock-offset` = offset minus `GO_TICK / 60` (`ours.world_clock_offset_s`).
The flag shifts the world meshes' clock only; unlike `--anim-seconds` the GO board and the HUD keep their own. Mean grey
difference over the 24 pairs **9.24 to 6.67** (every pair lower, Fort Gale Black 23.8 to 21.7 and the sign text now reads the
original's; table in the handover thread). Pulse PS2: the camera is now the original's own (2026-10-21, `ps2-widescreen-pairs`: eye, axes and fov
read from the camera object in the state's EE RAM, fed as `cam_*` columns and `--camera-fov`); `pcsx2-pulse-pair.py` still reads no world clock; the one PS2 pair is a chase view.

## A deterministic stop, and the posed craft's lean and boost (Pulse PSP, 2026-10-21, `pose-lean-boost`)

- **The stop.** `psp-state.py load --stop-at 0x08849618` (what `psp-pulse-pair.py` passes) arms a breakpoint on `Ship_UpdateCraft`
  with the pause menu open, presses Load, runs on and stops at its first hit: the state's own first frame (race clock
  37.3578 on `race-talons-venom-alongside.ppst`, the same bits in 3 of 3 loads). The old load, `cpu.stepping`, 6 s toast wait landed
  0.6 s of race clock past it and differently each run. `--target-clock T` then runs the stopped original to the first player
  tick at or past `T` in **one** resume (a breakpoint conditional on the player's craft and on the clock's float bits;
  stepping tick by tick with a stop per hit gave clocks 11 ulp apart in 1 of 3 runs, one resume 4 of 4). Two consecutive
  re-makes of `talons-alongside` at `--target-clock 38.4082` have a bit-identical `theirs.png` (0 grey difference), and
  re-makes of `talons-black` and `fort-gale-white` at their stored clocks are bit-identical to `data/reference` (0.00
  grey rise). `paused()` reads the debugger's own `paused` flag first (true with the menu open) and the cycle counter only as the fallback.
  The `rerender-cold` lane's "could not open the pause menu" did not reproduce on a fresh instance with the OpenGL backend
  (menu open: `paused` true, ticks still; load 2.4 s); the cause there is not known, only that the flag no longer depends on it.
- **A re-make without `--target-clock` lands on the state's own first frame** (not the old late freeze), `--run-ticks` counts
  from that frame, and `compare-pulse-psp-batch.py` passes each stored pair's `race_clock_s` (`--match`, default `data/reference/compare-page/pulse-psp`; `--match none` for the
  first frame). To reproduce a stored pair pass `--target-clock <its race_clock_s>`. The Autopilot timer is written **after** the run to that moment, so the
  HUD prints the pickup's own `5.00` at the frame's stop. `talons-black` through the batch: theirs.png and overlap bit-identical to the stored pair.
- **The lean.** `--pose-lean` (needs `--pose-from`) puts the pose row's `steer_lean` and `roll_phase` on ship 0 after the ticks:
  the drawn lean (`entity+0x854`, the hull turns by `steer_lean * 0.5` about its nose) is written by the original's per-tick
  update and a posed capture runs none, so ours was drawn flat. The follower behind it (`entity+0x858`, read by the HUD
  sway) has no trace column and is not carried. A trace without the columns is refused. Ours' `--trace-out` now writes both columns.
- **The boost.** `--pose-boost AGE` with `--field-warmup` arms the boost inside the warm-up instead of laying a straight
  trail (`Race::warm_up_field(.., boost_age)`); the pair reads the flare's `boost_timer` at the pose's stop and checks ours'
  row at the rendered tick (lean and roll phase equal, boost timer within a tick). `talons-boost`: `steer_lean` 0.3202,
  `boost_timer` 0.349067; `talons-alongside`: `steer_lean` 1.0161, no boost.
- **Re-make commands** (own Xvfb and PPSSPP, `PPSSPP_EXTRA_INI` from `compare_env.py ppsspp-ini pulse-psp`): `psp-pulse-pair.py --field --layout forward
  --team Assegai --track 16_Track --state data/saves/pulse-psp/race-talons-venom-alongside.ppst --target-clock 38.4082` and `... --target-clock 39.3429`.

## The presented frame lags the stop, per platform (2026-10-21, `rerender-all`)

A screenshot is the last PRESENTED frame, which is a little older than the RAM read next to it. Measured per platform, and what each pair script does about it:

- **Pulse PSP: one tick.** `scripts/psp-presented-lag.py` loads the race state (Autopilot flying at about 163 units/s, the camera moving several pixels a tick), records `psp-trace.py --camera` rows with a screenshot at every stop and
  renders ours at each of the five rows before a shot (`--pose-from trace.csv --pose-tick k`); the row with the smallest mean grey difference to the shot is `i - 1` on **all seven shots** (21.3 against 26.6 and up for every other row;
  the sharp minimum is what makes it a measurement). The older "about two frames" was a fire-and-look estimate. `psp-pulse-pair.py` reads the pose, camera, field and HUD at stop 0 and takes the screenshot at stop
  `PRESENTED_LAG = 1` (`--shot-every 1`), recorded in `pair.json` `original.presented_lag_ticks`. Not measured: a lag other than 1 on a state that loads in a different phase (all runs were one state).
- **The camera and the craft read at one stop are not one frame** (`psp-presented-lag.py --camera-skews=-1,0,1,2 --first 4`, race state, shots 4 to 7): at every shot the three best combinations all use **the camera of row `i - 1`** (mean grey 20.1 to 22.8
  against 30 and up for any other camera row), with the craft of row `i - 2` (20.1), `i - 1` (22.6) or `i` (22.8): the camera is firmly the lag-1 row, the craft's row is not resolved by a whole-frame score (a hull-only crop is dominated by the original's plume).
  So the pair keeps one row for both; the race pair's larger-than-ours player hull (an older craft against the camera would draw it larger) stays open, with this as its evidence. The lag is 1 at every stop from stop 1 (shots 1 to 8 on a second load).
- **The HUD text is one tick older again than the picture.** `Hud_Update` reads `PLAYER_HUD` before this tick's `PlayerStatus_Update`, so the frame's speed text is the block as the previous tick left it. Live, 3 of 3 runs: the frame
  read 390, 383 and 377 km/h, the block read at the CPU stop one tick before the pose's stop (the freeze after the state load) held 390.66, 383.24 and 377.20, and body `|velocity| * 3.6` at the pose's stop read 392.08, 384.52 and 378.15.
  **That is the 572 against 587 of `pulse-psp-race/talons-alongside`**: `--hud-state` imposes whatever the pair script passes (`CaptureHud::speed_kmh` replaces the readout's own), and the script passed `|v| * 3.6` of the row read at
  the stop. It now reads the `PLAYER_HUD` block itself (the registry entry whose `+0x24` is the player's lap time bit for bit, `psp-race-field.py` `Field.hud_kmh`) at the freeze and imposes that
  (`original.hud.speed_kmh`, the body value kept beside it as `body_speed_kmh`); the bar width follows (the frame's bar and text agree). Not done: the other `--hud-state` fields (place, lap, lap time, shield) are read at the pose's
  stop, so they can be a tick ahead of the frame's text too; the lap clock tenths matched on every pair looked at.
- **Pulse PS2: the grab is a separate process seconds after the load, and the HUD clock is the RAM clock less 4.25 s.** The RAM race clock (EE `0x70EE9C`) runs from the start of the countdown, the HUD's lap clock from GO. Three grabs of one
  state, the HUD digit read off the frame against the RAM clock at the key press and at the file's appearance (`grab_with_clock`, in-process, a few tenths of a second wide where the old `shot` subprocess plus a clock read either side
  spanned 0.7 to 1.65 s): 0:20.9 in 25.10..25.77, 0:22.2 in 26.24..27.89, 0:21.6 in 25.73..25.89, so the offset is **4.2 to 4.29 s**; it was `0.19` (read off a state where the two were not compared at one instant), which put ours' HUD clock 4.4 to
  4.7 s ahead of the frame's (`0:25.3` against `0:20.9`). 4.25 s is 255 ticks, **not our countdown (272) and not measured anywhere else**: the PS2's own countdown or clock start differs, recorded as `CLOCK_HUD_OFFSET_S`, and it is the one place a title's
  own countdown shows up. The craft rests on the grid in this state, so its pose from the state file is the grab's; a moving PS2 craft would need the pose read live at the grab. The state is frame-deterministic (the same bracket twice) but
  the grab's offset from it is not (it moved with the machine's load).
  The profile's PS2 aspect was `ps2` (4:3), which fits the picture inside a 16:9 window with black bars: ours came out pillarboxed against an original that fills the 1280x720 frame (mean grey difference 13.4 to 27.6 on that alone);
  it is `wide` now.
- **HD: the picture needs no correction after the wait for the RSX, the HUD clock text lags RAM by 3 ticks.** `rpcs3-drive.py capture` reads the pose in the pause and `OAG_CAPTURE_FLUSH=3` lets the RSX present what it was handed: the
  existing `hd-race/talons-pack` hull check (holding the player back by one tick, 2.4 units, already makes the hull far too large) says frame and pose are one instant. The HUD lap-clock text is not: on the 13 refreshed grid pairs the tenths digit the
  frame shows is one tenth below `trunc(RAM clock * 10)` on **9 of 13** (chenghou 14.149 shows 14.0, metropia 13.417 shows 13.3, talons 13.134 shows 13.0, ...) and equals it on 4, and the fractional tenths of the two sets bracket the lag between
  **0.054 s** (modesto-heights, 13.354 shows 13.2) and **0.055 s** (anulpha-pass, 13.955 shows 13.9): 3.3 ticks, so `rpcs3-hd-pair.py` runs ours `HUD_LAG_TICKS = 3` ticks short of the RAM clock (and the race pair's imposed clock),
  recorded in `pair.json` `original.presented_lag`. The set in `data/reference` made before this carries the uncorrected clock. Per-tick picture lag on RPCS3 is not measured (no per-tick stepping).
  HD's window opens at x = 240 on the 2000-pixel Xvfb, so a 1920-wide frame was clipped to 1760 and every pair refused ("original image size 1760x1080"); `rpcs3-hd-pair.py` waits for the resize to land and moves the window to the origin.

## Re-rendering ours only (2026-10-21)

`just compare-rerender-ours [--only <substring>]` re-draws `ours.png` of every pair with the current `oag-game` and
leaves every original alone; no emulator runs, so the maintainer can run it at any time. It replays the flags each pair
recorded in `ours.flags` against the pair's disc image (`--image` in `argv`, else the profile's), with a private
settings directory written from the profile. Those flags carry the pose, camera, field and clock read from the
original's RAM when `theirs.png` was captured, so with `theirs.png` kept they still match it: this is the one place a
stored pose is reused, where `compare-rerender` re-reads it because it re-captures the original as well. A recorded
path under a lane's scratch directory or a reaped worktree resolves to the file of the same name in the pair
directory (`trace.csv`, `field.csv`, `state-row.csv`), and a pair whose input does not resolve is skipped and named.
The old scores move to `previous_ours`; `ours.binary`, `ours.rendered_at`, `overlap` and the environment are
replaced, then the three manifests are rewritten from the pairs they list and the page is rebuilt. Each pair's line
prints mean grey, RMSE and edge correlation old -> new, green where the score improved, red where it regressed and dim
where the printed value did not move, and the run ends with a tally by mean grey; colour is on for a terminal and off
under `NO_COLOR` or a pipe (`FORCE_COLOR=1` forces it). It does not re-run
the pair scripts' checks on ours (pose read back from our trace, settings read back): a change that moves our pose
shows up only in the picture and the scores.

### The Safe Area Setting a pair's original ran at (2026-10-27, `pair-safe-area`)

HD draws its HUD and front end at the player's `Safe Area Setting` ([hd-hud](../formats/hd-hud.md), "The whole HUD is drawn
at the Safe Area Setting"; the slider is `save="true"`, the disc default 90), so a pair is only structurally comparable when
ours is drawn at the value the original ran with. Every HD pair therefore records it as `original.safe_area`:

```json
"safe_area": {"percent": 96, "source": "save file", "file_sha256_12": "a6278b4ef8f5", "file_mtime": "2026-10-21T10:28:40+02:00"}
```

- **Where it is read.** `compare_env.read_hd_safe_area()` parses the option's record out of the save RPCS3 reads,
  `<XDG_CONFIG_HOME>/rpcs3/dev_hdd0/home/*/savedata/BCES00664-AUTO-/DATA.BIN` of the capture's private profile. A record is a
  0x40-byte name field (`Safe Area Setting`), a u32 value length, a u32 0x40, a runtime pointer, and the value as a string at
  name + 0x60: `96%` in the maintainer's save, a bare `96` (no `%`) in a save an older run wrote. Nine distinct private saves
  were read on 2026-10-27 and every one holds 96, so the parse is proven against two spellings of one value; a second value
  is exercised only by a synthetic record in `check-compare-environment.py`. No emulator is started to read it.
- **Cold-booted pairs** (the menus, `rpcs3-hd-menu-pair.py`) show exactly what the game read at boot. **A pair that restores a
  save state** (`rpcs3-hd-pair.py --state`) carries the state's own RAM, which the file may not match, so its `source` says
  so; the live read of the RAM chain (TOC 0x008ad4d8, slot -0x7924(r2), pointer to pointer, +0x104 big-endian float) stays
  the check for a state taken at another value.
- **Ours is drawn at it.** The pair scripts and `compare-rerender-ours.py` pass the recorded percent to
  `compare_env.write_ours_settings(plat, dir, safe_area)`, which overrides the profile's `hud_safe_area` pin (the pin is the
  fallback); `read_back` compares `graphics.hud_safe_area` against it, and the re-render fails a pair whose rewritten
  settings file disagrees. A pair with no recorded value keeps the pin, and says so in `ours.safe_area_basis` and in its
  environment text. Proof the override reaches the picture: the same Ship Select pair at 90 instead of 96 moves its rules by
  (-6,3) to (8,3) px and mean grey 10.6 to 20.9.
- **A pair without it is reported.** `Pair.finish` fails the check "original Safe Area Setting recorded" on an HD pair that
  has none, `compare_env.py manifest` warns, and `python3 scripts/compare_env.py missing-safe-area DIR...` lists them (exit 1).
- **Backfill.** Pairs made before this carry no value; `python3 scripts/compare-backfill-safe-area.py DIR [--apply]` sets
  `{"percent": 96, "source": "RAM read 2026-10-21 (docs/formats/hd-hud.md)"}` on every HD pair that has none (all the HD
  states and the menus came from the one save). Point it at `data/reference/compare-page`: it filters by profile and so covers `hd/` and `hd-race/`.

A menu pair also records what its page was drawn with beyond the profile: `ours.front_end_style` and `ours.race`
(`race.team`, `race.track`, `race.variant_chosen`). `compare-rerender-ours.py` writes both back into the private settings
(`compare_menu.extra_settings`); before 2026-10-27 it did not, so a re-rendered picker drew the title's first team, Assegai,
on concept1, where the original shows Feisar. HD's Ship Select also needs `variant_chosen = true`: a profile that never picked
a model opens on concept1 (stats 080/085), where the original's classic Feisar reads 070/100/080/080. The Omega menu script records `ours.race` from now on; the 2048-menu and Pulse/Pure
menu scripts do not yet, so their re-renders still drop their `race` selection (older Omega pairs have none either).

Open, measured the same day: with the pairs at 96 and the right team, the title rules, footer rule, footer prompts and stat bars of
Ship Select and Track Select sit at 0 +/- 1 px, but ours' header text sits 1 to 2 px **left** of the original's (best shift of ours
onto theirs +2 px on Ship Select, +1 on Track Select), so the safe area is not what is left to fix there.

## Capturing a pair per title

Every recipe runs in its own display and data directory, muted, and is stopped by PID. A pair is only a pair when
its `pair.json` has no failed check.

| Title | Status | What matched | How |
| --- | --- | --- | --- |
| Pulse PSP | **works**, about 1.5 min per circuit (a mid-race **race shot** with the whole field: see "Race shots" below) | everything in the checks above | `PPSSPP_EXTRA_INI=<file from compare_env.py ppsspp-ini pulse-psp> scripts/ppsspp-start.sh <lane> <display> <port> <image>`, then `scripts/psp-pulse-pair.py --port P --home $PP_HOME --out D --track NN_Track --state <ppst>` (it resizes the SDL window to 960x544 for `psp-state.py`'s pause-menu clicks, loads the state, stops the CPU, resizes the window to the native 480x272 and crops the Xvfb root at the window origin: nothing is scaled), or `scripts/compare-pulse-psp-batch.py --out D` for all 24 kept states. No state yet: `psp-drive.py menu --unlock --track-down N --any-track` (about 105 s from a fresh profile, N from [the table](../reverse-engineering/ppsspp-debugger.md#every-circuit-not-three-the-dev-unlock-byte-2026-09-29)), then `psp-state.py save 1 --keep`. Full pipeline: [frame comparison](frame-compare.md) |
| HD (PS3) | **works**, about 1.5 min per circuit | pose, camera, fov, clock | `export OAG_COMPARE_PLATFORM=hd-fury` before `scripts/emu-restore-state.sh` (so `rpcs3-drive.py` writes the profile's video settings into the private config **and** the game's custom config), restore the state with `--restart`, then `OAG_RPCS3_ATTACH=1 scripts/rpcs3-hd-pair.py --out D --team harimau --state <state>`: it resizes RPCS3's window to 1920x1080 (the default 1760x1072 window carries black bars), captures, and checks the frame is exactly 1920x1080. One state per circuit: `scripts/emu-hd-timetrial-state.sh <lane> <circuit-index> <team-rights> <classic 0\|1> <name>` (Track Select index 0 is Vineta K, 8 Talon's Junction; Harimau classic is 9 `right`s and a `down`). The state only names the circuit and craft: **the pair is taken after `Restart Race`, a few seconds into a fresh Time Trial**, so the clock is small and the double-counted resting turn small. `--team` carries HD's variant (`Assegai_c1` is the Fury Concept craft). |
| Pulse PS2 | **works for Moa Therma White, camera matched** (widescreen) | pose, clock, circuit, layout, the state's `Aspect Ratio 16:9` (read back from the projection), the camera's look direction at the craft, PCSX2's `AspectRatio` | PCSX2 booted by `pcsx2-drive.py boot` with `OAG_COMPARE_PLATFORM=pulse-ps2` in the environment (its ini carries the profile's upscale, screenshot size and `AspectRatio 16:9`), then `scripts/pcsx2-pulse-pair.py --slot S --state <p2s with DAT_00284fe8 = 1> --track 03_Track --out D`. Pose, craft, clock and **camera** (`pcsx2_state.camera`: eye, axes and fov of the object `Camera_SubmitScene` runs on, [camera.md](../ghidra/functions/ps2-pulse-eu/camera.md)) are read out of the state file's own EE RAM, the frame is PCSX2's GS screenshot of the loaded state taken while the VM runs on (the race clock is read before and after the grab and ours is rendered at its mean). A widescreen state is made from a 4:3 one by writing `1` to `0x00284fe8` over PINE, advancing four frames and saving. **What is left is the original's `40/21` projection shown at `16:9`: ours draws everything 7.1 % wider** (ours at aspect `wide`, true 16:9, since 2026-10-21; the sentence assumed that and holds), edge correlation 0.11 against 0.59 with ours squeezed to match (camera.md). Only Moa Therma White has a state; more need a menu route for `pcsx2-drive.py` |
| Pure (PSP) | **works** (2026-10-22, `pure-pairs`), about 30 s per pair once a state exists; four Alpha-league circuits, Feisar, Vector | pose, circuit, camera (the original's Close eye read at `(0, 3.000, -11.250)` in the body frame; ours' own chase rig is checked by the hull's px offset, 0 to 1 px), fov 60, race clock (read at the state's own first stop, the HUD tenth equal on 4 of 4) | **USA disc** (`data/images/pure-psp-usa.chd`, `UCUS98612` 2.00): every live Pure address is recovered on it; the EU image is not read. `PPSSPP_EXTRA_INI=<file from compare_env.py ppsspp-ini pure-psp> scripts/ppsspp-start.sh <lane> <display> <port> <usa image>`, then `scripts/psp-pure-pair.py --port P --home $PP_HOME --out D --track 01_Vineta_K --state <ppst> --class vector --team Feisar --autopilot off --meta <json>` (`OAG_SHOT_DISPLAY`, `DISPLAY`, `HOME`, `XDG_CONFIG_HOME` as for Pulse). States: `scripts/psp-pure-walk.py --port P --track-index N --keep data/saves/pure-psp/<name>.ppst` walks pad presses from a running race (`start`, Quit) or the Main Menu to Time Trial, Vector, Alpha, circuit N, Feisar, and saves a state 16 s after Track Selection (a fresh profile has only Vector and Venom and the Alpha league: Vineta K, Modesto Heights, Chenghou Project, Blue Ridge, directories `01_Vineta_K`, `03_Modesto_Heights`, `04_Chenghou_Project`, `07_Blue_Ridge`). **Not matched: the shaded hull (ours darker), and the HUD (ours has no speed gauge and prints `TOTAL` where the original prints `BRONZE TIME`).** Details: "Pure (PSP) pairs" below |
| 2048 (Vita) | **works for a grid before GO** (2026-10-22, `pairs-2048`), about 1 min per pair once Vita3K's shaders are cached; three circuits (Empire Climb, Queens Mall and, from the DLC's HD Campaign, Vineta K), Feisar, 8th on the grid | circuit, craft, grid slot (8/8 on both HUDs, by eye), window size 960x544, ours' settings; **no pose, camera or clock is read** (Vita3K has no state channel, and the race clock reads zero for the whole countdown) | Vita3K on its own weston and Xwayland (`vita3k-drive.py display`), a private `HOME`, `pref-path` and config, `SDL_AUDIODRIVER=dummy`, then `scripts/vita-2048-pair.py --out D --walk --event "2048 - Event 2" --node X Y --autopilot off --meta <json>` (`OAG_VITA3K_CACHE`, `OAG_VITA3K_CONFIG`, `OAG_VITA3K_DISPLAY`, `XDG_CONFIG_HOME` as for the others). It boots, waits on each screen by the frame's mean luma and the HUD's white text rather than a fixed sleep, taps the campaign event, pauses, Retries and grabs frames every 0.16 s through the countdown; the grid frame is the one 3 s after the HUD's first frame, ours renders tick `round(60 * 3.0)`. **Not matched: the lighting (the original is darker, mean grey 45 to 57; since 2026-10-26 ours runs 2048's own composite with the exposure held at the original's 1.15, mean grey 23 to 39, "Exposure" below), the grid pose (heading, and Queens Mall's pitch: ours rests 1.15 degrees nose-down of the original, which is the whole of its floor sitting 8 px high; the "hull 30 px up and left" reading of 2026-10-22 was the yellow-centroid metric catching other yellow, the nose tips agree to 2 px), and on the DLC pair the lighting by far (original 48 mean grey, ours 80; the 170 first written here averaged ours' alpha in), the HUD and the hull's livery.** The original's drawn-hull shake is patched out in its memory by the pair script (`--walk`), and its grid pose is read out of that memory with `--read-pose`. Details: "2048 (Vita) pairs" and "The hull shake and the grid pose" below |
| Omega (PS4) | **works** (2026-10-22, `pairs-omega`), about 85 s per pair on a warm shader cache (136 s cold); two circuits, Vineta K and Anulpha Pass, Feisar, Venom | circuit, craft, mode, camera mode (the grid is a still; there is no pose readout on shadPS4) | `scripts/shadps4-omega-pair.py`: its own headless `sway` on the render node, its own `HOME`, one uinput pad, muted. Walks MAIN MENU > RACEBOX > Track Select (Right steps the circuit) > Ship Select (two Downs reach the first Feisar craft on a fresh profile) > loading > START RACE and grabs the first frames after the HUD shows, before the countdown board's red 3. **Two shadPS4 settings carry the picture**: `readback_linear_images_enabled = true` with `readbacks_mode = 1` makes the world draw (off: black scene, HUD and lens flare only); `copy_gpu_buffers = true` crashes the emulator at the race load, `direct_memory_access_enabled` alone leaves it black. Recipe: [online lobby, part 3](../networking/online-lobby.md). Profile row `omega`: 1920x1080. **Known gaps in ours**: the gantry board shows the grid position 8 where the original's is blank and counts 3, 2, 1, GO; scenery lighting is not read, so the original's sun, shadows and sky differ (Anulpha's road reads white); the original idles an exhaust flame and ours draws none. The craft offset is read by eye off the nose tip: (-2, +5) px on Vineta K, (0, 0) on Anulpha Pass (the camera follows the craft, so the nose does not move). **Since 2026-10-26 (`omega-grid-height`) ours holds HD's grid hover, which Omega's craft update carries ([hover-target](../ghidra/functions/ps4-omega-eu/hover-target.md)): before, ours floated high and the floor sat a pad's length too low on screen. Mean grey 56.2 to 46.7 (Vineta K) and 75.9 to 61.5 (Anulpha Pass), edge correlation 0.08 to 0.18 and 0.07 to 0.23.** The manifests were not regenerated here; `just compare-rerender-ours --only omega` adopts it. Whether the original's hull shake is visible on the grid is not determined (consecutive grid frames differ by 0.10 to 0.27 normalised RMSE over the hull region, which also holds the animated exhaust) |

## 2048 (Vita) pairs (2026-10-22, `pairs-2048`)

Two grid pairs, Single Player Campaign, Feisar (the campaign's craft on this save), the countdown before GO, 8th of 8, Vita3K at 960x544, ours rendered at tick
176 to 184 of 272 (`--event`, so the same circuit, craft and a full grid). Mean grey difference and **hull offset** (`pair.json` `craft_offset_px`: the centroid of the
livery-yellow pixels of the original against ours', columns 150 to 750 of the bottom 60 %; edge correlation was tried and reads 0 on two hulls that differ in
shading and a few degrees of yaw):

| Pair | Mean grey | RMSE | Hull offset (original minus ours) | Reading |
| --- | --- | --- | --- | --- |
| Empire Climb (`2048 - Event 1`) | 45.6 | 0.239 | (+4.7, -0.8) px | the hull sits where ours' does; ours' track and walls draw in flat colours here, the original's are lit and textured |
| Queens Mall (`2048 - Event 2`), boot A | 56.4 | 0.267 | (-32.1, -33.1) px | the circuit and slot are the same, the original's grid heading and lateral place are not ours' |
| Queens Mall, boot B (repeat) | 57.4 | 0.271 | (-25.6, -25.8) px | the same sign and size on a second cold boot |
| Vineta K (DLC1, the HD Campaign's Uplift Race; ours `--track ... --team Feisar`), boot C | 47.0 | 0.261 | nose tip (-5, 0) px, by eye (+-1) | the same circuit, camera, hull model and slot; see "The DLC pair" below |
| Vineta K, boot A / B (repeats) | 47.4 / 47.2 | 0.263 / 0.262 | nose tip (-5, -1) / (-5, 0) px (+-1) | the same on two more cold boots, nose tips read by eye on 4x crops |

**Repeats:** Queens Mall was captured on three scripted cold-boot walks (two kept); the original's frames of boots A and B differ by RMSE 0.081, which is the hull's sway: inside one
countdown the original's hull centroid wanders 3 to 20 px in `y` and 3 to 11 px in `x` (`original.hull_centroid_range_px`), so an offset under about 10 px is not
distinguishable from the original's own motion, and Queens Mall's (-26 to -32, -26 to -33) is. Empire Climb was captured once.
**What is not measured:** the phase inside the countdown is wall clock (the HUD's first frame is tick 0, the grab 2.98 +- 0.16 s later), no pose is read from
Vita3K (until 2026-10-26: `--read-pose` now reads it out of the guest's memory), and the original's hull shake is on while ours is drawn with `--no-hull-shake`
(until 2026-10-26: `--walk` now patches it out, see below).
**How the walk works** (the 2026-10-20 fixed-timing walk never left the intro): Return, then three `x`, reach the Game Mode grid with the PSN prompt gone; one tap on
Single Player Campaign (it opens at once); one tap on the event's node; two taps on `(882, 480)` (Play, the mode card); then the loading
screen, which is light (mean luma 0.9), is waited out until a dark frame; the ship flyby's check mark is tapped until the HUD's white `LAP` text is up; pause, Retry
(`(750, 480)`, `(402, 312)`) restarts the grid and the burst starts at the confirm. **The node coordinates are this save's campaign map** (a save that has finished
Event 1: Empire Climb `(190, 25)`, Queens Mall `(270, 68)`); another save needs its own `--node`. **Metro Park's Time Trial hangs Vita3K at 56 % on the loading bar**
(three boots, two configs, a bar that sat at 56 % for 3 to 5 minutes each time); the two races above load in 20 to 30 s.

### The DLC pair (2026-10-26, `2048-dlc-pair`)

The maintainer asked for a 2048 pair on a circuit of the DLC ("basically the HD assets of 2048"). **Claim: the page carries a third 2048 pair, on a DLC circuit,
captured the same way as the first two. True.** It is Vineta K from `dlc1.psarc` (`Data\art\published\DLC1\environments\Vineta_K\track.vex`), the HD Campaign's
`Uplift` group, the Race event (Venom class, 3 laps), 8th of 8, the countdown before GO, the same Retry-and-burst grab as the two above.

- **Ours races it as 2048**, and no DLC support had to be built: the base `data.psarc` already carries the `DLC1` paths (`formats/patches.md`) and `oag_2048` mounts `dlc1.psarc`
  (`crates/2048/src/lib.rs`). `oag-game <PCSF00007> --race --track 'Data\art\published\DLC1\environments\Vineta_K\track.vex' --mode single_race --team Feisar` draws it
  (the pair script's `--track`, `--mode`, `--team`). **No menu of ours reaches a DLC circuit or the HD Campaign**: only `--track` does.
- **The craft is the HD-derived Feisar, not 2048's own.** The HD Campaign flies `Feisar HD` (Select Team's HD roster, the default); ours' `--team Feisar` resolves it under
  `Data\art\published\hdships\Feisar` (`oag_2048::race::HD_SHIP_DIR`), and the hull model matches (same shape, fins and cockpit in both frames). 2048's own `feisar2048` is the
  yellow hull of the two pairs above, so the hull-centroid measure of those (`yellow_hull`) finds nothing in the original here; the pair script takes `--nose-original` and
  `--nose-ours` instead (the nose tip read by eye off 4x crops of `(400, 230)` size 160x100, about +-1 px) and records them as `craft_offset_px.nose_tip`.
- **Numbers.** Three cold boots, original minus ours: mean grey difference 47.4, 47.2 and 47.0; RMSE 0.263, 0.262 and 0.261; edge correlation 0.08 to 0.09; nose tip
  `(-5, -1)`, `(-5, 0)` and `(-5, 0)` px by eye (the registered pair, boot C, is read at `(476, 257)` against `(481, 257)`; a second reading of the same frames gave +1 in `y`): the
  original's nose is 5 px left of ours' and level with it. **Re-read 2026-10-26 on 8x crops with a grid, landmark the canopy top: `(-0.5, +0.4)` unpatched and `(+0.7, +0.4)` patched; the `-5` reading was not reproduced on that landmark ("The hull shake and the grid pose").** The cause (grid heading, lateral place, camera) was not separated. The original's picture is steady across the countdown (largest RMSE 0.083 to 0.085 against the first settled frame), so a
  single grab sits inside its own motion. The frame's mean grey is 48.1 for the original and 80.1 for ours (**re-measured 2026-10-26, `2048-exposure`**: the 170.4 first written here averaged ours' alpha channel in, `-colorspace Gray -format %[fx:mean]` on an RGBA PNG; take means with `-alpha off`. `compare_env.py`'s mean grey difference and RMSE were never affected).
- **Not matched, and the cause is ours**: the lighting (the original is a dark sunset with long shadows and a warm sky, ours is a flat bright day), the HUD (the original's HD Campaign
  HUD has no `OBJECTIVE` or `XP` lines; **since 2026-10-26 (`2048-dlc-hud`) ours draws neither either**, because the HUD binds the current `SP.xml` event and a DLC node never sets it,
  [race-hud-selection.md](../ghidra/functions/vita-2048-eu-v104/race-hud-selection.md#the-xp-and-objective-lines-are-bound-to-the-current-spxml-event-2026-10-26-2048-dlc-hud). Still different: the key-shaped
  icon left of the speed bar and the white ring round the hull, which both 2048 originals draw and ours does not, and `TOTAL` as `0.00.00`. The pair's manifest was not regenerated here;
  `just compare-rerender-ours --only 2048` adopts the change), the hull's livery (olive and teal in the original, blue and yellow in ours; whether
  that is the shading of the dark scene or a different livery was not separated) and the rivals' craft. The pair is a baseline for the DLC circuits' lighting and HUD, not a pose check.
- **How the walk differs**: Game Mode tile `(477, 162)` (twice), the node `(510, 340)` (Vineta K's Race; `(505, 345)` in `vita3k-capture.md` is the same hex), then **Select Team: the craft button
  `(622, 480)`, its check mark `(882, 480)` returns to the map, the node again, Play, the mode card**. Without the Select Team step the load sat at 56 to 75 percent for minutes on every one of five boots
  (a wedged Vita3K whose log is a loop of `Unhandled SIGSEGV`, and which ignores `SIGTERM`); with it the load takes about 6 s. `--game-mode hd` does this; the race then runs on its own through the
  countdown and the script's pause, Retry and burst take over. (Metro Park's Time Trial 56 percent hang above was not retried with a craft confirmed first.)
- **Omega** (2026-10-26): **checked, differs / not checkable.** Omega ships Vineta K (`omega-status.md`, its `01 Vineta K` is 2.28x HD's triangles, re-cooked), so a matching pair would use the same
  circuit; there is no PS4 emulator in this project's toolchain, so the original's side cannot be captured and no Omega pair was made.
- **Adopting it** (the manifests live under `data/reference/compare-page/`, which a member does not write): copy the pair directory `2048-vita/dlc-vineta-k/` there and regenerate `2048.toml`
  with `python3 scripts/compare_env.py manifest data/reference/compare-page/2048.toml data/reference/compare-page/2048-vita/{empire-climb,mall,dlc-vineta-k}`, then `just compare-page`.

### The hull shake and the grid pose (2026-10-26, `2048-queens-offset`)

The maintainer: "the 2048 shots are still not properly lined up", and for Queens Mall "could be the random swing that we disable in HD via memory patch".
**Claim: the cause of each 2048 pair's hull and grid misalignment is the original's hull shake, as patched out on HD, or ours' grid placement.
Answer: the shake is not it; the grid pose is, and per pair it is a different term.**

**Written before the capture, what would have falsified it:** the shake is zero-mean (`rand() * 2^-31 - 0.5`), so it cannot produce one
offset of the same sign and size on two independent cold boots. Patched, the offset had to fall to the original's own sway if the shake
were the cause; if it stayed, the cause was placement or heading.

- **The shake is 2048's too.** `HullShake_Update` `0x811b51bc` and `HullShake_Apply` `0x811b5500`, constant for constant HD's
  ([vita-2048-eu-v104/hull-shake.md](../ghidra/functions/vita-2048-eu-v104/hull-shake.md)). Vita3K has no patch mechanism, and its GDB stub
  halts the guest at the connect (the game then sat on the "Sony Computer Entertainment presents" screen or at 56 % of the first load, four
  boots). `vita-2048-pair.py --walk` instead overwrites the two hull `bl 0x811b5500` (`0x812cb010`, `0x811c15a0`) with `nop nop` through
  `/proc/<pid>/mem` of the Vita3K it launched (guest address plus `0x400000000`, the channel `scripts/vita3k-mem.py` serves reads over), before the guest runs them (Yama's `ptrace_scope` 1 allows only a child), and records the bytes
  it read back (`original.hull_shake`). The third call, `0x811be926`, shakes the chase camera's node with `invert = 1` and cannot be
  overwritten: with it nopped the HUD never came up.
- **The "(-26..-32) px" Queens Mall offset was the metric.** The yellow-pixel centroid (`craft_offset`) catches other yellow in the original's frame
  (its bounding box 1.3x wide). Read on the nose tip, 8x to 12x crops with a 20 px grid, +-1 px, original minus ours, ours at HEAD (after
  `omega-grid-height`):

| Pair | Original unpatched (the 2026-10-22 boot) | Original patched (cold boots) | Floor / scenery against ours, original minus ours |
| --- | --- | --- | --- |
| Queens Mall | nose `(0, -1)` | A `(0, 0)`, B `(+2, +1)`, C `(-1, -2)`; hull sway 5 x 8 px (A) | pad edge row **265 against 257: 8 px low**, before and after the patch; `frame-shift.py` ground `+3..+7`, scenery `+9..+10` on three boots |
| Empire Climb | nose `(-2.5, -1)` | A `(+4, 0)`, B `(-1, 0)` | scenery `dx -18 px`, before and after |
| Vineta K (DLC) | canopy top `(-0.5, +0.4)` (the 2026-10-26 reading of `(-5, 0)` took another landmark) | `(+0.7, +0.4)` | scenery `dx -6 px`; floor within 2 px |

  A ground landmark per pair, eye-read on 5x to 6x grid crops, original minus ours (x, y px): Queens Mall the pad lip's lower edge, Empire Climb the
  lamp post's centre at row 225, Vineta K the wall lamp's centre. Unpatched against patched original: the same to 1 px on all three (the shake does not
  move the world). Ours at HEAD (before): Mall `(-3, +8)`, Empire `(-24, 0)`, Vineta `(-9, -3)`. Ours put on the original's pose (after): Mall `(+1, +1)`,
  Empire `(+3, 0)`, Vineta `(0, 0)`.

  So the hull is where ours' is to 2 px or better on all three (the +-4 px spread of Empire's two boots is the camera's own shake, which stays on).
  **The patch moved nothing the maintainer sees**: the offset the pairs show is the world against the craft, which the shake does not move.
- **The grid pose is read out of the original's memory** (`--read-pose`: the drawn hull matrix, rows left, up and forward scaled by the hull scale
  0.75 and then the position, the float triple nearest ours' slot, seen 400 to 500 times because the renderer keeps a copy per draw), and put
  against ours' craft at the same tick. Identical on every boot (3 cold boots of Queens Mall):

| Pair | Position, original minus ours (units) | Heading (degrees) | Pitch at rest (degrees) | Ours put on the original's pose |
| --- | --- | --- | --- | --- |
| Queens Mall | `(0.000, +0.047, -0.017)` | `-0.35` (original `0.00`, ours `0.35` off the track axis) | **`+1.15`** (original `+0.29`, ours `-0.86`) | pad edge row 264 (against 265), ground `0..-2`, scenery `+1..+2`: **aligned** |
| Empire Climb | `(+0.465, +0.026, +1.453)` | **`-2.05`** (original `177.51`, ours `179.56`) | `-0.13` | scenery `dx 0`, `dy -1`: **aligned** |
| Vineta K (DLC) | `(+0.532, +1.846, -0.002)` | `-0.62` | `0.00` | with ours' own height: ground `dy +1`, scenery `-1`: **aligned**. The `+1.846` is the HD-derived hull's node sitting that far above ours' craft position (taken as is the floor is 33 px off); `--pose-ours-height` |

  **So the cause is the grid pose, and it is not one law**: Queens Mall's craft is placed exactly where ours' is and points the same way to 0.35
  degrees, but rests 1.15 degrees nose-up of ours' (the track climbs 0.84 degrees at the grid, spline tangent `(0, +0.0147, -0.9999)`, and
  ours settles 0.86 degrees nose-down, the `-0.015` it also settles to on Empire Climb: both 2048-hull circuits, where the DLC's HD-derived hull settles level); Empire Climb's craft is **1.45 units further
  along and 2.05 degrees more turned** than ours' (the authored `Start Position` there faces `(-1, 0, 0)`, 90 degrees off the track, and ours
  turns it to the spline's tangent, `face_the_way_the_track_runs`, which gives `-0.44` degrees; the original gives `-2.49`); the DLC's Vineta K is 0.62
  degrees off in heading only. Why the original rests where it does is **not recovered**: it is neither the authored node's heading (Empire) nor, as far as
  looked at, the spline's tangent (Mall's yaw `0.00` agrees with it; the one Empire sample read, 5.9 units from the slot, gave `-6.8` degrees against
  the original's `-2.49`, which settles nothing). Written up as a thread.
- **Reproducing**: `scripts/vita-2048-pair.py --walk --read-pose ...` (adds `original.hull_shake`, `original.pose_read`, `pose_delta` and
  `ours-original-pose.png` to the pair), `scripts/frame-shift.py THEIRS OURS [region ...]` for the floor and scenery shifts (masked
  normalised cross-correlation of the high-passed gradient magnitude, `ground_right`, `ground_left`, `far_scenery`, `hull`; the scenery band
  has an NCC of 0.15 to 0.4 and is a direction, not a number, on Queens Mall and Empire Climb). The pad edge row is the strongest step of the
  mean luminance of columns 575 to 625, rows 235 to 275.
- **Ours draws a hull shake on 2048's grid too**: the same law (`HullShake::HD`, inherited), ticks 60, 120 and 171 against the shake switched off
  moved the hull 3 to 7 px in x and 3 to 5 px in y, the order of the original's stock sway, so the shake is not a difference between the two sides.
- **Vita3K sometimes sticks on its intro**: one of 12 memory-patched launches sat on "Sony Computer Entertainment presents" with the GPU thread busy
  until killed (a relaunch booted in 30 s); the gdbstub launches did it twice in three. The sticks came at a host load near 28 and not at 10 to 12. Not attributed to the patch, not understood. A fresh `HOME` needs `~/.config/Vita3K/config.yml` (else `-r PCSF00007` answers
  `not in {}`: the `-c` file is not read for the installed-title lookup) and `~/.cache/Vita3K/shaders` copied in; `vita3k-drive.py` drops
  `XDG_*` for a private instance, which Vita3K reads before `HOME`.
- **Omega**: **checked, applies, not wired.** Its base `eboot.bin` has the same `HullShake_Update` at `0x012f7200`
  ([ps4-omega-eu/hull-shake.md](../ghidra/functions/ps4-omega-eu/hull-shake.md)); the patch v1.07 executable the pair races on is not in Ghidra and
  shadPS4's patch mechanism was not tried, so no patched Omega pair and no Omega pose read. Whether the grid pose differs on Omega the same way is **not
  checked**: its pair has no pose channel; the memory-scan method is Vita3K-specific.
- **Adopting it** (manifests under `data/reference/compare-page/` are the lead's): copy the three pair directories made by `--read-pose`
  (`theirs.png`, `ours.png`, `ours-original-pose.png`, `side.png`, `pair.json`, no `frames/`; the `meta` of each stays the existing pair's `id`, `title` and notes, pass them with `--meta`, and `craft_offset_px` is set from the eye reads above, not the centroid) over `2048-vita/{mall,empire-climb,dlc-vineta-k}`,
  then `python3 scripts/compare_env.py manifest data/reference/compare-page/2048.toml data/reference/compare-page/2048-vita/{empire-climb,mall,dlc-vineta-k}` and
  `just compare-page`.

### Exposure: every Vita3K frame is drawn at E = 1.15 (2026-10-26, `2048-exposure`)

2048 finishes a race frame in `wo_composite_nocc_fp`, whose exposure E adapts once per frame to the circuit's `Lighting.ExposureScale`
over the mean luminance the CPU reads back from the bloom gate's quarter-resolution target
([vita-2048-eu-v104/exposure.md](../ghidra/functions/vita-2048-eu-v104/exposure.md)). **Vita3K never writes that target into guest memory**,
so the readback is zero, the adaptation never runs, and E stays at the 1.15 it is reset to on every pair above (read live on all three).
A Vita settles at `ExposureScale` over the mean scene luminance (the gate's alpha is its luminance over four and the readback's 1/64 cancels it;
closed 2026-10-26, `2048-exposure-wire`, exposure.md "Where a Vita settles"). Read the pairs' brightness against that: they are references for
E = 1.15, not for a running Vita.

**So ours is drawn at E = 1.15 on every 2048 pair (2026-10-26, `2048-exposure-wire`).** Ours runs 2048's composite and adapts E in play, as a
Vita does; `oag-game --hold-exposure E` holds it, a comparison switch that is never the play default. The 2048 profile in
`scripts/compare-environment.toml` carries `exposure_hold = 1.15`, so `compare_env.ours_flags` passes `--hold-exposure 1.15`,
`compare-rerender-ours.py` adds it to a 2048 pair recorded without it, and the page shows **"Ours: exposure - exposure held at 1.15"** on every
such pair, read from the flags the pair recorded. No other profile holds anything. Frame means (`-alpha off`), original against ours:

| Pair | Original | Ours before | Ours held at 1.15 | Ours adapting (play) | Mean grey difference before / held |
| --- | --- | --- | --- | --- | --- |
| Queens Mall | 60.2 | 106.2 | 54.9 | 85.7 | 55.4 / 26.3 |
| Empire Climb | 83.1 | 107.7 | 61.4 | 111.4 | 45.7 / 39.0 |
| Vineta K | 48.1 | 80.1 | 36.7 | 80.3 | 41.4 / 23.4 |

Held, ours is now a little darker than the original: Empire Climb's track and walls still draw in flat colours, and Vineta K's sky is darker than
the original's. The vignette, the screen tint and `alphaTex` are unread and left out.

Three flags of `scripts/vita-2048-pair.py` read and write the guest's memory through the countdown burst, with `--walk` only (Vita3K must be the
script's own child; Yama's `ptrace_scope` 1 refuses `/proc/<pid>/mem` to anything else):

- `--ram-read NAME=ADDR:TYPE` reads `f32`, `u32`, `u8` or `hexN` (N bytes) before every grab; `ADDR` is a hex guest address or `[PTR]+OFF`
  (OFF past the u32 at PTR). The series lands in `pair.json` as `original.ram_read`, with the value at the grid frame printed as `MEASURED`.
  Read only, so the pair stays adoptable.
- `--ram-poke ADDR=VALUE:f32[@FROM]` writes an `f32` before every grab from grab FROM on (later pokes of one address win).
- `--ram-fill ADDR=BYTE:STEP:COUNT[@FROM]` writes BYTE at every STEP-th byte of COUNT.

A pair made with `--ram-poke` or `--ram-fill` is a probe: it always fails the check "the original's memory was only read", so it can never be
adopted. The two probes of the exposure page, for instance: `--ram-poke 0x815cb93c=0:f32 --ram-poke 0x8151c5e8=1.15:f32
--ram-poke 0x8151c5e8=2.3:f32@30` (bloom off, E doubled halfway) and `--ram-fill '[0x816aef2c]+3=0x40:4:34816@20'` (the gate target's alpha
filled, after which E adapts).

## Pure (PSP) pairs (2026-10-22, `pure-pairs`)

Four grid pairs (Time Trial, Vector, Alpha league, Feisar, a few seconds after GO, craft untouched), mean grey difference and **craft offset** (`pair.json`
`craft_offset_px`: the shift of the original's hull edges against ours' by normalised cross-correlation over the bottom-centre 120x96 px, +-12 px searched;
a lost camera shear would read about 24 px in `dy`): `01_Vineta_K` 28.9 grey, offset (0, 0) px; `03_Modesto_Heights` 13.2, (0, 0); `04_Chenghou_Project` 24.4, (0, 0);
`07_Blue_Ridge` 12.9, (-1, 0). The pose difference of 0.00 units and 0.00 degrees is **not** a measurement (ours is fed the original's row); fov is 60 against 60, race
clock tenth equal. **Repeats:** captured on three PPSSPP boots (two by hand, the third by `just compare-rerender-all --only pure-psp`, which now boots Pure's own
USA image for `pure-psp` pairs), `theirs.png` and every number identical: that shows the state load and the reads are deterministic, not that two different states agree.
The script needs no numpy: the offset is pure Python over `magick` edge maps. `trace.csv`, `ours-trace.csv` and `spawn-trace.csv` stay in each pair's directory.

**Falsifier, written before the capture.** A resting Feisar in the Close view has its eye at `(0, +3.000, -11.250)` in the body frame and fov 60
([camera.md](../ghidra/functions/psp-pure-usa/camera.md)). Read live off the loaded Vineta state: `(-0.000, 3.000, -11.250)`, `11.643` from the body, fov
`59.99997`. A different view or a lost shear would have broken it; neither did. The script refuses a pair whose eye is not that.

**Pose.** Read at the state's first `Ship_UpdateCraft` stop (`0x0892d214`, `a0` craft, `a1` body): basis at body `+0x00`, position `+0x30`, velocity
`+0x180`; the tripod at `*(controller+0x40)` at `Camera_CopyTripodToView` (`0x08849970`); the clock at `Race_UpdateLaunchGrade`'s `a0 + 0x130` (seconds, advances by the frame time `0.0333` per craft update, camera.md; read twice, a second load reads it at the picture's own stop). The camera node was located
already (camera.md), so the grid fallback was not needed. Ours takes the craft's row through `--pose-from` and builds **its own chase camera**
(`--no-camera`): the original's tripod matrix is a shear that ours' `look_at` re-orthonormalises, so feeding it draws the pitch 5.71 degrees wrong; the shear
is ported in the chase rig (`chase_view_sheared`). The presented frame lags the stop by one (Pulse's measurement, **inherited and unmeasured on Pure**;
a resting craft cannot tell, the clock can: it moved the HUD tenth in 4 of 4 pairs, matched by reading it at the stop and subtracting one frame).
The circuit is checked by ours' own spawn: 0.2 to 8.8 units and under 3.5 degrees from the original's grid pose on the four circuits.

**Known, not fixed here.** Ours draws the hull darker and less lit than the original on all four, and its Pure HUD has no speed gauge and shows `TOTAL`
where the original shows `BRONZE TIME` (the `edge_ncc` of 0.26 to 0.36 is these, not the geometry). A first state taken 595 s into an idle race (the maintainer's
`time-trial-vineta-feisar-grid-fresh`) also loads and pairs (eye and fov equal) but runs 36,000 ticks of ours for the clock; the restart states are the ones
kept. Only the Alpha league is on a fresh profile; further circuits need the dev unlock Pulse has (`psp-drive.py menu --unlock` writes Pulse's byte); a Pure equivalent was not looked for.

**No hull shake on either side (2026-10-22, `hd-hull-shake`).** HD turns and moves every drawn hull a little each frame
by a random curve ([hull-shake.md](../ghidra/functions/ps3-hdfury-eu/hull-shake.md)). A still pair cannot match that frame
for frame, so it is drawn on neither side:
- **Ours:** `compare_env.ours_flags` passes `--no-hull-shake`, and `compare-rerender-ours.py` adds it to an older pair's
  recorded flags.
- **The original:** `scripts/rpcs3-hd-no-hull-shake.py` patches the call out in the lane's private RPCS3 config.
  `rpcs3-hd-cold-pair.sh`, `compare-hd-batch.sh` and `compare-rerender-all.py` run it before booting;
  `OAG_HD_KEEP_HULL_SHAKE=1` keeps the shake.
- `pair.json` records the original's state as `original.hull_shake`. A pair captured before 2026-10-22 has no such field,
  and its original still shakes.

**HD pairs run with Pilot Assist on** (`pilot_assist = "on"` in the `hd-fury` profile): the original's states have it on, and its indicator (the bracket under the craft) is part of the frame; ours takes `--pilot-assist on` and the environment line says so (2026-10-21, [pilot-assist.md](../ghidra/functions/ps3-hdfury-eu/pilot-assist.md)).

**Re-making every pair with one command (2026-10-21, `rerender-all`).** `just compare-rerender-all [--out DIR] [--platform psp|rpcs3|pcsx2] [--only SUBSTRING] [--resume] [--dry-run]`
(`scripts/compare-rerender-all.py`) does what the next paragraph needs a person to do per title: per platform it boots the emulator
headless on its own Xvfb, HOME, port and config from the profile (`--lane` names `data/scratch/<lane>/`; `--psp-display 93 --psp-port 9317`, `--rpcs3-display 94 --rpcs3-gdb 2349`,
`--pcsx2-display 92 --pcsx2-slot 28015` are the defaults), restores each pair's state (RPCS3: `emu-restore-state.sh`, `--pause` for a pair whose argv has `--field`, `--restart` for a
Time Trial taken after `Restart Race`, with `OAG_COMPARE_PLATFORM=hd-fury`; PPSSPP and PCSX2 load the state inside the pair script), re-runs the pair script from the `argv` in its `pair.json`
with the current `oag-game` (`compare-rerender.py`'s `remake`, which remaps a `--state` kept in another lane's worktree to `data/saves/<title>/` and the recorded `--port`, `--home` and `--slot` to this
instance's), stops the emulator by the PID its start script recorded, rebuilds `fresh.toml` and `race-shots.toml` next to the pairs, and prints and writes (`DIR/rerender-all.md`) one table: per pair the drift of the
original's own state between the two captures, and `edge_ncc`, `rmse` and the **mean grey difference** (the mean absolute grey difference over the frame, 0-255, now recorded in `pair.json` `overlap.mean_grey`)
before and after, with the mean per platform. `--out DIR` copies the set there and re-makes the copy (nothing under the source is written; each re-made pair records the SOURCE directory as its `--out`, so a pair is adopted by copying its directory over the source's one and
regenerating the manifests with `compare_env.py manifest`; copy pair by pair, not whole platform directories with `--delete`, so a pair another lane added meanwhile survives); `--resume` skips pairs `DIR/rerender-all.json` already holds. A pair refused by a parity check,
failed, or timed out (540 s; an emulator that stalls is retried once, a refusal is final) keeps its old files and makes the exit status non-zero. A pair made before `--meta` existed gets its hand-written
manifest fields (id, title, track, craft, state, note) copied from the manifest entry that lists it into `pair.json` `meta` and into `argv`, so a regenerated manifest and every later re-make carry them.
The PPSSPP state load is not tick-exact (the CPU stops a few ticks to a second after the load), so two captures of one state differ in clock and, in a race pair, in pose: the table's drift column says by how much,
and ours follows whichever stop was captured.

**Re-making a pair in place.** When the renderer changes, `just compare-rerender [--only <substring>] [dir]`
(`scripts/compare-rerender.py`, default `data/reference/compare-page`) re-runs each pair's script with the arguments
recorded in its `pair.json` (`argv`): ours is therefore **always posed from the emulator's own savestate**, the ship
pose, camera and race clock are read out of RAM fresh and the pose row derived from that read, never taken from a stored `pair.json` or CSV. `theirs.png` is captured in the same run, every environment and
parity check runs again, and on success the pair's files are replaced; the old values become a record in the new
`pair.json` (`previous`), with `drift` (ship position and heading, camera, clock, the RMSE between the old and new
`theirs.png`) and a WARNING printed when the drift is beyond the profile's tolerance. A pair whose fresh run is
refused by a check is named, keeps its old files, and makes the exit status non-zero. The emulator has to be up
exactly as when the pair was made (the per-title rows above); the pair's own script is the only place the capture
is defined, so the command works for PSP, HD and PS2 alike. Then rebuild the page with `just compare-page`.

The fresh set lives in `data/reference/compare-page/` (`pulse-psp/`, `hd/`, `pulse-ps2/`, one directory per pair
holding `theirs.png`, `ours.png`, `side.png` and `pair.json`); the manifest is built with
`python3 scripts/compare_env.py manifest data/reference/compare-page/fresh.toml data/reference/compare-page/<dirs>`.

## Race shots: a mid-pack frame of a Single Race, weapons off (2026-10-20, `compare-race-shots`)

The grid and Time Trial pairs have one craft. A race shot is the player **in the pack of a Single Race**, flown by the
original's own Autopilot, with every craft of the field, the HUD's race state and the camera taken from the original's RAM
at the one stop the screenshot is taken at. `just compare-rerender` re-makes it from its savestate like any pair.

**Pulse PSP (works, measured).**

1. **Weapons off.** The Racebox page (`RACEBOX` > `CUSTOM RACE`) has the rows `RACE TYPE`, `SPEED CLASS`, `WEAPONS`, `AI
   DIFFICULTY` (screenshotted, 2026-10-20), and `WEAPONS` is editable in a Single Race (a Time Trial forces it off, Head to Head
   locks it). `psp-drive.py menu --single-race --weapons-off` sets it (`down` to `WEAPONS`, `right` once: On, then Off) and prints
   `g_weapons_enabled` (`0x08b30fa8`, a byte) once the race is up: it reads `0` ([shield.md](../ghidra/functions/psp-pulse-usa/shield.md)).
   **The control, measured:** `--weapons-on` (the row pinned to On) reads `1` at the same address, so the byte is the setting and not
   a constant. The pair script reads the byte out of the loaded state and **refuses** a state where it is not `0`.
2. **Autopilot.** `psp-race-field.py hunt` arms the player's Autopilot the way `psp-autopilot-rail.py` does (fire bit `0x1000`, timer
   `record+0x148` written huge); the original's driver then flies the race. It also works with weapons off.
3. **The hunt.** One conditional breakpoint on `Ship_UpdateCraft` (`0x08849618`) for the player's craft, once a tick; the update
   order is the player first and then slots 6 down to 0, so at that stop **all eight bodies are on the same tick boundary**. Per
   stop it reads the player's place (`entity+0xbd4`, 1-based, in the racer entity at `manager+0x78+4n`, not the craft), and only
   when that is in the wanted range the whole field and the camera node, and tests: other craft inside the camera's cone
   (`--half-fov`, `--max-dist` from the eye) and at least `--min-near` of them within `--near-dist` of the player. The player
   starts eighth and is on the grid's back for the first lap; with weapons off the autopilot reaches 6th to 5th only late in lap 1
   or in lap 2, and the field is spread out (the grid alone spans 344 units), so a hunt takes one to nine minutes and some
   races never match. **The camera's look direction is the negated third COLUMN of the recorded rows** (the node stores its rotation
   transposed), not the `cam_fwd` row: using the row put craft behind the camera "in view" in the first hunts. **The criteria are the brief's (place 3 to 6, craft close and in view); where a hunt had to settle for less,
   the state's note says so.**
4. **The state.** The pause menu cannot be opened while the debugger holds the CPU (its `paused()` reads the stop as the menu), so
   the hunt resumes, presses Escape at once and saves slot 3: the saved tick is a few frames after the match. The pair script
   reads the field again after loading, so nothing is trusted from the hunt. A state is `data/saves/pulse-psp/race-*.ppst` with a
   `.json` sidecar naming `mode single_race`, the class, the player's team and the layout.
5. **The pair.** `psp-pulse-pair.py --field --layout forward --team <player team> --track 16_Track --state race-*.ppst`: load,
   stop the CPU, `psp-trace.py --craft <player craft> --field-out field.json --camera --shot-every 2` captures at the first stop
   of the player's craft the player's row, the camera node, **every racer's body, team name, place, lap, lap time and best lap**
   (`psp-race-field.py` rows) and the screenshot. Ours is `oag-game --race --pose-from trace.csv --field-from field.csv
   --hud-state PLACE,LAP,LAP_S,RACE_S,KMH,SHIELD_PERCENT,BEST_S --ticks 0`:
   - `--field-from` (`race::Options::field`, `grid_teams`) keeps the Single Race grid's seven opponents alive, puts each on the
     pose its row carries (the same left/up/forward reading `--pose-from` uses, `oag_trace::replay::orientation_of_rows`) and flies
     the team the row names; the original's slot `n` is our opponent `n + 1`. The craft are drawn at rest: no tick runs.
   - `--field-warmup` (`CaptureOptions::field_warmup`, `Race::warm_up_field`, 2026-10-20, `race-shot-trails`) gives every craft the exhaust
     history of a craft that has been flying: the field file carries `vel_x,vel_y,vel_z` per row (the body's velocity read at the same
     stop: PSP `body+0x140`, HD `body+0x190`, units per second; HD's agree with the forward axis and `|v| * 3.6` is the HUD's km/h) and a
     slot 0 row for the player, and ours replays **the exhaust alone** (no physics, AI, pad, lap, weapon or clock step; `world.tick`
     stays 0) for `Race::field_warm_up_ticks` steps (the intensity's full climb, 1 / 0.25 s, plus a full HD tube: 294 ticks at 60 Hz),
     each craft on a straight path that ends exactly on its captured pose. A file without the columns is refused by name, never given a
     guessed velocity. **Chosen, not measured:** the straight path and fixed heading (the original's craft had curved, so a ribbon in
     a corner leans where ours is straight) and full thrust for every craft (the player's throttle reads 100; an opponent's is not
     read from RAM, nor any craft's flare block, `boost_timer` or `plume_timer`: all left at the engine's own values for a craft at that
     speed). The pair scripts pass it by default, so `just compare-rerender` picks it up from the stored `argv`.
   - `--hud-state` (`race::CaptureHud`) imposes the original's place, lap, lap time, race clock, speed, shield (as a percentage
     of the pool: PSP's maximum is `*(*(craft+0x6c) + g_skill_level*4 + 0x84)`, 135 here, so the 134.5 pool reads 99 %, and
     HD's is 140; ours is the handling file's) and best lap on the primary readout: all read from RAM, none
     derived. The lap on screen is `entity+0xacc - 1`.
   - Ours races with weapons off (`[race] weapons = "Off"` in the pair's private settings); the layout is **asserted** with
     `--layout` (the grid probe is a start-line test), and checked against the sidecar.
6. **Checks.** A race shot is refused unless **the frame is a pack**, measured on the captured RAM (`compare_env.pack_check`, recorded
   as `pair.json` `pack`): the player 3rd to 6th, at least two craft in the camera's cone (28 degrees half-angle, up to 110 units from
   the eye; HD: 45 degrees, 160 units, a 16:9 frame) and at least one within `--pack-near` (45) units of the player. The capture
   instant cannot be chosen (the state runs a few ticks before the CPU stops, PSP loads differ by up to about 90 units), so a miss is
   a refusal and a rerun; a rerender that lands off the pack keeps the old files and counts as failed. Everything in "Environment
   parity" except the race-clock phase (there is no run-up: ours renders at tick 0 and
   takes the clocks from `--hud-state`), plus `weapons off in the original's own RAM`, eight craft, a team name for each, and the
   pose and fov at the rendered tick. The re-capture drifts: the state loads, the CPU stops a few ticks after, so two captures of
   one state differ by about 4 ticks (10 units, 2.5 degrees in the first rerender); the pose, camera and field of each capture are
   from its own stop, so ours matches whichever stop it was.

**HD (works, one circuit).** The same four steps, with RPCS3's tools:

1. **Weapons off, Autopilot.** `scripts/emu-hd-race-walk.sh <lane> 8` cold-boots in the state-saving flavour (stub off, Compatible
   Savestate Mode) and walks `RACEBOX` (Single Race is the default), `down` x2 to `WEAPONS`, `right` (On to Off, the page was
   screenshotted: `RACE TYPE, SPEED CLASS, WEAPONS, AI DIFFICULTY, NUMBER OF PLAYERS`), Track Select index 8 (Talon's Junction),
   Ship Select. There is no stub in that flavour, so the Autopilot is armed through **`serve`'s memory, which can now be written**
   (`RemoteMem.write`, op `memw`: `ptrace_scope` is 1, only `serve` may touch the emulator's `/proc/<pid>/mem`):
   `*(ship+0x5edc)+0x200 |= 0x800`, `+0x158 = 10000`.
2. **The hunt.** `scripts/rpcs3-hd-race-hunt.py` polls guest memory without stopping the game (a few ms a read): the HUD's
   per-player record (found by matching the player's shield, `ship+0x5fa0`, bit for bit at `+0x48` with the pool maximum 140 at
   `+0x50` and the race's laps at `+0x10`; the heap moves per boot) for the place, and the eight bodies (`rpcs3_hd_field.py`) for
   a craft within `--near-dist` of the player and two in front. It then presses the PS menu's "Save Emulation State And Exit"
   (`emu-save-state.sh`), which writes the state about a second later and ends the emulator. The Autopilot passes the field in
   the first lap (8th at GO, 6th about 50 s in on `Novice`), so a match came on the first race.
3. **The pair.** `emu-restore-state.sh <lane> <state> --pause` restores and **pauses on the state's own tick** (`serve
   --pause-after-load`: the restored game otherwise runs on for the seconds the script takes), then `OAG_RPCS3_ATTACH=1
   scripts/rpcs3-hd-pair.py --field --team feisar --track 'Data\Environments\Talons_Junction\track.vex' --state <state>`:
   `rpcs3-drive.py capture --field --in-race --settle 0` takes the screenshot, the camera (from the push buffer, two shots 0.12 s
   apart, the first used), the player's body and **every craft's body, team string and lap clock, plus the HUD record**, in the
   same pause (`--first-at-pause`: the first shot is taken where the restored game stands, with no run-up, so the frame, pose, field and HUD record are
   one instant; the second shot runs 0.12 s, for the camera pick). Ours gets `field.csv` with the player as slot 0 (an exact pose, so no tilt is left uncarried) and the seven
   opponents on `--field-from`, `--camera-pose`, `--hud-state`, `--ticks 0`, weapons off in the private settings
   (`compare_env.set_ours_weapons_off`).
4. **The HUD record's word order differs between states**: `(lap, laps, place)` at `+0xc`, `+0x10`, `+0x14` in the hunt's boot, and
   `(place, laps, lap - 1)` in one read taken after the game had run on (`6, 3, 1`, which fits that order but matched no frame: no shot was taken at that instant). `hud()`
   decodes both and refuses (`place` is None, the pair stops) when both or neither is a valid reading. Not understood: it is a read
   of one structure at one address in two boots. The restored-and-paused state reads `(2, 3, 6)`, which matches the frame.

**HD pairs pin the GO board's phase and wait for the RSX (2026-10-21, `hd-emissive-glow`).** `rpcs3-drive.py capture` reads the gantry node's time out of RAM in the pause
(`ship.gantry_seconds`, `rpcs3_hd_field.gantry_seconds`; `None` when no node is inside a race-manager window) and `rpcs3-hd-pair.py` passes it as `oag-game --gantry-seconds`, so the
board shows the original's phase and not the one the race clock gives. The same script sets `OAG_CAPTURE_FLUSH=3`: with the game paused the RSX keeps presenting the frames it was
handed, and a grab taken at once was 0.1 to 0.3 s older than the RAM it is paired with (HUD lap clock `0.21.0` against RAM `21.2`; about one tick with the wait). Any other capture of a
paused RPCS3 frame next to a RAM read has the same lag. [hd-glow-mask.md](../rendering/hd-glow-mask.md).

**The gantry pin takes the presented lag too (2026-10-27, `hd-go-banner`).** The pair script already ran ours `HUD_LAG_TICKS` (3) ticks short of RAM's lap clock but passed the gantry node's raw RAM time, so
the pin sat 0.05 s ahead of the picture. The GO board's pulse has a 0.6 s period and the pins of Tech de Ra (4.3687) and Modesto Heights (4.9670) are one period apart, both on the pulse's rising
edge: ours drew the glyph, the original (a frame older) had not. `compare_env.gantry_pin(gantry_s, lag_ticks)` is the pin the script and `compare-rerender-ours.py` (an HD pair's recorded
`original.gantry_seconds` and `presented_lag.hud_clock_ticks`) now use; `python3 scripts/compare_env.py selftest` pins the arithmetic. Banner box mean abs grey: Tech de Ra `720,240 370x100` 22.28 to 4.79,
Modesto Heights `800,260 500x80` 8.33 to 1.76; whole frame over the 13 HD pairs, 11 down (Sol 2 8.46 to 7.35, Moa 10.19 to 9.37, Chenghou 5.48 to 4.63), Anulpha level, Metropia 7.75 to 7.93.
The GO glyph is the race manager's board, **not** `nr_twinblend`: `OAG_SKIP_MATERIAL=nr_twinblend` leaves it drawn (only the red AG square above-left of it moves, 740..874 x 39..226).

`just compare-rerender` re-makes a race pair only if the emulator is in the state it was made from: restore the state with
`--pause` first (the capture lets the game run for 0.12 s, so a second rerender needs another restore). Measured drifts between two
captures of one state: PSP 15 to 90 units (the CPU stops a few ticks after the load; two direct runs agreed exactly, a rerender did not),
HD 14.0 to 14.2 s on the lap clock in five restores and 19.7 s in one (the pause after the load did not hold).

**What is not matched in a race shot** (the pair's caveat says it): the plume's width and glow (with the warm-up ours draws each
craft's ribbon and plume, but the original's is a wider, brighter bloom: on the same instant the whole-frame RMSE went from 0.219 cold to
0.254 with the warm-up on PSP and from 0.258 to 0.280 on HD, because a thinner bright streak in a different place costs more than none; the
number is not the measure of this change, the picture is), a ribbon that curves where the original's did, the Autopilot icon and its
counter (the pickup row), HD's `Absorb` and `DAMAGE` strips and its best-lap line, any weapon or effect in flight, and the scene objects
that animate (the billboards).

**The player's hull in a race shot.** Checked 2026-10-20 (`race-shot-trails`), the hull's size against the camera read at the same stop:
on **HD** (`hd-race/talons-pack`) ours matches: the hull is 170 px wide in a 560 x 420 crop of the 1920 x 1080 frame on both sides and starts at the same
row; holding the player back by one tick (2.4 units at 141 u/s) already makes it far too large, so there is no body/camera instant skew
there, and the "lower and larger" look was the original's bright plume and bloom around it. On **Pulse PSP** the original hull is
larger and lower than ours (about 1.5x across): holding the body back one tick of its velocity (1.8 units at 106 u/s) lands close to the original's size
and two ticks is already far too large, so on PSP the drawn hull matches a body about a tick older than the one read at the stop (or a camera a tick
ahead). The display matrix (`entity+0x8b0`, two hops) is not the cause of a position offset: its translation equals the body's to the
last digit for all eight craft, and its rotation rows are scaled by 0.75 with an up axis tilted about 10 degrees from the body's (a visual
roll). Not wired: ours keeps the body as read, with the skew recorded in the handover thread.

## A posed race shot was lit, culled and animated as if no tick had run (Pulse PSP, 2026-10-26, `talons-alongside`)

Three faults, one cause each, all found on `pulse-psp-race/talons-alongside` (Talon's Junction, inside the tunnel,
`--field-from --ticks 0`), whole-frame mean grey difference to `theirs.png` **14.14 to 3.29** (rmse 0.131 to 0.049):

| Fault | Where | Evidence | Boxes, ours over the original's mean luma, before to after |
| --- | --- | --- | --- |
| The hull drew at the default light scale `255` | `Race::ship_hull_scale`: `standing.course_index` is `None` until a tick runs | the original's recorded GE list holds the first directional at `(122,114,75)` against the open road's `(254,237,156)`, a scale of about `123/255` | fins 1.92 to 1.05, right fin 1.86 to 1.03, body 1.95 to 1.04, left fin 1.43 to 1.13, hull box with the plume 1.31 to 1.05, nozzle box 1.20 to 1.05 |
| The visible set was every section | `Race::visibility_sections`: `Ship::segment` is `0` until a tick runs, far from the pose, so the craft read `UNPLACED` | 2,102 draws submitted against 638 once located; the original's list holds none of Talon's `ground1`/mountain chunks, ours drew them over the tunnel's ceiling | upper-left box (0,0,140,90) 2.68 to 1.02, whole frame 1.13 to 0.96 |
| The neon strips on the left wall were dark | `psp-pulse-pair.py`: a race shot draws at tick 0 like Zone, but took `offset - GO_TICK / 60` (the grid formula) | the strips flicker on a 2 s square wave of the world clock: lit cyan pixels in (0,140,150,60) 0 against the original's 924; at `offset + HUD clock` 946 | left wall box 0.70 to 1.04, whole frame 0.96 to 1.00 |

The first two are render-side fallbacks (`Course::locate`, `Spline::nearest`, read only: the simulation's own fix stays as the
tick left it); a running race sets both on its first tick, so only a posed capture and a race's first frame were wrong. The section fallback is **tick 0 only**: after a tick, a craft that reads off its sample (off the track, airborne, under a crossing stretch) stays *draw everything*, since a global scan can take it to the wrong stretch (`respawn::a_craft_no_tick_has_placed_is_found_where_it_stands_and_only_then`). Tests
(each fails with its fix removed): `hull_light_scale_ground_truth::a_craft_no_tick_has_located_is_lit_by_the_tunnel_it_stands_in`
and `posed_visibility_ground_truth::a_craft_no_tick_has_located_is_in_a_section`. Checked on the other pairs rendered from
their recorded flags: the 24 grid pairs and both Zone pairs are unchanged to the printed digit (they run ticks), `talons-boost`
mean grey 11.25 to 10.57; the 19 HD, HD race, Omega and 2048 pairs are unchanged to the printed digit (the fallbacks are generic
`oag-raceplay` code, not gated per title: the same bug reached every title that poses a craft at tick 0, and nothing of theirs moved); Pure's four pairs read 9.2-25.9 stored against 10.1-27.2 now with **or without** this change (the stored
ones predate other commits; not this lane's).

**To adopt in the page** (a manifest or `pair.json` is never written from a worktree): `just compare-rerender-ours --only
pulse-psp-race` from the main checkout re-renders both race pairs; it now recomputes `--world-clock-offset` for a stored
race shot as `original.world_clock_offset_s + original.race_clock_s` (178.078 + 38.408 = 216.486 on this pair).

**Not the cause, ruled out with numbers:** `3ff1d331f` (`pulse-vex-normals`) moved 195 of 130,560 pixels by a mean of 0.018 grey
on this pair. For the missing upper-left geometry: back-face culling off (data level, all three lists), the frustum and PVS
flags, LOD and the animation clock changed nothing, because the set was already every section (the terrain was in front of
the ceiling, not the ceiling absent). The pair is **not boosting**: its flare `boost_timer` is 0 and `plume_timer` 8.29 at its own
clock and ours renders the same ([exhaust.md](../ghidra/functions/psp-pulse-usa/exhaust.md), "The race pair is not boosting").

**Method worth keeping: bisecting the original's frame.** `psp-ge-dump.py dump` then `psp-ge-replay.py` with `--nop-prims` ranges
of the census's `ci` chunk indices, one replay per range (about 12 s, the previous instance needs about 14 s to release its
port, so wait between runs). Run under `ulimit -v` for the memory cap: inside `systemd-run --scope ... -p TasksMax=64` every
replay came back black on 2026-10-26 (the cause was not isolated; direct runs and `ulimit -v 12000000` were fine). It kept a
list of 975 original PRIMs to compare with ours by world bounding box and index count; ours had a twin for every prim that
projects onto the upper-left, which is what pointed at a visibility difference rather than missing geometry.
PPSSPP's own toast ("turning off software rendering") lands in the top rows of some replays: leave rows 0-30 out of a box
metric. A replay of the whole list reproduces the original's frame; `--nop-prims 85:8242` style ranges that remove the
first PRIM (`ci` 75) blank it.


## A posed race shot had no contact sparks (Pulse PSP, 2026-10-26, `collision-cages-and-contact-sparks`)

`pulse-psp-race/talons-boost` shows orange spokes at x 222..262, y 118..152 of `theirs.png`: the
player's nose and the craft ahead's tail, a craft-to-craft hit five ticks before the frame (live read,
[contact-response.md](../ghidra/functions/psp-pulse-usa/contact-response.md#live-2026-10-26-a-craft-to-craft-contact-sparks-both-hulls-and-the-dispatch-has-no-distance-gate)).
Ours drew nothing there for two reasons: a pair contact threw no sparks at all (only wall contacts did),
and a posed frame runs no tick, so a hit that happened before the frame cannot happen in it.

- **Pair contacts spark now** (`Race::throw_contact_spark`), so a race that runs ticks shows them with no flag.
- **A posed capture is seeded, not re-simulated.** `scripts/psp-contact-fx-read.py` loads the state, runs
  to the pair's `--target-clock` (the pair script's own functions) and logs the original's
  `ShipCollisionFx_Trigger` calls over the last 0.6 s, keeping those whose instance gate had opened (the
  trigger's own spawn test). It prints each as a `--pose-spark SLOT:NODE:AGE_SECONDS:SEVERITY` flag with
  the slot already mapped to ours' `--field-from` row (by team name) and the severity from the call's
  clamped intensity: `--pose-spark 3:0:0.0831:0.7238 --pose-spark 0:2:0.0831:0.7238` for this state.
  `--pose-spark` (needs `--field-warmup`, whose straight-line velocities it uses) throws each spark that long
  before the frame and runs the effect stage on to it (`Race::warm_up_contact_sparks`); nothing else
  moves. A re-simulated lead-in was not tried: a posed craft already turned twice and left its line
  within 0.8 s (above), and a pair hit on the exact tick is unlikely to recur.
- **Result**, `data/scratch` re-render with the flags above, window `--world-clock-offset 217.421`,
  box (205,115,70,50): bright orange pixels (r > 170, 80 < g < 220, b < 140, r - b > 80) **0 before, 31
  after, 20 in the original**, centroid (240.6, 154.4) against the original's (236.0, 148.9). Read by eye
  at 4x over the four consecutive frames the pair script records (`shots/tick0000{0..3}.png`, ages 4 to 7 ticks): the
  burst is where the original's is and fades over the same ticks, but it is a soft orange glow on the hull
  edge where the original draws thin bright spokes 30 to 40 px long, and its glow is brighter. **Whole-frame
  mean absolute grey difference 9.38 before, 10.01 after; the box 16.95 before, 18.66 after: the pixel
  metric prefers no spark to ours.** Not the age (4 to 7 ticks all lack the spokes) and not the
  severity as measured (0.724; a larger one draws spokes but floods the box white). The particle draw's
  streak length against severity is the open lead (`oag_fx::psys`, not this lane's).
- **Not measured:** the sparks' age is the HUD clock between the call and the frame (5 ticks); the stamps
  read off the world clock say 6 to 10 ticks in two runs (the clock does not tick at exactly 60 Hz in the
  emulator), and the pair's stored `theirs.png` came from an earlier run than the reader's.
- To adopt: add the printed `--pose-spark` flags to `pair.json`'s `ours.flags` and re-render with
  `compare-rerender-ours.py` (not written from a worktree).

## Zone pairs (Pulse PSP, 2026-10-21, `zone-pairs`)

A Zone frame is a mid-run state, so `psp-pulse-pair.py --zone` renders ours at tick 0 on the original's pose and camera with the zone number, score and zone seconds read from the race manager's mode object and written into the race (`--hud-state PLACE,LAP,LAP_S,RACE_S,KMH,SHIELD,BEST,ZONE,SCORE,ZONE_S`); `g_game_mode == 6` is checked. The state is made by walking `Racebox` > `Custom Race` > `RACE TYPE` right x5 (one key a second), Track Select (two circuits on a used profile: Talon's White, Moa Therma White) and flying on the Autopilot. HD's `rpcs3-hd-pair.py --zone` exists but no pair was made; see the compare-page handover thread.

## Menu pairs: HD/Fury and Omega front-end screens (2026-10-26, `menu-pairs-hd-omega`)

A front-end screen is a still, so a menu pair has no pose, camera or race clock. What makes it a pair is the same screen,
the same size, language and styling, the same highlighted row, and **ours drawn from the disc's own front-end XML, never
from `assets/ui/menu.toml`'s stand-in pages**. `scripts/compare_menu.py` is the menu route (`menu_pair` builds and checks one,
`write_gap` keeps only the original's frame where ours has no XML-driven page); `pair.json` carries `kind = "menu"`,
`original.screen` (the name the emulator reported), `ours.page`, `ours.drawn_from` (`xml`, `menu.toml` or `none`, with the file)
and both languages and stylings. `Pair.finish` refuses a menu pair when ours is not XML-driven, the original's screen name is
not the one asked for, the languages differ, the highlighted row differs (a state ours cannot select ships as a named
`Not matched` caveat instead, `selection.matched = false`), or a size is off. **What is read and what is declared:** only
HD's screen name (`TTY.log`) and the original's RPCS3 language are read; the highlighted row of both sides is read by eye and
declared in `WALK`, ours' language is the constant it runs with, and Omega's screen name is declared on both sides. So those
checks catch a `WALK` entry that names two different rows, not a cursor that drifted: the pair images are the evidence. Provenance kind `menu` needs no presented-frame
lag (`compare_provenance.refusals`); an HD menu pair still needs a cold boot, which a walked boot is. Tests:
`check-compare-environment.py` (`check_menu_route`) and `compare_provenance.py selftest`.

| Title | Script | How |
| --- | --- | --- |
| HD/Fury (RPCS3) | `scripts/rpcs3-hd-menu-pair.py` | a cold-booted `rpcs3-drive.py serve` (`OAG_COMPARE_PLATFORM=hd-fury`) on `Main Menu`, then `OAG_RPCS3_ATTACH=1 python3 scripts/rpcs3-hd-menu-pair.py --out D --oag-game target/release/oag-game --autopilot off`. It walks `WALK` (about 110 s), the window at 1920x1080 at the origin, 3 s settle before each grab. The screen name is HD's own `Switching Screen` line in `TTY.log`. `scripts/rpcs3-hd-menu-walk.py --plan "shot:a,cross,..."` photographs any plan for exploring |
| Omega (shadPS4) | `scripts/shadps4-omega-menu-pair.py pairs` | the `Rig` of `shadps4-omega-pair.py` (own headless `sway`, uinput pad, muted), cold boot to MAIN MENU in about 20 s, `WALK` in about 95 s. **The pad input reaches the game** (the 2026-10-26 worry that it did not was not reproduced: all four walks of this lane reached Ship Select). shadPS4 has no `TTY.log`: the screen is read by eye from the frame's SCREEN TITLE and declared in `WALK`, and `original.screen_basis` says so |

**What ours draws from XML and what from `menu.toml`** (read off `crates/game/src/capture/menu_page.rs`, `campaign_page.rs`,
`oag_hd::main_menu` and the `--screen` list; `--menu-page` is the only still route and `--screen` reaches only the boot
screens, `Main Menu`, `Team Selection` and the rest are not among its names):

| Screen (HD name) | Ours | Pair |
| --- | --- | --- |
| `Campaign Selection` | XML (DATA06 copy), HD only; Omega refuses (`this source has no Campaign Selection screen`) | HD yes, Omega **gap** |
| `Grid Selection Fury`, `Cell Selection` | XML (`grid_00.xml`, `CellMode_Definition.xml`); on Omega both `grid-select` and `grid-select-hd` draw Omega's own first grid (18 points, `01_TRACK`, no flyer card), which is none of the three campaigns the original offers | HD yes, Omega pair with the state caveat |
| `Track Creation` (Track Select) | XML (`Track_Selection_Definition.xml`) | both |
| `Team Selection` (Ship Select) | XML (`Team_Selection_Definition.xml`) | both |
| `Main Menu` | `menu.toml`'s `main` page with the disc's rows (`oag_title::MainMenu`, ADR-0066): title `OPENANTIGRAV`, row text from `english.toml` | **gap** |
| `Single Player` (Racebox setup) | `menu.toml`'s `race` page | **gap** |
| `Additional` (OPTIONS+) and Options | `menu.toml`'s `options` pages | **gap** |
| `Race Records` | this project's records page | **gap** (original seen, not kept) |
| pause menu | a `Stage::Menu` of `menu.toml` over the held frame; there is no `--menu-page` for it | **gap** (original not captured: needs a Racebox race) |
| title / press start | neither title has one: HD goes `Studio Logo`, `Save Warning`, `Main Menu` (`--until "Studio Logo"` draws ours), Omega reaches MAIN MENU about 18 s after launch with no press | not paired: boot-time animation, no settled frame |

`front_end_style` (`display.front_end_style`, HD offers `HD` and `FURY`) is pinned to `FURY` in ours' profile for HD, and read
back; the original's frames are Fury (red and black, the Fury backdrop). It only affects the loading screens in ours today
(`args::style_of` has no other reader), so the pages' own look does not follow it. Omega offers no choice. English on both
sides: RPCS3 `System: Language English (US)`, shadPS4 `console_language 1`, ours the default.

**Confounds, not fixed.** The RPCS3 profile is a copy of the maintainer's, so the save's `Safe Area Setting` is 96 and the
default team Feisar and first circuit Vineta K are the save's; ours is set to them (`race.team`, `race.track` in the pair's
private `settings.toml`). Omega is a fresh profile: most craft are locked and the cursor of Ship Select sits on a locked
hex, so ours cannot be put on the craft the original shows (caveat on the pair). The animated backdrop is not phase-matched:
two grabs of the original differ by 0.01 to 0.11 normalised RMSE (`theirs` against `theirs`, per pair), ours by exactly 0 run
to run (the pair's overlap moved by at most 0.003 RMSE, 0.8 mean grey between two runs).

**Layout, measured on the footer and title rules** (rows and columns of bright thin rules, 1920x1080 frames): HD original rule
`x 192..1727` (1536 px), title rule `y 131.5`, footer rule `y 961`; ours `x 160..1704`, `y 114.5` and `y 978`. So ours is
about **27 px left** of the original's centre, 0.5 % wider and **4.1 % taller** (863.5 px between the rules against 829.5),
centred vertically on the same line. Omega's original: `x 161..1759`, `y 112.5` and `948`; ours `x 161..1759`, `y 114.5` and
`937`, so the columns match exactly and ours is 1.6 % shorter. One pair of rules per title, two pages each (the same numbers
on `grid-select` and `track-select`); confidence 75.

**The pairs** (run 2026-10-26, overlap `RMSE / edge correlation / mean grey`, 1920x1080, ours `--menu-page` at the same size;
the page shows the same numbers per pair). The most visible difference is read off the pair image:

| Pair | Ours drawn from | Overlap | Most visible difference |
| --- | --- | --- | --- |
| HD Campaign Select | DATA06 `Campaign Selection` | 0.232 / 0.07 / 27.7 | (the HD card's dimmed grey-green is drawn since 2026-10-27, whole frame 28.2 to 9.4); the original's gold-medal hexagon icon is missing and `CAMPAIGN MODES` is a larger plain heading |
| HD Campaign grid | `grid_00.xml` | 0.196 / 0.10 / 15.0 | the flyer card: the original tilts it with two large arrows and draws the previous and next grid arrows; `EVENT 01/08` is bold capitals there, ours small mixed case |
| HD Campaign cell | `CellMode_Definition.xml` | 0.189 / 0.10 / 16.7 | closest pair; the selected hex has the white outline in the original, ours is flat grey; the original has a red particle haze right of the card |
| HD Track Select (Vineta K) | `Track_Selection_Definition.xml` | 0.184 / 0.02 / 21.6 | no circuit photo, no length or distance values (`4.4KM`, `13.2KM` read `-`), the circuit model is smaller and turned differently, the original's red backdrop cloud is much fainter in ours |
| HD Ship Select (Feisar) | `Team_Selection_Definition.xml` | 0.196 / 0.08 / 22.0 | the hull is drawn at a different pose and size, the red particle backdrop is much fainter, the statistic bars carry red markers the original lacks |
| Omega Campaign grid | Omega's own first grid | 0.397 / 0.27 / 55.0 | state not matched: no flyer card at all (a white page with the ghosted backdrop tower), 18 points against the original's 21 |
| Omega Campaign cell | Omega's own first grid | 0.658 / 0.10 / 129.0 | the original's dark-red panel fills about 80 % of the frame and ours is white, the largest difference of the set; `01_TRACK` against `TALON'S JUNCTION` |
| Omega Track Select (Vineta K) | `Track_Selection_Definition.xml` | 0.204 / 0.27 / 19.7 | circuit name `01_Track` (the unresolved name of Omega's entry: the load line names `Data\Environments\01_Vineta_K\screen.xml`, so the circuit is Vineta K) and no photo (the original names it and shows the stills), no length values, the model is turned differently |
| Omega Ship Select (Feisar) | `Team_Selection_Definition.xml` | 0.283 / 0.26 / 34.0 | hull about twice the original's size in a different livery (dark teal and yellow against pale blue and yellow), a ringed Feisar logo against the flat one, the cursor on the top hex against the third row |

Gaps (no pair, the original's frame kept in `gap-*` beside the pairs with `gap.json`): HD Main Menu, Racebox setup and Options+
(`Additional`); Omega Campaign Select, Main Menu, Racebox setup and Options. The ranked list of differences is in the
`menu-pairs-hd-omega` handover thread. Adopt: the pair directories are under `data/reference/compare-page/{hd,omega}/menu-*`;
the manifest is built from the main checkout with `python3 scripts/compare_env.py manifest
data/reference/compare-page/menus.toml data/reference/compare-page/hd/menu-* data/reference/compare-page/omega/menu-*`
(9 entries; **name the `menu-*` directories**: `manifest` walks every `pair.json` under a directory and `hd/` and `omega/`
also hold the race pairs of `fresh.toml` and `omega.toml`, which would appear twice), then `just compare-page`.

## Menu pairs: Wipeout 2048 front-end screens (2026-10-27, `menu-pairs-2048`)

The Vita3K counterpart of the HD and Omega menu route (`scripts/vita3k-2048-menu-pair.py`, on `vita-2048-pair.py`'s `Session` and
`vita3k-drive.py`'s display, launcher and xdotool input; `scripts/compare_menu.py` gained `route=`, the oag-game arguments that
draw ours when the title has no `--menu-page` tree). 2048's front end is a touch front end (ADR-0054) with no `menu.toml` page behind
it, so **ours is drawn through `--until STATE --press BUTTONS`** (the sequence: `--press` is one mask held on alternate ticks, see
`docs/formats/2048-frontend.md`) from the disc's own XML: `Skin.xml` and `Definition.xml` screens (`TitleScreen`, `GameModeChoice`,
`Home`, `newFEshell`), `SP.xml`'s events, `Team_Definition.xml`. `--screen NAME` draws one screen on black without the shell
around it, so it is not a comparable still. A cold-booted Vita3K on a **fresh save** (the profile's savedata removed before each
boot) and a fresh ours profile; 960x544 on both sides, English (`sys-lang: 1`), Vita3K muted (`SDL_AUDIODRIVER=dummy`,
`audio-volume: 0`), its own weston and Xwayland on `:91`. Vita3K has no screen log: the screen of each frame is **declared** in
`WALK` and read by eye, the walk itself waits on frame luma and the logo (`pu:` steps), because the first Return of a cold boot is
swallowed while shaders compile and a fixed-sleep walk ran one screen behind (both boots of the first attempt).

**Campaign difficulty.** The brief asked for the same campaign difficulty on both sides. 2048's map and event card show **no
difficulty control** on either side (the card reads `PASS FINISH THE EVENT`, a class, three laps and weapons-off icons), the walk
never touched one, and ours wrote no `records.toml` (so no campaign difficulty row exists for 2048 there; `[ai] difficulty` in
`settings.toml` is the race AI level, `elite` by default, which neither screen shows). Nothing to match; recorded rather than set.

**The pairs** (two cold boots, `overlap RMSE / edge correlation / mean grey`, then the share of pixels that differ by more than 40
grey, run 1; the pair images were read side by side):

| Pair | Ours drawn from | Overlap | Differs | Most visible difference |
| --- | --- | --- | --- | --- |
| Title | `TitleScreen` entry (`Skin.xml`) | 0.237 / 0.19 / 22.9 | 11.0 % | state not matched: ours stops on the tick the screen is entered, a white frame with the legal line, no logo, no `PRESS ANY BUTTON TO START`, no triangle pattern |
| Game Mode | `GameModeChoice` | 0.308 / 0.33 / 36.3 | 19.3 % | ours adds a RACEBOX / REMIX / MULTIPLAYER column the original does not show, a check mark, and rings Single Player Campaign; the original has the header icon and its underline, ours a plain navy square; ours' tile rows sit about 14 px lower |
| Campaign map | `newFEshell`, `SP.xml` events | 0.203 / -0.01 / 21.7 | 12.6 % | the original's map is a scrolled hexagonal canvas with the `A.G.R.C 2048` banner, the `TOUCH TO START` tag, the home button, three badges and two corner buttons; ours is bare hex nodes on a different layout (known: the cell-to-pixel mapping is chosen) |
| Event card, page 0 | `newFEshell` card | 0.140 / 0.39 / 10.5 | 4.9 % | closest pair; the subtitle reads `RACE` against `NO WEAPONS`, the map shows through ours' card instead of a blurred white ground, and ours has no home button or badges |
| Event card, page 1 | `newFEshell` card | 0.146 / 0.41 / 9.5 | 4.3 % | as page 0; the `CURRENT BEST` bar is orange in the original and grey-blue in ours |
| Home | `Home` | 0.102 / 0.56 / 10.1 | 3.8 % | nearest of all; the header icon and underline are missing, the original's backdrop is a blurred map, ours rings the Team tile |
| Select Team | `Team_Definition.xml` | 0.495 / 0.05 / 94.9 (run 2: 0.511 / 0.06 / 99.6) | 56.7 % | the largest: the original is a 5x4 grid of locked hexagons under the five team badges, the hull turntable and a statistics table (`FEISAR SPEED`, 8, 7, 4, 6); ours is two rows of labelled buttons and a skin list of `FE_SKIN_*` ids (chosen stand-ins, `docs/formats/2048-frontend.md`) |

**Repeat noise** (run 1 against run 2, cold boots): the pairs' overlap moves by at most 0.001 RMSE and 0.4 mean grey on every pair
but Select Team (0.016 RMSE, 4.7 grey: the hull's turntable). The original against itself: RMSE 0.000 (Game Mode) to 0.028
(map) and 0.134 (Select Team, the turntable); ours against itself exactly 0 on all seven.

**Gaps** (original kept under `gap-*` with `gap.json`): `PSN prompt` (the system's dialog over Game Mode), `Campaign welcome
panel` (`SPTutorial`, the first visit to the map; ours draws it only through `--screen` on black), `Options`, `Profile` and
`Extras` (ours draws them in the live flow but has no headless still route past Home's first tile, and a windowed `--script` run
is a 1440x816 window at llvmpipe speed, so no same-size frame), and the **pause menu** (`GAME PAUSED`: Options and Photo Mode
tiles, the event card, quit, retry and resume; ours' pause is `menu.toml`'s). Single Race setup and Track Select have no 2048
counterpart in either side's front end: `--menu-page track-select` and `ship-select` refuse on 2048 (`Wipeout 2048 authors no Track
selection screen`), and the campaign's circuit is the event's. The Racebox, Remix and Multiplayer rows of ours' Game Mode column are
`oag-extras` rows (ADR-0066), absent in the original.

**Omega and HD check:** checked, differs - Omega's front end is HD's `PI001` XML (not 2048's touch front end), so none of these seven pairs
transfers to Omega's, and HD's `CellMode` route (`campaign-select`, `grid-select`) refuses on 2048 as it does on Omega.

**Reproduce:** `source <scratch>/env.sh` (HOME, `OAG_VITA3K_CACHE`, `OAG_VITA3K_CONFIG`, `OAG_VITA3K_DISPLAY`, ours' `XDG_*` under
`data/scratch/<lane>/`), the private config's `pref-path` a copy of the Vita3K tree with its `ux0/app` and friends symlinked and
`savedata/PCSF00007` removed, then `uv run --with evdev python3 scripts/emu-run.py --silence 200 --total 270 --status S -- python3
scripts/vita3k-2048-menu-pair.py pairs --out D --oag-game target/release/oag-game --autopilot off [--pause-png P]` (about 110 s;
`walk --plan "wait:8,pu:Return:title,..." --keep` photographs any plan, `--attach` reuses the running Vita3K). The pause frame is a
`walk` of `PAUSE_PLAN`. Adopt: name the `menu-*` directories, `python3 scripts/compare_env.py manifest
data/reference/compare-page/menus-2048.toml data/reference/compare-page/2048/menu-*`, then `just compare-page`.

## Menu pairs: Wipeout Pulse and Wipeout Pure (PSP) front-end screens (2026-10-27, `menu-pairs-pulse-pure`)

The PPSSPP counterpart of the HD, Omega and 2048 menu route (`scripts/psp-menu-pair.py`, on `compare_menu.py`). The software
renderer at `InternalResolution = 1` (`compare_env.py ppsspp-ini`), the USA pressings the environment file pins (`compare-environment.toml`
names `pulse-psp-usa.chd` and `pure-psp-usa.chd`; the EU preference does not override a pinned platform), muted, own Xvfb and
debugger port, a **fresh memory stick** (Pulse's first-boot dialogs answered by `psp-drive.py`'s `FIRST_BOOT` table, Pure's profile made
with `New`). The frame is the emulator window at 480x272 (`xdotool windowsize`), grabbed off the Xvfb root and cropped at the window
origin: no scaling at any step. Ours is `oag-game --menu-page PAGE` (or `--screen`) at 480x272 in a private profile
(`prepare_profile`): the compare render settings and **`[dev] unlock_everything = false`** (a debug build unlocks everything, the original's
fresh profile does not; read back in the settings file the run rewrote). Pulse also gets `records.toml`'s `campaign_difficulty = medium`
row, which the still **ignores** (see the campaign paragraph).

**What is read and what is declared.** Pulse's screen name is read from the game's own state machine (`0x08b31784 + 0x18c`, the same
`Debugger.state_name` the race pairs use); every Pulse pair's `original.screen_basis` says so. Pure has no recovered state name: its
screens are declared in `PURE_WALK` and read by eye (`original.screen_basis`), and the walk waits on a frame predicate (`pu:cross:title`,
the orange logo) because the language and memory-stick screens swallow a fixed-sleep walk. The highlighted row of both sides is read by
eye and declared (`selection`), so the check catches a `WALK` entry that names two rows, not a cursor that drifted; the pair images are
the evidence.

**What ours draws from the disc and what from `menu.toml`:**

| Screen (the original's name) | Ours | Pulse | Pure |
| --- | --- | --- | --- |
| title, `Show Logo` / `Title Screen` | Pulse `--until "Show Logo" --press cross --ticks 200` (the sequence draws the ray backdrop; `--screen "Show Logo"` leaves it out: 14.9 mean grey, 6.0 % against 4.7, 2.6 %), Pure `--screen "Intro Screen->Title Screen"` | pair | pair |
| `Grid Selection`, `Cell Selection` (the Race Campaign) | `--menu-page grid-select`, `cell-select` (`CellMode_Definition.xml`, `grid_00.xml`) | pair, pair | none: Pure draws a list (`this source has no Race Campaign to show`) |
| Pure's Single Player chain: `Single Player`, `Class Selection`, `League Selection`, `Tournament Selection`, `Track Selection` | `--menu-page pure-chain[:MODE[:ROW...]]` | none | pair x6 |
| `Track Creation` (Track Select), `Team Selection` | `--menu-page track-select`, `ship-select` | pair, pair | `Team Selection` pair; Pure's circuit list is `Track Selection` above |
| `Main Menu`, `Racebox`, `Single Player` (Racebox setup), `Additional` (Options+), pause | `menu.toml`'s `main`, `race`, `options` pages; the pause menu is a `Stage::Menu` with no still route | **gap** x5 | **gap** x2 (`Main Menu`, pause; Pure's main page has no Racebox or Options row) |

**The campaign pair (required).** Pulse: the Race Campaign's Grid Selection (`GRID 1`, the first tier, cursor on tier one) and **Cell
Selection** (`SINGLE RACE`, `Moa Therma White`, the default cell), both with the footer's `AI difficulty (Medium)`. The difficulty
control is the footer's; the original's fresh stick opens on Medium and ours' default is Medium, so the frames agree. **`--menu-page
cell-select` does not read the persisted difficulty on Pulse**: with `records.toml` set to `easy` and to `hard` (title key `wipeout pulse`)
the footer still reads Medium, because `capture/campaign_page.rs` applies `with_opening_difficulty` only in its HD branch (checked
2026-10-27; the live flow reads it). So the pair matches by both sides' default, not by a set value, and a non-default rung cannot be
made to match until the still reads the row. Pure: **`Tournament Selection`** (Alpha, the only tournament open on a fresh profile, class Vector) after `Class Selection`,
the original's route Single Player -> Tournament -> Vector. Pure shows **no difficulty control** on either side (the class is the speed
setting), so there was none to match. The single-race path (`Class`, `League`, `Track`) is paired as well.

**The pairs** (`overlap RMSE / edge correlation / mean grey`, the share of pixels differing by more than 40 grey, run 1; each pair image
read side by side):

| Pair | Overlap | Differs | Most visible difference |
| --- | --- | --- | --- |
| Pulse Track Select | 0.180 / 0.26 / 19.4 | 16.2 % | the circuit photo (stacked cards, a different view), `1/3` against `1/11` (ours shows 11 circuits; checked 2026-10-27: it is the same under `race.mode` `time_trial` and `single_race`, and `[dev] unlock_everything = false` survives in the rewritten settings, so the cause is open), distance `5178` against `5094`, the empty records `-.--.--` against `-` |
| Pulse Ship Select | 0.129 / 0.34 / 9.5 | 7.1 % | the Assegai hull: the original's blue-white livery large and tilted, ours a dark livery smaller; `1/8` against `1/12` |
| Pulse title | 0.069 / 0.30 / 4.7 | 2.6 % | the type: the original's wide `PRESS START BUTTON` and legal line against ours' small mixed-case `Press START button`; the rays are at another phase (animated) |
| Pulse Grid Selection | 0.076 / 0.35 / 4.2 | 2.1 % | the cursor hex glows in the original, plain in ours; the footer ticker scrolls in the original and is empty in ours at tick 0 |
| Pulse Cell Selection | 0.077 / 0.39 / 4.3 | 1.8 % | closest of all Pulse: the same glow and ticker only |
| Pure Team Selection | 0.101 / 0.29 / 6.9 | 8.6 % | ours lists all 17 teams in small type (the original's fresh list is eight, large; the cause is open), the hull outline from another angle, the stat labels run into their values |
| Pure Track Selection (single race) | 0.092 / 0.40 / 5.0 | 5.5 % | ours adds the `SINGLE RACE - VENOM - ALPHA` context line under the heading (chosen, not measured, `docs/architecture/menus.md`), the records read `0.00.00` against `0'00"00`, lower-case unit `m`, no description line |
| Pure Single Player (mode) | 0.074 / 0.40 / 3.0 | 3.4 % | no context line here; the hint line and the `BACK` prompt differ (below) |
| Pure Tournament Selection (campaign) | 0.069 / 0.44 / 2.7 | 3.1 % | the context line `TOURNAMENT - VECTOR` (chosen), no description line, the footer prompt |
| Pure League Selection, Class Selection (x2) | 0.066-0.067 / 0.44 / 2.4 | 2.8-2.9 % | the same three: context line, no description line, footer |
| Pure title | 0.017 / 0.60 / 0.5 | 0.3 % | none visible (the cursor underscore blinks) |

Three differences recur on every Pure chain screen: the **context line** (ours' `SINGLE RACE - VENOM`, a known chosen stand-in of
`docs/architecture/menus.md`, absent from the original), the **description line** along the bottom (the original types it in; ours is
drawn settled, 5 s in, `ARRIVED` in `pure_chain_page.rs`, and still shows none, so it is a missing element, not timing) and the
**footer's BACK prompt** (the original shows a triangle and `BACK`, ours `BACK BUTTON` drawn over itself). Pulse's pairs share the footer
ticker (scrolling in the original) and the cursor glow.

**Repeat noise** (run 1 against run 2, cold boots on Pure; Pulse run 3 against run 1, two boots on one instance and a fresh one, the title two further boots each): ours against itself exactly 0 on
every pair. The original against itself: Pure 0.0 mean grey and at most 0.06 % of pixels differ over 40 on every screen (the walk is
deterministic); Pulse 0.7-4.7 mean grey, 0.1-3.5 % over 40, the Ship Select turntable and the footer ticker (Track Select's photo cards
2.5 %, the title 0.0 %).

**Gaps** (original kept under `gap-*` with `gap.json`): Pulse `Main Menu` (ours' page is `menu.toml`'s with extra REMIX, RECORDS and MODS
rows, title `OPENANTIGRAV`), `Racebox`, `Single Player` (the Racebox setup, ours' `race` page with different rows), `Additional`
(Options+: GAME OPTIONS, CONTROLS, MUSIC PLAYLIST against ours' `options`) and the pause menu (`GAME PAUSED`: CONTINUE, GAME OPTIONS,
PHOTO MODE, GAME MANUAL, RESTART RACE, QUIT RACE; a Time Trial restored from a savestate, Start pressed); Pure `Main Menu` (SINGLE PLAYER,
MULTIPLAYER, PROFILE, DOWNLOAD on the white frame, against ours' dark teal `menu.toml` page) and the pause menu (CONTINUE, RESTART, QUIT
over the dimmed race). Pure has no Options or Racebox screen in its main menu.

**Reproduce:** `scripts/ppsspp-start.sh <lane> 91 <port> <image>` with `PPSSPP_EXTRA_INI` from `compare_env.py ppsspp-ini pulse-psp|pure-psp`,
`source <scratch>/pp/env.sh`, ours' `XDG_*` under `data/scratch/<lane>/`, then `uv run --with websocket-client --with pillow python3
scripts/psp-menu-pair.py pairs --title pulse|pure --out D --autopilot off --difficulty medium|""` (about 90 s on Pulse from a walked
`Main Menu`, about 3 min on Pure from a cold boot; `walk --plan "..."` photographs any plan, `--ids` restricts a run; Pulse's title is a
separate boot: `--plan "st:Show Logo,wait:5,shot:title" --ids title`). Adopt: `python3 scripts/compare_env.py manifest
data/reference/compare-page/menus-pulse.toml data/reference/compare-page/pulse-psp/menu-*` (and `menus-pure.toml` over `pure-psp/menu-*`),
then `just compare-page`.

## Provenance: the trust model (2026-10-21, `ref-provenance`)

A pair is only as good as the original frame in it, and three things made a bad original look fine on
2026-10-21: HD originals restored from an RPCS3 savestate paint garbage on glass
([rpcs3-capture.md](../reverse-engineering/rpcs3-capture.md), "Savestate restore paints garbage on glass"), an
armed Autopilot showed `1000000000.00` on the original's HUD, and emulators present frames late. So every
`pair.json` carries `original.provenance` (`scripts/compare_provenance.py`), written by the pair script:

| Field | Records |
| --- | --- |
| `capture_path` | `cold_boot`, `savestate_restore` or `unknown`, and the state file |
| `presented_lag` | `value`, `unit` (ticks or seconds), `basis` (`measured`, `platform_measurement`, `assumed`, `unknown`), `what` |
| `autopilot` | `armed` (true, false or `"unknown"`), `timer_written`, `timer_authored`, where it was read from |
| `ram_writes` | every memory write made before the grab (the state's making included), `ram_writes_complete`, and `pair_script_ram_writes` (the pair scripts write none; they only read) |
| `emulator` | name, version, a hash of the settings recorded |
| `twin` | the RMSE against the previous capture of the same state; `agreed` stays `null` because no threshold is documented |
| `pair_kind` | `scenery` (grid, Time Trial), `race` or `zone` |

Each refusal is one more `pair.check`, so a pair that breaks one exits 3 like every other mismatch, and
`manifest` skips it:

| Rule | Refuses |
| --- | --- |
| `hd_savestate_scenery` | an HD `scenery` or `zone` pair whose original is not a `cold_boot`; the HD `race` pair is the labelled exception |
| `autopilot_timer` | an armed Autopilot whose timer was not the authored one, and a `race` or `zone` state (always flown on the Autopilot) whose arming was not recorded |
| `unknown_lag` | a presented-frame lag that is missing or `unknown` |

An override is `--accept-provenance RULE=REASON`, written into `provenance.overrides` and as a caveat; a reason is
required. The state-making flags are `--autopilot off|authored|timer`, `--autopilot-timer SECONDS` and
`--ram-write TARGET=VALUE:WHY`, all recorded in `argv`, so a re-make repeats them. `compare-rerender-all.py` tells
`rpcs3-hd-pair.py` how the emulator was brought up through `OAG_HD_CAPTURE_KIND`; unset reads `unknown`.

A record made before the field is backfilled from its `pair.json`, its `argv`, its state sidecar and this page, and
what none of them says is `unknown` (`compare_provenance.py backfill SRC DST`, on a copy; `table DIR` prints the
census; a pair that already has a block keeps it). The HD grid pairs are backfilled as `savestate_restore` by inference
(their state is a `SAVESTAT`, `compare-rerender-all` restored it, and the compare-page thread's own open item is to
recapture the 13 cold), and each is refused until it is recaptured cold (`scripts/rpcs3-hd-cold-pair.sh` sets
`OAG_HD_CAPTURE_KIND=cold_boot`; the restoring scripts set `savestate_restore`).

**Paths.** No `pair.json` or manifest names another lane's worktree or a lane's scratch directory: a worktree and the main
checkout are written as the repository-relative path, scratch as `{scratch}/` (`Pair.finish` and `compare-rerender`
do it; `compare_provenance.py portable DIR` does it to a copy), and manifests hold paths relative to their own
directory. A worktree is reaped after its merge and a manifest that named it broke the page. `just
check-pair-provenance` (in the gate) runs the refusal rules against constructed bad records (`selftest`), scans
`$OAG_PAIR_DIR` (default `data/reference/compare-page`; skipped, loudly, when absent, as in CI) for missing
provenance or a reaped path (a broken rule is listed as a note there, `scan --strict` fails on it: a pair on file may
be an accepted exception, a pair script never makes one silently). `check-docs` already refuses a tracked file that
cites a disposable `data/` directory; it also refuses one that names another lane's worktree. A re-made
`argv` that names `{scratch}` is refused by `compare-rerender` until that flag is remapped.

## Publishing is locked

`just publish-compare` uploads the built `data/compare/` with `wrangler pages deploy`
to a **separate** Cloudflare Pages project (`openantigrav-compare`, never the main
site's `openantigrav`; the script refuses if the two names are ever equal). It runs
only from the workstation, never in CI, and nothing in `.github/` calls it.

It **refuses to run** unless the screenshot-policy ADR,
[ADR-0074](../architecture/adr/0074-curated-screenshots-may-be-published-no-content-may-be-shipped.md),
has a `## Status` line that **starts with** `Accepted` (markdown emphasis ignored). Its number is one
constant, `POLICY_ADR` in `scripts/compare-page.py`. A missing or non-Accepted ADR prints the reason and
exits 2. The lock is checked before `wrangler` is looked up, so the refusal is what you see.
`just publish-compare <dir>` rebuilds the page strictly from the curated manifest directory it is
given first, so a missing `platform` refuses it before anything uploads. The directory is required:
the seed and live manifests are developer material, ADR-0074 wants every published image picked by
a person, and a default would publish them. `python3 scripts/compare-page.py --check-publish-lock` prints the verdict without publishing; with
ADR-0074 Accepted it reads `unlocked`. The maintainer runs the command.

A curated page should carry only frames the policy covers; the seed and live
manifests are developer material, not a publishing set.
