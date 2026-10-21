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
5 s on Pulse and HD): `psp-pulse-pair.py --field|--zone` writes it with the CPU stopped (`restore_autopilot_timer`, the moment
is exactly the state's), `rpcs3-hd-pair.py --field` through `rpcs3-drive.py capture --autopilot-timer 5` (the game then runs
0.3 s so the HUD redraws, the frame, pose, field and HUD record are from that moment). The HUD shows a pickup's own countdown
(`5.00`, `4.95`), the manifest carries a caveat, `pair.json` records the timer before and after, and a pair whose timer at the stop
is still above 600 s is refused. A run to let the pickup expire instead was tried on PSP and rejected: after 0.8 s the craft had
left its line and the pack was gone. Known and not fixed here: the HD frames carry RPCS3's "Press and hold the START button to
resume" toast (the pause the restore holds), and a PSP state loads a different number of emulated frames before the stop on
a slower instance (a Zone 8 frame moved from zone second 1.5 to 2.1, where its HUD is mid-fade).

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
| HD/Fury | 1920x1080 | RPCS3 Vulkan, `Resolution 1280x720`, `Resolution Scale 150`, `Output Scaling Mode Nearest`, `MSAA Disabled`, anisotropic override 0, `Write Color Buffers: true` in `config.yml` **and** the game's custom config | `--size 1920x1080`, aspect `wide` |
| Pulse PS2 | **1280x720, a chosen pick** (the maintainer asked for 720p; the game's own `Aspect Ratio` option is on `16:9` in the state and PCSX2 shows it at 16:9, so 1280x720 is the whole frame; adjustable in the profile) | PCSX2 OpenGL hardware renderer (the software renderer needs a Vulkan present queue Xvfb does not give), `upscale_multiplier 2`, `ScreenshotSize 1` (internal, aspect corrected), `AspectRatio 16:9` (1820x1024), then ONE Lanczos resize to 1280x720 | `--size 1280x720`, aspect **`wide`** (it was `ps2`, which is 4:3 and pillarboxes ours inside the 16:9 frame; 2026-10-21), rendered natively |
| Omega | 1920x1080 | shadPS4, no race reached, no pair script | not run |
| 2048 | 960x544, the Vita's own | Vita3K 1x, no race reached, no pair script | not run |

**Ours is the same on every platform** (`[ours_defaults]`): every render setting with an `original` value is set
to it (shadows `original`, model and texture detail `original`), upscaler, FSR, SMAA, MSAA, motion blur and screen
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

## Capturing a pair per title

Every recipe runs in its own display and data directory, muted, and is stopped by PID. A pair is only a pair when
its `pair.json` has no failed check.

| Title | Status | What matched | How |
| --- | --- | --- | --- |
| Pulse PSP | **works**, about 1.5 min per circuit (a mid-race **race shot** with the whole field: see "Race shots" below) | everything in the checks above | `PPSSPP_EXTRA_INI=<file from compare_env.py ppsspp-ini pulse-psp> scripts/ppsspp-start.sh <lane> <display> <port> <image>`, then `scripts/psp-pulse-pair.py --port P --home $PP_HOME --out D --track NN_Track --state <ppst>` (it resizes the SDL window to 960x544 for `psp-state.py`'s pause-menu clicks, loads the state, stops the CPU, resizes the window to the native 480x272 and crops the Xvfb root at the window origin: nothing is scaled), or `scripts/compare-pulse-psp-batch.py --out D` for all 24 kept states. No state yet: `psp-drive.py menu --unlock --track-down N --any-track` (about 105 s from a fresh profile, N from [the table](../reverse-engineering/ppsspp-debugger.md#every-circuit-not-three-the-dev-unlock-byte-2026-09-29)), then `psp-state.py save 1 --keep`. Full pipeline: [frame comparison](frame-compare.md) |
| HD (PS3) | **works**, about 1.5 min per circuit | pose, camera, fov, clock | `export OAG_COMPARE_PLATFORM=hd-fury` before `scripts/emu-restore-state.sh` (so `rpcs3-drive.py` writes the profile's video settings into the private config **and** the game's custom config), restore the state with `--restart`, then `OAG_RPCS3_ATTACH=1 scripts/rpcs3-hd-pair.py --out D --team harimau --state <state>`: it resizes RPCS3's window to 1920x1080 (the default 1760x1072 window carries black bars), captures, and checks the frame is exactly 1920x1080. One state per circuit: `scripts/emu-hd-timetrial-state.sh <lane> <circuit-index> <team-rights> <classic 0\|1> <name>` (Track Select index 0 is Vineta K, 8 Talon's Junction; Harimau classic is 9 `right`s and a `down`). The state only names the circuit and craft: **the pair is taken after `Restart Race`, a few seconds into a fresh Time Trial**, so the clock is small and the double-counted resting turn small. `--team` carries HD's variant (`Assegai_c1` is the Fury Concept craft). |
| Pulse PS2 | **works for Moa Therma White, camera matched** (widescreen) | pose, clock, circuit, layout, the state's `Aspect Ratio 16:9` (read back from the projection), the camera's look direction at the craft, PCSX2's `AspectRatio` | PCSX2 booted by `pcsx2-drive.py boot` with `OAG_COMPARE_PLATFORM=pulse-ps2` in the environment (its ini carries the profile's upscale, screenshot size and `AspectRatio 16:9`), then `scripts/pcsx2-pulse-pair.py --slot S --state <p2s with DAT_00284fe8 = 1> --track 03_Track --out D`. Pose, craft, clock and **camera** (`pcsx2_state.camera`: eye, axes and fov of the object `Camera_SubmitScene` runs on, [camera.md](../ghidra/functions/ps2-pulse-eu/camera.md)) are read out of the state file's own EE RAM, the frame is PCSX2's GS screenshot of the loaded state taken while the VM runs on (the race clock is read before and after the grab and ours is rendered at its mean). A widescreen state is made from a 4:3 one by writing `1` to `0x00284fe8` over PINE, advancing four frames and saving. **What is left is the original's `40/21` projection shown at `16:9`: ours draws everything 7.1 % wider** (ours at aspect `wide`, true 16:9, since 2026-10-21; the sentence assumed that and holds), edge correlation 0.11 against 0.59 with ours squeezed to match (camera.md). Only Moa Therma White has a state; more need a menu route for `pcsx2-drive.py` |
| Pure (PSP) | **not reachable, for now** | | `psp-drive.py menu` is Pulse's walk and stops at `the front end is in None`; `psp-trace.py --camera` breaks at Pulse USA's `Camera_UpdatePlayerView` (`0x0883c13c`), and Pure's counterpart is not located ([Pure camera rig](../ghidra/functions/psp-pure-usa/camera.md) is a static read of the rig only). Needs a hand walk to a grid, a save state and the camera node. The profile row `pure-psp` carries the PSP values |
| 2048 (Vita) | **partial** | nothing | `vita3k-drive.py race --screenshot` boots Vita3K on a private config, on its own weston and Xwayland display, and writes a frame; its fixed-timing tap walk never changed screen in the 2026-10-20 run and the frame was an intro cinematic, not a race. Profile row `2048`: 960x544 at 1x |
| Omega (PS4) | **partial** | layout only | shadPS4 0.20, the extracted base plus patch hard-linked into a game folder, a headless `sway` on the render node and its own `HOME` (recipe: [online lobby, part 3](../networking/online-lobby.md)). It reaches `MAIN MENU` unattended in about 45 s; `grim` takes the frame. No race was reached. Profile row `omega`: 1920x1080 |

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

## Zone pairs (Pulse PSP, 2026-10-21, `zone-pairs`)

A Zone frame is a mid-run state, so `psp-pulse-pair.py --zone` renders ours at tick 0 on the original's pose and camera with the zone number, score and zone seconds read from the race manager's mode object and written into the race (`--hud-state PLACE,LAP,LAP_S,RACE_S,KMH,SHIELD,BEST,ZONE,SCORE,ZONE_S`); `g_game_mode == 6` is checked. The state is made by walking `Racebox` > `Custom Race` > `RACE TYPE` right x5 (one key a second), Track Select (two circuits on a used profile: Talon's White, Moa Therma White) and flying on the Autopilot. HD's `rpcs3-hd-pair.py --zone` exists but no pair was made; see the compare-page handover thread.

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
