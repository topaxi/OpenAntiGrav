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
are in `theirs.png`, so the row follows the timer, as ours' readout does (`RaceSim::sub_weapon` and the icon fallback).
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

The hexagon itself is translucent in ours too: the backdrop word is the weapon's table word, alpha included (Autopilot `0x801CBC1C`,
alpha `0x80`; `oag_hud` `pickup_sprites`), and the scene shows through it. On `talons-boost`, regressing each frame's hexagon wings (1,304
pixels outside the icon and the countdown) against ours rendered without the row gives a blend alpha of 0.24 to 0.27 in ours and 0.26 to 0.29
in the original (the word's 0.5 times the sheet's own alpha, which was not read here). The wings differ by 4 to 8 grey on average, against 2 to 5
in a control region off the row; ours' wings read a little lighter (green about 225 against 202). On `talons-alongside` the same regression is
not usable (the pad and the rocket behind the hexagon make the per-channel slopes 0.17 to 0.55 for ours and 0.29 to 0.51 for the original),
but both frames show the scene through the hexagon and the wings differ by 8 grey in red and 4 in green and blue. What still differs in the
pickup box is the icon and the `5.00` glyphs. `talons-uturn-airbrakes` is a scenery pair (`--ticks 10829`, a Time Trial frame): it carried no `--hud-state`, so ours drew speed 0,
a full shield and no pickup row where the original showed 160 km/h and a held Turbo (the hexagon with chevrons). `psp-pulse-pair.py
--hud-readout` now reads the readout at the stop (the player's field row: place, lap, lap time, shield, the HUD block's speed; the held id
`record+0x1bc`, read **4**, Turbo in `pickup-colour.md`'s table, with the fire word `0` and no timer) and passes `--hud-state` and
`--hud-pickup Turbo` (a `CaptureHud` field, readout alone, requiring `--hud-state` like `--hud-autopilot`). `compare-rerender-ours.py` adds
`--hud-pickup` to a stored Pulse PSP pair whose record names a held id other than the running Autopilot, and a re-render from the record alone
is pixel-identical to the capture. Re-made from the stored state (`airbrake-uturn-c.ppst`): `theirs.png` pixel-identical to the stored
one, ours now draws the hexagon and the readout, the hexagon box (205, 0, 76x64) over 40 grey 48.2 % -> 5.8 % of its pixels, the whole frame
16.5 % -> 6.5 %. The bottom bar is the shield bar, red in the original because its post-hit flash timer was running (`hud+0x11c` read
0.1335 s at the stop with the pool at 99.98 %, `shield.md`, "The flash timer read live"); `psp-pulse-pair.py --hud-readout` reads that
timer and the blink accumulator beside the readout and passes `--hud-shield-flash TIMER,BLINK`, which sets the readout's flash and
blink phase alone, and `compare-rerender-ours.py` replays it from `original.hud.shield_flash_s`. The bar's box (x 304 to 448, y 245
to 259) over 40 grey went 68.06 % -> 1.14 % of its pixels and the whole frame 6.468 % -> 5.434 %; `talons-boost` and
`talons-alongside` re-render byte-identical (their records carry no `shield_flash_s`). What is left on that frame is the hull. The HD race pair (`hd-race/talons-pack`) and the
Zone pairs are unchanged (the flag is Pulse PSP's, from `restore_autopilot_timer`'s own pair types; the Zone pair's frame
shows the same row and has not been given it).

### The marquee and head-to-head bars of a race shot (2026-11-24, `hud-bars-headless`)

Two more `CaptureHud` pins, for a HUD part no headless route reached: `--hud-song TITLE [--hud-song-artist ARTIST] [--hud-song-ticks N]`
holds the now-playing marquee on a song (a capture runs no music, so none starts), and `--hud-head-to-head GAP,lead|trail` draws the
Head to Head bar on the readout alone (`--mode` has no head-to-head, and a posed capture runs no standings). Both need `--hud-state` and
reach nothing in the race: the marquee's display is stepped `N` times (default 60, past the fade-in, where the bar has stopped climbing) at load and then frozen, so the tick count of the capture does not move it, and the readout's gap is not read from the standings.
The bar exists only in a layout that authors it: the arcade one (`--mode single_race`), not the time trial's. `--hud-head-to-head` names a layout without the bar at load (a stand-in gap under the place, or nothing).

**Every `--hud-*` pin is capture-only** (`hud-flags-capture-only`): `--hud-state` requires `--screenshot` and conflicts with `--connect`, `--online`, `--drive` and `--drive-file`, and each other pin requires `--hud-state`, so a menu launch, a windowed `--race` or a network race refuses them at the command line. Every pair script already ends in `--screenshot`; `compare-rerender-ours.py` appends its own.

Measured on a posed Pulse PSP single race at 480x272, by regression of the frame with the pin against the frame without, over the bar's
pixels left of the scrolled text (1,512 pixels, per channel; the marquee bar is black, so its alpha is `1 - slope`):

| | marquee bar alpha | head-to-head bar |
| --- | --- | --- |
| with the fill drawn once (`4a445a7fa`) | 0.199 (about 51 over 255, where the fade-in ends: `Display::step`) | `0xFF7DEFC0` opaque |
| with that fix reverted | 0.35 (`1 - 0.8^2`: drawn twice) | identical |

The head-to-head bar is opaque in Pulse's layout (`HudColour2`), so the fix cannot show on it; it shows on the marquee's bar and on
a translucent fill under the HUD font (`a_translucent_fill_under_an_outlined_font_is_drawn_once` is the unit pin, and
`hud_boost_glow_ground_truth::the_capture_flags_draw_the_marquee_bar_and_the_head_to_head_bar` the frame). None of the 30 Pulse PSP
`theirs.png` in `pulse-psp/` (circuits) and `pulse-psp-race/` shows either bar: the marquee's rectangle (x 130 to 280, y 247 to 267) is
not darkened in any, and no pixel of the bar's mint is in the top right. The one original frame that shows the marquee is Pulse PS2's
`moa-therma-white` (its text tail), outside the PSP set; ours draws none there until a song is pinned for it.


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
filter off, render scale 100, anisotropic filtering off (no original forces it; Omega's does, 16x on its mipmapped colour samplers, so the `omega` profile is `16x`: [omega-status.md](../formats/omega-status.md#the-ports-anisotropy-2026-11-16-port-aniso)), the boost fov kick off (our own
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
read from the camera object in the state's EE RAM, fed as `cam_*` columns and `--camera-fov`); `pcsx2-pulse-pair.py` reads the world clock from the state file since 2026-11-12 (below, "The Orcus neon strips"); every PS2 pair is a chase view (the camera rig's eye 11.64 units from the craft).

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
- **HD: the HUD clock text lags RAM by 3 ticks; on a still craft the picture needs no correction, on a moving one it can (2026-11-10, `hd-talons-pack-align`).** `rpcs3-drive.py capture` reads the pose in the pause and `OAG_CAPTURE_FLUSH=3` lets the RSX present what it was handed. The 13 grid pairs are at 0 km/h, where an older frame looks the same, so they never tested "frame and pose are one instant"; the `hd-race/talons-pack` hull check that stood here was not evidence for it (see "A moving HD craft: the frame is not always the RAM instant" below). The HUD lap-clock text is not one instant either: on the 13 refreshed grid pairs the tenths digit the
  frame shows is one tenth below `trunc(RAM clock * 10)` on **9 of 13** (chenghou 14.149 shows 14.0, metropia 13.417 shows 13.3, talons 13.134 shows 13.0, ...) and equals it on 4, and the fractional tenths of the two sets bracket the lag between
  **0.054 s** (modesto-heights, 13.354 shows 13.2) and **0.055 s** (anulpha-pass, 13.955 shows 13.9): 3.3 ticks, so `rpcs3-hd-pair.py` runs ours `HUD_LAG_TICKS = 3` ticks short of the RAM clock (and the race pair's imposed clock),
  recorded in `pair.json` `original.presented_lag`. The set in `data/reference` made before this carries the uncorrected clock. Per-tick picture lag on RPCS3 is not measured (no per-tick stepping).
  HD's window opens at x = 240 on the 2000-pixel Xvfb, so a 1920-wide frame was clipped to 1760 and every pair refused ("original image size 1760x1080"); `rpcs3-hd-pair.py` waits for the resize to land and moves the window to the origin.
- **HD: a pair names the width its original's scene was drawn at** (2026-11-10). The 1080p path draws the scene 1300 to 1920 px wide in 20 px levels and stretches it, and the level moves a step a frame on wall-clock timing
  ([scene-width-throttle.md](../ghidra/functions/ps3-hdfury-eu/scene-width-throttle.md)). `pair.json` `original.scene_width` is `px`, `level`, `method`, and for a race frame the `score`, the `peak` it had to reach and the
  `runner_up`; a new pair also carries `at_pause` (the level and the enable byte read in the capture's pause, up to two steps from the frame's while the level moves). The stored set, measured off each `theirs.png` by
  `scripts/hd_scene_width.py --record`: **1300** on chenghou-project, metropia, moa-therma, modesto-heights, sebenco-climb, talons, talons-assegai-c1, tech-de-ra and ubermall (scores 15.0 to 38.3), **1400** on sol-2 (12.6),
  **1920** on anulpha-pass, the-amphiseum and vineta-k (no peak: 4.3, 3.4, 2.9 against the 5.5 bar; the weakest stretched frame of known width scored 6.7), and 1920 on the five menu pairs and the loading pair from how the
  front end is drawn (their scores are kept as `frame_score_unused`: the loading screen's dot grid reads 1840 at 9.1). So nine of thirteen race originals are 68 percent of full width horizontally, which bears on edge sharpness and
  fine texture, not on tone ([hd-track-tone.md](../rendering/hd-track-tone.md)).

## A moving HD craft: the frame is not always the RAM instant (2026-11-10, `hd-talons-pack-align`)

**Claim tested:** `hd-race/talons-pack` shows the same instant on both sides, so its remaining difference is rendering. **False for the pair as stored; partly false for the re-made pairs** (below).

**The stored pair (state of 2026-10-20, classic Feisar, 1280x720 at scale 150, captured 2026-10-21).** Edge correlation 0.014, RMSE 0.270, mean grey 47.6, against 0.39 to 0.45 on every other HD pair. The original's own HUD says its frame is older than the RAM it was paired with, with two independent clocks: the lap clock reads `0.14.3` where RAM 14.531 less the 3-tick text lag reads 14.48 (text 14.4), and the Autopilot readout `4.95` against RAM 4.817 (0.13 s, 8 ticks, 5 if it shares the text's 3-tick lag). At 502 km/h that is 18 units or more, a different stretch of road under the craft (ours showed green pad strips the original's road does not). The original's frame also carried a weapon-like purple lightning, a `+4.8` pickup readout, the DAMAGE and Absorb strips, a `0.46.64` BEST and RPCS3's own pause banner. Ours projected its own recorded state correctly: the recorded player and opponent positions through the recorded `view_proj` land where ours draws them (player (972, 571), Icaras (834, 380), EGX (1072, 243)); it is the original's picture that did not match its metadata. Confidence 90.

**The state file could not be reused.** RPCS3 0.0.43-20574 refuses the 2026-10-20 file (`No savestate file found or savestate not compatible`, in `serve/loaded.SAVESTAT.zst`'s log), so the pair could not be re-made from it. The state was regenerated (`scripts/emu-hd-race-walk.sh <lane> 8 0 0`, then `scripts/rpcs3-hd-race-hunt.py --place 3-6 --pin-scene-width`, a flag added here: it clears `0x00c81da4` after the race loads and again at the match, so the state draws its scene 1920 px wide). It is not the old file's circuit moment: lap 2 on Talon's Junction, 6th of 8. **Its player is `Feisar` (the Fury hull) where the old state's was `Feisar_c1` (classic).** The sidecar (`.zst.json`) says `Feisar` now; the old one said `Feisar_c1`, and a pair command line copied from the old pair drew the wrong hull. `scripts/rpcs3-hd-pair.py --field` now refuses a `--team` that is not the player's team name read from RAM.

**A restore with `--pause` does not land on the state's own tick.** Three restores of the one file stopped at race clocks 11.21, 7.31 and a third pose (590, -10, 79): the pause lands a variable game time after the load (about 4 s apart here). Two captures of one state are therefore two samples of two moments, not a twin pair.

**Presented lag on a moving craft, two samples, the same boot recipe.** `scripts/hd-pair-lag-scan.py <pair>` renders ours with the camera eye moved back along the player's recorded velocity and every craft back along its own, by whole or fractional 60 Hz ticks, and scores each against `theirs.png`:

| capture | race clock | best camera : craft rewind | opponent (green craft, colour mask) centroid, original / ours at that rewind | edge correlation 0:0 -> best |
| --- | --- | --- | --- | --- |
| `run1` | 11.206 s | **2.0 : 3.0 ticks** (camera 1.75, 2.0, 2.25 and craft 2.5 to 3.25 tried; the scenery peaks at camera 2.0, the opponent at craft 2.75 to 3.0) | (1299.8, 567.3) / (1300.1, 567.3); at 0:0 ours (1285.2, 560.4), 14.6 px off | 0.107 -> 0.189 |
| `run2` | 7.313 s | **0 : 0** for the scenery and the opponent (rewinds of 1 to 3 ticks put the opponent 8 to 17 px away); the hull wants about 0:1 | (1176.9, 534.1) / (1181.3, 533.8) | 0.091 at 0:0 |

So the frame is **2 ticks (camera) and 3 ticks (craft) older than the RAM read on one pause and, for the camera and the opponent, not older on the other**: the presented lag is not a constant, and RAM read in the pause is not the picture's instant on a moving craft without a check. Three ticks is the HUD text's own lag (3.3), which fits the craft term; the camera's 2 is not explained (the push-buffer matrix is built a tick after the craft update, not read). Confidence 60: two pauses, one state, one boot recipe; the metric is a whole-frame edge score (0.107 at 0:0, 0.185 at 2:2, 0.189 at 2:3, 0.127 at 3:3) and the opponent centroid agrees to 0.3 px. The per-frame pick: a pair whose opponent centroid is within 8 px at 0:0 is one instant (`run2`, 4.4 px for the near craft; the far one, 126 units from the camera, a speck about 12 px wide, reads (980, 484) against ours (982, 480) off a 4x crop, by eye); it is not `run1`. `rpcs3-hd-pair.py` does not yet record this (open: a recorded `presented_lag` per pair, now best read off the push buffer's hull matrix, below).

**What is left after the lag is accounted for.**

- **The player's hull is about 1.3 times larger and lower in the original than ours draws it at the recorded state, and it is a timing offset, not a hull-only view** (read off 1:1 crops of the original against `hd-pair-lag-scan.py` renders; width read by eye, +-10 px, not measured to the brief's 4 px). `run1` at its fitted rewind (camera 2, craft 3 ticks): the hull is 375 px wide against the original's 370 and sits where the original's does, where at 2:1 and 2:0 it is 270 px wide and high; the unrewound `ours.png` is 285 px against 383. `run2`: the hull is 270 px at 0:0 against 370 and matches at about **0:1** (380 px, same position) while its opponent is 4.4 px off at 0:0 and about 17 px off at 0:1 (centroid (1192.2, 541.1) against (1176.9, 534.1)). So the player's hull wants to be about **1 tick older than the camera on both samples**; the opponents want 1 tick older than the camera on `run1` and the same tick on `run2`. One tick is 2.3 units at 138 u/s, a sixth of the 13.9-unit eye to body distance, which is the whole size difference. The drawn node matrix is not the cause: `ship+0x40` (copy `+0x80`) of the player at 132.6 u/s holds the body's position to the printed digit (590.024, -9.925, 78.729), rows scaled 0.743, shake object zero. The eye to body distance reads 13.9 units on both (the grid pairs 12.7). Settled 2026-11-11: RAM is a whole tick or two ahead of the push buffer's frame for the craft and the camera alike, see "The skew is the pause, not the original's draw order" below.
- **State differences** (not ours to import): the RPCS3 pause banner, the Autopilot readout (the state's own, 9949.49 s and 9953.36 s, because the timer is armed for good rather than the authored 5 s written back, so the pause lands on the state's tick with nothing run after it), the DAMAGE and Absorb strips, the BEST row `0.44.24` against ours' `0.00.00` (the HUD record `--hud-state` does not carry), the pickup row and sight, the original's own speed blur on the road edges.
- **Render differences:** edge correlation stays near 0.1 with the picture aligned (run2): the original's radial speed blur on a 495 km/h frame (ours is off in the profile), hue and bloom. The 0.30 acceptance bound is **not reached and not reachable by alignment**: 0.19 is the best seen, and the grid pairs that score 0.4 are still frames.

### The skew is the pause, not the original's draw order (2026-11-11, `hd-worst-pairs-2`)

**Question:** the player's hull is about one tick older than the camera on both re-made samples. Is that how the original draws (the hull's pose from the previous tick), or the capture? **The capture.** At an RPCS3 pause the game's RAM is **1 or 2 whole ticks ahead of the frame in the push buffer, for every craft and for the camera alike**; in the frame itself the hull and the camera are one tick. Confidence 90.

`scripts/hd-frame-skew.py probe` pauses once and reads, in that pause, every craft's body (`+0x200`, velocity `+0x190`), the player's ship and the three push-buffer regions `capture` reads; `read` then compares. Two things in the push buffer can be set against RAM without a picture:

- **The hull's own world matrix.** Each craft's draws load a plain row-layout world matrix, rotation rows scaled 0.75 (the craft scale), 7 to 15 times a frame; its translation is where the frame draws the hull.
- **The camera's eye**, off the register-256 view-projection, against the craft's rig eye in RAM, `ship+0x69a0` (11.6 to 11.8 units from the body on every sample, 0.13 units from the push-buffer eye once the tick is taken out).

Eight pauses of `hd-race-talons-venom-pack` over two restores, 109 to 170 units a second (a tick is 1.8 to 2.8 units); offsets in ticks along the craft's own velocity, off-axis residue in units:

| Pause | Push-buffer hull minus RAM body, player | An opponent | RAM rig eye minus push-buffer eye |
| --- | --- | --- | --- |
| `p1` | (regions held no hull) | | +1.0 (0.13 off) |
| `q2` | -1.003 (0.04) | | (trackside camera) |
| `q3` | -1.043 (0.26) | | (trackside camera) |
| `r1` | (not held) | -1.998 (0.05), -1.999 (0.01) | +1.997 (0.01) |
| `r2` | -0.998 (0.01) | -0.998 (0.02) | +1.004 (0.04) |
| `r3` | -1.007 (0.04) | | +0.992 (0.14) |
| `r4` | -2.004 (0.07) | | +1.915 (0.52, in a turn) |
| `r5` | -2.041 (0.16) | | +1.920 (0.67, in a turn) |

The node matrix at `ship+0x40` equals the body on all eight craft on every pause, so RAM is self-consistent and simply newer. With the tick taken out, the eye is 11.6 units from the hull at speed (the 13.9 above was a tick-N camera against a tick-N+1 body; the grid pairs' 12.7 is a craft at rest).

**So a pair that takes the camera from the push buffer and the craft from RAM draws every craft 1 or 2 ticks ahead of its camera**: smaller, higher and farther up the road. On the stored `hd-race/talons-pack`, ours re-rendered with every craft moved back along its own velocity (sigma 7, mean absolute RGB): frame **18.82 at 0 ticks, 17.98 at 1, 21.80 at 2**; the hull's box (`640,330 640x420`) **17.29, 13.34, 17.47**; at one tick the hull is the original's size and sits where the original's does. That pair's skew is one tick. The earlier note that `run2`'s opponent wanted no rewind is not borne out by the push buffer (`r1`, `r2`: opponents skewed with the player).

**Wired for `--detonator` (2026-11-23, `det-camera`, "Detonator pairs: the hull's frame pose") and for the Rocket film pairs (2026-11-26, `compare_env.py film-pair-field`, which reads the hull from the frame's walked command stream; [particle-fill.md](../rendering/particle-fill.md), "The first wave and the drawn hull"); still open for `--field`:** `rpcs3-hd-pair.py --field` should read the player's hull matrix from the push-buffer regions it already dumps, record the whole-tick skew in `pair.json` (`original.presented_lag.craft_ticks`), refuse a pause where it is not within 0.1 of a whole number, and write the field with every craft moved back that many ticks of its own velocity (or take each hull's translation from the push buffer outright). The HUD clock's 3-tick text lag is a separate thing and stays. Until then a moving HD pair is checked with `hd-pair-lag-scan.py`.

**A ringless pair is moved after the fact with `compare_env.py rewind-pair-field <pair-dir> <ticks>` (2026-11-26, `pair-tooling-fixes`).** It moves `field.csv` back the whole ticks the scan picks, records `original.presented_lag.craft_ticks` with the basis (a whole-frame score, not a push-buffer read), replaces the record's claim that the picture needed no correction, and refuses a pair that already records `craft_ticks` (as `film-pair-field` now does too). The camera is not moved. On `hd-race/talons-pack` (current binary, scratch copies, each arm rendered twice with bit-identical results):

| Craft rewind, camera as recorded | mean grey difference | RMSE | edge correlation | hull box grey (`640,330 640x420`, mean absolute RGB) |
| --- | --- | --- | --- | --- |
| 0 ticks (stored) | 23.05 | 0.171 | 0.092 | 25.71 |
| **1 tick** | **22.39** | 0.166 | 0.094 | 21.83 |
| 2 ticks | 26.21 | 0.191 | 0.089 | 25.86 |

By eye at 1 tick the hull has the original's size and height, where the stored pair's is small and high. Confidence in 1 tick relative to the camera 85: `hd-pair-lag-scan.py`'s RMSE also prefers craft = camera + 1 when the camera is rewound (1:2 over 1:1, 2:3 over 2:2), and the eye to hull distance at 1 tick is 11.6 units, the rig's.

**Open: the camera of this pair may itself be newer than the picture**, which would be another sample of "The picture is older than the push buffer's camera" below (`det-mine-look-re`, not reconciled with this section). Rewinding the camera too scores lower on RMSE and grey (camera 1 craft 2: mean grey 18.78, one render) and goes on falling to camera 2 craft 3 (scan RMSE 0.136 against 0.165 at camera 0 craft 1), while edge correlation falls (0.094 to 0.074). A whole-frame score does not settle it. What would: that section's measure (static things near the camera, a lag fitted per grab) or a re-capture that keeps the frame's command ring.

## Pulse PS2: a race pair for every circuit (2026-11-11, `gallery-pulse-ps2`)

**Claim:** the page carries a Pulse PS2 race pair for every circuit ours can race on the PS2 disc (about 14), up from 1. **True:**
ours races all 16 `NN_Track` directories on `pulse-ps2-eu.chd` in both layouts (`oag-game --race --track ...\NN_Track\track.vex
--ticks 1 --trace-out`, 32 of 32 start; `17_Track` and `18_Track` do not start), and a pair exists for each: 15 new and the Moa Therma
White one made again from nothing. Each passes all 14 parity checks (`pair.json` `refused` false). Original: PCSX2 v2.7, OpenGL
hardware renderer, 2x upscale, `AspectRatio 16:9`, one Lanczos resize to 1280x720, the game's own `Aspect Ratio` option on `16:9`; the
craft rests on grid slot 8 of 8, no input, the HUD clock 22 to 24 s after GO, the camera the original's own. Ours: `--size 1280x720`,
aspect `wide`, the profile's settings.

**The route (from a memory card holding a profile, no state to start from).** Racebox, Single Race, Track Select lists 3 circuits
until the dev-unlock word is set (`g_settings + 0x46c`, `FUN_0017fa68`, evidence in
[definition-unlock](../ghidra/functions/ps2-pulse-eu/definition-unlock.md)); one write puts 32 entries on the list. A menu savestate
on Track Select (slot 6) is the start of every walk: `scripts/pcsx2-pulse-walk.py race --entry N --name X --circuit ...` loads it,
presses `down` until Track Select's cursor word reads the entry (a PINE read: a key press is lost on a loaded machine and doubled on an
idle one, so counting presses did not land), confirms track and ship (Assegai), presses `cross` through the circuit's description page
until the race HUD's hexagon is drawn, finds the race clock, waits for RAM clock 24.5 s, sets `DAT_00284fe8 = 1`, advances four frames
and saves. About 2.5 minutes a circuit on a loaded machine, 1.5 on an idle one.

**Three things that cost runs, each fixed in the scripts.**

- **The race clock is not at `0x70EE9C`.** That is the Moa Therma White state's. It moves with the heap layout of each boot (`0x70DB7C`
  in the next one) but stays **`0x12084` to `0x120a4` below the player's craft struct** on all 16 saved states (`0x12084` on 7,
  `0x12094` on 2, `0x120a4` on 7): `pcsx2_state.race_clock_address` searches that window for the float whose partner `0x100` after it,
  the HUD's own clock, is 3.9 to 4.45 s below it, and it agrees with the address each sidecar recorded on all 16. Five states first
  failed that search only because the craft came from the table-row bug below. `race_clock_between` finds the same word from two
  savestates a few seconds apart (the float that ran on by the frame counter's difference) with no craft needed, and is what the walk
  uses; it gave the same five addresses. The word `0x100` after the clock reads 4.09 s below it in these boots (192.439 and 188.351 with
  the HUD at `3:08.3`), where the Moa Therma White grabs gave 4.2 to 4.29: the pair keeps 4.25, and ours' HUD clock is within 0.1 s of
  the frame's on all 16 pairs read (`0.22.1` against `0.22.0` on Talon's Junction and `0.24.1` against `0.24.0` on Tech de Ra are the
  worst).
- **`player_craft` picked table rows.** Where the craft heap sits low (`0x6xxxxx`) a body-shaped pointer with a position like
  `(16.6, 0, 0)` passed the camera-rig test; a position with two zero components is now refused. The cached basis also trails the body
  by 2e-4 on a craft left resting for minutes (it was 1e-5): `CACHE_TOLERANCE`. The five states first saved with the wrong craft were
  identified from the camera object, their sidecars rewritten, and every position checked against our own start (below).
- **`frames 4` leaves the VM paused**, so the next `loadstate` came up paused and every menu press did nothing; the walk now runs the VM
  before and after the menu load.

**Circuit and layout, identified, not assumed.** The state's craft position (read from its EE RAM) against ours' start on every
`NN_Track` forward and reversed: 0.0 to 1.2 units on 12 states, 2.8 to 3.0 on Vertica and The Amphiseum, and 34.9 (de Konstruct) and
41.6 (Platinum Rush) where the grid bends and slot 8 is not ours' start; the nearest wrong candidate is 25.3 units from Arc Prime's state (`15` reversed) and 64 or more from every other. The pair
script then checks the layout by heading (`--layout-tolerance-deg`, default 5 degrees; Vertica needed 10 and Platinum Rush 25, recorded in
`ours.layout_tolerance_deg`, the other layout being 150 degrees or more worse). The Track Select order is Talon's Junction (`16`),
Moa Therma (`03`), Metropia (`02`), Arc Prime (`10`), **Orcus (`15`)**, de Konstruct (`05`), Tech de Ra (`04`), **Vostok Reef (`08`)**,
The Amphiseum (`09`), Fort Gale (`14`), Basilico (`01`), **Edgewinter (`11`)**, Platinum Rush (`13`), Vertica (`06`),
**Gemini Dam (`12`)**, Outpost 7 (`07`), each a White then a Black entry; the four in bold are not on the PSP disc (the four DLC packs ship them: [dlc-pack.md](../formats/dlc-pack.md)). The forward entry is the
White one on Talon's Junction, Moa Therma, Arc Prime, Tech de Ra, Platinum Rush, Vertica and Outpost 7 and the Black one on
Metropia, de Konstruct, The Amphiseum, Fort Gale and Basilico, as on the PSP ([track.md](../formats/track.md#white-and-black)); on the
new four, Orcus White and Edgewinter White are forward, and **Vostok Reef White and Gemini Dam White are the reversed layouts** (ours
`08` and `12` `track_reversed.vex`: start heading within 4 degrees and 1.2 units). Orcus Black, Edgewinter Black and the other
Black or White entries were not walked.

| Pair id (`pulse-ps2-...`) | Circuit, layout | RMSE | Edge corr. | Mean grey | Second grab | Theirs, grab 1 against 2 (RMSE) | HUD clock |
| --- | --- | ---: | ---: | ---: | --- | ---: | ---: |
| `talons-junction-white` | `16` forward | 0.178 | 0.062 | 19.99 | 0.181 / 0.074 / 20.54 | 0.025 | 0:22.0 |
| `metropia-black` | `02` forward | 0.146 | 0.101 | 13.13 | 0.148 / 0.101 / 13.23 | 0.029 | 0:23.8 |
| `arc-prime-white` | `10` forward | 0.134 | 0.091 | 11.20 | 0.134 / 0.091 / 11.21 | 0.020 | 0:23.8 |
| `de-konstruct-black` | `05` forward | 0.162 | 0.094 | 16.65 | 0.162 / 0.095 / 16.59 | 0.025 | 0:23.4 |
| `tech-de-ra-white` | `04` forward | 0.174 | 0.072 | 15.47 | 0.174 / 0.072 / 15.46 | 0 | 0:22.8 |
| `amphiseum-black` | `09` forward | 0.134 | 0.093 | 11.13 | 0.135 / 0.092 / 11.13 | 0.017 | 0:24.0 |
| `platinum-rush-white` | `13` forward | 0.159 | 0.070 | 13.64 | 0.159 / 0.070 / 13.64 | 0 | 0:22.3 |
| `vertica-white` | `06` forward | 0.155 | 0.053 | 14.18 | 0.154 / 0.052 / 14.07 | 0.030 | 0:23.6 |
| `outpost-7-white` | `07` forward | 0.151 | 0.087 | 12.89 | 0.151 / 0.087 / 12.89 | 0.019 | 0:23.7 |
| `vostok-reef-white` | `08` reversed | 0.162 | 0.071 | 13.50 | 0.160 / 0.071 / 13.30 | 0.025 | 0:22.0 |
| `fort-gale-black` | `14` forward | 0.133 | 0.082 | 11.45 | 0.127 / 0.088 / 11.01 | 0.025 | 0:23.6 |
| `basilico-black` | `01` forward | 0.160 | 0.060 | 14.99 | 0.160 / 0.059 / 14.99 | 0 | 0:23.7 |
| `orcus-white` | `15` forward | 0.161 | 0.080 | 13.14 | 0.158 / 0.083 / 12.96 | 0.017 | 0:23.4 |
| `edgewinter-white` | `11` forward | 0.162 | 0.091 | 14.38 | 0.162 / 0.091 / 14.38 | 0 | 0:23.7 |
| `gemini-dam-white` | `12` reversed | 0.161 | 0.127 | 13.83 | 0.160 / 0.128 / 13.81 | 0.040 | 0:23.6 |
| `moa-therma-white-r2` (the stored pair, made again) | `03` forward | 0.158 | 0.098 | 13.36 | | | 0:21.1 |

The stored Moa Therma White pair reads 0.154 / 0.109 / 13.12; this boot's reading of the same circuit and slot, 0.158 / 0.098 / 13.36.
Five of the fifteen second grabs of the original are bit-identical to the first (RMSE 0); the rest differ by 0.017 to 0.040, the
animated surfaces of a grab a few tenths of a second after the load. The metrics repeat within 0.006 RMSE and 0.012 edge correlation
(Talon's Junction, 0.062 to 0.074). Confidence 85: two grabs of one state per pair, one boot of the walk per circuit.

**What the pictures show (all 16 pairs read as half-scale contact sheets; Edgewinter, Orcus and Platinum Rush also at native 1280x720).** Circuit, grid slot, camera, GO board, HUD and its clock agree on all
sixteen. Differences that are ours, for a lane that owns them: the original's GO board has not yet turned on at Talon's Junction
(a flat green bar where ours draws `GO`, and on Vostok Reef too); the neon strips on Orcus were yellow in the original and
magenta in ours, which was ours' clock and not a rendering defect (below); a pink streak beside the hull on Platinum Rush (the ribbon `--pose-hold` left behind, below); ours draws no `pos 8/8` readout at the top right on any pair; a faint caption at the bottom left of the original on Talon's Junction,
Platinum Rush and Vostok Reef that ours does not draw. The
mean grey difference is 11 to 20, the edge correlation 0.05 to 0.13, near the stored pair's 13.1 and 0.11.

**The Orcus neon strips are the world animation clock (2026-11-12, `ps2-orcus-neon`).** The strips draw `lob_gradient_glow.tga`, a rainbow gradient,
under an authored texture scroll with a 600 s loop (`crates/render/tests/ps2_neon_clock_ground_truth.rs`), so their colour is the clock they are drawn
at: ours' own tick count (28.05 s at the pair's 1683 ticks) reads magenta, the original's clock reads yellow. The original feeds every world mesh
`*(g_ingame + 0x40)` (`g_ingame` at `0x27DDBC`, `docs/ghidra/functions/ps2-pulse-eu/anim-transform.md`), 33.53 s in the Orcus White state against the
RAM race clock's 26.38 s; `pcsx2_state.world_clock` reads it and `pcsx2-pulse-pair.py` records `original.world_clock_s` and passes ours
`--world-clock-offset` = that clock, run on by the race clock's advance from the state to the grab, less ours' `ticks / 60` (6.88 s on Orcus White).
`compare-rerender-ours.py` puts the flag back on a pair recorded without it, from the state file and the grab bracket. Ours' left strip, box
`330x120+0+130` of the 1280x720 frame, mean RGB: before 99,39,111, after 100,114,35, the original's 108,122,33; a sweep of the offset on the same frame
reads magenta at 0 s and 12 s, cyan at 3 s and 15 s, yellow from 5 s to 9 s and at 20 s (a period near 13 s to 14 s). **Confidence 75** that the texture animation's clock is `*(g_ingame + 0x40)`: the shared per-race clock is the PSP finding carried over (`docs/ghidra/functions/ps2-pulse-eu/anim-transform.md`,
read for `Anim Transform` nodes, not live-broken on the PS2 texture updater), and two things measure it here. The Orcus strip's yellow window
(offsets 5 to 9 s) holds the computed 6.88 s, a chance of about one in three alone. And on the short-loop surfaces, which are phase-sensitive to 0.1 s,
the offset sweep's whole-frame rmse against the original bottoms out beside the computed offset on both pairs swept finely: Fort Gale Black computed 3.81 s,
rmse 0.141 / 0.135 / 0.130 / 0.132 at 3.5 / 3.7 / 3.9 / 4.1 s; Gemini Dam White computed 7.77 s, rmse 0.161 / 0.166 / 0.160 / 0.162 at 7.3 / 7.6 / 7.9 / 8.2 s. Across the 15
race pairs (a re-render with the original's clock against ours' tick clock, rmse): better on Orcus (0.159 to 0.138), Metropia and Vostok Reef (each by 0.007), Talon's
Junction and Tech de Ra (0.003), Amphiseum (0.002), Arc Prime, Basilico and de Konstruct (0.001); flat on Edgewinter and Outpost 7; worse by 0.001 to 0.004 on Vertica, Fort Gale and Gemini Dam, where
a 0.2 s step in the offset moves the rmse by more than that (a tick-clock phase can land beside the original's by chance on a 1.7 s loop). The three menu pairs gain no flag (no grab bracket).
Platinum Rush's pink streak beside the hull is not on this clock: its crop (`300x180+290+540`) is pixel-identical at offsets 0, 3, 7.43 and 12 s (mean 66,65,66).
It was the engine ribbon, left behind by `--pose-hold`: the pair runs 1,617 ticks of the clock, the resting craft on the banked slot yaws about 18 degrees
(bank-to-yaw, `docs/physics/grid-state.md`) and laid its trail along the turned hull, and the hold then put the hull back and left the ribbon where it was.
Ours at tick 120 (before the turn) and the original both hang the ribbon straight back. `Race::place_player` now carries the ribbon rigidly with the hull
(`Exhaust::move_rigidly`); the box `300x180+290+540` reads 66,66,66 before, 58,61,58 after, original 55,58,55, and the nozzle box `90x140+590+580` 70,70,75, 98,86,100, original 79,71,83
(ours' nozzle glow is brighter than the PS2 original's, a separate difference). Test: `pose_hold_ground_truth::a_held_pose_takes_the_engine_ribbon_with_the_hull`.
Pulse PSP's pair takes the same path (its craft turned 9 degrees over 1,646 ticks) and moves by 19 pixels below a 2 % difference.

**Open.** The other 16 Track Select entries (the second layout of each circuit, Black or White) were not walked: a pair is one
`--entry` away. The states and sidecars are in `data/saves/pulse-ps2/` (a sidecar records the circuit, the craft, the grid position
and the clock's address); `pcsx2-pulse-walk.py` needs a Track Select savestate with the unlock word set, taken by hand on one boot.

## Re-rendering ours only (2026-10-21)

A pair with no `pair.json` is skipped here (`kept <page>.toml as written`) and its `ours.png` is never made. The Wipeout 2048
race-load pairs are made by `scripts/vita-2048-race-load-pair.py [--pairs DIR]` (a `pair.json` for each of `race-load-tip`,
`-mode` and `-late` beside its kept `theirs.png`, then ours drawn); run it once on the main checkout's pair directory, after
which this script re-renders them like any other loading pair.

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
a model opens on concept1 (stats 080/085), where the original's classic Feisar reads 070/100/080/080. The Omega menu script records `ours.race` from now on, and since 2026-11-23 the Pulse/Pure menu script does too
(`psp-menu-pair.py`; Pure's team item no longer needs one, a fresh profile opens on Feisar as the original's does); the 2048-menu script
does not yet, so its re-renders still drop their `race` selection (older Omega pairs have none either).

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
| Pulse PS2 | **works for all 16 circuits** (2026-11-11, `gallery-pulse-ps2`; section below): one grid pair each, forward layout where the original's entry has one, widescreen, camera matched | pose, clock, circuit, layout, the state's `Aspect Ratio 16:9` (read back from the projection), the camera's look direction at the craft, PCSX2's `AspectRatio` | PCSX2 booted by `pcsx2-drive.py boot` with `OAG_COMPARE_PLATFORM=pulse-ps2` in the environment (its ini carries the profile's upscale, screenshot size and `AspectRatio 16:9`). States: `scripts/pcsx2-pulse-walk.py` (below) saves one per Track Select entry into `data/saves/pulse-ps2/`. Pair: `scripts/pcsx2-pulse-pair.py --slot S --state <p2s> --track NN_Track [--reversed] --out D --autopilot off --meta <json>`. Pose, craft, clock and **camera** (`pcsx2_state.camera`: eye, axes and fov of the object `Camera_SubmitScene` runs on, [camera.md](../ghidra/functions/ps2-pulse-eu/camera.md)) are read out of the state file's own EE RAM, the frame is PCSX2's GS screenshot of the loaded state taken while the VM runs on (the race clock is read before and after the grab and ours is rendered at its mean). **What is left is the original's `40/21` projection shown at `16:9`: ours draws everything 7.1 % wider** (camera.md) |
| Pure (PSP) | **works** (2026-10-22, `pure-pairs`), about 30 s per pair once a state exists; twelve circuits (the Alpha, Beta and Classic leagues; 2026-11-11 `gallery-pure`), Feisar, Vector | pose, circuit, camera (the original's Close eye read at `(0, 3.000, -11.250)` in the body frame; ours' own chase rig is checked by the hull's px offset, 0 to 1 px), fov 60, race clock (read at the state's own first stop, the HUD tenth equal on 4 of 4) | **USA disc** (`data/images/pure-psp-usa.chd`, `UCUS98612` 2.00): every live Pure address is recovered on it; the EU image is not read. `PPSSPP_EXTRA_INI=<file from compare_env.py ppsspp-ini pure-psp> scripts/ppsspp-start.sh <lane> <display> <port> <usa image>`, then `scripts/psp-pure-pair.py --port P --home $PP_HOME --out D --track 01_Vineta_K --state <ppst> --class vector --team Feisar --autopilot off --meta <json>` (`OAG_SHOT_DISPLAY`, `DISPLAY`, `HOME`, `XDG_CONFIG_HOME` as for Pulse). States: `scripts/psp-pure-walk.py --port P --track-index N --keep data/saves/pure-psp/<name>.ppst` walks pad presses from a running race (`start`, Quit) or the Main Menu to Time Trial, Vector, Alpha, circuit N, Feisar, and saves a state 16 s after Track Selection (a fresh profile has only Vector and Venom and the Alpha league: Vineta K, Modesto Heights, Chenghou Project, Blue Ridge, directories `01_Vineta_K`, `03_Modesto_Heights`, `04_Chenghou_Project`, `07_Blue_Ridge`; **`--unlock --league N` writes Pure's dev-unlock byte and picks Beta (1: `08_Sinucit`, `09_Citta_Nuova_V`, `10_Sebenco_Climb`, `12_Sol_2`) or Classic (2: `ClassicsNS/01_Karbonis`, `02_Sagarmartha`, `03_Manor_Top`, `04_Mandrashee`, which `psp-pure-pair.py --track` takes as a `Data`-relative path and `--hull-differs` for the Classic hull)**). **Not matched: the plume glow and the ground shadow, the HUD (ours has no speed gauge and prints `TOTAL` where the original prints `BRONZE TIME`).** Details: "Pure (PSP) pairs" below |
| 2048 (Vita) | **works for a grid before GO** (2026-10-22, `pairs-2048`), about 1 min per pair once Vita3K's shaders are cached; fourteen circuits (Empire Climb, Queens Mall and, from the DLC's HD Campaign node, Vineta K plus eleven more DLC circuits by "Eleven pairs through a renamed grid cell" below), Feisar, 8th on the grid | circuit, craft, grid slot (8/8 on both HUDs, by eye), window size 960x544, ours' settings; **no pose, camera or clock is read** (Vita3K has no state channel, and the race clock reads zero for the whole countdown) | Vita3K on its own weston and Xwayland (`vita3k-drive.py display`), a private `HOME`, `pref-path` and config, `SDL_AUDIODRIVER=dummy`, then `scripts/vita-2048-pair.py --out D --walk --event "2048 - Event 2" --node X Y --autopilot off --meta <json>` (`OAG_VITA3K_CACHE`, `OAG_VITA3K_CONFIG`, `OAG_VITA3K_DISPLAY`, `XDG_CONFIG_HOME` as for the others). It boots, waits on each screen by the frame's mean luma and the HUD's white text rather than a fixed sleep, taps the campaign event, pauses, Retries and grabs frames every 0.16 s through the countdown; the grid frame is the one 3 s after the HUD's first frame, ours renders tick `round(60 * 3.0)`. **Not matched: the lighting (the original is darker, mean grey 45 to 57; since 2026-10-26 ours runs 2048's own composite with the exposure held at the original's 1.15, mean grey 23 to 39, "Exposure" below), the grid pose (heading, and Queens Mall's pitch: ours rests 1.15 degrees nose-down of the original, which is the whole of its floor sitting 8 px high; the "hull 30 px up and left" reading of 2026-10-22 was the yellow-centroid metric catching other yellow, the nose tips agree to 2 px), and, on the DLC pair, the texture detail, shadows and hull shading (the lighting itself matches, 48.3 against 45.1 mean luminance, re-measured 2026-10-29).** The original's drawn-hull shake is patched out in its memory by the pair script (`--walk`), and its grid pose is read out of that memory with `--read-pose`. Details: "2048 (Vita) pairs" and "The hull shake and the grid pose" below |
| Omega (PS4) | **works** (2026-10-22, `pairs-omega`), about 85 s per pair on a warm shader cache (136 s cold); two circuits, Vineta K and Anulpha Pass, Feisar, Venom | circuit, craft, mode, camera mode (the grid is a still; there is no pose readout on shadPS4) | `scripts/shadps4-omega-pair.py`: its own headless `sway` on the render node, its own `HOME`, one uinput pad, muted. Walks MAIN MENU > RACEBOX > Track Select (Right steps the circuit) > Ship Select (two Downs reach the first Feisar craft on a fresh profile) > loading > START RACE and grabs the first frames after the HUD shows, before the countdown board's red 3. **Two shadPS4 settings carry the picture**: `readback_linear_images_enabled = true` with `readbacks_mode = 1` makes the world draw (off: black scene, HUD and lens flare only); `copy_gpu_buffers = true` crashes the emulator at the race load, `direct_memory_access_enabled` alone leaves it black. Recipe: [online lobby, part 3](../networking/online-lobby.md). Profile row `omega`: 1920x1080. **Known gaps in ours**: the gantry board shows the grid position 8 where the original's is blank and counts 3, 2, 1, GO; scenery lighting is not read, so the original's sun, shadows and sky differ (Anulpha's road reads white); the original idles an exhaust flame and ours draws none. The craft offset is read by eye off the nose tip: (-2, +5) px on Vineta K, (0, 0) on Anulpha Pass (the camera follows the craft, so the nose does not move). **Since 2026-10-26 (`omega-grid-height`) ours holds HD's grid hover, which Omega's craft update carries ([hover-target](../ghidra/functions/ps4-omega-eu/hover-target.md)): before, ours floated high and the floor sat a pad's length too low on screen. Mean grey 56.2 to 46.7 (Vineta K) and 75.9 to 61.5 (Anulpha Pass), edge correlation 0.08 to 0.18 and 0.07 to 0.23.** The manifests were not regenerated here; `just compare-rerender-ours --only omega` adopts it. Whether the original's hull shake is visible on the grid is not determined (consecutive grid frames differ by 0.10 to 0.27 normalised RMSE over the hull region, which also holds the animated exhaust). **Second original, 2026-11-01: the `wipeout_pc` native port** (PS4 code run natively, replacement runtime, shadPS4 `b4e7ae776071` video core with `gow3:` changes; built and run headless on Linux with a muted SDL driver; capture driver: `scripts/wipeout-pc-omega-pair.py`, below). Its Vineta K grid has the track floor and near walls lit with shadow streaks in 3 of 3 boots (whole-frame MAE between boots 0.022), where the shadPS4 0.20 pair above draws them near black (floor MAE against ours 0.098 against 0.280 on the first boot): confidence 80 that the black floor is a shadPS4 readback artefact, not the game. The port edits the eboot (212 writes: native 4K, startup hang, zone and effects fixes above 60 FPS, model slots, Eliminator slider limit), none touching a physics value and none changing a 1080p/60 frame by reading; two render hacks (start-gantry mip chain, per-sample livery shading) are unconfirmed (confidence 40) and not visible at the grid. **Since 2026-11-01 (`omega-native-pairs`, maintainer decision) the native port is the original of both Omega race pairs**, replacing shadPS4 0.20: `theirs.png` is the port's frame, the shadPS4 frame is kept as `theirs-shadps4.png` and cited in each note, and `vineta-k-native-port` is folded into `vineta-k`. Same pose (Feisar, grid before GO, HUD up plus 2 frames), walked by `--track-steps` (Anulpha Pass is one Right on Track Select). Metrics, rmse / edge correlation / mean grey, shadPS4 then native: Vineta K 0.268 / 0.18 / 48.16 to 0.196 / 0.21 / 30.74; Anulpha Pass 0.249 / 0.26 / 40.85 to 0.221 / 0.28 / 34.03 (both read off `side.png`). Vineta K: 3 boots, whole-frame MAE 0.022 between them. Anulpha Pass: one clean boot (confidence 55, seen once); a second boot ran at the same time on the machine-wide uinput pad, took a different craft (a locked one, other livery) and was discarded, so a concurrent port run is never a repeat. Hull livery varies from boot to boot on both circuits. Live memory reads through `/proc/<pid>/mem` of the launching process (`gdb -p` is blocked by Yama `ptrace_scope` 1): image base `0x800000000`, class-name table and HUD strings read at per-segment deltas; the race state, mode and class name tables and racer table were then found at the patch eboot's addresses ([patch-v107-addresses](../ghidra/functions/ps4-omega-eu/patch-v107-addresses.md): our Ghidra docs are the base eboot, confidence 98), the tone map object is a heap pointer and not yet read. **Twelve circuits, 2026-11-11 (`gallery-omega`): every circuit Track Select lists in WipEout HD mode, up from two.** `scripts/wipeout-pc-omega-pair.py pair` (own headless `sway` on the render node, the port started through its `run.py`, one uinput pad; `emu-run.py` watchdog, about 90 s a boot) walks the same route as `shadps4-omega-pair.py` (whose helpers it imports) and records the port commit (`84690ed`), its patch list and ours. What it adds over the shadPS4 walker: (1) **`SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=1`**, without which the window never sees the pad; (2) a `for_window` rule making the window full screen, without which the frame carries the port's title bar and a 2 px border; (3) **a fresh save every boot** (`GOW3_USER_DIR` deleted first) so Track Select opens on Vineta K and Ship Select on its first craft, with the GPU caches in one persistent directory; (4) refusal while another Xbox-360-shaped pad exists (the pad is system-wide); (5) **proof of the state in the pair directory**: `track-select.png`'s name crop and `ship-select.png`'s Feisar logo are compared with reference crops made once from the original (a wrong circuit's name reads 0.15 or more against the right one's 0.00 to 0.04; limit 0.08), the countdown board must not have risen 0.08 above its reading at the first HUD frame (the board sits over scenery, so its absolute reading is 0.19 below to 0.05 above zero depending on the circuit), and the frame must show no window bar. The reference crops (`refs/` beside the pairs: the twelve name crops `NN.png` by Track Select step and `feisar.png`) were made once from a Track Select and Ship Select walk of the original, cropped at `700x60+70+360` and `420x140+220+170` of the 1920x1080 frame; the driver is re-run with `--name-ref refs/NN.png --ship-ref refs/feisar.png`. Track Select order on a fresh save, step 0 to 11: Vineta K, Anulpha Pass, Moa Therma, Chenghou Project, Metropia, Sebenco Climb, Ubermall, Sol 2, Modesto Heights, The Amphiseum, Talon's Junction, Tech De Ra (all selectable on a fresh save; Right wraps after 12). Ours races all twelve (`Data\Environments\<folder>\track.vex`, folders `01_vineta_k`, `15_anulpha_pass`, `03_moa_therma`, `04_chenghou_project`, `02_metropia`, `10_sebenco_climb`, `05_ubermall`, `12_sol_2`, `modesto_heights`, `amphiseum`, `talons_junction`, `tech_de_ra`). Metrics, rmse / edge correlation / mean grey, adopted boot then a second cold boot, then the whole-frame normalised RMSE between the two boots of the original (frame pick, camera settling and hull livery all move it): Vineta K 0.188 / 0.21 / 29.2, 0.190 / 0.21 / 29.1, 0.098; Anulpha Pass 0.206 / 0.30 / 30.3, 0.217 / 0.27 / 32.3, 0.111; Moa Therma 0.230 / 0.16 / 38.5, 0.231 / 0.14 / 38.8, 0.119; Chenghou Project 0.193 / 0.15 / 30.1, 0.184 / 0.16 / 28.0, 0.116; Metropia 0.200 / 0.28 / 35.4, 0.206 / 0.26 / 36.7, 0.108; Sebenco Climb 0.224 / 0.14 / 40.2, 0.229 / 0.16 / 41.4, 0.120 (the second boot's grab shows the timer at 0.00.1, just past GO); Ubermall 0.149 / 0.29 / 21.1, 0.150 / 0.28 / 21.6, 0.088; Sol 2 0.214 / 0.14 / 33.2, 0.212 / 0.16 / 32.5, 0.141; Modesto Heights 0.238 / 0.09 / 40.7, 0.236 / 0.09 / 40.4, 0.138; The Amphiseum 0.330 / 0.10 / 56.5, 0.324 / 0.13 / 55.0, 0.120 (the original was read as much brighter than ours; on main `13b22ac77` ours is 5.0 grey brighter, signed, and the 49 is decorrelation: [omega-status.md](../formats/omega-status.md#hd-mode-frame-brightness-the-tone-map-is-not-the-gap-2026-11-20-omega-hd-exposure)); Talon's Junction 0.203 / 0.17 / 32.4, 0.200 / 0.18 / 31.9, 0.081; Tech De Ra 0.198 / 0.29 / 32.7, 0.195 / 0.30 / 32.1, 0.089. A boot-to-boot spread of 0.08 to 0.14 against a pair's 0.15 to 0.33 means the metrics are good to about 0.01 to 0.02, not to the third digit (confidence 70, two boots each). The re-captured Vineta K differs from the 2026-11-01 frame (port commit `e56904d27a1e`) by whole-frame MAE 0.049 (upper half, no hull: 0.046), the same size as boot to boot; the port's later commit is not separated from that noise. **The port runs the Omega profile's `GOW3_ENGINE_FPS=240`** (`scripts/game_profiles.py`; the engine steps by the time between frames), so the "60 FPS" the 2026-11-01 record carried is the ini's patch choice (no frame-rate patch), not the engine rate (the profile already set 240 at `e56904d`); the driver records `engine_fps_ini` and the log line from this revision on (checked on a Vineta K run of the committed script), while the twelve adopted pairs carry only `engine_fps: 60`, the ini value. **WipEout 2048 mode, 2026-11-12 (`gallery-omega-2048`): eight of its ten circuits paired, on top of the twelve.** `wipeout-pc-omega-pair.py pair --game-mode 2048` (`--class-steps N`, `--hud-labels-ref`, `--hud-timer-ref`). The walk: Race Box, GAME MODE one Right (WipEout HD to WipEout 2048); SPEED CLASS is C, B, A, A+ (Down twice from GAME MODE reaches it, Right steps it; a fresh save opens on C CLASS; Sol, "speed class A or A+ only", is raced at A CLASS, `--class-steps 2`); confirm on that row goes straight to Track Select, whose order on a fresh save, step 0 to 9, is Altima, Rockway Stadium, Capital Reach, Downtown, Queens Mall, Metro Park, Sol, Unity Square, Subway, Empire Climb (the `environments2048` folders `altima`, `arena`, `bridge`, `cathedral`, `mall`, `park`, `sol`, `square`, `subway`, `tower`; the ten name crops are 0.148 or more apart, the limit being 0.08). Ship Select opens on the yellow Feisar2048 (no Downs). **The race load ends in an intro that loops** a fly-over of the circuit and, after one tap of cross, a craft presentation, under a START RACE prompt, and returns to the fly-over: a second tap about 2.5 s after the first starts the race, so the driver taps every 2.5 s until the HUD shows and stops then (no tap, and the single tap HD mode uses, both ended in the loop). **The 2048 HUD** (LAP 1/4, POS 8/8, TOTAL, CURRENT, a temperature gauge) is white text at full strength on the grid, TOTAL reading 0.00.00 for at least 2.5 s and then running before GO is seen on the board. Telling it from the fly-over needed a reference: a white-pixel fraction (1.0 on a fade, 0.39 on a bright sky, about 0.10 on the HUD) and a bottom-bar line both failed. What holds is the normalised RMSE of the frame's crop, thresholded to a white mask (gray over 97 %), against a reference mask made once from a HUD frame of the original: the label block (`270x230+0+0` of 1920x1080) reads 0.00 to 0.07 on HUD frames (0.15 with white scenery behind it) and 0.34 or more on every fly-over, presentation, fade and menu frame of six exploratory boots (about 180 frames), limit 0.20; the TOTAL reading (`200x50+280+138`) reads 0.00 and 0.04 on two frames showing 0.00.00 (0.05 to 0.11 in the pair runs) and 0.35 on 0.02.16, limit 0.15, and is the before-GO check that replaces HD's countdown-board check (both are recorded in `pair.json`'s checks). **What it has been shown to separate is 0.00.00 from 0.02.16; a reading a few hundredths past GO was never seen**, so a late HUD detection (the thinnest label margin is Sol's 0.146 against 0.20) could pass a frame just after GO, which is how the HD route let one Sebenco Climb boot through; every `side.png` of all eleven 2048 boots was read by eye and shows 0.00.00. The grab is the first HUD frame plus 2, as in HD mode; the HD yellow-ring detector is not run. The reference masks are made from a game frame and so live beside the pairs (`omega/refs2048/`: `00.png` to `09.png` name crops by Track Select step, `feisar2048.png`, `hud-labels.png`, `hud-timer.png`). A false negative stalls the run, so every threshold was set against the worst false positive. **Metrics**, rmse / edge correlation / mean grey (the page's own scores), adopted boot then a second cold boot where one was taken, then the whole-frame normalised RMSE between the two boots of the original: Altima 0.236 / 0.05 / 39.66, 0.234 / 0.06 / 38.92, 0.109; Capital Reach 0.296 / -0.01 / 55.30, 0.296 / -0.01 / 55.53, 0.071; Sol (A CLASS) 0.258 / -0.02 / 48.31, 0.259 / -0.02 / 48.53, 0.074; Subway 0.240 / 0.16 / 41.07, 0.237 / 0.18 / 40.49, 0.086; Downtown 0.243 / 0.00 / 40.54; Queens Mall 0.296 / -0.02 / 52.56; Metro Park 0.273 / 0.02 / 47.23; Empire Climb 0.306 / 0.02 / 58.67 (one boot each, confidence 55 for those four; confidence 70 for the four with two boots, the metrics repeating to 0.003 rmse and 0.8 mean grey). The geometry lines up (read by eye on all eight `side.png`: the same arches, stands, banners and kerbs) and the pair's difference is the HUD and the light; **the edge correlations near 0 (HD pairs 0.09 to 0.30) are the two HUDs not overlapping at all and the light, not a pose mismatch** (`craft_offset_px` is recorded in each `pair.json`, informational). **Not matched, recorded as caveats in each pair:** (1) ours drew HD's HUD in Omega's 2048 mode (lap hexagon, position disc, BEST and time bar, speed readout) where the original draws 2048's: checked, differs (visible in every `side.png` made before 2026-11-12; since `omega-2048-hud-laps` ours draws 2048's, `docs/formats/omega-status.md`, "2048 mode: HUD and laps"); (2) the original's Feisar2048 hull is lit and graded by the scene (orange at the sunset circuits, yellow and white on a grey one; identical across two boots) and stands in a white start ring, ours lights it flatter and draws no ring; (3) **the class mapping is assumed and its lap counts disagree**: the original races 4 laps at C CLASS (all seven C circuits) and 5 at A CLASS on Sol, ours on Sol shows venom 3, flash 4, rapier 4, phantom 5 laps (and 3 on Altima at venom), so the pairs use venom for C and rapier for A (the ladder's order) and the lap count differs; matching laps would read C as flash or rapier and A as phantom, which a physics measurement, not a grid still, would have to decide. **Two circuits not paired: Rockway Stadium (`arena`) and Unity Square (`square`)**, because ours cannot race them: the load panics with wgpu "Buffer size 308016200 (arena) / 310295700 (square) is greater than the maximum buffer size (268435456)" on the circuit's one `vertices` buffer (`oag_mesh::mesh_render::device_descriptor` raises the texture-resolution and binding-array limits, not `max_buffer_size`, so wgpu's 256 MiB default stands). The other eight load and race. **2048 cross-check: checked, differs.** 2048's own `arena` and `square` (`oag-game <PCSF00007> --race --track 'Data\art\published\environments\arena\track.vex' --mode single_race`, same for `square`) load and render (192,202 and 195,172 triangles in the frame, Altima 201,323), because Omega's re-cooked meshes are about 3.5 times heavier (Altima 726,699 triangles in the frame, so the two largest circuits' one vertex buffer passes 256 MiB). **Fixed 2026-11-12 (`big-vertex-buffer`): ours now loads and draws both, its vertex buffer cut in two ([`vertex-buffers.md`](../rendering/vertex-buffers.md)); the pairs need only the original's capture.** The fix was an engine change, outside a capture lane; the two original frames were not captured because the pair is one atomic run. The four Zone circuits (`zone_1` to `zone_4`) were not paired: Zone is a different race type (a `zone` pair kind with its own walk), and ours decodes `zone_3` to no triangles on a single-race load. **2048 and Omega cross-check:** checked, differs (Omega's 2048-era race draws HD's HUD where the original draws 2048's). Open: the class mapping (A+ CLASS's lap count), `arena` and `square`. |

## The world clock and random craft motion, per title (census 2026-11-18, `compare-clock`)

Maintainer, 2026-11-17: "properly match world clocks in all titles for comparison shots, as well as removing random wiggle/tilts of
the crafts, as we already do in HD". Two questions per pair script: does the original's scenery clock go into `pair.json`
(`original.world_clock_offset_s`) and into ours (`--world-clock-offset`), and is every random motion that is applied **after the
body** (drawn hull, camera rig) off on both sides. A motion that lives in the body is already carried by a pair that poses ours
from the original's body (`--pose`, the field file, `--read-pose`); only post-body motion needs a patch and a flag.

| Title / script | World clock read | Passed to ours | `compare-rerender-ours` keeps it | Confidence |
| --- | --- | --- | --- | --- |
| Pulse PSP `psp-pulse-pair.py` (grid, race, Zone) | `Mesh_UpdateTextureTransforms` `0x0890e160` `mesh+0x40`, most common non-zero value over 40 hits, less the HUD clock | yes, `ours.world_clock_offset_s` | yes (flag stored; race shot recomputed) | 85 for the read (24 states, two loads identical); the "time since the circuit loaded" meaning is inferred |
| Pulse PS2 `pcsx2-pulse-pair.py` | `*(g_ingame + 0x40)` in the state file's EE RAM (`g_ingame` `0x27DDBC`), run on by the race clock to the grab | yes | yes (flag stored; rebuilt from state file if absent) | 75 (`anim-transform.md`) |
| Pure PSP `psp-pure-pair.py` | `*(0x08a8c5f4) + 0x44` (the world-state object's running time), read at the same stop as the race clock, less the presented-frame lag (`psp-pure-usa/world-clock.md`) | yes, `ours.world_clock_offset_s` = world clock less ours' tick over 60 | yes (flag stored) | 85 for the read (three loads of a state: one frame apart each), 75 that it places the scenery (one pair, `10_Sebenco_Climb`) |
| HD `rpcs3-hd-pair.py` | `MeshImporter` nodes (vtable `0x00869fd0`) `+0xc0` with `+0x15c` stamp, most common value past the gantry windows, RAM in the same pause as the frame | yes | yes (flag stored) | 85 (`rpcs3_hd_field.scenery_clock`, 12 circuits) |
| HD sea pair `hd-sea-place-dump.py` | none until this lane; now reads the same `scenery_clock` and the lap clock in the pause and writes `world.json` (`world_clock_s`, `lap_clock_s`, `world_clock_offset_s`) | by hand: `--world-clock-offset` = `world_clock_s` less `ticks / 60`; the sea pair still has no `pair.json` | n/a | 85, same read as `rpcs3-hd-pair.py`; see "Closed" below |
| 2048 `vita-2048-pair.py` (`--walk`) | the value ~1,000 scenery nodes share, in pairs `0x40` apart, scanned from `/proc/<pid>/mem` before the Retry and re-read at each grab (`vita-2048-eu-v104/world-clock.md`) | yes, `ours.world_clock_offset_s` = world clock less ours' tick over 60 | yes (flag stored) | 85 for the read; 60 that it places the scenery (the one pair's vehicle was 2.5 s off) |
| Omega `shadps4-omega-pair.py`, `wipeout-pc-omega-pair.py` | **none** | no | no | memory **is** readable (the native port, `/proc/<pid>/mem`, below), not scanned in the `clock-titles` lane; the executable's node class is not read |
| Weapon, camera and menu pair scripts (`psp-weapon-pair.py`, `psp-camera-pair.py`, `hd-weapon-pair.py`, `hd-missile-film-pair.py`, `rpcs3-hd-trail-hit-pair.py`) | none | no | no | crops of a craft or an effect; a moving prop elsewhere in the frame is not matched. Open, low priority |

| Title | Random craft motion the original applies | Post-body? | Off on the original | Off on ours | Evidence |
| --- | --- | --- | --- | --- | --- |
| HD | drawn-hull shake (`HullShake_Update`/`_Apply`, `ship+0x6b00`, cubic B-spline of `rand()` points, faded by thrust) | yes | yes: `bl 0x000d39d0` at `0x000ebc84` nopped by `rpcs3-hd-no-hull-shake.py` (`original.hull_shake`) | yes: `--no-hull-shake` (`compare_env.ours_flags`, re-render adds it) | `ps3-hdfury-eu/hull-shake.md`, confidence 72 to 78 |
| HD | `HullShake_Apply` with `r5 != 0` (the 2048 "internal tripod" counterpart) | yes if a caller passes it | **unknown**: the `r5 != 0` path `0x000d3dd0` is unread; Ghidra lists no call xref to `0x000d39d0` (the call is a direct `bl`, the data xref is the OPD), so the callers are not enumerated | n/a (ours draws no camera shake) | `hull-shake.md`, "What is not read" |
| 2048 | drawn-hull shake, HD's constant for constant; plus the chase camera's "internal tripod" shake (`0x811be926`, `invert = 1`) | yes | hull: yes, two `bl 0x811b5500` overwritten through `/proc/<pid>/mem` (`--walk`). **Tripod: no**, nopping it left the HUD down | hull: `--no-hull-shake`. Tripod: ours has none, so the camera differs by the original's shake | `vita-2048-eu-v104/hull-shake.md`, confidence 85 / 70 |
| Omega | drawn-hull shake, HD's (`0x012f7200` in v1.00) | yes | **no patch**: the running v1.07 executable is not in Ghidra and `Apply` was not read | `--no-hull-shake` is passed, so ours is off and the original's is on | `ps4-omega-eu/hull-shake.md`, confidence 80 for `Update` |
| Pulse PSP / PS2, Pure | none found. The random generators (libc `rand`, the particle RANROT) feed AI, pickups, HUD, particles, clouds, mines; none writes a craft's display matrix. The Zone-mode hits and weapon-hit shakes are events, not idle motion, and are carried by posing from the body | n/a | n/a | `hull_shake` is `None` in these titles' race data, so nothing to turn off | `psp-pulse-usa/prng.md` lines 22-23; absence of evidence, **confidence 65** (no instruction sweep of the display-matrix builder was done for Pure) |

**Closed and open (2026-11-18, `compare-clock`).**

- **Closed, HD sea pair.** `scripts/hd-sea-place-dump.py` now reads the scenery clock and the lap clock in the same pause as the dump and
  the window grab. Two attached runs on one Moa Therma Time Trial restore: world clock 91.043 s / lap 8.645 s and 119.976 s / lap 37.578 s,
  offsets **82.3983 and 82.3981 s** (a third pause, 88.743 / 6.345, gave 82.3983): the offset is stable to 0.0003 s, the clock itself is
  not (it runs on through the wait), which is why a pair must read it. The 119.976 s frame is the discriminating one: it shows a liner
  (yacht) at the left edge of the sea and the airship under the ring, and ours at `--world-clock-offset 119.810 --ticks 10` draws both,
  the yacht 0.2 s of travel from the original's (it sits 30 px further right at 1920x1080), where ours at the old pair's `--ticks 572` (clock
  9.5 s) draws neither. A sweep of the offset (0, 30, 60, 90, 100 to 125, 130, 140, 150 s) puts a yacht in the box at 60, 100 and 121 to 125 s and none at 0, 30, 90, 105 to 119, 130, 140 or 150 s (the box metric jumps
  from 34 to 54 at 60 s and 41 to 45 at the others), so the prop is placed by the offset to within about a second, and the falsifier the lane set (the minimum landing elsewhere) did not
  fire. **Region metric** (mean abs RGB difference, 1920x1080, yaw fitted below): yacht box `0,330,280,530` 34.5 before and 35.4 after; the metric
  does not move because the box is mostly sea and the ring above it differs (next bullet): the picture is the evidence, read in the
  `theirs.png` / `ours` pair. **Confidence 80** that the world clock is what places the yachts.
- **The dump camera is a frame newer than the window frame.** The pushbuffer dump and the window grab of a moving (falling, placed) craft
  differ by a frame or two: the camera decomposed from the dump (`ps3_pose.pick_camera` needs two dumps; here the one frame-unique
  dual-register candidate after removing the first dump's, eye `196.926 41.743 -367.921`) renders about 12 degrees of yaw away from the window's frame (a
  yaw sweep -17 to +15 degrees, fitted on the tower and pad structure: minimum at -12 to -13.5). So a sea pair is only registered after a yaw
  fit; the craft should be held still (not falling) for a dump and a grab to agree. **Open:** a placement that rests, so no fit is needed.
- **Open, ring panel strobe.** At the original's clock ours draws the ring's glass panels cyan where the original's are dark blue; at
  other clocks ours is dark (offsets 0 to 150 give a dark ring, 119.81 a lit one). The panels' phase is not the world clock's
  linear read, or they run on a faster flicker; not investigated.
- **HD: every grab of the original records the world clock (2026-11-18, `hd-clock-gate`; maintainer: "the tooling shall also ensure that future HD/Fury screenshots will include world clock time").**
  The one grab path is `grab_with_clock` in `scripts/rpcs3-drive.py` (`screenshot()` and `record_clock()` wrap it; `rpcs3-drive.py grab PATH` is
  the shell form). It takes the frame, then reads the clock through `rpcs3_hd_field.world_clock_record` and writes `<stem>.clock.json` beside the
  image (`world_clock_s`, `lap_clock_s`, `world_clock_offset_s`, `nodes`, `total`, `scan`, `screen`, `grabbed_at`). Outside a race (no player craft with a
  lap clock in the craft array) it writes `null` and a `reason` instead of a clock: after a race the freed `MeshImporter` nodes still read a plausible
  frozen time, so the read is gated on a live player craft. The first read of a process scans the heap (0.7 to 1.3 s over `serve`'s socket), later
  ones re-read 24 cached nodes (0.01 s) and rescan when they stop agreeing. `rpcs3-drive.py capture` takes its `ship.scenery_clock` from the same
  grab, so `rpcs3-hd-pair.py`'s `original.world_clock_offset_s` / `world_clock_s` come from this path. Caveats: `world_clock_s` is read right
  after the grab, so on a running game it is the moment of the read, not of the presented frame (which lags it); and the offset `world clock - lap
  clock` is per moment, because the lap clock restarts at each lap (offset 5.84 s on lap 1 and 48.12 s on lap 3 of one Talon's Junction state).
  A film (`rpcs3-hd-shipselect-film.py`, ffmpeg x11grab) gets one sidecar for its start (`film.clock.json`); its frames are not stamped.
  Two checks: `just check-hd-clock-grab` (`scripts/check-hd-clock-grab.py`) fails an RPCS3/HD script that shells out to `import`, `xwd`,
  `grim` or `x11grab` itself, and a helper that stopped writing the sidecar; `check-pair-provenance` (rule `hd_world_clock`, in
  `compare_provenance.py`) fails an HD race, scenery or zone pair without a numeric `original.world_clock_offset_s`, and a menu or loading pair
  without `original.world_clock_note`; `Pair.finish` refuses it at make time too (override: `--accept-provenance hd_world_clock=REASON`). `HD_CLOCK_BASELINE` is empty since the re-capture below: every HD pair records its clock (or the no-race note), and an entry may not be added. Other titles: not covered, see the rows above.
- **Pure and 2048 world clocks (2026-11-18, `clock-titles`).** Pure: closed for the read, one pair (`10_Sebenco_Climb`, Time Trial restart, race clock
  0.80 s): world clock 10.84 s, ours at offset 5.508 s and tick 320. The cyan energy beam across the track scrolls with the clock; a sweep of ours' offset (0 to
  8 s in 0.25 s steps, 9, 12, 20) correlated with the original on the beam band gives 0.862 at 5.5 s (0.853 at 5.75 s, 0.43 at 0, 0.22 at 7.0, 0.68 to 0.70 at
  12 and 20), so the falsifier did not fire (`psp-pure-usa/world-clock.md`). 2048: the read works (the pair script scans the guest for the value ~1,000 scenery
  nodes share and re-reads it at each grab), but the one pair made (`dlc-vineta-k`, clock 28.17 s) places its animated element, a dark vehicle crossing the sky,
  2.5 s before the read clock, so **2048 stays open**: the read is not shown to be the clock that places the vehicle (`vita-2048-eu-v104/world-clock.md`).
  Omega: the 2026-11-18 census said shadPS4 exposes no readable guest memory; **the native port's does** (`/proc/<pid>/mem` of the launching process, image base
  `0x800000000`, "Second original, 2026-11-01" above), so the scan is possible there; not run in this lane, open. The scan needs the nodes of the HD class
  (`MeshImporter`), unread in the PS4 executable.
- **Open, random craft motion.** HD is closed (hull shake). 2048: the chase camera's "internal tripod" shake stays on in the original
  and ours draws none, a difference no patch can remove (nopping it kept the HUD down); the alternative is a flag on ours that draws the
  same shake from the same law (`HullShake::HD` already ships), recorded as `ours.camera_tripod_shake`, and a pair would then match
  in law, not frame for frame. **A patch that keeps the call is read, not tried (2026-11-18, `clock-titles`):** zero the three products that make a new spline point
  (`0x811b5380`, `0x811b53a4`, `0x811b53ca` in `HullShake_Update`), `vita-2048-eu-v104/hull-shake.md`, "Where the shake's random points are made". Omega: the hull shake is on in the original (no patch located for v1.07). HD `Apply` with `r5 != 0`: callers
  not enumerated (Ghidra holds no call xref to `0x000d39d0`); recipe: scan `.text` for `bl` encodings that resolve to `0x000d39d0` from the
  extracted `EBOOT.elf`, which is not under `data/extracted`.
- **The re-render keeps a recorded offset.** `compare-rerender-ours.py` now adds `--world-clock-offset` from `ours.world_clock_offset_s`
  when a stored pair's flags lack it (any title).

## Every HD/Fury pair re-captured through the clock-recording grab (2026-11-18, `hd-reclock`)

Maintainer, 2026-11-18: "redo all HD/Fury screenshots with the correct clock". All 19 Time Trial, menu and loading pairs under
`data/reference/compare-page/hd/` were re-made in one session by `just compare-rerender-all --platform rpcs3` (cold boot, hull shake patched out by
`rpcs3-hd-no-hull-shake.py`) plus two hand runs of `rpcs3-hd-menu-pair.py` (the menu walk and `--walk loading`), every grab going through
`grab_with_clock`. Each `pair.json` now carries the original's `world_clock_offset_s` (race pairs) or the no-race `world_clock_note` (menu and
loading), ours is re-rendered at that offset with `--no-hull-shake`, and `HD_CLOCK_BASELINE` is empty. **`hd-race/talons-pack` was not re-made**
(below); it already records a numeric offset, read in the same pause as its frame.

| Pair | Clock offset now (s) | Before (s) | Mean grey difference | Edge NCC |
| --- | --- | --- | --- | --- |
| `hd-race/talons-pack` | 66.58 | 66.58 | 23.09 to 23.09 | 0.09 to 0.09 |
| `hd/anulpha-pass` | 21.34 | 37.52 | 3.62 to 4.36 | 0.43 to 0.44 |
| `hd/chenghou-project` | 21.34 | 35.13 | 2.58 to 2.9 | 0.4 to 0.39 |
| `hd/loading-loading` | null (menu or loading) | none | 1.73 to 2.07 | 0.49 to 0.48 |
| `hd/menu-campaign-select` | null (menu or loading) | none | 3.76 to 5.09 | 0.24 to 0.23 |
| `hd/menu-cell-select` | null (menu or loading) | none | 4.81 to 5.07 | 0.46 to 0.46 |
| `hd/menu-grid-select` | null (menu or loading) | none | 7.94 to 7.69 | 0.32 to 0.31 |
| `hd/menu-ship-select` | null (menu or loading) | none | 8.68 to 5.69 | 0.41 to 0.41 |
| `hd/menu-track-select` | null (menu or loading) | none | 10.42 to 11.54 | 0.36 to 0.35 |
| `hd/metropia` | 35.46 | 34.66 | 3.12 to 3.67 | 0.45 to 0.45 |
| `hd/moa-therma` | 32.61 | 40.47 | 4.46 to 6.36 | 0.46 to 0.45 |
| `hd/modesto-heights` | 34.80 | 40.09 | 5.41 to 4.99 | 0.41 to 0.42 |
| `hd/sebenco-climb` | 35.18 | 35.01 | 5.12 to 5.01 | 0.39 to 0.39 |
| `hd/sol-2` | 29.65 | 34.75 | 4.53 to 6.45 | 0.43 to 0.43 |
| `hd/talons-assegai-c1` | 33.68 | none | 3.1 to 4.94 | 0.43 to 0.41 |
| `hd/talons` | 34.15 | 39.72 | 3.03 to 3.49 | 0.43 to 0.43 |
| `hd/tech-de-ra` | 34.24 | 31.39 | 5.42 to 4.69 | 0.44 to 0.45 |
| `hd/the-amphiseum` | 35.96 | 40.04 | 4.68 to 6.28 | 0.43 to 0.41 |
| `hd/ubermall` | 34.39 | 38.36 | 3.8 to 4.54 | 0.45 to 0.45 |
| `hd/vineta-k` | 33.26 | 40.16 | 2.77 to 3.39 | 0.44 to 0.43 |

The offset moved by up to 16 s between two boots of the same circuit and hull (Anulpha 37.5 to 21.3, Moa Therma 40.5 to 32.6): the world clock at the
start of a Time Trial is not a constant of the track, which is why a stored offset is never reused. Side by side, every Time Trial pair lines up
(craft, HUD, moving props, the Moa Therma airship, signs); the grey difference moved by under 2 and edge NCC by under 0.03 on all 13 Time Trial pairs.
Menu mean grey 5 to 12 (before 4 to 10) is the same order. Maintainer, 2026-11-18: re-making "does not have to reuse the same ship/craft/skin, as long
as ours also uses the same in the end" - both changes below follow that.

- **Harimau is the standard hull now.** The classic hull (the `down` on Ship Select) is locked on this machine's profile
  (padlock on the third hex of the column), so the cold-boot walk stops on Team Selection. No press count holds across profiles:
  the cold boot now steps Ship Select until RAM names the pair's `--team` hull ([rpcs3-capture.md](../reverse-engineering/rpcs3-capture.md), "Picking the hull on Ship Select").
  RAM's team name is `harimau` either way and the pair's own `the player's craft (RAM) is the craft ours draws` check passes. The 12 pairs'
  captions say "Harimau (standard hull)". `talons-assegai-c1` (`(7, 0)`) was already the unlocked hull.
- **The save moved (2026-11-26, `hd-flame-regrab`): Ship Select opens on the Fury hull, and Pilot Assist was on.** On the
  maintainer's save of that day Ship Select opens Harimau's column on the Fury hull, which RAM names `Harimau_c1`
  (`*(*(ship+0x6298)+0x1b4)`, `Data\Ships\Harimau_c1`); one `down` gives `Data\Ships\Harimau`, the orange, yellow-green and blue
  hull the 12 stored grid pairs fly (read by eye on all 12 regrabs against the stored frames). Assegai's column opens on `Assegai_c1`. No press count is right on every save, so the cold boot steps Ship Select
  until RAM names the pair's `--team` hull (`rpcs3-hd-ship-step.py select --hull`, called by `emu-hd-glass-boot.sh`;
  `compare-rerender-all.py selftest` holds that `--team` reaches it). The option byte `*(*0x008b098c)+0x473` read `01 01`, so every race frame drew the
  indicator's bracket under the craft while the profile renders ours off. Two checks in `rpcs3-hd-pair.py` now refuse both:
  the player's hull directory read in the capture's pause against `--team` (a Fury `Harimau_c1` grab with `--team harimau` had passed
  all nine earlier parity checks), and the Pilot Assist byte against the profile's `pilot_assist`
  (`check-compare-environment.py`, `check_hd_hull` and `check_hd_pilot_assist`). `rpcs3-hd-cold-pair.sh` writes the option off
  before `Restart Race` (`scripts/rpcs3-hd-pilot-assist.py --off`; the save is not written), and the bracket is gone from the frame.
- **The original's Menu Style must be FURY for a menu or loading pair.** This profile boots the front end in the HD (white) style and ours' `--menu-page`
  draws only the Fury one whatever `display.front_end_style` says (mean grey 115 to 240 against 4 to 10 on the first try), so the original is switched: `Main Menu` ->
  `OPTIONS` -> `GAME`, down 8 rows to `MENU STYLE`, `right`, **`cross`** to confirm (`circle` cancels and the style reverts), back with `circle`.
  The choice then survives a reboot of the same private config. Presses on this emulator drop about one in three, so every step reads the screen
  (`rpcs3-drive.py status`) or the frame's brightness (below 0.3 on a Fury menu) before the next.
- **`compare-rerender-all.py` now skips a front-end walk** (`rpcs3-hd-menu-pair.py` pairs are re-made by that script itself, in one walk: its argv has no
  state to restore) and refuses a pair with no `--state` that has no cold-boot route by name instead of a `ValueError`.
- **Cold boots fail more often than 1 in 6 on a first pass**: `anulpha-pass`, `chenghou-project` and `metropia` failed on the first
  pass (`emu-hd-glass-boot.sh` exit 1, in the boot step, cause not read), `sol-2` was refused by the capture parity check ("capture has no camera"); a second and third pass
  (`--resume`, `--attempts 3`) made all four.
- **The Oct 20 savestates on the second workstation do not load here**: restoring `hd-timetrial-*-harimau-classic-start` crashes this RPCS3 build
  (`0.0.42-19777`) in `ppu_load_prx` ("Verification failed ... errno=2"), so a savestate cannot stand in for a cold boot on a profile with the classic hull locked.

### Why `hd-race/talons-pack` stayed

Two re-makes from the local `hd-race-talons-venom-pack` state (the restore with `--pause` lands at a variable game time) came out at lap 1, 0:33 at 460
km/h in a banked turn, not the stored pair's lap 2, 0:07: ours draws the craft small and centred where the original's is large at the bottom left (edge NCC
0.09 before, 0.03 and 0.04 after; mean grey 23 before, 46 and 48), the craft-pose-versus-presented-frame lag of "A moving HD craft" above, worse at that speed, and the
pair's caption still says lap 2. The stored pair's clock (offset 66.58, world clock 73.9, lap clock 7.3 s, equal to the HUD's 0:07.2) was read in
the same pause as its frame, so it keeps its place; a re-make wants the state it was made from (a 140 MB state dated 2026-11-10 on the second
workstation, which stopped answering ssh before it could be fetched) and a stop at a calm moment.

## Pad seats: two agents' emulators at once (2026-11-16, `seat-pads`)

A uinput pad is a system-wide device: every emulator on the machine sees every pad, so the pair drivers used to refuse to start
while any Xbox-360-shaped pad existed, and two agents could not drive emulators at the same time. A **seat** gives one agent's pads
their own identity and tells that agent's emulators to look at nothing else (`scripts/pad_seat.py` holds all of it).

| | Seat `N` (`OAG_PAD_SEAT=N`, 1..255) | No seat (the default) |
|---|---|---|
| Pad names (`scripts/uinput-pads.py`) | `OAG seat N pad A`, `B`, ... | `OAG test pad A (Microsoft X-Box 360 pad)` |
| USB ids | vendor `045E`, product `4F00 + N` (`4F01`..`4FFF`) | `045E:028E` (what `oag-game`'s split-screen walk reads through gilrs) |
| SDL emulator (the port, shadPS4) | `SDL_GAMECONTROLLER_IGNORE_DEVICES_EXCEPT=0x045e/0x4f0N` plus `SDL_GAMECONTROLLERCONFIG` with an Xbox-360 layout for that id (`pad_seat.sdl_env`) | `...EXCEPT=0x045e/0x028e` (the original pads only: a seated agent's pad, and RPCS3's DualShock-3 ones, stay invisible) |
| RPCS3 (evdev handler, by name) | `OAG seat N pad rpcs3`, DualShock-3 ids; `OAG_RPCS3_PAD_NAME` still wins; `scripts/emu-env.sh` writes the name into the private input profile | `OpenAntiGrav Virtual Pad` |
| PCSX2, Vita3K (X key events, no pad) | `pad_seat.sdl_blind_env`: `...IGNORE_DEVICES_EXCEPT=0x0000/0x0000`, so they see no pad of any seat | same (always applied) |
| A driver refuses to start when | its own seat's pad already exists | an original-id pad exists (as before) |
| Sway runtime dir of a `Rig` | `/tmp/oag-omega[-port]-<lane>-sN` | `/tmp/oag-omega[-port]-<lane>` |

Run each agent's drivers with its own `OAG_PAD_SEAT` (the lead hands out the numbers), and give each its own scratch directory
and display. A driver started without a seat behaves as before and is not disturbed by another agent's seat pad: it still refuses
when an original-id pad exists, and its SDL emulator sees only the original ids.

**Re-checking an emulator's SDL** (after an update, or for one not yet probed): make pads on two seats and ask the library
what a program with each seat's environment sees. `--lib` takes any `libSDL3.so.0`, such as the ones RPCS3 and PCSX2 bundle.

```sh
mkfifo a b; (sleep 300 > a &); (sleep 300 > b &)
OAG_PADS=1 OAG_PAD_SEAT=1 setsid uv run --with evdev python3 scripts/uinput-pads.py < a &
OAG_PADS=1 OAG_PAD_SEAT=2 setsid uv run --with evdev python3 scripts/uinput-pads.py < b &
python3 -I scripts/pad_seat.py probe --all            # no hint: every pad
python3 -I scripts/pad_seat.py probe --seat 1 --lib /opt/rpcs3/usr/lib/libSDL3.so.0   # only "OAG seat 1 pad"
echo quit > a; echo quit > b                          # the pads vanish
```

**What was measured (2026-11-16, confidence 90 for SDL3, the system's 3.4.20 and the copies bundled with RPCS3 and PCSX2).**
`SDL_GAMECONTROLLER_IGNORE_DEVICES_EXCEPT` filters both the gamepad and the joystick lists (a ctypes probe with seat 1's and seat 2's
pads: no hint lists both, EXCEPT seat 1 lists seat 1, EXCEPT `0x0000/0x0000` lists none). SDL3 maps the new product id from the
pad's evdev capabilities by itself, to the same layout `XBOX360_LAYOUT` supplies; the supplied mapping is insurance for an SDL
without that guess (the GUID's name-CRC field is left zero, which SDL matches against any name). The wipeout_pc port reads the
gamepad API (`SDL_GetGamepads`, `SDL_OpenGamepad`), so the hint reaches it: two ports on seats 1 and 2 at once each logged
`gamepad connected: OAG seat N pad` and no other. Vita3K's binary carries the hint string (statically linked SDL, not probed).
**shadPS4 is not verified**: `/usr/lib/shadps4` is `drwx------ root` on this machine, so it cannot be started or inspected
(`chmod -R a+rX /usr/lib/shadps4` fixes that); `shadps4-omega-pair.py` passes it the same environment as the port.

**The proof, two seats at once** (two `wipeout_pc` ports, each in its own headless `sway`, started 2026-11-16 on seats 1 and 2;
both were on the "PRESS ANY BUTTON TO START" title): one press on seat 1's pad took only the seat-1 port to MAIN MENU, a second
moved only its highlight from CAMPAIGN to RACEBOX; a press on seat 2's pad then took only the seat-2 port to MAIN MENU, and two more
moved only its highlight to ONLINE while seat 1 stayed on RACEBOX. Judged on the full frames, since the title and menu
backgrounds animate. The seated runs used a scratch harness that calls the pair driver's `Rig` (sway, port, pad), not its `main()`. A single-seat run of `wipeout-pc-omega-pair.py` (no seat) still reached the grid and passed its seven parity checks.

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

### A fresh Vita3K profile and the circuits a fresh save reaches (2026-11-11, `gallery-2048`)

**Claim: the page carries a 2048 pair for every circuit ours can race (about 14, up from 3). Not met that day: no pair was added.** The profile was
installed, the game booted, and the walk stopped at the Game Mode grid because a tap did not register without the Vita firmware (confirmed by
the next lane: [vita3k-capture.md](../reverse-engineering/vita3k-capture.md), "A fresh profile on a new machine"; the pairs are in "Eleven pairs through a
renamed grid cell" below).

- **Pref-path (persistent, reused by later lanes):** `data/saves/2048/vita3k/` (`ux0/app/PCSF00007` base 1.04 with the patch merged,
  `ux0/addcont/PCSF00007/DLC1W2048PACKAGE` and `DLC2W2048PACKAGE`; 2.3 GiB). Installed with `vita3k -c <config> -w --pkg <pkg> --zrif <zRIF>`
  under a private `HOME` and `env -u XDG_CONFIG_HOME -u XDG_DATA_HOME`: `data/images/2048-vita-eu.pkg`, `2048-vita-eu-patch.pkg`, then
  `data/dlc/WipEout 2048 - DLC1 (Europe) (DLC).pkg` and `DLC2`. **The DLCs need their own zRIF** (the base one gives `header signature is
  invalid`; `data/keys/vita-zrif.tsv` carries only the base row); it comes from the pkgi pair list's rows `EP9000-PCSF00007_00-DLC1W2048PACKAGE`
  and `DLC2W2048PACKAGE`. Config keys that keep the boot unattended: `show-welcome: false`, `warn-missing-firmware: false`.
- **What a fresh save opens** (read off `data.psarc`'s `data/plugins/grids/grid_00..15.xml`, `PI_Grid`/`PI_Cell`): the base campaign opens
  `2048 - Event 1` and 15 side events with no gate (`2048-campaign.md`); the HD Campaign opens `grid0` (Uplift) and the Fury Campaign `grid8`
  (Blitzed, `Unlock Grid="None"`), and in each grid only the two cells that author `Locked="false"` (HD: `grid0_3_1` Vineta K Race and
  `grid0_3_2` Anulpha Pass Speed Lap; Fury: `grid8_3_1` `17_Track` Race and `grid8_3_2` `19_Track` Elimination). Every other grid needs the
  previous grid's earned points (`RequiredPoints` 10, 13, 16, 19, 22, 25, 30, 40 for HD) and every other cell its neighbours' medals.
  The track keys map `01..08_Track` to Vineta K, Metropia (`02`), Moa Therma (`03`), Chenghou Project (`04`), Ubermall (`05`), Sebenco Climb
  (`06`), Sol 2 (`07`), Anulpha Pass (`08`) (the `FEModelLocation` `track0N` in `dlc1.psarc`'s `plugins/tracks/Definition.xml`);
  Fury's `17..20` are Talons Junction, Amphiseum, Modesto Heights and Tech De Ra, `21..24` their reverses, `25..28` `Zone_1..4`.
  Metropia first appears in `grid2` (needs `grid1`'s 13 points), so **an unlock route is needed for Metropia and for most circuits**.
- **Unlock route, a hypothesis not yet tried (the emulator was not reachable past the Game Mode grid):** the grid record parsed by
  `FUN_810bd2a4` (EU v1.04) keeps `Locked` as a byte at `+0xac`, `RequiredPoints` at `+0xb0`, `FlyerName` inline at `+0xcc`; the cell record
  parsed by `FUN_810bbfce` keeps `Locked` at `+0xe9` and the track name inline at `+0xea` (0x18 bytes, `01_Track`). A scan of the
  guest's heap for those inline strings, then writing the two flag bytes through `/proc/<pid>/mem` (the channel the hull-shake patch
  uses), may open the map. Confidence 40: the map colours off the progress word `event+0x2d8` and the vtable's lock query
  (`campaign-map-projection.md`), which may not read these bytes.
- **`CameraP1`:** not readable off the save. `DATA.BIN` has no readable strings, so the `strings -a` route returns nothing; the options
  screen is the way, and was not reached.

### Eleven pairs through a renamed grid cell (2026-11-12, `gallery-2048b`)

**Claim: the page carries a 2048 race pair for every circuit ours can race (about 14), up from 3. Partly true: 14 circuits, not every one.** Eleven pairs
were added (the HD pack's Metropia, Anulpha Pass, Moa Therma, Chenghou Project, Ubermall, Sebenco Climb and Sol 2, and the Fury pack's Talons Junction,
Amphiseum, Modesto Heights and Tech De Ra), which with Empire Climb, Queens Mall and Vineta K makes 14. **Not paired: the other eight base circuits
(Altima, Arena, Bridge, Cathedral, Park, Sol, Square, Subway) and the four Zone circuits**, and the reversed circuits (below).

- **Vita3K and its firmware.** `PSP2UPDAT-firmware.PUP` (128 MB, firmware 3.74, build 571880) installs **nothing** with `vita3k --firmware` (it logs
  "100 %" in 0.9 s and leaves `vs0/` and `os0/` empty, no error line even at `log-level: 0`); `PSP2UPDAT-fonts.PUP` installs the fonts (`sa0/`, `pd0/`).
  What worked is an `os0/` (13 MB) and `vs0/` (159 MB, with `vs0/sys/external/libsystemgesture.suprx`) copied from another machine's working Vita3K into
  `data/saves/2048/vita3k/`. With it `libsystemgesture` loads and a tap registers.
- **A tap needs the pointer there first.** `xdotool mousemove x y mousedown 1` in one command is ignored on the Game Mode grid and the map; a move, 0.3 s,
  then the press opens them (`vita3k-drive.py tap`, fixed). The Game Mode tile is then selected (orange) by the first tap and opened by the second; the pair
  script looks at the tile's colour and taps again.
- **The guest hangs about every second boot** (a frame frozen on "Sony Computer Entertainment presents" or on the Game Mode grid, the process at 0 to 60 %
  CPU, no log line; `sceKernelTryLockLwMutex returned ... FAILED_TO_OWN` is the last error), at a load average of 3 as well as of 40; every attempt that
  did not hang took about 2 minutes. The walk now looks at the frame instead of counting key presses and stops with `STALLED` after 90 s, and the batch
  runs a failed attempt again (of the eleven pairs below five needed a second attempt and Sol 2 five).
- **The unlock route that worked is not an unlock.** Every grid cell of every campaign is a record in the guest's memory from the boot on
  (`FUN_810bbfce`, EU v1.04: `Locked` byte at `+0xe9`, the track name NUL padded in 0x18 bytes at `+0xea`, another string at `+0x102`; confidence 85,
  from 10 records per HD circuit name on each boot and the cards below). Clearing the `Locked` byte of all of them did open a node the map drew dark
  (Vineta K, 10 zones; confidence 60, the node was not shown untappable before the write). Writing **another circuit's name over the track name** of the
  HD Campaign's first node (Vineta K, Race) makes that node's card show the new circuit's name and picture ("METROPIA", its photo) and the race load that
  circuit, in that event's own mode: Race, Venom, 3 laps, Feisar HD, 8th on the grid. That needs no map navigation and no unlock, so it reaches every
  circuit of both packs from one node. The pair's `original.track_rename` records the write; the pair is a **substituted event**, not an authored one
  (`scripts/vita3k-mem.py rename-track OLD NEW`, `unlock`, `find`; `vita-2048-pair.py --rename-track OLD NEW`).
- **Retry now shows the mode card again** on the HD Campaign and holds the countdown until the check mark is tapped (7 s of card on every grab otherwise, the
  burst's HUD never came up); the burst taps `(882, 480)` every 2 s until the HUD is up. The first HUD frame is still tick 0.
- **Same recipe as the Vineta K pair**: Retry and a 44-frame burst, the frame 3 s after the first HUD frame (2.9 to 3.1 s measured) against ours' tick
  174 to 187, hull shake patched out, `--skip-craft-offset` (no yellow livery to find, no nose read), exposure held at 1.15, Pilot Assist on in both (below).

| Circuit | Pack | RMSE | Edge correlation | Mean grey difference |
| --- | --- | --- | --- | --- |
| Metropia | HD | 0.138 | 0.17 | 17.8 |
| Anulpha Pass | HD | 0.129 | 0.15 | 14.8 |
| Moa Therma | HD | 0.196 | 0.17 | 33.7 |
| Chenghou Project | HD | 0.139 | 0.13 | 16.5 |
| Ubermall | HD | 0.188 | 0.25 | 24.6 |
| Sebenco Climb | HD | 0.140 | 0.15 | 21.3 |
| Sol 2 | HD | 0.163 | 0.14 | 23.6 |
| Talons Junction | Fury | 0.182 | 0.13 | 23.2 |
| Amphiseum | Fury | 0.167 | 0.12 | 28.0 |
| Modesto Heights | Fury | 0.176 | 0.09 | 30.8 |
| Tech De Ra | Fury | 0.187 | 0.18 | 26.8 |

- **Seen once or twice.** A second cold boot of Anulpha Pass gave RMSE 0.127 against 0.129 and its original differs from the first boot's by 0.047 RMSE
  (the animated sign boards, the rivals' craft), so the RMSE column is good to about 0.02. A first set of the same eleven pairs, taken before the Pilot Assist
  setting below was fixed, had second boots of Anulpha Pass, Metropia and Tech De Ra at 0.144, 0.139 and 0.189 (first boots 0.126, 0.139, 0.194; the
  originals 0.079, 0.037 and 0.055 apart): the same agreement. The other ten pairs are seen once (four second boots hung out of their four attempts).
  Circuit, craft and the HUD place (8/8) were read off both frames of every pair; the **mode** is Race on both sides, so no Speed Lap or Elimination is in this set.
- **Pilot Assist had to be set in the original.** The compare profile pins `pilot_assist = "on"` and ours draws the indicator; this profile's original
  started on **Normal**, which draws none, so the first eleven pairs showed the icon on one side only (no parity check reads the original's options). Home,
  Options, Pilot Assist `(297, 325)`, the right arrow `(910, 243)` once to **Extreme**, the check mark (saved to `DATA.BIN`) puts it up; the pairs here were
  recaptured after that and the icon is drawn on both sides in all eleven (crops of the box `575,494,130x44`, read side by side).
- **What the frames show.** Same circuit and slot everywhere; ours is darker on the hull (black where the original is lit blue and green) and brighter on
  some floors, the sign boards sit at another phase, the rivals are other craft. Anulpha Pass shows a defect of ours: wide cyan glow quads on the floor and
  a flat blue block of sky where the original is dark (box `0,280,300,60`, mean grey 74.1 ours against 40.1 original). **It is not the `alpha_emissive` family**
  (a render with `OAG_ALPHA_EMISSIVE_OFF=1` is pixel-identical in that box), so it is another material and open.
- **`alpha_emissive` glow, one box per pair** (the pixels that change with `OAG_ALPHA_EMISSIVE_OFF=1`; 23 on Anulpha Pass, 206 on Metropia): Anulpha Pass
  box `172x73+535+77` mean grey 25.8 original, 20.8 ours, 19.9 ours with the family off; Metropia box `409x102+254+57` 73.4 original, 80.3 ours, 78.2 ours
  with it off. The family adds 0.8 and 2.0 grey in those boxes and is not what separates the frames; ours' lit pixels there are a speckle (isolated cyan or
  yellow pixels on the sign boards) where the original is smooth.
- **The chase camera:** the options screen (Home, Options, Camera) reads **CLOSE** on this profile (the pairs above were taken on it; the earlier three
  were not, "The chase camera and the craft's size" below).
- **Not reached.** The base circuits: the Single Player Campaign's events are gated by rank (25 to 34 for the ten Phantom Challenges) and its nodes name a
  `TrackDefinition` by instance id, not by a string in a cell. Overwriting the three `tower` strings that stand alone in the guest's memory with `mall`
  left Event 1 on Empire Climb (card and race), so the field that picks the circuit is another one (open). The Zone circuits and the reversed circuits
  (`track_reversed.vex`, 11 of them in `dlc1.psarc` and `dlc2.psarc`) have cell records (`Zone_1` to `Zone_4`, `Talons_Junction_R` ...) that the same rename
  reaches, but no pair was made of them (not attempted, the Zone circuits race in the Zone mode and a Race cell would not be that mode); open.
- **Omega:** the rename route is **checked, differs**: Omega's front end is HD's `PI001` (`omega-status.md`), not 2048's grid map, so there is no 2048 cell
  record to rewrite, and Omega's original runs on shadPS4 with its own pair script. Anulpha Pass's floor is **checked, applies, not isolated**: Omega's
  Anulpha pair has the original flat grey-teal where ours draws the chevron family (`omega-status.md`, "Anulpha Pass's chevron floor", 2026-10-27), and
  that page's "2048: checked, applies, not wired visually; no 2048 pair shows them" is now answered by this pair, which shows ours' bright cyan bars against
  the original's flat dark floor on 2048 as well. That the bars are the `cf_chevron_pulse` materials was not isolated here (no render with them off); no Omega
  frame was rendered in this lane.
- **Adopting it:** copy the eleven pair directories (`theirs.png`, `ours.png`, `side.png`, `pair.json` each) to
  `data/reference/compare-page/2048-vita/<id>/` (ids as in the table, lower case with hyphens) and regenerate with `python3 scripts/compare_env.py manifest
  data/reference/compare-page/2048.toml data/reference/compare-page/2048-vita/{empire-climb,mall,dlc-vineta-k,metropia,anulpha-pass,moa-therma,chenghou-project,ubermall,sebenco-climb,sol-2,talons-junction,amphiseum,modesto-heights,tech-de-ra}`,
  then `just compare-page`.

### Real events for the substituted 2048 pairs (2026-11-12, `vita-2048-race-pairs`)

**Claim: every 2048 race pair compares the original's frame against ours on the same event, craft and circuit, or its `caveats` say what differs.
Partly true: 5 of the 11 substituted pairs are re-captured from authored events, 6 are kept with a caveat, and the substitution is shown to render the
same as the authored event (confidence 80 on the five circuits tried, 65 on the other six, read off the cells' own values).** `dlc-vineta-k` was never
substituted: it is the authored `grid0_3_1` node, so the brief's twelve are eleven.

- **Every substituted circuit has an authored Race, Venom, 3-lap, seven-rival cell** in the HD or Fury grids (`data2.psarc`'s `data/plugins/grids/grid_00..15.xml`):
  Anulpha Pass `grid0_5_1`, Moa Therma `grid1_3_1`, Metropia `grid2_3_1`, Ubermall `grid3_3_1`, Sol 2 `grid4_3_1`, Chenghou Project `grid5_2_3`, Sebenco Climb
  `grid6_3_1`, Talons Junction `grid8_3_1`, Modesto Heights `grid8_4_1`, Tech De Ra `grid9_3_1`, Amphiseum `grid10_2_1`. Against `grid0_3_1` (the node the rename
  rewrites) their `<Values>` differ only in `skillEasy`, `skillMedium`, `skillHard` (the rivals' skill, 0.3 to 0.9 and so on; Moa Therma's cell is identical) and
  `Locked`; mode, class, `Weapons`, `damage`, laps, `AICount` and `ShipChoice` are equal. The grid's `FlyerName`, `BillboardName` and colours are the grid's own.
  The craft is Feisar HD on both routes: Select Team opens on it on the Fury Campaign as on the HD one (read off the Select Team screen on a Fury node).
  Ours draws the `hdships` Feisar on a DLC track (`oag_2048::race::GUEST_ROSTER`), the hull shape matches in every side image, so the hull-luma sign flip between
  circuits that the scout read as two crafts is the circuits' lighting.
- **Authored against substituted, one boot each.** RMSE of the new original against the old (substituted) original of the same circuit, and ours (this tree,
  `4d76ba866`) against each:

  | Pair | Authored cell | Original vs substituted original | Ours vs original, substituted | Ours vs original, authored |
  | --- | --- | --- | --- | --- |
  | `talons-junction` | `grid8_3_1` | 0.0566 | 0.1804 | 0.1759 |
  | `modesto-heights` | `grid8_4_1` | 0.0577 | 0.1735 | 0.1689 |
  | `anulpha-pass` | `grid0_5_1` | 0.0550 | 0.1284 | 0.1320 |
  | `tech-de-ra` | `grid9_3_1` | 0.0633 | 0.1405 | 0.1477 |
  | `amphiseum` | `grid10_2_1` | 0.0652 | 0.1666 | 0.1727 |
  | `dlc-vineta-k` (control, same node, not renamed) | `grid0_3_1` | 0.0632 | 0.1364 | 0.1403 |

  The edge correlation of the original pairs is 0.86 to 0.95, the circuit, craft, 8/8 slot and HUD agree, and the sign boards, rivals' craft and a few lamps are at another
  phase. The spread between two boots of the same node (the control, 0.063) is as large as the substituted-to-authored gap, so the rename changes nothing that
  a frame shows. Each ours-against-original number is one boot; the old and new ours numbers move by 0.004 to 0.008 either way, the size of the original's own spread.
- **Vita3K 4285 against the pinned 4095.** This machine's Vita3K is `v0.2.1 4285-885f9168`; the 2048 profile pins `v0.2.1 4095-84184a36`
  (`scripts/compare-environment.toml`), and `environment_text` writes the pin, so a re-render writes 4095 into the five new pairs again. The control (Vineta K, unchanged
  recipe, 4285) is 0.063 from the stored 4095 frame, inside the spread, so the version is not a confounder here. The pin moved to `v0.2.1 4285-885f9168` on 2026-11-12 (maintainer); pairs captured on 4095 keep 4095 in their own `pair.json`.
- **The map route.**
  - **The tiles:** the Fury Campaign tile is at `(659, 162)` on the Game Mode grid (the HD one at `(477, 162)`); a first tap highlights it (orange) and the pair
    script taps again.
  - **The map re-centres** on the node last opened each time its card closes (the cross, `(752, 480)`), (the Fury Race icon: `(602, 352)` on the map as it opens, `(518, 264)` after two cards were closed, `(533, 335)` after Select Team). A node's tap
    point read once is wrong after Select Team: the first run of `talons-junction` tapped `(600, 352)` twice and raced the Speed Lap `grid8_2_2` ("LAP 1/7, GOLD 0.47.5"), a
    Speed Lap card where the Race card had been. `vita-2048-pair.py --node-template` finds the node's icon in a fresh frame (`scripts/vita-2048-map-node.py`, zero-mean
    normalised cross-correlation, scores 0.96 to 1.0 on the right icon, 0.6 on the nearest wrong one) and `--node-pick N` takes the N-th match from the left; after
    Select Team the match nearest `(533, 333)` is the opened node (Talons Junction, Modesto Heights `(533, 335)`, Anulpha Pass `(532, 330)`).
  - **The Race icon template** is a crop of a frame of the Fury map, `magick <frame> -crop 90x64+473+232 +repage race-icon.png`, taken with the map centred on `grid8`
    after one card was opened and closed (the template's frame is not stored in the repository).
  - **Locked nodes tap open.** An unplayed Race node in an open grid (Modesto Heights `grid8_4_1`, Anulpha Pass `grid0_5_1`) opens its card and loads without `vita3k-mem.py unlock`.
  - **Later grids by hops (Fury).** `--hop X Y` opens the card at a point and closes it, so the map re-centres and the next grid shows. From the cold map: `(693, 303)`
    then `(928, 455)` reaches `grid9` (Impact: Tech De Ra, pick 1); then `(800, 345)` and `(800, 190)` reach `grid10` (Voltage: Amphiseum, pick 1). The points worked
    unchanged over four boots.
  - **The HD blocker.** On the HD map the next grid (Warped, `grid1`) shows only as a sliver under the two bottom-right buttons (`(846..944, 482..528)`); a drag
    (`xdotool` press, move, release) and the d-pad do not pan the map, and a tap on the sliver at `(805, 515)` and `(872, 538)` opened nothing. Chenghou Project,
    Metropia, Moa Therma, Sebenco Climb, Sol 2 and Ubermall were not reached in this effort (about 20 minutes of exploration), and their six pairs carry a caveat saying so.
    Next: tap the Warped hex through a first hop from a node on `grid0` that sits further right, or unlock and open the grids with `vita3k-mem.py unlock`.
  - **Load time.** At a load average of 25 to 50 the HD-campaign load took more than the pair script's 20 s wait twice (98 % on the loading bar): `--load-wait 45` to `75`
    fixed it. About every second boot hung on "Sony Computer Entertainment presents" or on the Game Mode grid and were relaunched; `--boot-timeout 220` and the watchdog's
    `--silence 280` were needed at that load.
- **Fury's default craft** is Feisar HD (Select Team on a Fury node opens on it), where `2048-dlc.md` said it was unmeasured.
- **Omega:** checked, differs. Omega reaches HD's and Fury's campaigns through HD's own `PI001` front end, with no 2048 map to tap or hop and no cell record to rewrite
  (`omega-status.md`), so none of this ports; Omega's pairs run on shadPS4 with their own script.

### The chase camera and the craft's size (2026-11-11, `c2048-chase-camera`)

**Question:** is the original's craft drawn 1.3 times larger than ours on the three 2048 grid pairs because the two sides flew different chase views
(`OPT_CLOSE` or `OPT_FAR`)? **No: the sizes agree on the nearer view, and the recorded 1.31 x 1.35 was the yellow mask.** Confidence 75 for "the
original is on `OPT_CLOSE`, as inferred from the frame"; the original's stored `pair.json` records no camera setting, and each original is one capture.

- **Ours' setting.** `CameraView::default()` is `Close`: `crates/raceplay/src/start.rs` builds a race with it, `chase_block_for` picks the
  `<ExternalCameraClose>` block, `[graphics] camera_view` defaults to it (`crates/game/src/settings.rs`), and the compare profile and the pair flags
  pin nothing. A render with no flag and one with `--camera-view close` are byte-identical on all three pairs. Load report on the 2048 Feisar:
  Close eye 2.775 up and 8.25 back, Far 3 up and 11.25 back (both fov 60). So ours is the nearer of the two views the original offers, and moving
  it to Far can only make the craft smaller.
- **The original's setting is not recorded.** `original` in the three `pair.json` carries the emulator, the shake patch, the grab timing and the pose
  read, no camera view; the Vita3K save that held `CameraP1` is not stored (the grid pairs read no camera, "What is not measured" above). Its authored
  default is `OPT_CLOSE` (`CameraP1 default="OPT_CLOSE"`, `docs/ui/menus-original.md`), which is what a fresh profile starts on. A profile that
  has cycled to `OPT_FAR` is the HD trap ([rpcs3-capture.md](../reverse-engineering/rpcs3-capture.md), "the shared profile flies the Far camera").
  So the setting is **inferred from the frame**, below.
- **Landmark boxes** (source px at 960x544, read by eye on 2x and 5x crops with a 10 and 50 px grid, +-4 px, nose and tail +-8: the nose tip, the
  outermost left and right wing tips, the rearmost wing tip row; the same landmarks on every frame; the 5x re-read of Empire Climb's tips moved no
  edge by more than 3 px). Width x height, ours at the pair's own flags, ticks and profile, build `c1b14f0d4`:

  | Pair | Original | Ours on Close (the default) | Ours on Far | Original over Close, wide x high | Far over Close, wide |
  | --- | --- | --- | --- | --- | --- |
  | Empire Climb | 343 x 248 (nose (477, 262), x 304 to 647, tail y 510) | 345 x 248 (nose (477, 264), x 305 to 650, tail y 512) | 235 x 143 (nose y 307) | 0.99 x 1.00 | 0.68 |
  | Queens Mall | 341 x 255 (nose (477, 262), tail y 517) | 342 x 256 (nose (477, 264), tail y 520) | 232 x 143 (nose y 306) | 1.00 x 1.00 | 0.68 |
  | Vineta K (DLC) | 368 x 263 (nose (477, 257), x 297 to 665) | 368 x 263 (nose (477, 257)) | 237 x 149 (nose y 301) | 1.00 x 1.00 | 0.64 |

  Ours on Far is 0.64 to 0.68 times Close's width (the eye 11.25 back against 8.25), where the recorded difference was 31 % the other way. The
  original's hull matches Close within 2 % on all three pairs and sits 32 to 36 % from Far, so it flew `OPT_CLOSE`.
- **Why the recorded 1.31 x 1.35 and 1.27 x 1.01 were wrong.** `craft_offset`'s bounding box is that of the pixels `r > 178, g > 140, b < 102` in the
  lower 60 % of the frame, columns 150 to 750, and it is not a hull box: the original's Empire Climb mask box is `x 316..741, y 236..541` against
  landmarks at `x 304..647`, tail y 510, with 5,015 mask pixels outside the landmark box (4,129 below row 515, 1,029 right of column 655; what they
  are was not read further); on Queens Mall the same rig's `ours.png` and `ours-original-pose.png` read 420 and 319 px wide; Vineta K's Feisar has
  no yellow at all. The Queens Mall paragraph below already said the centroid is not a hull measure; the box has the same fault.
  `scripts/vita-2048-pair.py` still writes the ratio into each caveat; open, not changed here.
- **What is left.** Not the camera setting. The pose offset (`pose_delta`), the white ring under the original's craft and the light on the
  paint stay open, in that order, as the thread's steps 1 and 2.
- **To decide the original's setting outright:** read `CameraP1` from the Vita3K save of the profile the pairs were taken on. In its pref-path:
  `strings -a ux0/user/00/savedata/PCSF00007/* | rg -A1 CameraP1`; expected value `OPT_CLOSE`, `OPT_FAR` or `OPT_INT` (HD's `DATA.BIN` stores
  it as a plain string; 2048's save is not confirmed to). If `strings` finds no `CameraP1`, the save is binary or encrypted, and a re-capture
  that reads the rig's eye out of guest memory decides it instead. Run it on the machine that holds that profile; this one has none.
- **Omega:** checked, applies, not wired. The Omega pairs' `pair.json` record only ours (`ours.camera`: "the title's default chase view; the grid
  is at rest, no pose is fed", the same `CameraView::default()`), nothing for the original, so the same inference is open for them; not read here.

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
- **Re-measured 2026-10-29 (`2048-dlc-vineta-k`), claim: "the lighting is the gap" is false today.** Whole-frame mean luminance (`-alpha off`) is 48.3 for the original and 45.1 for
  ours (the 80.1 above predates the exposure wire and the pack-copy `track.EnvSettings` lookup, `read_circuit_envsettings`); per-box mean absolute grey difference, original against
  ours: sky `(0,0,960,170)` 32.8, hull `(380,250,620,544)` 22.7, `TOTAL` `(100,60,250,95)` 25.7, speed bar and icon `(600,495,960,544)` 27.6 before and 15.9 after the HUD fixes below.
  The sunset is drawn (warm sky, long shadows); what separates the two frames is texture detail (the original's red wall carries sponsor decals, ours is plain), the hard shadows on the
  right wall and the hull's shading.
- **The livery is not a different livery.** Every Feisar skin `data.psarc` ships (`Livery1/FeisarHD_Skin_1024` and `Skin1_1024`, `Livery2`, `Livery3`) is blue, yellow and white; none
  is olive or teal. The original's olive-and-teal hull is the blue and yellow paint under the dark scene's light and a specular rim, so the hull gap is shading, not paint (read by
  decoding the four textures). `Feisar_c1` (the HD `concept1` default) is a different hull shape and does not match either.
- **Fixed 2026-10-29: the icon left of the speed bar and `TOTAL`.** The icon is `PilotAssist` (`HUD_assist_indicator.xml`, centred at 642,516, a 64x32 box over a 256x128 texture):
  Pilot Assist is **on** in the original's 2048 captures (all three pairs draw it), the compare profile now says so (`[platform.2048] pilot_assist = "on"`) and
  `compare-rerender-ours.py` adds `--pilot-assist` to a recorded pair that lacks it. The widget authors no `TxtrWidth`, so it samples its whole texture; ours cropped a 64x32 patch off the
  texture's corner and drew a white bar (`oag_hud::Sprite::uv` `[_, _, 0, 0]` is now "the whole texture"). `TOTAL` reads `0.00.00`, hundredths, in the original here and on the
  native frame (`TOTAL 0.29.76`) while `CURRENT` stays in tenths: `oag_title::HudArt::total_time_hundredths`, 2048 only. Test:
  `vita_2048_hud_event_ground_truth::pilot_assist_draws_its_icon_whole_and_the_total_reads_hundredths`.
- **Still different**: the white ring round the hull and the blue blob over the engine (both are the original's and neither is a widget of `Arcade_HUD.xml`'s composed set; the ring
  is 3D or runtime-drawn), the sponsor decals and shadows above, the hull shading, the rivals' craft, and the HUD (the original draws no `OBJECTIVE` or `XP`; **since
  2026-10-26 (`2048-dlc-hud`) ours draws neither either**,
  [race-hud-selection.md](../ghidra/functions/vita-2048-eu-v104/race-hud-selection.md#the-xp-and-objective-lines-are-bound-to-the-current-spxml-event-2026-10-26-2048-dlc-hud)).
  The pair is a baseline for the DLC circuits' lighting and HUD, not a pose check.
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
the original's. **The vignette is drawn since 2026-11-11 (`w2048-vignette`)**: the composite is the original's fan-and-ring mesh round clip-space `(0, -0.885)`, so the top corners go black and the bottom corners are untouched, as in the original (outermost 8x8 luma at the top corners 0.9 to 1.8 original, ours 7 to 67 before and 0.4 to 4.8 after; exposure.md, "The vignette"). The screen tint and `alphaTex` are unread and left out.

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
is ported in the chase rig (`chase_view_sheared`). The presented frame lags the stop by **two** of the original's frames (2026-11-27, read off the HUD's
digits: "The presented lag, wired" below; it was inherited from Pulse as one until then). A resting craft cannot tell, the clocks can: both are the stop's
read less two `frame_dt_s`.
The circuit is checked by ours' own spawn: 0.2 to 8.8 units and under 3.5 degrees from the original's grid pose on the four circuits.

**Known, not fixed here.** Ours draws the hull darker and less lit than the original on all four, and its Pure HUD has no speed gauge and shows `TOTAL`
where the original shows `BRONZE TIME` (the `edge_ncc` of 0.26 to 0.36 is these, not the geometry). A first state taken 595 s into an idle race (the maintainer's
`time-trial-vineta-feisar-grid-fresh`) also loads and pairs (eye and fov equal) but runs 36,000 ticks of ours for the clock; the restart states are the ones
kept. Only the Alpha league is on a fresh profile; the other eight circuits are opened by Pure's own dev-unlock byte (next section).

### Pure: eight more circuits through the dev-unlock byte (2026-11-11, `gallery-pure`)

**Claim: the page carries a Pure race pair for every circuit the USA disc ships that ours races and a Time Trial can start.** Twelve pairs, up from four: the
eight `PI_Track` rows of type `Race` and the four of type `Classic` of `Definition.xml`. The four `Zone` circuits and Pure's twelve DLC circuits are not paired (below).

**The unlock route.** `Definition_IsUnlocked` (`0x088b43d4`, [campaign.md](../ghidra/functions/psp-pure-usa/campaign.md), "Definition_IsUnlocked") is Pulse's function
([race-box-screens.md](../ghidra/functions/psp-pulse-usa/race-box-screens.md)) at another address: a hidden byte at `+0xa1` fails the row, then `sceKernelGetGPI() & 1` or a byte of
the profile object passes everything, then the `<Unlock>` list at `+0xa4` is walked. Disassembled and read back relocated out of the running game: `lui a0, 0x08b1; lw a0, 0x7794(a0); lbu a0, 0x381(a0)`,
so the profile object's pointer is at **`0x08b17794`** (it read `0x08e759a0` on both boots) and the byte is **`*(*0x08b17794 + 0x381)`**. **Control, same boot, same profile:** with the byte `0`, League
Selection lists ALPHA only; with it written `1` and the screen rebuilt (back out, in again) it lists ALPHA, BETA and CLASSIC, and Track Selection lists Sinucit,
Citta Nuova, Sebenco Climb, Sol 2 and Karbonis, Sagarmartha, Manor Top, Mandrashee. A plain `memory.write` works with the CPU running; the byte is in the loaded profile, so a boot or a profile load clears it,
and it has to be written before League Selection is built. `scripts/psp-pure-walk.py --unlock --league N --track-index K` writes it at the Main Menu and walks; `--league` counts downs on League Selection.
(The USA disc's profile `PROFILE.BIN` was taken from another lane's memory stick: a cold boot's Profile > Load listed no data, and the Set Name screen was not walked.)

**Pairs added** (`pure-psp-<id>`, each Time Trial, Feisar, Vector, a few seconds after GO, craft untouched; metrics as the page computes them, `overlap` in `pair.json`):

| Id | Circuit | League | rmse | edge correlation | mean grey | spawn check (units, deg) | race clock |
| --- | --- | --- | ---: | ---: | ---: | --- | ---: |
| `sinucit` | `08_Sinucit` | Beta | 0.132 | 0.358 | 26.99 | 1.2, 1.7 | 2.10 s |
| `citta-nuova` | `09_Citta_Nuova_V` | Beta | 0.097 | 0.370 | 8.95 | 0.6, 1.0 | 2.87 s |
| `sebenco-climb` | `10_Sebenco_Climb` | Beta | 0.127 | 0.410 | 25.91 | 2.1, 1.5 | 0.80 s |
| `sol-2` | `12_Sol_2` | Beta | 0.103 | 0.360 | 10.43 | 0.5, 0.5 | 1.80 s |
| `karbonis` | `ClassicsNS/01_Karbonis` | Classic | 0.183 | 0.308 | 19.28 | 0.5, 0.4 | 1.47 s |
| `sagarmartha` | `ClassicsNS/02_Sagarmartha` | Classic | 0.192 | 0.300 | 27.33 | 2.1, 0.8 | 5.10 s |
| `manor-top` | `ClassicsNS/03_Manor_Top` | Classic | 0.256 | 0.327 | 40.93 | 1.0, 0.0 | 4.03 s |
| `mandrashee` | `ClassicsNS/04_Mandrashee` | Classic | 0.225 | 0.331 | 25.44 | 0.8, 0.1 | 3.90 s |

Every pair passes the same 14 checks as the Alpha four (13 for a Classic pair, see below): eye 11.643 from the body, fov 60 against 60, pose 0.000 units apart, ours' own spawn
within 25 units of the original's grid pose (the circuit check, which also **names** a state's circuit: the four Classic states were first identified by the HUD's bronze time and then by this check), and
a Beta pair's hull on the original's within 3 px (offset (0, 0) on all four, edge correlation 0.43 to 0.52 on the hull region).
**Repeats:** all eight were made twice on one PPSSPP boot and `theirs.png`, `ours.png` and every number were identical; three of them (Sinucit, Karbonis, Sol 2) again on a second boot, identical to the byte.
That shows the state load and the reads are deterministic, not that two different states agree.

**The Classic hull is `VR\Ship.vex`, and ours flies it.** In the Classic league the original draws a differently shaped, differently coloured Feisar than `Data\Ships\Feisar\Ship.vex`: it is `Data\Ships\Feisar\VR\Ship.vex`, which `Ship_LoadModel` builds for every craft when the circuit's `PI_Track` `type` is `Classic` ([ship-models.md](../ghidra/functions/psp-pure-usa/ship-models.md), "What selects the VR hull"). Before it was wired, the four Classic frames read an edge correlation of 0.09 to 0.11 on the hull region (best shift +7, -4, 0 and 0 px, noise from two unlike shapes). With `oag_title::RaceDefaults::hull_by_track_type` wired, a fixed hull box (`x 188..300, y 172..272`) reads, per pair, edge correlation 0.29 -> 0.86 (Manor Top), 0.34 -> 0.89 (Mandrashee), 0.29 -> 0.28 and 0.78 at a 4 px shift (Sagarmartha), 0.16 -> 0.63 (Karbonis), and mean absolute difference 46.7 -> 20.4, 38.7 -> 12.5, 39.4 -> 29.4, 39.1 -> 15.2. Sagarmartha's best correlation sits at a 4 px horizontal shift; whether that is the pose or the hull's origin is not measured. The stored Classic pairs still carry `psp-pure-pair.py --hull-differs "<caveat>"`, which skips the hull-offset gate; re-captured without it, three pass (below) and await adoption, and Sagarmartha keeps a caveat naming its 4 px shift; the eye and fov checks hold the camera either way. **The "darker Classic worlds" claim is false at main, and the Classic frames' remaining grey was the scenery's animation phase (`pure-classic-vr`, 2026-11-24, ours at `bf0dbe4`).** On the four stored pairs the whole-frame signed grey difference (original minus ours, HUD boxes left out) reads -1.2, +0.8, +1.9 and +0.3 (Karbonis, Mandrashee, Manor Top, Sagarmartha) and the mean grey 9.32, 12.09, 13.75, 9.12 - Beta's Sinucit and Sebenco Climb read 15.4 and 15.6 and the four Alpha pairs 6.2 to 7.6. Pasting the original's pixels over one region of ours (the HD method below), the largest region on three of the four is the background above the road (everything outside the four HUD glyph boxes, the hull box `x 188..300, y 172..272` and the road, rows 150 and down): 13.75 -> 6.07 on Manor Top, 12.09 -> 5.71 on Mandrashee, 9.32 -> 6.38 on Karbonis (the timer box, a lightning bolt of ours, is 3.48 of it); on Sagarmartha it is the hull box (9.12 -> 6.57) with the background next (6.27). The background is **not lighting**: ours re-rendered at the same held pose with only `--ticks` moved reads Manor Top 14.53, 13.47, 13.75, 10.65, 7.49 at 330, 345, 360, 375, 390 and 6.78 at 540, so the scenery's animated materials were out of phase. The twelve Pure pairs stored then predated `--world-clock-offset` (`original.world_clock_offset_s` was null on all of them); the four Classic states re-captured with the world clock read ([world-clock.md](../ghidra/functions/psp-pure-usa/world-clock.md); the original reads 10.04 s on its world clock at GO on all four, ours put on `5.51 s` + its tick) read **Karbonis 9.32 -> 4.31, Mandrashee 12.09 -> 6.86, Manor Top 13.75 -> 6.36, Sagarmartha 9.12 -> 9.04**, with the Classic hull now passing the hull-offset gate unexcused on three (edge correlation 0.48, 0.52, 0.51 at (0, 0)); Sagarmartha's hull sits 4 px left of ours (0.481 at (-4, 0) against 0.093 at (0, 0), pose and camera identical) and keeps its `--hull-differs` caveat. What is left on the re-captured pairs is the hull box (signed +2 to +6 grey, the original brighter on all four), the HUD, and on Sagarmartha the 4 px shift. The ground shadow's shape is not the original's. Modesto was re-captured the same way by `pure-modesto-hull` (mean grey 6.19 -> 4.38; 4.16 once Pure's texture levels were read, `hull-shine.md`, "The hull's texture level"); the other seven (Sebenco Climb, Sinucit, Sol 2, Citta Nuova, Chenghou, Vineta, Blue Ridge) were re-captured the same way afterwards ([world-clock.md](../ghidra/functions/psp-pure-usa/world-clock.md), "The other Pure race pairs, re-captured with the clock"), so all twelve Pure race pairs carry `original.world_clock_offset_s` and ours' `--world-clock-offset` (checked at main `41d94285f`; mean grey now 4.20, 6.73, 6.24, 9.06 on the four Classic pairs and 4.16, 6.51, 6.43, 6.83, 14.01, 14.71, 9.23, 6.04 on Modesto, Blue Ridge, Chenghou, Citta Nuova, Sebenco Climb, Sinucit, Sol 2, Vineta). **The hull box's "original brighter" is mostly ours' song display, not the hull** (confidence 80) (`pure-classic-vr-set`, 2026-11-25; the four Classic pairs re-rendered from a copy, ours at `41d94285f`): the box `188..300 x 172..272` reads signed (original minus ours) +5.62, +5.09, +2.58, +2.54 (Karbonis, Mandrashee, Manor Top, Sagarmartha); its rows 172 to 246 read +3.24, +2.09, -1.68, +0.77 and its rows 247 to 272, +12.8, +14.1, +15.4, +7.8. Those bottom rows hold the dark backing of ours' now-playing bar (`assets/fallback/Data/XML/NowPlaying_Pure.xml`: `x 130..280, y 247..267`, colour `0x50000000`, the song scrolling through it), which darkens what it covers by 10 to 42 grey where it lies over road (rows 247 to 251 of its left part, signed); the original draws no bar and writes the artist and the track as two plain lines at `x 80` instead ([now-playing.md](../formats/now-playing.md), "Pure's own music text"). Pasting the original's pixels over ours inside the two boxes that lie outside the hull box (`130..190 x 247..267`, the bar's left part, and `76..152 x 242..266`, the original's text) lowers the whole-frame mean absolute RGB-channel difference (the page's Gray score reads 4.20, 6.73, 6.24, 9.06 before; this base reads 4.31, 6.75, 6.28, 9.00) by 0.40, 0.63, 0.66, 0.46 on the four Classic pairs and by 0.41 to 0.62 on Blue Ridge, Chenghou, Citta Nuova, Sebenco Climb, Sinucit and Sol 2 (0.01 and 0.06 on Modesto and Vineta, whose original frames are past the display's end, race clock 17.8 and 16.7 s). On the hull's own pixels (the 3,200 to 3,300 pixels of rows 172 to 246 that ours' `--no-hull-shine` render changes) the signed difference is +7.2, +4.6, -2.5 and -1.6 grey: not one sign, so no fixed material offset of the VR hull (the same hull and material on all four) explains it; a circuit-dependent lighting or shine term would. Ours' ground shadow (about 400 pixels of the box, `--shadows off` against on) reads 28, 41, 42 and 48 grey darker than the original over them, about +1.4 of the box's signed value (the shadow lies under the bar's right part, so the two overlap). **Wired afterwards** (`pure-classic-vr-set`, [now-playing.md](../formats/now-playing.md), "Pure's own music text"): ours draws the disc's two lines and no bar, and the hull box's mean absolute grey difference falls 11.24 -> 5.39, 8.44 -> 6.04, 8.21 -> 5.52 and 25.40 -> 24.78 on the four Classic pairs (whole frame 4.20 -> 3.74, 6.73 -> 6.48, 6.24 -> 5.89, 9.06 -> 9.18).
**Pairs taken as a state's clock reads 0.8 to 5.1 s** because PPSSPP's software renderer runs the race slower than real time; `retime` is a load, a wait and a save over the same file (scratch script, not kept).

**Not paired.** The Zone circuits `Data\Zone\02_Zone`..`04_Zone`; `01_Zone` (Pro Tozo) is paired at zones 8 and 9 by `psp-pure-pair.py --zone` ("Zone pairs: Wipeout Pure" below).
Pure's twelve **DLC** circuits are not on the disc image and cannot be paired from the USA disc: Delta (Iridia, Anulpha Pass, Koltiwa, Khara Descent, `Environments\BrandedTracks\Puma_*`), Gamma 1 (Staten Park, Exostra Run, Ubermall,
Sebenco Peak, `06_`, `02_`, `05_`, `11_`) and Omega (JonBurgerman, MarkJames, NeilMcFarland, 123Klan, `18_` to `21_`) declare them in their manifests (read through `oag_source::dlc::pure_packs`); the packs in `data/dlc/` are EU packs, and the original
checks a pack with a region key from its own executable ([dlc-pack.md](../formats/dlc-pack.md)), so they would need the EU disc and the EU pressing's live addresses (not recovered; `psp-pure-pair.py` reads the USA ones). Not attempted here.

**Adopting.** Copy the eight pair directories from the lane's `pairs/` to `data/reference/compare-page/pure-psp/<id>` (each `pair.json` `argv` `--out` already names that final path), then
`python3 scripts/compare_env.py manifest data/reference/compare-page/pure.toml data/reference/compare-page/pure-psp` (12 entries) and `just compare-rerender-ours --only pure-psp`.

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

**HD pairs run with Pilot Assist off since 2026-11-19** (`pilot_assist = "off"` in the `hd-fury` profile; `hd-circuit-pairs`). The pairs of 2026-10-21 were made from a save state with the option on, and their frames carry the indicator's bracket under the craft ([pilot-assist.md](../ghidra/functions/ps3-hdfury-eu/pilot-assist.md)). The 2026-11-18 recaptures are cold boots of a fresh profile (`capture_path.kind = "cold_boot"`) and none of their race frames draws it (read by eye on Sebenco Climb, Sol 2, Moa Therma, The Amphiseum, Vineta K, Metropia, Tech de Ra and Talon's Junction; confidence 85, the option byte itself was not read from RAM for these captures), while ours kept drawing it from the recorded `--pilot-assist on`. `compare-rerender-ours.py` now rewrites a recorded `--pilot-assist` to the profile's value (`compare_env.profile_pilot_assist_flags`, as it does for anisotropy). Mean grey, before to after, ours re-rendered at main `e81d373bc`: Sebenco Climb 4.996 to 4.80, Sol 2 6.41 to 6.31, Moa Therma 6.33 to 6.20, The Amphiseum 6.25 to 6.09, Vineta K 3.35 to 3.21, Anulpha Pass 4.32 to 4.22, Chenghou Project 2.89 to 2.68, Metropia 3.63 to 3.47, Modesto Heights 4.87 to 4.75, Talon's Junction 3.44 to 3.31, Talon's Assegai 4.90 to 4.77, Tech de Ra 4.67 to 4.54, Ubermall 4.50 to 4.41: all thirteen better, none worse. Omega's pairs carry no `--pilot-assist` (ours' default is off) and need no change; 2048's profile keeps `on` (its three captures draw the icon).

**Where the grey sits on the four HD pairs over 5 (2026-11-19, `hd-circuit-pairs`).** Method as the Pulse PSP section: paste the original's pixels over a region of ours and recompute `mean_grey`. After the indicator fix the largest regions are:

| Pair | Region (1920 x 1080 px) | mean grey with the region taken from the original | Cause |
| --- | --- | --- | --- |
| `sol-2` 6.64 (before the fix) | GO board `700,150 - 1700,400` | 4.23 | the board's glyph brightness at the grab: ours bright white, the original grey with a halo |
| `moa-therma` 6.38 | upper left `0,0 - 1000,540` | 3.66 (the board alone 5.53) | scenery: ours flatter and lighter, the original's building front carries sun streaks and darker glass; not isolated |
| `sebenco-climb` 4.99 | upper right sky and tower `1300,0 - 1920,400` | 4.03 | snow and tower brightness, ours lighter; not isolated |
| `the-amphiseum` 6.19 | far floor patch `1300,540 - 1700,680` | 5.90 | a material whose name contains `floor` draws a flat green sheet where the original's floor is a blue sheen with light reflections (`OAG_SKIP_MATERIAL=floor` removes it); the exact slot is not found |

**The GO board is the original's state at the grab, not a renderer gap.** Ours pinned at the original's `gantry_seconds` less the HUD lag draws `GO` at full white where the original's is grey; the board's glyph ramps within about three ticks (Sol 2, ours re-rendered with the HUD lag set 0, 3, 6, 9, 12: board mean abs difference 20.0, 20.0, 12.4, 20.6, 28.8, so the original reads dimmer than ours at the pin and brighter than ours three ticks earlier), and Moa Therma's pin sits 4 ticks past the window start 3.83 s where ours holds `GO` lit and the original's is dim. The picture is 0 to 3 ticks older than the RAM read ([above](#a-moving-hd-craft-the-frame-is-not-always-the-ram-instant-2026-11-10-hd-talons-pack-align)), so the true node time of the presented frame is not recorded; no pin was adopted (a lag chosen because the frame matches is retuning ours to the frame). **Missing value:** the gantry node's `+0xc0` (`AnimNode_GetTime`) at the presented frame. **Read recipe:** read `+0xc0` on both sides of one flip in `rpcs3-hd-pair.py`'s pause (two reads, the frame between them), or take the `billboard8` quad's texture transform out of the push-buffer dump the pair already makes; record it in `pair.json` and pin ours with it. Not checkable on Omega's twelve pairs the same way (the native port runs the board on its own clock).

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

### The Pure circuit pairs' residual, split by region and cause (2026-11-27, `pure-circuit-split`)

Ours re-rendered on copies at `266236da7` from each pair's recorded flags (scores within 0.08 of the stored `pair.json` values). Scorer is the page's (magick `Gray` difference, mean 0 to 255);
every pair's regions were checked to sum to its score. No code changed.

**Falsifiers, written before measuring.** (1) One shared cause: one region class must be the largest on at least 4 of the 6 clustered pairs (Chenghou, Blue Ridge, Citta Nuova, Mandrashee,
Vineta, Manor Top) and hold at least 25 % there. (2) Animation phase: moving ours' scenery clock by one or two ticks must lower the share above the horizon by more than 0.3 grey.

**Regions** (boxes are `x0..x1, y0..y1` on 480x272, claimed in this order): HUD = clocks `0..100, 0..56`, target time `370..480, 0..34`, lap `0..72, 232..268`, speed and shield
`306..480, 218..268`; song/pilot text `76..180, 242..266`; craft `178..302, 176..272`; the light strip at craft height `150..480, 156..186`; sky = the pixels that change by more than 6 grey
when ours' sky pass is not drawn (a scratch build that skips `TrackStage::Sky` in `crates/raceplay/src/scene/record.rs`); background = the rest above `y 137`; near track and walls = the rest below. The HUD boxes include the scenery behind the glyphs,
so their shares are upper bounds.

| Pair | Score | HUD | Song text | Craft | Strip | Sky | Background | Near |
| --- | ---: | --- | --- | --- | --- | --- | --- | --- |
| Chenghou | 6.60 | 1.81 (27 %) | 0.64 | 0.69 | 0.36 | 0.12 | 1.99 (30 %) | 0.98 |
| Blue Ridge | 6.57 | 1.69 (26 %) | 0.48 | 0.96 | 0.27 | 0.09 | 2.23 (34 %) | 0.86 |
| Citta Nuova | 6.51 | 2.34 (36 %) | 0.55 | 0.26 | 0.53 | 0.05 | 2.25 (35 %) | 0.51 |
| Mandrashee | 6.49 | 2.61 (40 %) | 0.59 | 0.54 | 0.45 | 0.49 | 1.53 (24 %) | 0.28 |
| Vineta | 6.01 | 1.79 (30 %) | 0.05 | 0.58 | 0.31 | 0.28 | 1.71 (29 %) | 1.28 |
| Manor Top | 5.90 | 1.99 (34 %) | 0.57 | 0.55 | 0.47 | 0.00 | 2.02 (34 %) | 0.32 |
| Sagarmartha | 9.18 | 2.11 (23 %) | 0.58 | 2.54 (28 %) | 0.53 | 0.11 | 2.55 (28 %) | 0.76 |
| Sol 2 | 9.03 | 2.49 (28 %) | 0.59 | 0.77 | 0.34 | 1.07 | 2.88 (32 %) | 0.89 |
| Karbonis (low control) | 3.74 | 1.56 (42 %) | 0.50 | 0.45 | 0.38 | 0.22 | 0.38 | 0.25 |
| Modesto (low control) | 4.15 | 1.72 (41 %) | 0.01 | 0.24 | 0.26 | 0.20 | 1.30 (31 %) | 0.42 |

**Falsifier 1 is not decided by the box split.** The HUD and song boxes cover 17.8 % of the frame and hold scenery too (the speed and shield box is mostly road). Counting whole boxes, with
the song text as HUD (it is HUD on screen; the table keeps it apart because it is capture state), the HUD is the largest region on 5 of the 6 (31 to 49 %; Blue Ridge 33 % against background
34 %); without the song, on 3. Charging each box's area at the mean difference of the region behind it (background for the two top boxes, near track for the other three) leaves a lower bound
of 12 to 43 % for the HUD, which then leads on 1 of the 6 (Mandrashee); the background, with that area added back, leads on the other five at 36 to 42 %. No single cause is established:
the HUD's part splits into text layout, the clock lag and the song, and the background's cause is unidentified.

**Falsifier 2 fired.** Above the horizon the share fell by 0 to 0.29 grey over the eight pairs swept, so the lag below is **not** the scenery residual's cause. It is a separate cause.

**Cause: the picture is two stops behind the read, not one** (confidence 75, one capture per pair). `psp-pure-pair.py` subtracted `PRESENTED_LAG = 1` frame (inherited from Pulse PSP) until it was wired (below).
Primary evidence is the HUD's own digits. For a lag of N frames the picture's clock is `race_clock_at_stop_s - N x frame_dt_s`; it must truncate to the original's `CURRENT` tenth, and
`target - clock` to its `BRONZE TIME` tenth (targets: Chenghou 160, Blue Ridge 158, Vineta 130 s from `stats.xml` as
[hud-time-ladder.md](../ghidra/functions/psp-pure-usa/hud-time-ladder.md) records them; Citta Nuova 176 and Manor Top 141 s inferred from ours' own countdown, as whole seconds).

| Pair | Clock at stop (s) | `CURRENT` | Lags it allows | `BRONZE` | Lags it allows | Both |
| --- | ---: | --- | --- | --- | --- | --- |
| Chenghou | 6.573 | 6.5 | 0..2 | 2.33.5 | 2..4 | **2** |
| Blue Ridge | 6.774 | 6.7 | 0..2 | 2.31.3 | 2..4 | **2** |
| Manor Top | 4.071 | 4.0 | 0..2 | 2.17.0 | 2..4 | **2** |
| Citta Nuova | 2.903 | 2.8 | 1..3 | 2.53.1 | 0..2 | 1, 2 |
| Vineta | 16.683 | 16.6 | 0..2 | 1.53.3 | 0..2 | 0..2 |
| Mandrashee | 3.937 | 3.8 | 2..4 | | | 2..4 |
| Sagarmartha | 5.139 | 5.0 | 2..4 | | | 2..4 |
| Sol 2 | 1.835 | 1.7 | 2..4 | | | 2..4 |
| Sinucit | 2.135 | 2.0 | 2..4 | | | 2..4 |
| Sebenco Climb | 0.834 | 0.7 | 2..4 | | | 2..4 |
| Karbonis | 1.502 | 1.4 | 1..3 | | | 1..3 |
| Modesto | 17.818 | 17.7 | 1..3 | | | 1..3 |

2 is the only value consistent with all twelve; eight pairs refuse the script's 1. Re-rendered with ours' `--ticks` two lower (race and scenery clocks together), the eight tenths that were
wrong (five `CURRENT`, three `BRONZE`) read as the original's, and all twelve scores fall: Chenghou 6.60 to 6.13, Blue Ridge 6.57 to 6.32, Citta Nuova 6.51 to 6.34, Mandrashee 6.49 to 5.98,
Vineta 6.01 to 5.85, Manor Top 5.90 to 4.95, Sagarmartha 9.18 to 8.78, Sol 2 9.03 to 8.71, Karbonis 3.74 to 3.33, Modesto 4.15 to 3.82, Sinucit 14.12 to 13.86, Sebenco Climb 13.73 to 13.45.
The score is corroboration only (a fitted value can always lower it); Citta Nuova and Sebenco Climb score better still at 3 frames, which Citta Nuova's `BRONZE` digit refuses and
Sebenco's digit allows, so a per-grab 3 is not excluded.

**Ranked by share.**

1. **Background detail above the horizon**, 24 to 35 % of each score as drawn; cause **not identified**. Not animation phase (falsifier 2). Not a global shift (best shift 0 on 8 of 10; Sagarmartha
   -1 px, Blue Ridge -0.25 px). Not a flat offset except Chenghou (ours +2.7 grey on flat pixels, R +4; Vineta +1.2) and not 565 quantisation (the original's grab uses all 8-bit levels;
   quantising ours raises the error). 56 to 90 % of the world error sits on edge and detail pixels, and ours carries more high-frequency energy above `y 137` (Laplacian ratio ours/original
   1.05 to 1.16 on seven of ten, 0.99 to 1.02 on three): a hint, confidence 40, that the original's unread per-texture level bias ([hull-shine.md](../ghidra/functions/psp-pure-usa/hull-shine.md),
   "The hull's texture level") applies to scenery too.
2. **HUD text layout**, on all twelve, not moved by the lag. Pieces: the lap counter's `/3` sits 17 px right of the original's (best paste offset 17 px on 12 of 12; pasting it saves 0.03 to 0.24
   grey). The time digits' roughly 3 px wider run over six glyphs is the earlier finding; the clocks box reads 0.65 to 0.86 grey on nine of the ten pairs split (Sol 2 1.55, an advert behind it), and the target box up to 1.50 (Mandrashee). Box figures include the scenery behind.
3. **The presented lag**, above: 0.16 to 0.96 grey on all twelve. Tooling, not the renderer.
4. **Song lines** (capture state: ours holds `COLD STORAGE / ONYX`, the original plays what it plays), up to 0.64 grey.
5. **Single-pair items.** Sagarmartha's craft box 2.54 (the known 4 px hull shift). Billboard slots: Pure has `adverts: None`, so every slot's placeholder is suppressed (6 to 25 draws on the
   Alpha and Beta circuits) and the original's signs are missing (Citta Nuova's blue sign, Vineta's orange panel, Sol 2's flag, Blue Ridge's small boards). Their footprint holds 1.30 grey on
   Sol 2 (0.37 outside HUD boxes), 0.36 on Sinucit, at most 0.14 on the others: visible to a player, small in the score.

**Reproducing.** Copy a pair directory, re-render with `rerender()` of `scripts/compare-rerender-ours.py`, take the difference with `magick theirs ours -colorspace Gray -compose difference
-composite`, and sum it over the boxes above. The split above was measured at one frame of lag (before the wiring below).

#### The presented lag, wired (2026-11-27, `pure-pair-lag`)

`compare_env.PURE_PRESENTED_LAG = 2` is the one number: `psp-pure-pair.py` records it, and `compare-rerender-ours` re-times a stored Pure PSP record made
with another (`compare_env.pure_lag_flags`): race clock = `race_clock_at_stop_s - 2 x frame_dt_s`, `--ticks` from it by the script's own tenths rule, the
scenery clock back by the same frame. The record's `presented_lag_ticks` and provenance follow, so a second run changes nothing. Confidence stays 75
(the digits above). On all twelve race pairs `--ticks` fell by 2. The two Zone pairs have no race clock: only their scenery clock moved back one frame,
labelled inherited (the digits were not read on a Zone state), and their pose is still the stop's uncorrected moving read. Neither frame changed by a pixel: nothing in view animates over 33 ms.

Copies of the stored pairs, re-rendered with the same binary before and after (mean grey; the clocks and target boxes are `0..100, 0..56` and
`370..480, 0..34`, as mean grey inside the box):

| Pair | Score | Clocks box | Target box |
| --- | --- | --- | --- |
| Chenghou | 6.60 -> 6.13 | 16.03 -> 15.98 | 19.32 -> 14.39 |
| Blue Ridge | 6.57 -> 6.32 | 16.86 -> 16.40 | 16.34 -> 14.71 |
| Citta Nuova | 6.51 -> 6.34 | 16.67 -> 16.48 | 39.45 -> 39.45 |
| Mandrashee | 6.49 -> 5.98 | 16.41 -> 15.82 | 52.46 -> 52.12 |
| Vineta | 6.01 -> 5.85 | 15.15 -> 15.17 | 15.72 -> 15.72 |
| Manor Top | 5.90 -> 4.95 | 18.38 -> 18.38 | 27.82 -> 16.35 |
| Sagarmartha | 9.18 -> 8.78 | 20.09 -> 18.07 | 26.33 -> 26.31 |
| Sol 2 | 9.03 -> 8.71 | 36.14 -> 34.11 | 17.93 -> 17.93 |
| Sinucit | 14.12 -> 13.86 | 22.39 -> 21.03 | 18.27 -> 18.49 |
| Sebenco Climb | 13.73 -> 13.44 | 19.54 -> 17.89 | 15.60 -> 14.22 |
| Karbonis | 3.74 -> 3.33 | 17.98 -> 17.98 | 12.98 -> 12.98 |
| Modesto | 4.15 -> 3.82 | 18.33 -> 17.93 | 14.02 -> 14.02 |
| Zone 8 | 35.23 -> 35.23 | not read | not read |
| Zone 9 | 34.54 -> 34.54 | not read | not read |

No pair worsened in score. The eight wrong tenths now read as the original's: `CURRENT` on Mandrashee, Sagarmartha, Sol 2, Sinucit and Sebenco Climb,
`BRONZE TIME` on Chenghou, Blue Ridge and Manor Top; all 24 time readouts of the twelve pairs now show the original's digits (read on 3x and 4x crops).
The boxes also hold the scenery behind (Sinucit's target box rose 0.22 and Vineta's clocks box 0.02 with unchanged digits), and a `CURRENT` tenth can run
past `x 100`, so the box figures are not a digit test. Mandrashee's `BRONZE TIME` digits still sit to the right of the original's (the layout item above).

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
     stays 0) for `RaceSim::field_warm_up_ticks` steps (the intensity's full climb, 1 / 0.25 s, plus a full HD tube: 294 ticks at 60 Hz),
     each craft on a straight path that ends exactly on its captured pose. A file without the columns is refused by name, never given a
     guessed velocity. **Chosen, not measured:** the straight path and fixed heading (the original's craft had curved, so a ribbon in
     a corner leans where ours is straight) and full thrust for every craft (the player's throttle reads 100; an opponent's is not
     read from RAM, nor any craft's flare block, `boost_timer` or `plume_timer`: all left at the engine's own values for a craft at that
     speed). The pair scripts pass it by default, so `just compare-rerender` picks it up from the stored `argv`.
   - **Opponent lean (2026-11-02, `opponent-tilt`).** The field file also carries `steer_lean` and `roll_phase` per row (`entity+0x854`
     and `+0x87c` read at the same stop by `psp-race-field.py`; every slot is the player's entity struct), and `--field-from` puts
     them on each opponent's ship after the warm-up, as `--pose-lean` does for the player (`PoseLean::apply_to`). Without the
     columns (an older `field.csv`, the RPCS3 path) an opponent is drawn flat, as before. `talons-alongside`: slot values
     0.038, -0.008, 0.769, 0.786, 0.374, 0.869, -0.194 (roll phase 0 on all seven). Nearest opponent's hull box (x 210, y 112,
     40x36 px, 480x272 frame): grey mean absolute difference against the original 0.0382 before, 0.0239 after; the fin tilt now
     agrees with the original's. **Stored pair re-made 2026-11-02 (`talons-recapture`)**: `pulse-psp-race/talons-alongside` was re-captured with
     `psp-pulse-pair.py` at `--target-clock 38.4082` (24 parity checks passed, twice; `theirs.png` pixel-identical to the old one, `field.csv`
     identical across both boots) so its `field.csv` now carries the lean columns. Old stored `ours.png` against new, same `theirs.png`: nearest
     opponent's box grey MAD 0.0381 -> 0.0242; whole frame grey MAD 0.0094 -> 0.0092, RMSE 0.0293 -> 0.0283. **HD: checked, applies, not wired**: the drawn lean is documented at `ship+0x6a54`
     ([ship-lean](../ghidra/functions/ps3-hdfury-eu/ship-lean.md)) and `scripts/rpcs3_hd_field.py` reads other fields at the same
     ship, but HD's roll-phase offset is not recorded, and a lean with a guessed zero roll would be a half-read pair; both are needed.
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
on **HD** (`hd-race/talons-pack`) this was read as "ours matches" (the hull 170 px wide in a 560 x 420 crop on both sides) **and is withdrawn** (2026-11-10): on the pair as it stood the
original's frame was not the recorded instant, and on the re-made pairs the original's hull is about 1.3 times larger and lower than ours (see "A moving HD craft: the frame is not always the RAM instant" below). On **Pulse PSP** the original hull is
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

- **Pair contacts spark now** (`Stage::throw_contact_spark`), so a race that runs ticks shows them with no flag.
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

## Zone pairs: Wipeout 2048 (Vita, 2026-11-17, `zone-pair-2048`)

**Claim: the page carries two 2048 Zone pairs at zone 5, one on a native circuit and one on an HD DLC circuit inside 2048, against Vita3K. True, both
with a gap.** `vita-2048-pair.py --zone-capture SECONDS` (with `--walk`) flies a Zone race and samples it; `--zone ZONE,SCORE[,STAGE] --zone-frame N
[--zone-from DIR]` builds the pair from one sample. Pair kind `zone`, `--autopilot off` (a 2048 Zone race is flown by the game, no pickup is armed).

- **The state.** The HD Campaign's Uplift grid carries a Zone event on Vineta K that is open on a fresh save (`(600, 395)` on the map; a first-visit "NEW RANK"
  tutorial box can swallow the first tap, so the walk dismisses it, `popup_up`). It needs no input: the craft flies itself and enters zone 5 about 30 s
  after the HUD, zone 7 at about 55 s, then crashes. Two boots gave the same scores to within 0.3 % (zone 5 at score 4315 to 4598 on Vineta K both times).
- **The native circuit** is the same cell with its circuit name overwritten (`vita3k-mem.py rename-track Vineta_K altima`, 10 records in a manual run on
  this save): the card reads `39_Track`, the race loads Altima (read off the frames, and two sampled hull positions are 21 and 26 units off Altima's spline and 3.2 to 3.4
  above its surface, against 130 to 4,200 units off the other nine base circuits' splines, `oag-game --pose` on each). A substituted event, as the DLC pairs
  of "Eleven pairs through a renamed grid cell": the mode (Zone), the Venom to Flash class ladder and the craft (Feisar HD) are the HD cell's, not a
  native 2048 Zone event's. The base campaign's own Zone events are rank gated and were not reached (confidence 60 that the rename carries the
  authored circuit's data, as it did for the Race events). The save's `DATA.BIN` still carries its 2026-10-20 mtime after these runs: no race was finished, so no progress was written.
- **The pose** is read, not guessed: the guest is stopped (`SIGSTOP`) for the frame grab and a scan of its memory (`/proc/<pid>/mem`) for drawn-hull matrices
  (orthonormal rows scaled 0.75, a zero fourth column, then the position, 0.5 s a stop). The craft is the candidate seen 38 times in the stop (the renderer's
  copies); other matrices stay put from sample to sample (a parked model, 178 copies) and are dropped. Ours renders at tick 0 with `--pose x,y,z,yaw`
  (the yaw solved against ours' own attitude, forward agreement 0.9992 and 0.9998; the hull's tilt off the spline, 1.2 to 2.3 degrees, is not carried).
  **The original's camera is not read** (it is sprung): ours keeps its own chase camera (`--zone-camera chase`, the default, `--pose-from` with the exact hull
  matrix); `--zone-camera rigid` puts the close chase rig at rest behind the hull instead (`--pose x,y,z,yaw` plus `--camera-pose`: eye 2.775 up and 8.25 back, look-at 3 below
  and 18.75 ahead, fov 60, Feisar HD's `ExternalCameraClose` times the 0.75 craft scale; the matrix's tilt off the spline, 1.2 to 2.3 degrees, is dropped). The two renders of
  one sample differ by 0.14 (Vineta K) and 0.07 (Altima) normalised rmse, so neither camera is the original's; on Altima ours' hull stands yawed against the view,
  which the original's does not. Zone, score and shield are read off the original's HUD by eye (2048's Zone counters are not located in RAM) and passed to ours.
- **Numbers** (rmse / edge correlation / mean grey, the page's own scores; boot 1 adopted, boot 2 in brackets, drawn with `--zone-camera rigid`): Altima zone 5 0.490 / 0.06 / 112.9
  (0.303 / 0.07 / 45.9, another frame at the same place); Vineta K zone 5 0.367 / 0.02 / 86.0 (0.300 / 0.02 / 59.3). The boot-to-boot spread is the frame pick
  (the Altima mean grey difference moves from 112.6 to 45.9 with the view), so the scores are good to about 0.1, not to the second digit.
- **What differs, read side by side (both PNGs read).** *Vineta K:* the geometry lines up (the purple tower, the lime wall and its trim, the road's curve); the original's
  road is a dark grey deck with purple glow and yellow lane lines under a dark purple sky, ours' road, wall and floor are one lime (the Scene colour): the **expected
  gap, the road is Scene-coloured**, and ours' purple tower and sky are right. *Altima:* the same landmarks (the rock face, the lake tower, the road's rise)
  are in both; the original is dark navy to grey with a red glow and a purple jet of exhaust, ours' road is black with large yellow slabs (the glow strip,
  drawn as a surface) and a purple band for the water. **Ours' zone grade barely moves this frame**: the log reads `the Zone recolour draws nothing - the file authors no
  Texture U/V scale` and the unpinned zone 5 commits stage 2, and the 12 stages pinned one by one differ from it by under 5 % of the pixels (0.42 to 0.46
  rmse against the original at every stage, so no stage fits), so the yellow slabs are stage independent: **on a native circuit ours draws no Zone recolour of the
  road, where the original recolours it dark**. *Vineta K's* grade does move (zone 1 against zone 5 differ in 16 % of the pixels, the unpinned zone 5 commits stage 4,
  the ten stages tried give 0.30 to 0.44 rmse against the original): the lime road is not a wrong stage. *Both:* ours' HUD is the Zone caption "ZONE", the shield gauge
  (at the frame's value) and the speed arc; the original's adds SCORE and its number at the top, the class name (FLASH) and the zone number: **ours draws no 2048 Zone
  score, class or zone number on either circuit** (a render with zone, score and seconds set in `--hud-state` shows the same). The original's craft carries a red
  rim light and an exhaust jet (a posed capture runs no tick, so ours has none). Pilot Assist's indicator is on neither frame (a caveat in both pairs).
- **2048 and Omega: checked, differs on the grade, applies on the HUD.** Ours on Omega's `Environments2048\altima` in Zone mode (headless, same pose, `--hud-state` zone 1 against
  zone 5): the two frames are pixel-identical, so the Zone grade is not drawn there at all (the held-back path of merge `19f18c1b6`), where 2048's own Altima draws a stage
  that barely moves; the HUD is the same "ZONE" caption and shield only, so the missing score, class and zone number apply to Omega too. No Omega original frame was captured
  in this lane (`zone-pair-omega` makes that one).
- **Traps found.** A `vita3k-drive.py`/`vita3k-mem.py` display left over from an earlier run presents black frames (the guest runs, every shot is 0 grey): `vita3k-drive.py
  stop` and boot again. A Vita3K left running by an earlier run makes the next `--walk` attach to it ("already running", hull shake not patched): stop it first.
  `oag-game` takes a negative `--camera-pose` or `--pose` only as `--flag=VALUE`; `--camera-pose` refuses `--pose-from`.

## Zone pairs: Wipeout Pure (PSP, 2026-11-17, `zone-pair-pure`)

**Claim: the page carries one Wipeout Pure Zone pair (PPSSPP software renderer against ours, Pro Tozo, zone 8) with a second at zone 9. True, both with a gap.**
`psp-pure-pair.py --zone ZONE,SCORE` renders ours at tick 0 (`--mode zone`) on the original's body pose and fov with `--hud-state 1,1,0,0,KMH,100,0,ZONE,SCORE,0`
and `--hud-autopilot` (the timer read from the pickup record); both flags are honoured on Pure. Pair ids `pure-psp-zone-pro-tozo-z8` and `-z9` (`data/reference/compare-page/pure-psp/`).

- **The state.** `psp-pure-zone-state.py`: Main Menu, SINGLE PLAYER, three downs to ZONE (Single Race, Tournament, Time Trial, Zone), Zone Track Selection (Pro Tozo,
  Mallavol, Coridon 12, Syncopia = `Data\Zone\01_Zone` to `04_Zone`; no class or team step), the race loads about 12 s later and shows SCORE and ZONE. A fresh profile
  (language, name, tag and "do not save" answered by hand, about 4 min on a cold boot) got there with the dev-unlock byte written (`psp-pure-walk.py`'s). **The craft is flown by the
  Autopilot pickup**, armed by a debugger write (running byte `+0x154` = 1, timer `+0x15c` = 600 s at the weapons loop's tail `0x0884d0d4`, `autopilot.md`): it passes about one
  zone every 10 s (zone 4 at 25 s, zone 6 at 45 s, zone 8 at about 60 s). The pickup record is at `0x095bb380` in the boot that made the two pair states and at `0x095bb340` in a
  later one, so it goes into the state's `.json` sidecar (`autopilot_record`). The HUD prints the timer (553.18 s at zone 8) at the top centre; ours draws it from `--hud-autopilot`.
  States: `data/saves/pure-psp/zone-pro-tozo-autopilot-zone8.ppst` and `-b.ppst` (zone 9), reusable by the next lane. From the lane's start to the first built pair: 13 min (build 1,
  cold-boot menu walk 4, Zone race up 1, Autopilot to zone 6 and the first save 1, script edit and ours' probes 3, the pair 1), then the eye fix, the second state and its pair 5 more.
- **Read by eye, not RAM.** No Pure address for the zone number, score or zone seconds is documented, so the zone (8, 9), score (10450, 11817) and km/h (567, 749) come off the original's
  HUD; ours' zone seconds are written as 0 (a caveat in each `pair.json`). Pose, fov and the Autopilot timer are read from RAM.
- **Numbers** (rmse / edge correlation / mean grey, the page's own scores): zone 8 0.325 / 0.06 / 52.7; zone 9 0.390 / 0.10 / 64.1. Pose agreement 0.000 units and 0.000 degrees at the posed tick
  (ours is posed on the original's body), fov 81.54 and 85.59 degrees fed as read. **Seen once**: every number comes from one PPSSPP boot and one Zone race. The zone 8 pair was rebuilt three times from the same state (a state load is a restore, so the original's frame, ours' frame and every metric
  were bit-identical: a check of the script, not a repeat); a second cold boot of the Zone race has not been made (confidence 55 on the numbers below).
- **What differs, read off both `side.png`** (**corrected 2026-11-24, "Why the Pure Zone pairs read 53 and 64" below: (1) is Zone_01's authored Close block, not a spring, and ours drew fov 60 where the original drew 81 to 86; (2)'s brightness gap was a stale figure, the frames read 144 against 135 and 177 against 186 mean grey; the craft is Zone_01's hull, with Assegai's handling**). (1) **The camera** (confidence 55, one boot, two states): the original's eye at Zone speed is 7.48 units from the body (right 0.32, up 2.23, back 7.13 at 158 units/s; 0.26 left, 2.27 up, 7.12 back
  at 208 units/s: one tick-aligned read each, `eye_body_frame` in `pair.json`) against the Close rig's 11.64 (3.0 up, 11.25 back) at rest, so a Zone craft is drawn about 1.6 times larger and the road
  fills more of the frame; **whether this is a speed spring or a fixed Zone rig is open** (7.48 at both 158 and 208 units/s, no at-rest Zone read); ours keeps its rest rig, so its hull is small and sits higher, and the wall panels and floor pads the original's wider, closer view shows are off-frame in ours. The hull offset
  (+6, -6) and (+10, -3) px is recorded, not gated. The eye must be read against the body at the tripod's own stop: the tripod is one tick after the pose read at `Ship_UpdateCraft`'s entry (the body advances 5.24 and 6.88 units between the two stops, 0.0331 s of
  speed each, within 1 % of the `frame_dt_s` of 0.03332 and 0.03331 s the pairs read: one tick of Pure's 30 Hz simulation), and against that earlier pose the eye reads 2.97 and 2.30 units.
  **The fov is not 60 in Zone**: 81.54 and 85.59 degrees at 158 and 208 units/s, where the Time Trial pairs read 60 at rest ("no speed term read for Pure" no longer holds; the Zone speeds are the first moving reads, confidence 60). (2) **Brightness**: ours reads 220 and 194 mean grey against 176 and 143: the original's walls and ceiling carry black bands and a pink-violet bloom at the lights that ours
  draws pale. (3) The original's hull shows a bright blue engine glow; ours draws none and puts a shadow under the hull. (4) **Palette**: ours has no per-zone palette on Pure (`--zone-stage` is a no-op: stages 0 and 5 pixel-identical; zone 1 and 8 identical scenery, only the HUD digits differ). For the original, **not observed and not tested at a
  fixed place**: the lower-half mean colour moved by 3 to 6 % between zone 1 and zone 8 across six sampled frames taken at different places on the circuit, a measure that cannot see a change in the pink-violet strips
  and bloom, which is where the pair differs (confidence 50). So a zone where the original's palette has changed is not established, and the pair tests geometry, camera, HUD and light.
- **The craft**: ours flies **Assegai** (Pure's own default when no team is named; `oag_title::ZoneCraft`: Zone's `Zone_01` handling block is unread). The original's Zone hull was not identified (`Data\\Ships\\Zone_01` exists on the disc); the two hulls look alike in `side.png`.
- **Open**: the at-rest Zone camera and a fixed-place palette read, the Zone counters' RAM addresses, a second cold boot of the Zone race, and the three unpaired circuits (`02_Zone` to `04_Zone`).
- **Not matched, in each pair's caveats**: the zone clock, ours' exhaust and trail history (no ticks), the shield bar (drawn full), the ZONE label (fading in the original).
- **Pure against the other titles**: not checked here (the Pulse Zone camera and HD's Zone frame were not compared in this lane); the camera spring is a Pure measurement only.

### Why the Pure Zone pairs read 53 and 64 (2026-11-24, `pure-zone-pairs`)

**Claim: the dominant residual of `pure-psp-zone-pro-tozo-z8` and `-z9` is named by region with a measured cause, and wired. True: two
disc-authored inputs were wrong in ours, the chase camera and the field of view. Both pairs drop (mean grey, edge correlation):
z9 64.14 to 34.54 (0.10 to 0.26), z8 52.87 to 35.22 (0.06 to 0.21).** Confidence 85 on the cause (two Zone states, one PPSSPP boot, a
disc value matching two live reads; no second cold boot).

- **Region split** (the original's pixels pasted over one region of ours, mean grey of the whole frame after; the HD method, a short numpy script over
  `theirs.png` and `ours.png`; boxes: HUD glyphs and bars 16 % of the frame, hull `x 130..340, y 150..272` 19 %, road rows 150 and down 20 %, background above row 150
  46 %). Before, z9 (whole 64.3): HUD -8.0, hull -9.8, road -8.6, **background -37.8**; z8 (52.3): -7.9, -8.1, -6.7, **-29.7**. After
  the camera and the field of view, z9 (34.4): -6.4, -7.8, -2.2, **-18.0**; z8 (35.0): -6.5, -10.1, -4.6, **-13.9**. So the walls and ceiling above the road
  dominated, as a framing error does: the frames' whole-frame mean grey is the same within 9 (original 143.8 and 176.6, ours 134.6 and 185.9), so it was never a colour or palette law.
- **Cause 1, the camera** (`docs/gameplay/race-modes.md`, "Pure's Zone cameras"): the Zone craft's own `handlingstats.xml` authors
  `<ExternalCameraClose pos_height 3.0 pos_length -9.5>`; times the `0.75` craft scale that is `(2.25, 7.125)`, 7.473 from the body, and the original's tripod read
  7.48 at 158 and at 208 units/s. The race had flown Assegai's `(4.0, -15.0)`, 11.64 away. Ours reads the five camera blocks from the Zone file now. Alone: z9 64.14 to 47.73, z8 52.87 to 41.19.
- **Cause 2, the field of view**: `--camera-fov` was dropped whenever `--no-camera` was (Pure's sheared tripod is never fed), so every Pure pair drew the authored field plus no speed
  widen: 60 here against the original's 81.54 and 85.59. The Zone file authors `70`, and `70 + 0.075 * dot(fwd, vel)` is 81.88 and 85.60 (the velocity's forward
  component is a little under the speed at 158). `Options::fov_override` carries the flag with the chase rig kept, only with `--no-camera` (a sweep of the stored pairs: the 14 that pass `--camera-fov` with `--no-camera` are all Pure, 11 at 60.0000, blue-ridge at 59.9999 and these two; the Pulse and HD pairs pass a camera and are untouched); a pair at rest and at fov 60
  (the Time Trial and Classic pairs) renders pixel-identical (modesto and karbonis compared: 0 pixels differ). The pairs were then re-made from their states with the script, which read the world clock this time (`original.world_clock_offset_s` 93.06 and 84.15, ours put on it): the scores did not move to four digits, so the scenery's animation phase is not a cause here. `psp-pure-pair.py`'s "field of view" parity check passes the fov as both sides and could not have caught cause 2 (thread `pure-zone-pairs-what-is-left-after-the-camera`). Alone with the camera: the two numbers above.
  The hull that looked 1.33 times too wide at the new eye was this: at the right field it is the original's size and place.
- **The hull**: the original's drawn hull node (`*(entity + 0x9ec) + 0x80`, read at `Ship_UpdateCameraRigs` in both states) has axes of length `0.750`, as in a race, and its shape in
  the frame is `Data\Ships\Zone_01\Ship.vex`, which ours draws. The pair's "ours: Assegai" craft line is the handling, not the hull.
- **What is left** (z9, by region after paste): the hull box -7.8 and -10.1 (the original's bright blue engine glow ours does not draw, ours' hull shadow the original lacks), the HUD
  -6.4 (ours' zone-clock digits and the ZONE label's fade, the shield bar), the walls' black bands and pink-violet bloom at the lights, and the road's pads. Not measured: Pure's Zone
  engine, turning and antigrav numbers (the Zone file's six physics blocks are unread, so a Zone race flies the player's team's), a Far-view read, a second cold boot.

## HD Zone race-start pair (RPCS3, 2026-11-19, `hd-zone-replace`)

`hd-zone-vineta-k-z5` is replaced by a Zone race-start shot on Vineta K (Feisar Zone hull, zone 1, score 0): cold boot, no savestate, original grabbed
through `grab_with_clock` (world clock offset 53.383 s recorded, `capture_path.kind = cold_boot`), ours rendered at the recorded pose. The old mid-race shot was
refused on heading (1.86 deg, tilt uncarried); the start shot passes all eight parity checks (position 0.000 units, heading 0.175 deg, fov equal, uncarried
tilt 0.21 deg). Mean grey difference 53.04, RMSE 0.322, edge NCC 0.222 (`compare_env.overlap_score`); seen on one boot (a first grab, moving at lap clock
7.9 s, was refused at 3.03 deg and discarded). Confidence 70 that the recipe reproduces; the number is one measurement.

**Menu path** (RACEBOX cursor sits on RACE TYPE; the screen names are `Single Player`, `Track Creation`, `Team Selection`): Main Menu, Racebox, RACE TYPE
to **Zone** (the setting is remembered between visits; from Single Race it is 4 `right`s; the list ends Zone, Zone Battle, Detonator, Tournament, so read the row before pressing), `cross` to Track
Select (Vineta K is the first circuit, no steps), `cross` to Ship Select (Feisar is the first team), `cross` loads (`TTY.log`: `GetMode()==6`, `Loading World
"Data\Environments\01_Vineta_K\track.vex"`), then START RACE. On a loaded machine a `press` is dropped about one time in four and the screenshot lags the
cursor by several seconds: read the highlighted row off a grab before every `cross`. Pilot Assist is written off first (`rpcs3-hd-pilot-assist.py --off`).

**The grab (2026-11-27, `hd-zone-pair-capture`, replaces the world-clock lead).** `rpcs3-hd-pair.py --zone 1,0 --countdown --autopilot off --state <sidecar
{"mode":"zone","circuit":"vineta-k"}>` is started **first**; it waits in-process, without pausing, for the race manager to be `--countdown-at` seconds
(default 2.4) into phase 1, and only then attaches the debugger. The caller then starts the race: `cross` at the fly-over's START RACE prompt, or `cross` on
`RACE AGAIN` (`EndRace Menu`, once the craft has crashed, about 60 s). The countdown is phase 1 of the race manager (`*0x0095ae78 + 0x1950`, word `+0x20`) and
lasts 3.0 s; about 3 s of phase 0 precede it after `RACE AGAIN`. The lap clock reads 0 through the countdown and at GO alike, so the pair refuses a grab on the
phase, and draws ours without `--hud-state` (the original draws no HUD yet). The earlier lane's grabs landed mid-race at lap clock 10 s; the cause is
inferred, not timed: the capture's default `--settle` is 12 s and only `--field` overrode it (with `--settle 0` the pause lands 0.02 to 0.09 s after the
wait's target, and the 12 s arm was not re-run).

## The countdown frame (HD Zone, 2026-11-27, `hd-zone-pair-capture`)

Two boots of RPCS3, Vineta K Zone, each grab 13 of 13 parity checks. "Tone" is the original's blue channel mean on a flat patch of the right-hand building
(`1500..1700 x 150..230`; ours reads 218.3 to 218.8 on every grab, the stored 2026-11-19 original 220.3). "Shift" is the largest best-fit offset, in px, of
ours against the original on three scene regions with no craft or board.

| grab | s into countdown | tone | shift px | mean grey | RMSE | edge NCC |
| --- | --- | --- | --- | --- | --- | --- |
| stored 2026-11-19 | not recorded | 220.3 | 0.3 | 6.26 | 0.0763 | 0.301 |
| boot A, early | 0.22 | 229.4 | 1.4 | 9.78 | 0.0822 | 0.285 |
| boot A | 1.06 | 223.7 | 0.3 | 6.62 | 0.0692 | 0.298 |
| boot A | 2.22 | 220.0 | 0.8 | 6.51 | 0.0786 | 0.289 |
| boot B | 2.26 | 219.9 | 2.0 | 8.16 | 0.0950 | 0.270 |
| **boot B, adopted** | 2.43 | 220.1 | 0.07 | 5.92 | 0.0731 | 0.303 |
| boot B | 2.48 | 221.2 | 0.3 | 6.55 | 0.0746 | 0.300 |

- **The original's tone settles during the countdown** (confidence 75: seven grabs and one unpaused poll). The Environment block's adapted luminance
  (`FunkLayer` `0x00c50ee0 + 0x254`) read 0.311 at the countdown's start, 0.348 at 1.0 s and 0.359 at 2.4 s (frame luma 0.358 to 0.361): the exposure follows it.
  Ours holds one tone, near the late original's. A countdown grab before about 2 s is 3 to 9 levels brighter for a reason ours does not model.
- **The craft hovers with a jitter** of about 0.05 units the whole countdown (polled unpaused), and the presented frame can be older than the camera read in the
  pause. The scene then sits 1 to 2 px off ours (the doubled edges of the 2.26 s grab) and mean grey rises by up to 2. Judge a grab by its shift, and re-grab.
- **A single grab's mean grey is not a measure of a change in ours**: two aligned grabs on one boot read 5.92 and 6.55.
- The adopted grab was chosen by a rule written before it was taken (at least 2.2 s in, every region's shift at most 0.5 px, first to pass), not by score.
  Craft box: best gradient correlation 0.81 at offset (0, 0). Every grab's craft overlays within 1 px.
- Left unmatched: the fade on the gantry board's outer pieces
  ([start-gantry.md](../rendering/start-gantry.md), "The Zone board"), the two raised flaps on the hull's top, which the original shows on the grid, and a few
  floating scenery shapes whose animation phase differs.

**Ours renders the original's countdown moment (2026-11-27, `hd-zone-start`).** The two countdowns differ in length (HD 3.0 s, ours longer), so they share the
release: `compare_env.countdown_flags` renders ours at `GO_TICK - round((3.0 - race_phase.elapsed_s) * 60)` (238 for the adopted pair), rebases
`--world-clock-offset` onto that tick so the scenery still shows the original's time, and holds the craft on the pose (`--pose-hold`). `rpcs3-hd-pair.py
--countdown` records it and `compare-rerender-ours.py` derives it from `original.race_phase`. No presented lag is subtracted: the 3 ticks measured on HD are the
HUD clock's and this frame draws no HUD. The flame clock pin is absolute and does not move with the tick.

| re-render of the adopted pair, twice, identical | pair mean grey | RMSE | board `800..1382 x 200..356`: grey, share of the pair |
| --- | --- | --- | --- |
| ticks 0, board clock 210 ticks (before) | 5.92 | 0.0731 | 28.20, 1.23 |
| tick 238, board clock 210 ticks | 5.45 | 0.0660 | 19.87, 0.87 |
| **tick 238, board clock 180 ticks** | **5.49** | 0.0663 | 19.99, 0.88 |

The middle row scores as low as the last because the board's mean grey barely moves between 212 and 262 ticks; it draws the wrong glyphs (an extra piece of the
centre icon, other pieces outside it). **The glyph overlap tells them apart.** The intersection over union of the board's bright pixels (luma over 75, box
`810..1370 x 208..350`) against the original's, swept a tick at a time, peaked at 207 to 212 with the 210-tick clock and at 238 to 241 with the 180-tick one
(0.666 at both peaks, 0.55 to 0.58 at 20 to 30 ticks off): the original's board stood 2.4 to 2.5 s into its own timeline 2.433 s into the countdown, as the
executable's zero says ([gantry-clock.md](../ghidra/functions/ps3-hdfury-eu/gantry-clock.md), "What zeroes it in the countdown").

**Differences left (not matched, judged by eye on `side.png`).** The original's zone-1 colour grade is a strong cyan wash; ours renders its natural zone 1 grade
(pale, warm sky). `--zone 1,0,0` (grade pinned to stage 0) is a grey scene and scores worse (mean grey 88.9), so the stage is left unpinned. The original's start gantry
shows its countdown glyphs and no HUD yet.

**No Zone HUD before GO (2026-11-19, `zone-hud-gate`).** HD and Omega draw no Zone HUD until the race is released
([hud-start-gate.md](../ghidra/functions/ps3-hdfury-eu/hud-start-gate.md)), and ours follows. With that fix the
pair scores 28.77 (RMSE 0.165), against 30.02 before.

**The Zone frame's light and post chain (2026-11-19, `zone-grade-sky`).** With the Zone surface lit by the
stage's ambient, the Zone bloom block and the resolve's feedback term
([zone-frame.md](../ghidra/functions/ps3-hdfury-eu/zone-frame.md)) the pair scores 7.42 (RMSE 0.085). What is
left is the gantry board's glyph, the hull's light and the exhaust.

**A capture that imposes `--hud-state` is drawn with its HUD on screen,** gate or not: such a pair depicts a
mid-race frame. Every Omega and 2048 Zone pair is one (`omega/zone-vineta-k-z5`, `omega/zone-capital-reach-2048-z5`,
`2048-vita/zone-altima-z5`, `2048-vita/zone-dlc-vineta-k-z5`), and they render as before. A pre-release pair must
**not** pass `--hud-state`. `rpcs3-hd-pair.py --zone` emits it for a mid-race frame; with `--countdown` it does not.

## Omega Zone pairs (2026-11-17, `zone-pair-omega`)

`scripts/wipeout-pc-omega-pair.py pair --zone` (the `wipeout_pc` native port; shadPS4 race shots stay untrusted). Two pairs in `omega/`:
`zone-vineta-k-z5` (an HD-lineage circuit, WipEout HD mode) and `zone-capital-reach-2048-z5` (`Data\Environments2048\bridge`, 2048 mode).

**The state.** RACEBOX, RACE TYPE four Rights (Single Race, Time Trial, Speed Lap, Eliminator or Combat, **Zone**; the same count in both GAME MODEs),
Track Select (the HD list is the Race Box's own: Vineta K, Anulpha Pass ... Tech De Ra; 2048 mode's is Altima, Rockway Stadium, Capital Reach ...), Ship Select
(no Downs: the cursor opens on the first team), loading, START RACE. 2048 mode needs two taps of cross on the intro, 2.5 s apart. The Zone ship **flies
itself with no input**: on Vineta K it held the line for 60 to 70 s (zone 5 from a score near 5300, zone 6 near 7300, then a crash to the results screen);
on Tech De Ra it hit the first wall at a score near 1100 and sat there at zone 1, so that circuit was not paired (an HD-lineage circuit that survives was
taken instead; open: a steered run). A Capital Reach run reached zone 5 in both boots.

**Freezing instead of pausing.** The port has no pause, and the race state is not located in RAM, so the script `SIGSTOP`s the `gow3-probe` process
`--zone-wait` (56 s) after the START RACE tap, grabs the frame twice 0.4 s apart (RMSE 0.00000 on every capture: the glass holds the last drawn frame),
reads the player's racer row through `/proc/<pid>/mem` (the script is an ancestor of the port, so Yama allows it) and `SIGCONT`s. The caller reads the zone
number, score and shield off `theirs.png` and runs the command again with `--zone-state ZONE,SCORE,SHIELD`, which renders ours at tick 0 on that pose
(`--pose`, `--hud-state`, `--mode zone`). 2048's Zone HUD prints no score, so that pair carries score 0 and only the zone and the shield are the original's.

**The pose (confidence 85).** Racer row 0 (patch Ghidra `0x01a7ae00`, runtime `0x800000000 + 0xa7ae00`, `patch-v107-addresses.md`): `+0x10` position (xyz, w 1.0),
`+0x20` up, `+0x30` forward, `+0x40` the third axis, all unit and mutually orthogonal to 1e-4. Checked against ours rather than assumed: at the row's position
ours' track gives an up equal to `+0x20` (dot 0.9997) and a forward equal to `+0x30` (0.998) on Vineta K, and the forward agreement in the pair is 0.99995 on
Capital Reach, where the game-state mode word at `0x0201280c` does **not** read a mode id (it reads 1017715749 in 2048 mode; it reads 6, Zone, in HD mode), so
the row is validated by the track there. The camera is not located: ours draws the title's chase view at the pose, and the row is the sim's state while the
frame is the last one presented (at most one frame, about 1 unit, apart; recorded as bounded, not measured).

**Repeats.** Vineta K: two cold boots, both zone 5 (score 5900 and 5911, shield 23 both), poses 22 units apart, whole-frame RMSE between the two original
frames 0.142. Capital Reach: two cold boots, both zone 5 (shield 49 and 50), RMSE 0.237 (the scenery windows animate). Confidence 70 (two boots each).

**What differs (read on both `side.png`).** *Vineta K, zone 5:* the original is a dark green road with yellow-green lane lines and walls and an orange sunset sky
with a lens glow, a silver hull, and the Zone ladder with `FLASH` and `SUB-RAPIER` captions; ours has a pale blue-white road, a teal cloudy sky with violet
towers, a dark brown hull, a `PERFECT ZONE` caption and no ladder captions. RMSE 0.340, edge correlation 0.01, mean grey 74.3: the HD-style Zone grade ours
wired in `19f18c1b6` is not the original's zone 5 look on this circuit. *Capital Reach:* the expected gap. The original draws 2048's flat Zone look (a red and green
road, magenta and violet skyline, cyan and yellow window panels); ours draws the ordinary daytime circuit with no grade (held back) and only the word `ZONE`.
RMSE 0.427, edge correlation -0.01, mean grey 69.2. Ours' camera also sits nearer the wall than the original's. **2048 and Omega cross-check: checked, differs in part.** The `zone-dlc-vineta-k-z5` pair of 2048's own Vineta K DLC (Vita3K, zone 5, FLASH
stage) shares the yellow-green walls and lane lines and the stage name with Omega's Vineta K at zone 5, but its road is grey (Omega's dark green) and its sky violet
(Omega's orange); the two frames are at different places on the circuit, so the sky is a view-direction difference as much as a grade one and the road colour is the
reading that stands (one frame each, confidence 40). The racer-row offsets are Omega's patch eboot only; 2048 is a separate executable (not checkable from this lane).

## Zone pairs: Wipeout Pulse PS2 (2026-11-17, `zone-pair-ps2`)

**Claim: the page carries one Pulse PS2 Zone pair, Talon's Junction White at zone 5, against PCSX2 v2.9.168. True, with a gap.** Pair `pulse-ps2-zone-talons-white-z5`
(`pulse-ps2/zone-talons-white-z5/`), 12 parity checks passed, rmse 0.297 / edge correlation 0.069 / mean grey 27.4. Start to pair took about 25 minutes, 12 of them walking from a cold boot.

- **Old PS2 states do not load.** Every state in `data/saves/pulse-ps2/` was made by PCSX2 v2.9.138; v2.9.168 refuses each ("This save state was created with PCSX2 version v2.9.138. It is no longer compatible"), so none of the grid pairs of "Pulse PS2: a race pair for every circuit" can be re-captured from its state until it is walked again. (Seen again 2026-11-27: the refusal for the Metropia Black and Talon's Junction White grid states names v2.9.148.) `data/saves/pulse-ps2/menu/main-menu-pcsx2-2.9.168.p2s` is a v2.9.168 state on the Main Menu (fresh card, name `AAAA` and tag `AAA` as the profile record in EE RAM reads, formatted) to start walks from. The v2.9.168 GS screenshot of the Zone race is 1592x896, while the older grid states' in-race grabs are 1820x1024; it is 16:9 either way, so the one Lanczos resize to 1280x720 stands.
- **The state.** Main Menu, `down` x3, `cross` (Racebox), `right` x5 on RACE TYPE (reads `ZONE`; Speed Class, Weapons and AI Difficulty grey out), `cross`, Track Select (Talon's Junction White, 1/2 on a fresh card), `cross` (Assegai, Zone, all four stats 10), `cross`, `cross` through the description page. **A Zone race on the PS2 flies itself with no input**: zone 0 at about 12 s after the HUD, zone 5 about 36 s (real time under Xvfb), class names `sub-venom`, `venom`, `sub-flash`, `flash` as the zone climbs. The state is `data/saves/pulse-ps2/zone-talons-white-assegai-z5.p2s`, made by running the race in 3 s slices (pause, `savestate` per slice; a slice's picture is read by loading it); zone 5 begins between slices 3 and 4 of the second set.
- **The grab.** `pcsx2-pulse-pair.py --zone-capture` loads the state, writes the game's `Aspect Ratio` option (`DAT_00284fe8 = 1`, a fresh card has 4:3: the first build read projection aspect 1.4286 and the pair refused), advances four frames, holds F8 (a paused VM keeps the screenshot request until a frame is presented: no file appeared), taps F7 once (the file appears; the frame counter moved by exactly 1) and saves a state with the VM still paused. `--zone ZONE,SCORE[,KMH[,SHIELD]] [--zone-class flash] [--zone-from DIR]` then reads the craft, body, camera eye/axes/field and the world clock out of that state and renders ours at tick 0 (`--mode zone --team Assegai --hud-state 1,1,0,0,KMH,SHIELD,0,ZONE,SCORE,ZONE_S`). The race pair's running-VM grab cannot be used: a craft at Flash speed moves many units in the few tenths of a second it lags the state. A flying craft's cached basis trails its body by 0.0135 at this frame (a resting one by under 5e-3), so `pcsx2_state.player_craft` takes a `cache_tolerance` and the Zone path passes 0.05; it picks the craft 11.64 units from the camera eye, the chase rig's distance.
- **HUD values read by eye**, since Pulse PS2's Zone counters are not located in RAM (no PS2 Zone page exists; the PSP's are `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`): zone 5, score 4211, 94 km/h (the grab before the aspect fix read 100 km/h and 4209), shield 19 (the bar blinks, so the grab shows the number only), class FLASH. The grab is deterministic: a second capture of the same state gave an identical `theirs.png` (rmse 0) and the same HUD. The seconds into the zone are not readable off the frame; ours uses 0 and 6 s changes 15 pixels.
- **What differs (side.png read).** The geometry lines up: the cyan arch columns, the round hatch window, the green rail with its white chevrons and the road's tilt are in both, and the craft sits at the same place and heading (0.000 units, 0.000 degrees, since ours is posed on the state). Differences: (1) **the original's craft has a bright white-blue exhaust plume and a glow at the engine; ours has none** (a posed capture runs no tick) and draws a dark hull shadow on the road under it; (2) **ours draws no Zone colour grade on the PS2**: the load log reads `no table of its own: not in the archive set - the zone race keeps the circuit's own colours`, and the zone number moves ours by about 900 pixels (zone 1 and zone 9 against zone 5; not checked beyond the pixel count that it is the HUD digit alone). The original's zone-5 frame is teal and white like its zone-0 frame, so no grade is visible there either; whether the PS2 grades its road at all is unmeasured (the PSP does); (3) the original's shield bar blinked off at the grab, ours draws it red; (4) at the right edge ours' road bends up further than the original's, a small camera disagreement (the eye and field are the state's own; the cause is not isolated). HUD text (LAP, SPEED CLASS, ZONE, SCORE, class name, speed, shield number) is present on both and reads the same values.
- **Checked against PSP:** the zone, score and class readout works on the PS2 build through the shared `--hud-state` zone fields; the missing grade is the open item (the loader finds no Zone table in the PS2 archive set; whether the disc ships one under another name is not checked).

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
| HD/Fury loading (RPCS3) | `scripts/rpcs3-hd-menu-pair.py --walk loading` | the same cold-booted `serve`, then `OAG_RPCS3_ATTACH=1 ... rpcs3-hd-menu-pair.py --walk loading --out D --oag-game <main checkout's binary> --settle 1`: Racebox single race, Vineta K, Feisar (`LOADING_WALK`), CROSS on Team Selection, one photograph 4 s later (`kind = "loading"`; the original's bar stood at 32 of its 166 columns, 19 %). **The walk ends in a race**, so a second attempt is a fresh cold boot. The screen name is declared (`TTY.log` names no loading screen, its last line reads `InGame`). **The feature is the original's random draw** (`Libc_Rand` in `LoadingScreen_Construct`): the pair reads the constructor's `Feature type == N` TTY line, and ours runs `--loading-draw` for the first seed in 0..16 whose logged feature slot equals it, so no cold-boot re-roll is needed; the pair is refused when none does. **The bar is stated, not live:** `compare_menu.lit_bar_columns` counts the lit columns off `theirs.png` (32), ours runs `--loading-step race --ticks 10 n + 5` (325) with no `--loading-live` (its real load made the fill, and so the pair, vary run to run), and the pair is refused when ours' recount differs, or when the original's bar is past the first target (33 columns) that only a load stage can pass. `--loading-screen`'s `DONE/TOTAL` is ignored on a race load, so the tick is the stated state ([hd-loading.md](../formats/hd-loading.md), "The bar's stated state"). Ours is drawn from `LoadingScreen_HD.toml` ([screen-layout.md](../formats/screen-layout.md)). Share of pixels differing by more than 40 grey: 10.52 % with the old code layout, 0.76 % with the file (2026-11-01) |
| Omega (shadPS4) | `scripts/shadps4-omega-menu-pair.py pairs` | the `Rig` of `shadps4-omega-pair.py` (own headless `sway`, uinput pad, muted), cold boot to MAIN MENU in about 20 s, `WALK` in about 95 s. **The pad input reaches the game** (the 2026-10-26 worry that it did not was not reproduced: all four walks of this lane reached Ship Select). shadPS4 has no `TTY.log`: the screen is read by eye from the frame's SCREEN TITLE and declared in `WALK`, and `original.screen_basis` says so |

**The HD Track Select and Ship Select pairs hide the Fury backdrop on both sides** (2026-11-19, `hd-fury-backdrop`): the cloud, its path and
its music pulse are a new roll on every boot and two paths in three are modes ours does not draw, so `rpcs3-hd-menu-pair.py` writes zero over
the tint of the `<ScreenSetting>` row in use before the settle (`scripts/hd_fury_backdrop.py`: `row + 0x40`, three floats), waits 180 of the
widget's frames for the trail to decay, grabs, and writes the authored tint back; ours is drawn with `--fury-tint 0`. `pair.json` carries
`original.backdrop` (row, both tints, frames waited, the adapted luma at the grab) and the write under `provenance.pair_script_ram_writes`;
the flag is in `ours.flags`, so a re-render keeps it. Two grabs of the original then agree to 0.05 to 0.09 mean grey where they differed by
1.4 to 1.7; the pairs read 7.83 to 5.84 and 4.51 to 3.05
([menu-backdrop.md](../ghidra/functions/ps3-hdfury-eu/menu-backdrop.md), "Hidden for a compare shot"). `--show-backdrop` photographs the
boot's own roll instead. `--taps-before ID=BUTTON[,BUTTON]` adds taps on a screen before its photograph: `ship-select=down` steps a save
that opens Team Selection on the Fury hull to the classic one `WALK` declares (the maintainer's save of 2026-11-18 does). **The hex cursor is
kept across visits**, so a second walk on the same boot starts from where the first left it: `up` (or no tap) where the first took `down`;
read the pair's frame (a locked hull is a padlock) before trusting it.

**All five Fury pairs of the walk hide the backdrop** (2026-11-19, `hd-fury-backdrop-2`: `HIDDEN_BACKDROP` also names `campaign-select`,
`grid-select` and `cell-select`). The adapted luma each pair read at the grab (`original.backdrop.adapted_luma`) is stated to ours as
`--fury-adapted-luma`, which sets the craft's exposure on Ship Select and pins the backdrop chain's adaptation on every page whose card the
chain reads (`oag_post::hd_bloom::Chain::pin_adapted`; a capture only, play adapts on its own). Whole-pair mean grey, before to after: Campaign
Select 4.77 to 3.32, cell 3.00 to 2.81, Ship Select 3.05 to 3.04 (the luma moved the hull by under a grey level), **grid 7.28 to 7.75**: its
card animates, two grabs of the original differ by 15.9 mean grey over the card against 1.2 on cell, and the page region fell 1.50 to 0.07
regardless. `original.backdrop.ours_exposure` records the stated value.

**The HD Track Select pair is drawn at the original's turntable angle** (2026-11-19, `hd-track-select`): the circuit model's angle is a global
the original's update steps every frame and keeps across visits, so `rpcs3-hd-menu-pair.py` reads it out of RAM just before the grab
(`*(*(0x008ad4d8 + 0x49ec) + 0x1c4)`, `original.turntable_angle`) and renders ours with `--menu-picker-seconds` set to the seconds that turn
ours' table that far (angle modulo a turn, over 0.6 rad a second). The wall time since the tap does not give it. A pair made before has no
angle recorded: its phase is unknown, and re-rendering it draws ours at the default 1 s.

**What ours draws from XML and what from `menu.toml`** (read off `crates/game/src/capture/menu_page.rs`, `campaign_page.rs`,
`oag_hd::main_menu` and the `--screen` list; `--menu-page` is the only still route and `--screen` reaches only the boot
screens, `Main Menu`, `Team Selection` and the rest are not among its names):

| Screen (HD name) | Ours | Pair |
| --- | --- | --- |
| `Campaign Selection` | XML (HD's DATA06 copy, Omega's own); both read by the one `Chooser::read` | HD yes, Omega yes since 2026-11-05 (below) |
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
| Omega Campaign Select (2048 card) | `CellMode_Definition.xml`, `Campaign Selection` | 0.369 / 0.15 / 53.1 | (`omega-chooser`, 2026-11-05, the original frame being the one kept as `gap-campaign-select`) mean grey 52.7 over the whole frame, **78.4 over the chooser's bracket** (x 160-1755, y 170-950; 61.6 over its left part, 99.1 over the right where the unselected cards stand); both images read: the title, `CAMPAIGN MODES`, `RANK` block and footer are where the original has them to within 6 px, the 2048 card's centre is about 26 px left of the original's and its art is wrong (its navy banner and wordmark do not draw), and the HD and Fury cards stand 45 and about 190 px left of the original's |
| Omega Campaign grid | Omega's own first grid | 0.397 / 0.27 / 55.0 | state not matched: no flyer card at all (a white page with the ghosted backdrop tower), 18 points against the original's 21 |
| Omega Campaign cell | Omega's own first grid | 0.658 / 0.10 / 129.0 | the original's dark-red panel fills about 80 % of the frame and ours is white, the largest difference of the set; `01_TRACK` against `TALON'S JUNCTION` |
| Omega Track Select (Vineta K) | `Track_Selection_Definition.xml` | 0.204 / 0.27 / 19.7 | circuit name `01_Track` (the unresolved name of Omega's entry: the load line names `Data\Environments\01_Vineta_K\screen.xml`, so the circuit is Vineta K) and no photo (the original names it and shows the stills), no length values, the model is turned differently |
| Omega Ship Select (Feisar) | `Team_Selection_Definition.xml` | 0.283 / 0.26 / 34.0 | hull about twice the original's size in a different livery (dark teal and yellow against pale blue and yellow), a ringed Feisar logo against the flat one, the cursor on the top hex against the third row |

**2026-11-18 (`hd-menu-cell`)**: HD's campaign cards are drawn through the front end's bloom chain (the original's
exposure and bloom, read live per screen: `docs/ghidra/functions/ps3-hdfury-eu/front-end-model-lighting.md`, "The
flyer card is drawn before the resolve"). Mean grey, RMSE, edge correlation, re-rendered the same day from the
pairs' recorded flags: `hd/menu-cell-select` 5.06, 0.0813, 0.456 to **3.00**, 0.0684, 0.454; `hd/menu-grid-select`
7.68, 0.1269, 0.308 to 7.28, 0.1239, 0.325; `hd/menu-campaign-select` 5.12, 0.0739, 0.229 to 4.77, 0.0698, 0.236;
`hd/menu-ship-select`, `menu-track-select`, `loading-loading` and the five `omega/menu-*` unchanged to the digit.

**2026-11-18 (`hd-menu-ship`): HD's menu pairs were made with every team and circuit unlocked.** `rpcs3-hd-menu-pair.py`
never wrote `[dev] unlock_everything = false` into ours' profile (the Pulse and Pure scripts do: `compare_menu.extra_settings(...,
locked=True)`), so no HD pair records `ours.unlock_everything` and a debug build drew Ship Select's honeycomb with every team
open, where the original's fresh walk shows its padlocks (37.6 % of that pair's grey difference). The script now builds the
profile through `prepare_profile` (locked), and `check-compare-environment.py` holds it to that. A pair already made keeps
its record: set `ours.unlock_everything = false` on it and re-render (`compare-rerender-ours.py`). Measured on the six HD pairs:
only `hd/menu-ship-select` moves (5.72 to 5.09 with the locks, then to 4.55 with "The honeycomb fades" in
[campaign-screens.md](../ui/campaign-screens.md)); the cell, campaign, grid and track pairs are unchanged to the digit locked or
not, `hd/loading-loading` moves 2.0696 to 2.0718 locked (not adopted there). Omega's menu script has the same gap; its
Ship Select original shows padlocks too but ours cannot be put on its selection, so locking it alone moves it 29.15 to 29.75
and it is not changed.

Gaps (no pair, the original's frame kept in `gap-*` beside the pairs with `gap.json`): HD Main Menu, Racebox setup and Options+
(`Additional`); Omega Main Menu, Racebox setup and Options (Omega Campaign Select was a gap until 2026-11-05: `scripts/shadps4-omega-menu-pair.py`'s `WALK` row is now an XML page, so a re-run of `pairs` makes it a pair; adopting the pair made without the emulator from the kept frame: replace `omega/gap-campaign-select` with `omega/menu-campaign-select`, then add it to the `manifest` command below, 10 entries). The ranked list of differences is in the
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

**2026-11-27 (`2048-menu-pairs`).** Home and Game Mode re-take their stills past the entry
fade, and four pairs fell: [campaign-screens.md](../ui/campaign-screens.md), "2048 menu pairs:
what the four worst differed by".

## Menu pairs: Wipeout Pulse and Wipeout Pure (PSP) front-end screens (2026-10-27, `menu-pairs-pulse-pure`)

The PPSSPP counterpart of the HD, Omega and 2048 menu route (`scripts/psp-menu-pair.py`, on `compare_menu.py`). The software
renderer at `InternalResolution = 1` (`compare_env.py ppsspp-ini`), the USA pressings the environment file pins (`compare-environment.toml`
names `pulse-psp-usa.chd` and `pure-psp-usa.chd`; the EU preference does not override a pinned platform), muted, own Xvfb and
debugger port, a **fresh memory stick** (Pulse's first-boot dialogs answered by `psp-drive.py`'s `FIRST_BOOT` table, Pure's profile made
with `New`). The frame is the emulator window at 480x272 (`xdotool windowsize`), grabbed off the Xvfb root and cropped at the window
origin: no scaling at any step. Ours is `oag-game --menu-page PAGE` (or `--screen`) at 480x272 in a private profile
(`prepare_profile`): the compare render settings and **`[dev] unlock_everything = false`** (a debug build unlocks everything, the original's
fresh profile does not; read back in the settings file the run rewrote). Pulse also gets `records.toml`'s `campaign_difficulty = medium`
row, which the still **ignores** (see the campaign paragraph). **Pure's plan Loads a profile** (`PURE_PLAN`: START, Profile, Load): on a new stick it stops on the system's "There is no data" dialog, so make one first on that instance (2026-11-23: Profile, NEW, cross through the name `NAME` until the SAVE PROFILE prompt, YES, then the system Save dialog's cross), then cold-boot it and run the pairs.

**What is read and what is declared.** Pulse's screen name is read from the game's own state machine (`0x08b31784 + 0x18c`, the same
`Debugger.state_name` the race pairs use); every Pulse pair's `original.screen_basis` says so. Pure has no recovered state name: its
screens are declared in `PURE_WALK` and read by eye (`original.screen_basis`), and the chain screens' `seconds_since_entry` is the emulator's tick counter from the entering pad press to the CPU-stopped grab (`PURE_CLOCKED`, about 35 ms), and the walk waits on a frame predicate (`pu:cross:title`,
the orange logo) because the language and memory-stick screens swallow a fixed-sleep walk. The highlighted row of both sides is read by
eye and declared (`selection`), so the check catches a `WALK` entry that names two rows, not a cursor that drifted; the pair images are
the evidence.

**What ours draws from the disc and what from `menu.toml`:**

| Screen (the original's name) | Ours | Pulse | Pure |
| --- | --- | --- | --- |
| title, `Show Logo` / `Title Screen` | Pulse `--until "Show Logo" --press cross --press-until-reached --ticks 200` (the sequence draws the ray backdrop; `--screen "Show Logo"` leaves it out: 14.9 mean grey, 6.0 % against 4.7, 2.6 %), Pure `--screen "Intro Screen->Title Screen"` | pair | pair |
| `Grid Selection`, `Cell Selection` (the Race Campaign) | `--menu-page grid-select`, `cell-select` (`CellMode_Definition.xml`, `grid_00.xml`) | pair, pair | none: Pure draws a list (`this source has no Race Campaign to show`) |
| Pure's Single Player chain: `Single Player`, `Class Selection`, `League Selection`, `Tournament Selection`, `Track Selection` | `--menu-page pure-chain[:MODE[:ROW...]]` | none | pair x6 |
| `Track Creation` (Track Select), `Team Selection` | `--menu-page track-select`, `ship-select` | pair, pair | `Team Selection` pair; Pure's circuit list is `Track Selection` above |
| `Main Menu` | Pure: `--menu-page pure-main` (`Skin.xml`'s `Main Menu`, the chain's drawer; the live main page is `menu.toml`'s, ADR-0066). Pulse: `--menu-page pulse-main` (`MainMenu_Definition.xml`, the menu model) | pair | pair |
| `Racebox`, `Single Player` (Racebox setup), `Additional` (Options+) | Pulse: `--menu-page pulse-racebox`, `pulse-single`, `pulse-options` (`RaceBox_Definition.xml`, `Additional_Definition.xml`) | pair x3 | none (Pure has no Racebox or Options row) |
| pause menu | `--drive-file scripts/pure-pause.drive` / `scripts/pulse-pause.drive` with `--original-menu` (our own session: a Time Trial, START) | pair | pair |

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
| Pulse Track Select | 0.180 / 0.26 / 19.4 | 16.2 % (14.6 % re-made 2026-10-29) | the circuit photo (stacked cards, a different view), `1/3` on both sides since 2026-10-29 (ours showed `1/11`: the three circuits that author no `<Unlock>` plus eight DLC-pack circuits from the maintainer's `data/dlc`, which the pair run mounted; the compare menu route now sets an empty `$OAG_DLC`), distance `5178` against `5094`, the empty records `-.--.--` against `-` **2026-11-23 (`pulse-psp-track`), three footer fixes on one fresh grab (the TKR_TTTIM frame, `TICKER_PINNED`):** whole frame 1.60 % to 0.69 % over 40 grey; the footer below y 232 1543 to 358 px (ticker box 1267 to 226, `AAA` tag box 172 to 34, the line at y 249 316 to 0); everything above the footer unchanged at 543 px. The ticker is pinned (`tick:480` before `shot:track`, as `team`), the tag read by the shared pair script, and the footer strips are cut to their viewports (`docs/ui/menus-original.md`, "A viewport clips its images"). **Open, with the evidence:** the heading's glyph pens (every letter but the first and last one pixel right of the original's, `TRACK SELECT`: ours 50, 60, 68, 77, 86, 100 ... against 50, 59, 67, 76, 85, 99 ...; the other Pulse headings miss by one glyph each; a pen-rounding difference of the title face); one grab in four (the one at 26.3 s since entry) showed a bright teal haze left of and under the circuit name on the original that no frame of the backdrop movie reproduces (best of 270 scored on the panel box 15.1 against 16.4 grey at frame 0), so it is another clock (the text glow's `s` ladder is `caught up to the game clock`, `Text::glow_stamp`) and that grab read 2.22 % whole against 3.13 % before the three fixes, the glow-phase read being the next step (read the stamp off EDRAM as `selector_stamp` does); the footer prompts' icons are one pixel left of the original's (the nav legend, not this lane's files). **2026-11-23 (`pulse-fe-leftovers`):** the heading's pens, the prompt icons and the circuit name's haze are closed ("Pulse Cell and Track Select: glyphs, pens and the name's glow" below): fresh grabs, main's binary against this branch over 40 grey, whole frame 0.54 % to 0.14 %, 2.32 % to 0.15 % (the haze grab), the photo box 0 to 50 px |
| Pulse Ship Select | 0.129 / 0.34 / 9.5 | 7.1 % (7.0 % re-made) | the Assegai hull: the original's blue-white livery large and tilted, ours a dark livery smaller; `1/8` on both sides since 2026-10-29 (it was `1/12`: four DLC teams) |
| Pulse title | 0.069 / 0.30 / 4.7 | 2.6 % | the type: the original's wide `PRESS START BUTTON` and legal line against ours' small mixed-case `Press START button`; the rays are at another phase (animated) |
| Pulse Grid Selection | 0.076 / 0.35 / 4.2 | 2.1 % (0.57 % on a 2026-11-23 grab, 1.32 % before the ticker and tag reached the campaign still) | the footer's prompt glyph rings and one bar row (the cursor halo is the bloom, drawn since 2026-10-29); rings and pens closed 2026-11-23 (`pulse-fe-leftovers`), re-rendered on a copy 1.41 % to 1.13 % against the same `theirs.png` (Team 1.38 % to 0.96 %) |
| Pulse Cell Selection | 0.077 / 0.39 / 4.3 | 1.8 % (0.65 % re-made 2026-11-23 on one fresh `theirs.png`, 2.14 % with main's binary on the same frame; see "Pulse Cell Selection: the footer, the tag and the title's glow phase" below) | **2026-11-23 (`pulse-fe-leftovers`): 0.04 % (56 px) on two fresh grabs, main's binary 0.39 % and 0.41 % on the same frames**; what is left is the Confirm and Back prompts' dim top rows (below); the glyph rings and the silver bar's row y 249 are closed |
| Pure Team Selection | 0.101 / 0.29 / 6.9 | 8.6 % (7.5 % re-made 2026-10-29, eight teams, Feisar to Harimau, as the original; 3.2 % on 2026-11-01 with the list in the menu face at the chain's 18-line pitch, the stat labels in the `Stats` face and the hull at the original's slideshow seconds (7.57 % the same fresh capture before)) | closed 2026-10-29: 17 teams listed (DLC and gated). Closed 2026-11-01: the list type (the `<Menu>` names no `font`, so it draws in the menu face at the chain's 18-line pitch, where it was the `default` role's 13/22 scale), the hull's angle (the three stills `Info`/`Side`/`Top` change every 2 s, the pair now at the original's seconds since entry), the stat labels (`font="Stats"` is Pure's `LTe50325.fnt`). Left: the description line the original types along the bottom, the two frame rules, the cursor arrow |
| Pure Track Selection (single race) | 0.092 / 0.40 / 5.0 | 5.5 % (5.6 % re-made 2026-10-29 with the type-in; 4.2 % on 2026-11-01 with no context line in the still and the `Stats` face for the labels, cells and the description (5.63 % the same fresh capture before)) | what is left: the record separators (the original's are small mid-height dots with wide gaps, no `'` or `"` at 8x, and no Latin-1 character as the separator matches), the frame's dashed rule at y 217 and solid rule at y 251, the picker's scroll arrows against the original's pink cursor arrow. Closed 2026-11-01: the context line (the original draws none; the live screen keeps ours), the `m` case and the label type (the `Stats` face) |
| Pure Single Player (mode) | 0.074 / 0.40 / 3.0 | 3.4 % | no context line here; the hint line and the `BACK` prompt differ (below) |
| Pure Tournament Selection (campaign) | 0.069 / 0.44 / 2.7 | 3.1 % (3.0 % re-made 2026-10-29 with the type-in) | the context line `TOURNAMENT - VECTOR` (chosen), the description line (typed in since 2026-10-29, at the original's seconds), the footer prompt |
| Pure League Selection, Class Selection (x2) | 0.066-0.067 / 0.44 / 2.4 | 2.8-2.9 % (League 2.8 %, Class 2.6 % re-made 2026-10-29) | the same three: context line, description line (typed in since 2026-10-29), footer |
| Pure title | 0.017 / 0.60 / 0.5 | 0.3 % | none visible (the cursor underscore blinks) |

Three differences recur on every Pure chain screen: the **context line** (ours' `SINGLE RACE - VENOM`, a known chosen stand-in of
`docs/architecture/menus.md`, absent from the original), the **description line** along the bottom (the original types it in, so its frames hold a prefix; ours types it in too since 2026-10-29, and draws it settled when no `--menu-picker-seconds` is given: the string is the disc's `Text->SCREEN->ITEM->VALUE` entry, for example
`Text->Track Selection->Track->Vineta K`, laid in the footer `TextInfo` viewport at `(21, 217)` 427 wide, amber `TextInfoBoxColor`, wrapped
to its three `Text` lines; its scale `0.7` is **chosen** because the `FEGlobals` it names are not declared; the type-in (58 characters a second, a 3.12 s hold and a loop, a cursor) was measured and wired 2026-10-29 and the pairs are drawn at the original's seconds since entry, so the shares fell again: Track Selection 6.4 to 5.6 %, Class 3.6 to 2.6 %, League 4.5 to 2.8 %, Tournament 4.7 to 3.0 % (`docs/ui/menus-original.md`, "Wipeout Pure types its description line in"; before that wiring they had **risen** on a full text against the original's few typed letters; the original's face is the `Scroll` face, `LTe50325.fnt`, which ours now draws too (2026-11-23))) and the **footer's BACK prompt** (fixed 2026-10-29: the skin's `Back button` is the table's `Back Button`, found case-folded,
and draws the triangle glyph). Still open on the same frames: the dashed cyan rule across the bottom at y 217 and the solid footer rule at y 251,
which the original draws and ours does not. Pulse's pairs share the footer
ticker (scrolling in the original) and the cursor glow.

**Repeat noise** (run 1 against run 2, cold boots on Pure; Pulse run 3 against run 1, two boots on one instance and a fresh one, the title two further boots each): ours against itself exactly 0 on
every pair. The original against itself: Pure 0.0 mean grey and at most 0.06 % of pixels differ over 40 on every screen (the walk is
deterministic); Pulse 0.7-4.7 mean grey, 0.1-3.5 % over 40, the Ship Select turntable and the footer ticker (Track Select's photo cards
2.5 %, the title 0.0 %).

**Gaps** (original kept under `gap-*` with `gap.json`): none left on Pulse. Pulse's `Main Menu`, `Racebox`, `Single Player`, `Additional` and
pause menu were gaps (ours drew them from `menu.toml`, or not at all) until 2026-11-23 (`pulse-gap-pairs`, below); Pure's `Main Menu` and pause menu likewise
(`pure-main-pause`, below). Pure has no Options or Racebox screen in its main menu.

**Screen-entry clock (2026-10-29).** `original.seconds_since_entry` (`value`, `basis: measured`) is the wall clock from the state-name poll
that saw the screen replace another (polled every 0.03 s, so +-0.015 s) to the middle of the frame grab; PPSSPP holds 60 fps, but entries
spread about +-0.1 s in the first second (`docs/ui/campaign-screens.md`, "The `Selector` pulse"). It exists only when the walk watched the
change (a plan step `key:BUTTON,st:SCREEN`, which skips the tap's 0.7 s settle); otherwise it is absent, never guessed. Ours gets it as
`--menu-picker-seconds` for the pages in `CLOCKED` (`grid`, `cell`, `team`, `track`) only.

**The wall clock is replaced on `team` and `track` (2026-10-29, `pulse-picker-clock`).** Their `seconds_since_entry` is the front end's own
animation clock (`*(0x08b317b0) + 0x40`, emulated seconds, read with the CPU stopped for the grab) minus its value as the entering press
starts (the screen changes a frame or two later; sampling at the poll that saw the new name is 0.15 s late, because the press call
returns after the button's six frames). `team` also reads the live ship `Anim Transform` node's own time (`node+0x40`, the node whose
`+0x80` equals the clock and whose loop is 5.9833 s) and passes it as `--menu-model-seconds`: the hull's sway started when the model
loaded, 0.5 to 1.9 s before the screen entry, so it is not `--menu-picker-seconds`. Against the wall clock the Track Select photo box went
from a mean abs diff of 44.9 grey to 0.28 and the Ship Select hull box from 29.0 to 5.2.

**Reproduce:** `scripts/ppsspp-start.sh <lane> 91 <port> <image>` with `PPSSPP_EXTRA_INI` from `compare_env.py ppsspp-ini pulse-psp|pure-psp`,
`source <scratch>/pp/env.sh`, ours' `XDG_*` under `data/scratch/<lane>/`, then `uv run --with websocket-client --with pillow python3
scripts/psp-menu-pair.py pairs --title pulse|pure --out D --autopilot off --difficulty medium|""` (ours is run with an empty `$OAG_DLC`, `compare_menu.render_ours`, so `data/dlc` packs do not join its lists; about 90 s on Pulse from a walked
`Main Menu` without a `tick:` step (the default plan has two, each waiting up to 480 s for a pinnable message: run it with `emu-run.py --bg --status F` and `wait F`, or one id's plan with `--plan` as the foreground watchdog's 570 s allows), about 3 min on Pure from a cold boot; `walk --plan "..."` photographs any plan, `--ids` restricts a run; Pulse's title is a
separate boot: `--plan "st:Show Logo,wait:5,shot:title" --ids title`). Adopt: `python3 scripts/compare_env.py manifest
data/reference/compare-page/menus-pulse.toml data/reference/compare-page/pulse-psp/menu-*` (and `menus-pure.toml` over `pure-psp/menu-*`),
then `just compare-page`.

**The Track Select ticker (2026-11-23, `pulse-psp-track`).** `track` is in `TICKERED` beside `grid`, `cell` and `team`: `PULSE_PLAN` has `tick:480` before `shot:track`, the grab waits for a pinnable message and ours is drawn with `--menu-ticker ITEM@X` and the matching lifetime row; the tag is read as on the other pairs. The walk to the shot takes 20 to 30 emulated seconds longer, and `--menu-picker-seconds` (photo cards, turntable) held at 21 to 26 s: the photo box read 0.2 % of its area differing on three of four grabs (50, 51, 50 px) and 0.7 % (215 px) on the fourth (22.0 s, the adopted pair), where it is now the largest region left and unexplained (a slideshow card or crossfade phase). `wait_ticker` makes a fresh `Watch` each wait, so a record's item remembered from the previous screen cannot pair with a new message.

### Pure Main Menu and pause menu pairs (2026-11-23, `pure-main-pause`)

Claim: Pure's Main Menu and pause menu each have a pair (the original's screen against ours, environment-checked, both English, a fresh profile) and each pair's largest
differing region is fixed or measured. Share of pixels over 40 grey, whole 480 x 272 frame, one fresh capture each:

| Pair | Before | After | Route |
| --- | --- | --- | --- |
| Main Menu | gap (ours' `menu.toml` page against the same frame: 4.89 %, mean grey 3.68) | 2.97 % (mean grey 2.26, edge 0.48) | `--menu-page pure-main` |
| Pause menu (Time Trial, Vineta K, Vector, Feisar) | gap | 2.82 % (mean grey 6.10, edge 0.85); song box 335 to 54 px | `--drive-file scripts/pure-pause.drive --original-menu --state-log /dev/null` |
| Single Player (mode), its row descriptions | 3.13 % (the committed pair) | 1.90 % | `--menu-page pure-chain` |

**Main Menu.** The page ours draws live is `menu.toml`'s `main` (ADR-0066: this build's rows, title `OPENANTIGRAV`), so a compare shot draws the disc's own screen
instead: `--menu-page pure-main` reads `Skin.xml`'s `Main Menu` `Mode` entries (`Single Player`, `Multiplayer`, `Profile`, `On Line`; `Teasers` is a `noEntry`), the
heading `Main Menu`, the row descriptions (`Text->Main Menu->Mode->ROW`) and the footer with `CONFIRM` alone (`backward="none"`), through the chain's drawer
(`oag_ui_screens::pure_chain::draw_list`, `campaign::pure_chain::Chain::main_view`). The pair walk shoots it after BACK from Single Player, so the description's type-in clock
(`--menu-picker-seconds`) is the press-to-grab tick count like every chain screen. **Largest regions** (px over 40 grey, of 3,029 on the pair as first shipped; the typed description was 1,342 of them
until the `Scroll` face was drawn on the snapped pen at 0.8, 150 of 572 on main now, one-pixel misses on the first glyph of each word: `docs/ui/menus-original.md`, "Pure's right-aligned text ends on its ink"). **Row 4 reads `ON LINE` in ours and `DOWNLOAD` in the original**: the USA disc loads `PI012`, a US-spelling overlay
(`Data\Plugins\PI012\Definition.xml`: `<Entry ID="On Line" String="Download">`, with `Téléchargement` and `Descarga` for French and Spanish), and ours does not
(`oag_pure::LANGUAGE_MANIFESTS` leaves it out of the offered list as "not a language"); it also holds `Synchronizing`, `favored` and the Zone and team descriptions. **Fixed**: the
manifest's `overlays` name it and `load_strings` lays its section for the chosen language over the table (`docs/architecture/frontend-boot.md`, "The offered languages are each executable's plugin manifest"), live and in this
still alike; the EU disc ships the same file and its manifest never loads it, so a EU boot reads `On Line`. One fresh render of the shipped pair's flags, whole 480 x 272 frame: pixels with any channel more than 40 from the original 4,119 to 3,595 (3.16 % to 2.75 %; the shipped pair's `ours.png` predates later pure-menu work, which is why it is not the 2.97 % above), the same count on luma alone 1,381 to 1,027 (1.06 % to 0.79 %); row 4's band (y 100-120) 651 to 127 any-channel px and 354 to 0 luma px; mean grey 1.43 to 1.28, edge 0.533 to 0.543. The project's own main page (`menu.toml`) draws neither row.

**Pause menu.** Ours is a menu over a parked race, so no still draws it: the route plays our own session the way the original is walked (`boot`, SINGLE PLAYER, TIME TRIAL, Vector,
Alpha, Vineta K, the team, `@TICKS@` race ticks, START) with `--original-menu`, which now leaves this project's PHOTO MODE row out of a pause screen that authors none (the original has
three rows). The racebox page offers no Vector class (Venom first), so the campaign flow is the route that races the original's class. The drive draws at 1440 x 816 and the pair
script box-filters it to 480 x 272 (`compare_menu.box_down`); the pair keeps `ours.drive`, which `compare-rerender-ours.py` runs (re-rendered on a copy: byte-identical `ours.png`).
The original is `data/saves/pure-psp/time-trial-vineta-vector-feisar-restart.ppst` loaded, 4 s, START, 2 s (`PURE_PAUSE_PLAN`). The grab's race clock varies by seconds between loads
(wall waits), so `--pause-clock` is the `CURRENT` the frame shows, read by eye (22.3), and ours is played to it (`PAUSE_CLOCK_ZERO_TICKS`, 1,612 ticks); with that both HUD clocks read
0.22.3 and `BRONZE TIME` 1.47.6. `--from-png pause=PNG` re-makes ours' side from a frame already grabbed, with no emulator. **Fixed**: the race's song-name line under the tint (the
original shows none: the music widgets are `InGame`'s and the pause screens are its siblings): `PauseMenu::song_line_under_scrim` is `false` on Pure, `true` elsewhere (unmeasured); song
box (130, 238, 200 x 24) 335 px to 54. The band itself matches: rules at rows 75 and 175, ink rows 103-114, 121-132, 139-150, peak ink 252/156/149 against 253/157/149.
**Largest regions left** (px over 40 grey of 3,688): the HUD top 1,396 (the digits' glyph edges and the blimp, the HUD lane's), the scene below the band 948, the band's rows 821 (read then as ours' words 3 px narrower over 105: wrong, corrected below), the
footer line 522.

**The band's words and the HUD top, measured again (2026-11-24, `pure-pause-hud`; ours re-rendered from the branch, luma over 110, per-glyph column runs).** The words are **not narrower**:
`CONTINUE` is 108 px wide on both (original 187-294, ours 186-293), `RESTART` 102 on both (190-291 against 189-290), `QUIT` 48 on both (216-263 on both), and every glyph's start is
the original's less the same shift (the pen steps 15, 15, 15, 15, 4, 15, 15 match). The earlier "3 px" came from `scripts/pause-band-metric.py`'s `min`-over-channels threshold of 95, which
loses the right edge's antialiased column on one side. What differs is **where the string starts**: ours is 1 px left on `CONTINUE` and `RESTART`, equal on `QUIT`. The pen ours uses is
`ceil(240 - end / 2)` (`end` the snapped run's 108.4, 104.4, 50.4); the original's first quads are at 187, 189, 215. `floor(240 - sum_of_advances * 1.2 / 2)` (187.8, 189.6, 215.4) fits all three
rows and moves the band-word box (y 98-155, x 100-380) from 610 to 240 px over 40 grey, but applied to every centred snapped string it moves the footer `Press X to Confirm` (pen 143.5 on a
193 px advance sum; the original starts it at 144, as the unchanged pen end does) one pixel left and worsens the footer box 352 to 565 px. **On the disc they are different elements**
(`InGame_Definition.xml`, `InGame Main Menu`): the rows are `<Menu name="Option">` `Entry`s (`align="centre" scale="1.2" x="240" wrap="true"`), the footer a `<Text>` (`align="centre" x="240"`, scale 1),
so each can have its own rounding; the rows' floor is a fit of three words in one frame, **not shipped** (placement 1 px left on two of three rows: confidence 85; the rounding rule: 40). The
ink edge, the snapped end and a raised `<Menu>` scale (1.2 times 1.017, 1.025, 1.033: the box went from 610 to 802, 814 and 884 px) do not fit. **To decide it** (done: the next paragraph), capture the Free Play pause
(`End Session`) and the Tournament pause at scale 1.0 in the original: an odd advance sum is a tie, and which side it falls on says whether the rows' rule is a floor at every scale. Ours' rows 2 and 3
are also 1 px taller at the top (121 and 139 against 122 and 140) with 7 to 8 % more ink mass: the 3x draw box-filtered down, not the original's native 1x sprite.

**The rows' rounding, decided at scale 1.0 (2026-11-24, `pure-row-centring`): the floor rule is refuted, and no single law of the string's advance sum fits all seven rows.** The Free Play pause
(`InGame Free Play Menu`: `Continue`, `Restart`, `End Session`) and the Tournament pause (`InGame NoRestart Main Menu`: `Continue`, `Quit`), both `<Menu>` rows at scale 1.0, were captured on PPSSPP
1.20.4 (software renderer, USA disc, a fresh stick with a profile made through `NEW`; Vineta K, Feisar, Vector) on **two separate boots** each (the second a cold restart on the saved profile), and the Time Trial pause (scale 1.2) on the second boot and in the earlier lane's committed pair, with ours
drawn from the same flow (`oag-game --drive-file`, `--original-menu`, 480x272) and the same race. Each row is measured by shifting ours' ink mask (luma over 100) against the original's inside the row's
y band and keeping the shift with the fewest differing pixels, and by per-glyph column runs (luma over 110): every row has the same glyph pitch on both sides, so a shift is the string's start.

| Row (advance sum at scale) | x - sum / 2 | Original's first quad (pen) | Ours today `ceil(240 - end / 2)` | `floor(240 - sum / 2)` |
| --- | --- | --- | --- | --- |
| `Continue`, 1.2 (104.4) | 187.8 | 187 | 186 (1 px left) | 187 |
| `Restart`, 1.2 (100.8) | 189.6 | 189 | 188 (1 px left) | 189 |
| `Quit`, 1.2 (49.2) | 215.4 | 215 | 215 | 215 |
| `Continue`, 1.0 (87) | 196.5 | **197** (first ink 197 on both boots) | 197 | **196** |
| `Restart`, 1.0 (84) | 198 | 198 | 198 | 198 |
| `End Session`, 1.0 (118) | 181 | 181 | 181 | 181 |
| `Quit`, 1.0 (41), Tournament | 219.5 | **220** (first ink 221 on both boots) | 220 | **219** |

Ours' rows at scale 1.0 are **already the original's, to the pixel** (best shift 0 on `Continue`, `Restart`, Tournament's `Continue` and `Quit`; `End Session` ties between 0 and -1: its glyphs start at the same columns on both sides except the `S`, 224 on ours against 225, and ours reads 1 px wider on the right of nearly every
glyph, which is the box-filtered 3x draw, not a space advance or a pen): the two odd sums, which are ties, round **up**, as today's ceil does and as the floor does not. At 1.2 the three rows are the original's
less 0, 1 and 1 px (Time Trial, two captures: shifts +1, +1, 0 in both). A search over the width (raw sum, snapped end, ink edge, sums of per-glyph ceil, floor or round advances) and the rounding (floor, ceil, round,
trunc, with an offset of -1 to 1, with `x - W / 2` rounded before or after) finds **no law that fits all seven rows**: the floor of the raw sum fits 5 of 7 (the two 1.0 ties wrong), ours today 4 of 7, and the
best of the others 5 of 7; centring in unscaled units then scaling fits 5 of 7. The fractional part of `x - sum / 2` alone cannot decide it: .8, .6 and .4 fall below (scale 1.2) while .5 falls above (scale 1.0).
**Verdict: the rule `floor(x - sum_of_advances * scale / 2)` for `<Menu>` rows is false (confidence 85 that it does not hold at scale 1.0; two ties on two boots each).** What stays true: a start 1 px left of
the original on `Continue` and `Restart` at scale 1.2 (confidence 90, two captures), and ours exact at 1.0 (confidence 85). Whatever the original does at 1.2 is a scale-dependent step (a scaled sprite run's own
rounding), which needs the original's text-draw code, not more frames; nothing was wired and no code changed. The Free Play and Tournament frames the 2026-11-07 pause lane cited as `data/reference/pure-pause-menu/`
are gone from that directory, so these were captured again; the lead adopts them into `data/reference/pure-pause-menu/`. **Method caveat:** the original's leading column moves 1 px with the cursor (`Continue` at 1.2 reads 187 focused and 188 unfocused, Tournament's `Quit` 220 and 221 for the same reason: the grey rows' ink starts a column later under the threshold), so every comparison above has the cursor on the same row on both sides.

**The rows' rounding, recovered from the executable (2026-11-24, `pure-centre-rounding`): a centred string starts at `ceil(x - W / 2)`, `W` the executable's own measure, and it fits all seven rows (confidence 80).** `docs/ghidra/functions/psp-pure-usa/menu-text-centring.md` has the function
(`Text_BuildGlyphQuads`, `0x08947ae0`; the measure `Text_MeasureString`, `0x08947868`; `Math_Ceilf`, `0x088da5e0`). `W` is not an advance sum: it starts the pen at the first glyph's ink (`-b0 * s`), steps every glyph's pen up to a whole unit (`ceil(pen + b * s)`, then `- b * s + advance * s`) and ends on
the last glyph's ink (`bearing + authored width`). In `f32`, `FX300ANG.fnt`, x 240: `Continue` 1.2 measures 107.8 (start 186.1, first quad 187), `Restart` 1.2 103.8 (188.1, 189), `Quit` 1.2 50 (215, 215), `Continue` 1.0 87 (196.5, 197), `Restart` 1.0 84 (198), `End Session` 1.0 118 (181) and
`Quit` 1.0 41 (219.5, 220): seven of seven, the two ties rounding up, with no parameter chosen (`crates/ui/tests/pure_centre_ground_truth.rs`). Wired for Pure's snapped pens (`oag_ui::font::snapped_centre_pen`, `Renderer::push_text` and `hints::scan`): the band-word box (y 98-155, x 100-380) goes from **610 to 240 px
over 40 grey**, every band row's best shift against the original is 0 (it was +1, +1, 0), and the rows of the Free Play and Tournament pauses (scale 1.0) are as exact as before. The footer, a `<Text>` centred through the same function, renders pixel for pixel as before. Every other Pure pair (the nine other menu pages and the fourteen circuit shots), a Pulse PSP and a Pulse PS2 menu and two HD menus
re-render pixel for pixel equal. Still open, listed on the Ghidra page: the quad's fractional right edge (the code) against `floor(w * s)` (ours, measured), and the quad's left edge for a glyph with a bearing. The remaining 240 px of the band box are the 3x draw's box filter and the ink mass, not a pen.

**The HUD top, 1,396 px over 40 grey** (`CURRENT` clock 338, `BEST` clock 93, `BRONZE TIME` 383, the rest 582 in the scene behind): the clock digits' glyph edges are the race HUD's own text
(shifting ours' `CURRENT` box by up to 2 px moves its mean difference only from 18.3 to 16.3, so it is the glyph and its shadow, not a placement), which the brief leaves to the Pure race HUD
lane; the scene behind is moving: the pennants on the pole at the left and the blimp wave and drift with the world clock, which differs between a state load and ours' ticks (their shape differs
in `side.png`). Nothing here was changed.

### Pulse list screens and the pause menu (2026-11-23, `pulse-gap-pairs`)

Claim: Pulse PSP's Main Menu, Racebox, Single Player, Additional (Options+) and pause menu each have a pair against a fresh capture of the original (ours drawn from
the disc's own screen, both English, a fresh profile), with the footer ticker and tag pinned, and the largest region of the worst pair is fixed or measured. Share of
pixels over 40 grey, whole 480 x 272 frame, one fresh capture each (the BEFORE is `menu.toml`'s page against a fresh capture with its own ticker pin; the grabs differ, so the
columns are two measurements, not one frame twice):

| Pair | Before | After (mean grey, edge) | Route |
| --- | --- | --- | --- |
| Main Menu | gap; `menu.toml`'s `main` 11.40 % (mean grey 15.10) | 0.97 % (3.01, 0.41) | `--menu-page pulse-main` |
| Racebox | gap (no page: RACEBOX opens `race`) | 0.44 % (1.66, 0.31) | `--menu-page pulse-racebox` |
| Single Player | gap; `menu.toml`'s `race` 1.91 % (3.90) | 1.04 % (2.69, 0.41); 1.71 % before the focused value's bloom | `--menu-page pulse-single` |
| Options+ (`Additional`) | gap; `menu.toml`'s `options` 5.88 % (9.15) | 0.44 % (2.01, 0.32) | `--menu-page pulse-options` |
| Pause menu (Time Trial, Moa Therma White, Venom) | gap | 1.56 % (4.51, 0.94); 4.10 % on the first grab (2 s after START, header mid-slide) | `--drive-file scripts/pulse-pause.drive --original-menu` |

**The four list screens are compare-only pages of the menu model** (`oag_game::capture::disc_screen_page`, named by the title's `FrontEnd::list_screens`: Pulse's four, empty elsewhere): the title, the rows and the help lines are read off the screen's own XML
(`MainMenu_Definition.xml`'s `Main Menu`: `FE_MM`, the seven `Entry` rows, `helptext0` to `6`; `RaceBox_Definition.xml`'s `Racebox` and `Single Player`: the three rows, and the
five `<List>`s with their `Text` labels and `default`s; `Additional_Definition.xml`'s `Additional`), through the same `oag_ui::menu::draw_list` the live pages use. The live pages stay
`menu.toml`'s (ADR-0066, rule: our menu additions stay in play, compare shots draw the original's screen), every row of a still points at its own page (it is never activated), and
Pulse's `MenuSettings.pages` lists `pulse-single` beside `race` so the settings look (rules, value column, step arrows) applies. `oag_ui::screen::Menu` keeps its `Entry` ids and
`ListWidget` its literal entries (`Eliminations`' `5`, `10`, ...). The header of `Racebox` and `Single Player` is not authored on either screen (`AreaController area="RaceBox"`
names it and the executable draws it); the frames read `RACEBOX`, the string of the Main Menu's `FE_RACEBOX` row, so that id is **chosen, not measured**. The footer is the same
pin the campaign pages take (`--menu-ticker`, the stored tag, the Back prompt where the screen has a way back: `Main Menu` has none, `backward="none"`). The focused row's colour pulse is a
clock a still does not have: the pair script reads its level off the frame (`focus_level`: the brightest red in the focused row's ink against the row below's) and ours pins to it
(`--menu-focus-level`, `Skin::pin_pulse_level`); Options went 1.87 % to 0.55 % with it on the same grab (the focused `GAME OPTIONS` was the original's white and ours cyan). The backdrop movie's frame is
fitted per pair (`compare-menu-movie-phase.py`: Main Menu needs frame 42, 4.75 to 0.45 grey over the bare pixels).

**Largest regions left** (px over 40 grey): Main Menu 1,265: the typed help line and the focused row's glow 814, the footer ticker 324 and prompt rings 110. Racebox 571 and Options 580: the
footer ticker 322 to 345 (glyph edges of the scrolling tip) and the prompt rings 225 (`Confirm`/`Back`, the half-texel softness `pulse-psp-cell` already measured). **Single Player was the worst of the four
list screens at 1.71 % (2,238 px): the focused value `SINGLE RACE` 910 (the original blooms it: a haze round the glyphs; ours stamped no glow on a settings page, the code's "unmeasured"), the step arrows 66 on row 1 and 277 on rows 2 to 4, KILLS 128, the `Help` prompt (a triangle and `Help` at (20, 252): the screen's own `Text`s, not drawn) 191.
Fixed: the focused value of a settings page stamps the glow (`rows.rs`, `skin.selected_stamp()`, the pulse's own ladder; the label and the other rows' values do not), 910 px to 35, whole frame 1.71 % to 1.04 %; it is also a live change to the RACE page's focused value on Pulse.**
Left: the original's step arrows are dimmed where the list is at its first entry (every row is at entry 0 here, so every left arrow is dark and only the right one lit); ours lights both, because
ours' lists wrap (the `race` page's design). KILLS is greyed on the original (ink peak 36 against ours' dimmed (114, 127, 142)) and holds `5` where the list's authored default is `10` (the
native screen's rule for a disabled row is not read), and ours greys it by a rule read off this one frame (`Eliminations` is inert while `Mode` is not `Elimination`). Measured, not fixed.

**Pause menu.** No still draws a menu over a parked race, so the route is `scripts/pulse-pause.drive` (`boot`, RACEBOX, the race page's `race.mode=time_trial`, Track Select `03_Track`,
Ship Select, `@TICKS@` race ticks, START, 120 ticks) with `--original-menu`; `psp-menu-pair.py pairs --title pulse --plan "$PULSE_PAUSE_PLAN" --ids pause --pause-clock C`. The original is
`data/saves/pulse-psp/time-trial-moa-therma-white-venom-grid.ppst` loaded (`psp-state.py`, the serial default is Pulse USA's), 4 s, START, 4 s. The saved state reads the HUD clock 0.40.6 to 0.40.8 at the
pause (the CPU is held after the load; the wall waits do not move it), so `--pause-clock` is the `CURRENT` the frame shows, read by eye, and ours is played to it (`PAUSE_CLOCK_ZERO_TICKS["pulse"]` = 276:
ours read 0.35.4 at 2,400 ticks, 0.40.8 at 2,724); with that the record 1.17.1, `current`, the GO board and the camera match. Fixed: the HUD under the tint, the tag off the footer, GAME MANUAL
in a compare shot, and the header (taken settled, see `docs/ui/pause-menu.md`, "Pulse"). **The pause menu is the worst pair now (1.56 %).** **Largest regions left** (of 2,031): the rows 807 and the focused row 279: the original's rows are
brighter (peak (65, 204, 225) against ours' (49, 155, 173), the bloom of the front end's glow stamps), because ours' glow pass runs only on a cleared target (`Renderer::render`'s `glowing` needs
`LoadOp::Clear`) and a menu over a parked race draws with `Load`: **the cause, not fixed** (a renderer change); the music pill and `FENIX FUNK 5 (WIPEOUT EDIT)` 672, **since drawn** (2026-11-24: the pause
`MusicList`, `docs/ui/pause-menu.md`; on a fresh grab of the same state the whole frame went 1.56 % to 1.05 % and the
music row 674 px to 12 px); the `Confirm` ring 212. The focused row's pulse level is not pinned on the drive route (the session's own clock runs it).

### Pulse Cell Selection: the footer, the tag and the title's glow phase (2026-11-23, `pulse-psp-cell`)

Claim: the Cell Selection pair has a lower share of pixels differing by more than 40 grey, whole frame and in the region of each item fixed, on one fresh
`theirs.png` (480 x 272, ticker `TKR_TOURTIM`, head 87, the title glow byte 205, movie frame 42). Whole frame **2.14 % to 0.65 %** (main's binary against
this branch, same `theirs.png`, same movie frame; the committed pair on `main` reads 2.97 %, its frame being another grab):

| Region (x, y, w, h) | before | after | what fixed it |
| --- | ---: | ---: | --- |
| footer ticker box (76, 236, 392, 18) | 1127 px | 370 px | the campaign still drew its ticker with no tips, so `--menu-ticker` reached no pixel on Cell or Grid Selection: it now takes the picker footer (`menu_page::picker_footer`, the tips from the records and the pin) |
| footer tag box (12, 236, 64, 18) | 142 px | 5 px | the pair script reads the profile's tag (`profile + 0x457`, `psp_ticker.profile_tag`, `AAA`) and stores it as ours' `player.tag`; the campaign still tags its legend (`footer_tag`) |
| card title glow (260, 34, 150, 50) | 1065 px | 18 px | the title's `pulse="true"` ladder and the `Selector`'s are two counters in the original: the pair script reads the title's glow-mask byte over its glyph box (`220 - 15 k`, EDRAM, CPU stopped) and passes `--menu-title-seconds`, apart from `--menu-picker-seconds` (the Selector's phase) |
| everything above y 230 | 1100 px | 53 px | the title row above and the rest: the backdrop movie is drawn at frame 42 on both sides (`compare-menu-movie-phase.py`; at frame zero its bright streak at (440, 136) alone cost 693 px) |
| cursor hex (100, 70, 100, 100) | 0 px, mean abs 0.14 | same | the halo is the front end's bloom, drawn since `pulse-frontend-bloom`: nothing left to draw |
| footer prompt glyph rings (four boxes) | 419 px | 419 px | open: ours' rings are a half-texel softer than the original's |
| silver bar, row y 249 (the lower strip's first row) | 341 px | 341 px | open: theirs shows the background there, ours a 55 % blend of a grey; rows 250 and on match to 1 grey |

`scripts/psp-menu-pair.py`: `tick:480` runs before `shot:grid` and `shot:cell` (`TICKERED`), the same pin `team` had. The ticker's lag fit (0 or 1 frame) now means
something on these pages (it settled on 1 and on 0 on two grabs). `--menu-title-seconds` is read back from the byte at the grab: 0.275 on a grid grab (byte 145), 0.075 on the cell
grab (205). One cell grab found no ladder byte over the title box (the line `title glow: no ladder byte` is printed and ours keeps the Selector's phase for it); run again.
The title box is `TITLE_BOX`: Cell (262, 30, 420, 60), Grid (262, 88, 340, 116).

**Checked, not wired: Grid Selection's title** is pinned the same way and measured on one grab (title box 0 px over 40 grey); HD and Omega draw no pulse text (`frontend_glow` is `None`), so
`--menu-title-seconds` is Pulse's alone (`campaign_page` sets it only in the Pulse branch).

### Pulse Cell and Track Select: glyphs, pens and the name's glow (2026-11-23, `pulse-fe-leftovers`)

Claim: the Cell Selection and Track Select pairs have a lower share of pixels differing by more than 40 grey on one fresh `theirs.png`, item by item.
**True.** Each grab is one PPSSPP run (`:94`, USA, software renderer), ours drawn by main's binary and by this branch's from the same `pair.json`
(`compare-rerender-ours.py`'s `rerender()` on copies; the movie frame set by `compare-menu-movie-phase.py --min-gain 0.1`):

| pair, grab | main | this branch | what fixed it |
| --- | ---: | ---: | --- |
| Cell, grab 1 (`TKR_SRTIM`) | 0.39 % (505 px) | 0.04 % (56 px) | button glyphs from `Pulse_14`, the bearing pen |
| Cell, grab 2 (`TKR_TTTIM`) | 0.41 % (536 px) | 0.04 % (56 px) | the same |
| Track Select, grab 1 (`TKR_SLTIM`, photo mid-crossfade) | 2.38 % (3103 px) | 1.98 % (2581 px) | taken before the name's glow phase was read, so its haze is not pinned: name box 2370 px |
| Track Select, grab 2 | 0.54 % (705 px) | 0.14 % (179 px) | the same three, the name's glow phase pinned (name box 26 px) |
| Track Select, grab 3 | 2.32 % (3030 px) | 0.15 % (200 px) | name box 2304 px to 0 |

Items, in the order the earlier lanes ranked them:

1. **Cell prompt glyph rings (419 px, then 127):** closed, `docs/ui/menus-original.md`, "The footer's button glyphs are Pulse_14's" (confidence 90).
2. **Cell silver bar row y 249 (341 px):** already gone on main (the 13-row footer clip of `pulse-psp-track`): 0 px.
3. **Track Select photo box:** 0, 50, 49 and 50 px on the four grabs, 215 px once before; the crossfade of grab 1 matched ours' `--menu-picker-seconds`
   without a pin (the 49 px), so no slideshow clock was read; not worth a pin.
4. **Track Select heading pens:** closed, "A text's pen starts at its first glyph's ink" (confidence 80); the heading's 197 px is 0 in the region.
5. **The circuit name's haze:** closed. The panel's `pulse="true"` texts stamp the title's `220 - 15 k` glow ladder on a counter of their own
   (probed over the name box, 3,730 pixels stepping 15 each sample): `TITLE_BOX["track"]` (300, 30, 440, 80) reads it like the card title,
   `--menu-title-seconds` now also pins `Picker::text_seconds` on `track-select` (the `glow_stamp` of the panel's texts). Test:
   `pulse_menu_ticker_ground_truth::track_selects_circuit_name_takes_its_own_glow_phase`.
6. **Footer prompt icons 1 px left:** closed by the glyph face (the icons now sit at the original's pixel columns).

Reproduce a grab: `scripts/ppsspp-start.sh pulse-fe-leftovers 94 45094 <usa image>` with `PPSSPP_EXTRA_INI`, `source <scratch>/pp/env.sh`, then
`emu-run.py --bg ... -- uv run --with websocket-client --with pillow python3 scripts/psp-menu-pair.py pairs --title pulse --ids cell|track --plan "back:Main Menu,wait:1,key:cross,st:Grid Selection,wait:3,key:cross,st:Cell Selection,wait:3,tick:480,shot:cell"`
(Track Select: `key:down,key:cross,wait:2,st:Racebox,wait:2,key:cross,wait:2,st:Single Player,wait:2,key:cross,st:Track Creation,wait:3,tick:480,shot:track`), each about 45 s.

**Open, with the evidence.** (a) The Confirm and Back prompts read dimmer over their first ten rows in the original (likely the footer's lower strip drawn over them,
confidence 50, `docs/ui/menus-original.md` above): 56 px on both stills, the whole of Cell's remaining 56 px and a third of Track Select's. (b) Track Select's circuit map: the bright
arcs that run along the outline sit elsewhere (94 to 115 px); a clock of its own, not read. Pulse PS2's skin gets the pen law (inherited from PSP) and not the glyph face (no third slot).

### The backdrop movie's phase (2026-11-05, `pulse-track-select-streaks`)

Pulse's menus play a nine-second looping movie that never restarts, so a reference still catches
whatever frame the walk's length landed on, and a craft's fly-by is bright. `--menu-page` draws
frame zero unless `--menu-movie-frame N` says otherwise. To line a pair up:

```sh
python3 scripts/compare-menu-movie-phase.py <pair-dir>...   # records --menu-movie-frame in pair.json
```

Then re-render that pair with `scripts/compare-rerender-ours.py`. The script scores every frame of
the movie against `theirs.png` on the pixels of `ours.png` that are bare movie, and keeps frame zero
unless the best frame beats it by a grey level (`--min-gain`): a dark stretch names no frame. It
runs no emulator and reads the transcoded movie from `~/.cache/oag/movies/`. A re-capture of a pair
changes the phase, so run it again after one. `pulse-psp/menu-track` needs 38: mean grey 13.89 -> 4.46
(`docs/ui/selection-screens.md`, "The teal streaks behind the panel are the backdrop movie").

`compare-rerender-ours.py` now gives a menu or loading pair an empty `$OAG_DLC`, as the pair scripts
always have: without it a re-render counted the maintainer's DLC packs (Track Select `1 / 11`, not
`1 / 3`; Ship Select `1 / 12`, not `1 / 8`).

## Menu pairs: Wipeout Pulse (PS2) front-end screens (2026-10-29, `pulse-ps2-hex-align`)

`scripts/pcsx2-menu-pair.py pairs --slot S --out D` makes Grid Selection and Cell Selection pairs (`menu-grid`, `menu-cell`) from
PCSX2 savestates (`data/saves/pulse-ps2/menu/`), on `compare_menu.py` like `psp-menu-pair.py`. The state is copied into the
instance's own savestate folder as slot 1, loaded with the VM running, PCSX2's GS screenshot (1592x896, `AspectRatio 16:9`) taken and
resized ONCE (Lanczos) to the profile's 1280x720; ours is `oag-game --menu-page grid-select|cell-select` through `compare_menu.render_ours` (no DLC
pack mounted, `[dev] unlock_everything = false`, aspect `wide`). The screen is read by eye (the PS2's front-end state name has no recovered
address) and the script's docstring lists the key walk that makes a state; the selector pulses on its own clock, so compare
placement, not brightness.

`scripts/menu_hex_offsets.py THEIRS OURS [--search=N] [--ps2-grid] NAME:CX:CY:W:H ...` measures per-axis offsets of ours against the original on
patches by gradient cross-correlation (Gaussian, strongest edges, parabolic sub-pixel peak; self-tested on a known 3 px shift, and
`0.00 / -0.01` on the existing PSP Grid Selection pair). `--ps2-grid` takes patch centres in the 640x448 grid of a GS dump
(`scripts/pcsx2-gsdump.py`), the exact quads. The selected hex mixes the outline and the ring: measure the ring alone with a brightness mask
(ring pixels above 140 in the pair; the outline is dimmer).

**Display offset (2026-10-29, `pulse-ps2-pair-dy`; corrected 2026-11-24, `ps2-fe-layout`).** The PS2 menu pairs score ours with the game's `DISPLAY1/2.DY = 52` applied: `compare_env.DISPLAY_OFFSETS[("pulse-ps2", "menu")]` is **2.0 frame rows** (3.21 px at 1280x720, confidence 90): PCSX2's default `D0` is 50 and a synthetic bar frame replayed on two of the game's own dumps landed every bar exactly `DY - 50 = 2.000` rows low (scale 2.000, 0.000 at `DY = 50`; `scripts/pcsx2-gs-bar-probe.py`, `docs/ps2/aspect-ratio.md`, "Measured against PCSX2's frame"). The table said 2.3 until 2026-11-24: that was a fit to the pairs, and the 0.3 row over the display law is a constant half output pixel between ours' textured quads and PCSX2's replay of the game's, in both axes (`dx -0.5` below is the same thing), not display. `compare_menu.menu_pair` and `compare-rerender-ours.py` shift `ours.png` down by the table's offset after the render (bicubic, black above) and record `ours.display_offset` in `pair.json`; **a rerender reads the table, not the record stored in `pair.json`** (it used the stored 2.3 until 2026-11-24), so the three stored PS2 menu pairs move to 2.0 at their next rerender. Compare shots only: play stays centred. Two fresh captures of the three pairs scored over 40 grey at 2.3 against 2.0: Cell 2.86 / 2.70 % against 2.95 / 2.79 %, Grid 2.26 / 2.30 % against 2.41 / 2.45 %, Team 6.37 / 6.32 % against 6.40 / 6.34 % (run 1 / run 2): 0.02 to 0.15 points worse at 2.0 every time, inside the capture-to-capture noise at one offset (0.16, 0.04, 0.05). The hexes that read `dy +0.01` at 2.3 read about `-0.4 px` at 2.0, the same size as `dx`. Other platforms are untouched.

**Result (1280x720), cause settled 2026-10-29 (`pulse-ps2-frame-offset`).** Against PCSX2's frame ours' unselected hexes (3 on Grid Selection, 5
on Cell Selection) lay a frame-level `dx = +4.19 - 0.00728 x`, `dy = -3.68` (sd 0.04) px off, from **two separate causes**:

- **x was ours**: `Space::PS2` showed the 640-column grid at 480/272, a 0.74 % pillar (4 px a side) while the console's 16:9 output, and so
  PCSX2's screenshot, fills the raster edge to edge. `Space::PS2.display_aspect` is now `16/9`; the three Grid hexes read `dx -0.49 / -0.48 /
  -0.50` and the three Cell hexes `-0.45 / -0.47 / -0.26` (was `+2.08 / +1.56 / +0.93` and `+2.90 / +2.24 / +1.90`), a uniform
  `-0.5 px` left, inside the half pixel.
- **y is the original's, and ours does not apply it**: the game's own `DISPLAY1/2.DY = 52` puts the picture below the frame's centre.
  PCSX2 honours `DY`: copies of the Grid dump with `DY` rewritten to 40, 50, 51, 52, 53 and replayed (`pcsx2-qt -batch -nogui -- dump.gs`) move
  the GS screenshot by `+24, +4, +2, 0, -2` screenshot rows (2 rows per `DY`, one frame row per unit); `DX` (640 to 600) and
  `DBY` (1 to 0) change nothing. The draws themselves are at the XML's coordinates (title bar quad at frame row 0, `XYOFFSET_1` the window
  centre). So `dy` stays `-3.6 to -3.7` px (2.27 frame rows; `DY - 50` would be 2.0) on all eight hexes, the rules, the footer and the
  Main Menu rows: that is the console's own display position, which ours does not model (no display offset in `Space` or the viewport).
  Confidence 90 that it is `DY - 50` (2026-11-24: exactly 2.000 rows on a synthetic frame); the rest of the 2.27 rows, 0.46 output px at every size and upscale, is a sampling difference of textured quads, not display (`docs/ps2/aspect-ratio.md`).

`menu_hex_offsets.py` therefore reads **`dy -3.2` (3.21 px at 720p) as the console's display offset, not a layout error**, and the 0.45 px left over in both axes (`dx -0.5`) as the pair's
residual. (The decoded frame buffer cannot arbitrate: a dump's VRAM is the state before its packets, so FBP 0 and 140 hold the previous
frame. `scripts/gs_fb_decode.py` decodes PSMCT32 from it, VRAM at byte 425 of the state; `scripts/gs_patch_display.py` rewrites the display
registers of a dump for the replay above.) The selected ring: before the selector fix `+15.4 / +8.2` (grid) and `+13.9 / +7.7` (cell) px, 12 px
and 11 px of that the PSP's `(-5, -7)` used on the PS2's `(-11, -14)` (`docs/ui/campaign-screens.md`, "PS2: the selector sits at...").

**PS2 menu pairs: the footer ticker and the tag are read from PCSX2 RAM (2026-11-23, `ps2-ship-select`).** `scripts/pcsx2-menu-pair.py` now does for the PS2 what
`psp-menu-pair.py`'s `tick:` step does for the PSP (`scripts/ps2_ticker.py`), and the Ship Select pair carries the original's footer.

- **The text record, found by signature** (confidence 80: the same two addresses after the walk and after two loads of the saved state; a heap object, so the script scans EE RAM rather than trusting an address).
  The PS2 ticker widget's own fields were **not recovered**: the row count 121 lies 0x34 before the first record where the PSP's `+0xc8` lies 0x40 before it, so the PSP's offsets do not carry over. The record layout is the
  PSP's: `+0` active, `+4` head x (float; a message enters at the viewport width and moves left 1 a frame), `+8` the viewport width **493.0** (`0x43f68000`, the PS2 footer
  ticker's width, `Selection_Definition.xml`), `+0xc` minus the text width, `+0x10` the NUL-terminated message. Two records, `0x114` apart (`0x00395db0` and `0x00395ec4` in the
  v2.9.168 Ship Select state); one of them holds the message on screen, the other waits inactive at head 492 with the next message until the first is mostly gone, so a frame shows one
  message or two. The signature is the `0x43f68000` word at `+8` with a negative width at `+0xc`, printable text at `+0x10` and a second such
  record `0x114` away; about 70 other 493.0 floats elsewhere in RAM fail it.
- **The item comes from the text.** The message is a tip (`TKR_NOSR`, `NOSL`, `NOTT`, `NOTOURN`, `NOELIM`, `NOZONE`: what a time item says with nothing counted), looked up in
  the disc's English `entries.xml` (`oag-wad cat --expand <image>:54748/WADS2.WAD Data\Plugins\PI000\entries.xml`) and named by the item that says it
  (`oag_ui_screens::news::untried_tip`'s pairs). Any other text names no item and the frame is not grabbed (the PSP reader refuses an item ours does not count the same way).
  Ours is drawn with `--menu-ticker ITEM@X`, the head in the record's own units.
- **A paused VM cannot be grabbed**: PCSX2's GS screenshot fails while the VM is paused, even after a frame advance, so the screenshot runs against the running VM and is bracketed by
  two reads of the record (about a millisecond each): the grab waits for a frame with exactly one message on screen at least 40 units inside the viewport, reads, presses F8, reads
  again when the file appears, and refuses the pair when the same message is not alone at both reads. The bracket was 6 to 10 frames wide (124 to 118, 124 to 114). Ours is then
  rendered at every third head in the bracket and at the neighbours of the best, scoring the ticker box (210, 628, 1030 x 40 at 1280x720, text rows only) against the original's;
  the head is **fitted within the bracket** (117; the box grey is 14.4 there and 19.0 / 22.7 one unit either side). `original.footer_ticker` carries the item, text, both
  addresses, both reads and the fit table. The text sits at the record's head plus the PSP's 2 px origin (inherited from PSP, not measured on the PS2; the fit absorbs it).
  The state carries `rand()`'s state, so a reload draws the same message: two grabs of the Ship Select state both caught `TKR_ZTIM`.
- **The tag is the profile's, read from RAM.** The profile record keeps the name as an 10-byte space-padded field and its NUL with the 3-character tag straight after it (`AAAA      \0AAA\0`
  at `0x00d9463c`, the tag at `0x00d94647`; the one such record in 32 MiB). The state the walk started from, `main-menu-pcsx2-2.9.168.p2s`, reads name `AAAA` and tag `AAA` there. The pair script writes `[player] name` and `tag` into ours'
  profile (`compare_menu.extra_settings(player=)`, recorded in `ours.player` so `compare-rerender-ours.py` puts them back) and the footer's tag box, already drawn
  (`NavigationLegend::set_tag`), shows `AAA`. A re-render of the new pair on a copy is byte-identical to the pair script's own.
- **The old Ship Select state does not load.** `data/saves/pulse-ps2/menu/team-selection.p2s` was made by PCSX2 v2.9.138, which v2.9.168 refuses; the grid and cell states are the
  same. The Ship Select state was walked again from `main-menu-pcsx2-2.9.168.p2s` (cross, cross, cross: Grid Selection, Cell Selection, Ship Select) and is to be kept as `data/saves/pulse-ps2/menu/team-selection.p2s`, the old file renamed `team-selection-pcsx2-2.9.138.p2s` (it is unloadable).
  `menu-grid` and `menu-cell` need the same walk before they can be re-made.

| `pulse-ps2/menu-team`, same original frame | RMSE | edge | mean grey (magick) | >40 grey, whole frame | >40 grey, footer box (30, 626, 1220 x 44) |
| --- | ---: | ---: | ---: | ---: | ---: |
| before: ours with an empty ticker and no tag (this pair's `ours.png` re-rendered with the pin and the player removed) | 0.1358 | 0.279 | 5.92 | 9.19 % | 17.15 % |
| after: `TKR_ZTIM@117`, tag `AAA` | 0.1294 | 0.301 | 5.61 | 8.70 % | **8.80 %** |

(Both rows are scored against the one `theirs.png`, the `TKR_ZTIM` grab; an earlier grab of the same state, which caught the Single Race tip, scored 0.1374 / 18.01 % for the empty footer. `>40 grey` is the share of pixels whose grey differs by more than 40 of 255; the box is the tag box and the ticker bar, measured on `theirs.png`. The whole-frame remainder is the
backdrop movie's streaks and the hull's turntable phase, not this change; the footer box's remainder is the silver bar's gradient and the glyph antialiasing, which a one-pixel
shift costs 25 to 70 grey on.)

**`menu-grid` and `menu-cell` re-made on PCSX2 2.9.168, with the footer and with translucent fills drawn once (2026-11-23, `ps2-grid-cell`).**

- **States.** `grid-selection.p2s` and `cell-selection.p2s` were walked again from `main-menu-pcsx2-2.9.168.p2s` (cross, then cross again: Grid Selection, then Cell Selection on Single Race, Moa Therma White, AI difficulty Medium, read off the footer) and are to be kept under their old names, the 2.9.138 files renamed `*-pcsx2-2.9.138.p2s` (they do not load).
  The two states carry two more strings of the profile record's shape (`...nceBetween\0DOF\0`, `3838383838\0836\0`), so `ps2_ticker.profile_name_tag` refused them as "not one record"; the name field now has to start after a non-identifier byte.
  The record the state holds is still the one at `0x00d9463c`.
- **Ours drew no footer on these two pages.** `--menu-ticker` and the stored tag were applied to the picker pages' legend and ticker (`capture.rs`), and the campaign pages build their own from the screen file (`Campaign::nav_legend`, `Campaign::ticker`) and drew
  an empty bar and a blank tag box whatever was passed (the head fit's table read 28.618 at every head). `campaign_page` now sets the tag on its legend, pins its ticker, measures through the `Default` face and returns the ticker's clip; with no pin it draws the capture seed's first tip, as the picker pages do (the `before` rows below were made
  with the pin and tag removed on the first version of this wiring, an empty bar). `pulse-psp-cell` wired the same footer for the PSP in parallel; the two were merged into this one (`Footer`), and the PSP cell pair re-renders byte-identical.
- **`--menu-picker-seconds` on the PS2 pairs (2026-11-24, `ps2-pulse-clock`).** The PS2 pairs were drawn at clock 0 while the states kept their own phase (the sweep below). `scripts/ps2_pulse.py` finds the `Selector`'s controller (phase at `+0xa0`) and the card title's `Text` (ladder byte at `+0xed`) in EE RAM; `pcsx2-menu-pair.py` reads both on each side of the screenshot, places them at the fitted ticker head's frame and passes the phase (`original.seconds_since_entry`); laws and live reads in [`frontend-pulse.md`](../ghidra/functions/ps2-pulse-eu/frontend-pulse.md). `Selector` box over 40 grey, ours with and without the flag on the same `theirs.png`, five grabs: Grid 15.5/20.0/12.1 % to 6.5/4.8/7.2 %, Cell 19.2/23.4 % to 5.0/16.7 % (the second at the white end, where the hex's shape and halo remain). **The title's ladder is recorded (`original.title_glow`) and not passed**: it runs in RAM, but the original's title box is pixel-identical across grabs whose ladder read 75 to 135, and a sweep of ours' 11 steps fits the dimmest whatever the RAM holds. The PS2 title shows no pulse, so ours' halo on `SINGLE RACE` and `GRID 1` was ours' alone: closed 2026-11-24 (`ps2-title-no-pulse`), the PS2 skin stamps no title glow (`MenuSkin::pulse_text_glow`, `docs/ui/campaign-screens.md`, "The card title on Pulse PS2 has no halo"); title box over 40 grey Grid 13.3 % to 4.9 %, Cell 11.0 % to 10.1 %, whole frame 2.14 % to 2.03 % and 2.43 % to 2.41 %. The `Selector`'s halo shape is closed too (`ps2-selector-halo`): the PS2 front end blooms through the PS2 chain over the `Selector`'s texel alpha (`MenuSkin::glow_by_texel`); `Selector` box over 40 grey, two grabs each, Cell 24.1/5.7 % to 5.3/1.8 %, Grid 5.2/9.2 % to 1.6/3.1 % (`docs/ui/campaign-screens.md`, "The selector's halo on Pulse PS2 ...").
- **`main` and `options` on the PS2 (2026-11-24, `ps2-fe-glow-pages`).** `scripts/pcsx2-menu-pair.py`'s `WALK` also lists the Main Menu (`pulse-main`, state `main-menu.p2s`) and Options (`pulse-options`, `options.p2s`), both made by walking from Grid Selection (triangle) and from the Main Menu (five downs, cross) and saving a state on the current PCSX2; the older `main-menu.p2s` of the 2.9.138 batch no longer loads (a version mismatch). Pulse PS2's front end blooms nothing there (`docs/ui/campaign-screens.md`, "The Selector's sprite is the only alpha writer of Pulse PS2's front end"): main rows box 7.3 to 4.1, options 3.4/2.6 to 2.3/1.2 mean grey.

| `pulse-ps2` pair, one `theirs.png` per row group | RMSE | >40 grey, whole frame | >40 grey, footer box (30, 626, 1220 x 44) |
| --- | ---: | ---: | ---: |
| grid, no pin, no tag (final renderer) | 0.0740 | 3.29 % | 17.31 % |
| **grid, `TKR_ZTIM@117`, tag `AAA`** | **0.0616** | **2.80 %** | **8.96 %** |
| cell, no pin, no tag (final renderer) | 0.0797 | 3.46 % | 17.21 % |
| **cell, `TKR_SRTIM@116`, tag `AAA`** | **0.0682** | **2.96 %** | **8.60 %** |
| grid, earlier grab, before the fill fix: no pin / pinned | 0.0783 / 0.0666 | 4.61 % / 4.12 % | 17.24 % / 8.87 % |
| cell, earlier grab, before the fill fix: no pin / pinned | 0.0870 / 0.0765 | 5.24 % / 4.73 % | 17.28 % / 8.64 % |

(The stale pairs read RMSE 0.077 and 0.095. The head fits have a clear minimum now: 14.5 and 14.2 mean grey at the fitted head against 19.0 one unit away. The message is the original's `rand()` draw, so each grab catches a different tip.)

- **The largest region was ours' translucent fills, drawn twice.** On the cell pair the stat-row bars and the panel behind them were 31 % of the pixels over 40 grey, on the grid pair the rows and panel were about half of them. Ours' bar read 110 at its peak where the original's reads 62 (`0x3f` is alpha 0.247, so 63 is one draw), and
  the ratio over the whole bar is `2a - a^2`: 0.43 for 0.247. The cause is not the data: `ui.wesl` draws an *outlined* atlas's text in two passes (border alpha, then body alpha), `Pulse PS2`'s menu atlas is outlined, and a fill samples the same atlas's solid patch with a border colour equal to its own, so
  the second pass drew it on itself. Fills (`Draw::Fill`, `GradientFill`, `ChamferedFill`, solid triangles) now carry a border alpha of zero. This is the renderer's, not PS2's: any fill under an outlined main atlas changed (a PSP pair at 480x272 never went through it; its menu atlas is not outlined). Whole-frame
  >40 grey on the one `theirs.png`: grid 4.12 % to 2.54 %, cell 4.73 % to 3.13 % (`render::tests::a_translucent_fill_under_an_outlined_font_is_drawn_once` reads 112 where 64 is one draw).
- **Fill fix across every front-end pair it can reach** (33 menu pairs of `menus-pulse`, `menus-pulse-ps2`, `menus-pure`, `menus` (HD, Omega) and `menus-2048`, each re-rendered on a copy with the branch binary with and without the fix, same `theirs.png`, whole-frame >40 grey): only the three PS2 pairs move (grid 4.25 to 2.54 %, cell 6.54 to 4.98 %, team 8.70 to 6.78 %, the committed pairs without the pin); the five PSP, eight Pure, five HD, five Omega and seven 2048 pairs read to the last digit as before, none worse. The 42 race pairs of `race-shots` (talons-alongside, talons-boost), `pulse-ps2-race` (15 circuits) and `fresh` (25 Pulse PSP grid pairs), re-rendered the same way, are byte-for-byte the same score before and after (0.3 to 41.4 % over 40 grey): none of them draws a translucent fill under an outlined atlas. Which `.fnt` an atlas is built from decides `is_outlined` (`fnt::Font::is_outlined`: more than one distinct grey in the palette; a texture pack's picture the same way); HD, Omega and 2048 draw their `Default` and `Menu` role fonts through the same `Atlas::from_font`, and their 17 menu pairs score the same with and without the fix, so none is outlined where a translucent fill is drawn (their palettes were not read one by one).
- **What is left, cell (2.96 % over 40 grey), ranked by pixels over 40 grey** (regions found by labelling the dilated mask, `theirs.png` against `ours.png`): the footer box 34 % (silver bar gradient and glyph antialiasing, as on Ship Select); the `SINGLE RACE` title 28 %
  (ours has a white halo, the original none); the `RACE CAMPAIGN` header 17 % (the original's is a bold dark face, ours a thinner, wider one: ours' PS2 menu `Text` draws through the outlined atlas, not measured further); the `Points`/`Best` area 8 % (the backdrop movie's streak crossing the original's frame); the selector hex 5 %.
  **Phase sweep (measured on this pair, not wired):** ours rendered at `--menu-picker-seconds` 0 to 1 in 0.05 steps against the one `theirs.png`: the whole-frame >40 grey is 2.93 % at 0, **2.57 % at 0.2 and 0.8**, 2.94 % at 0.5 (the pulse is a triangle of period 1 s, symmetric about 0.5), but the title's box mean grey falls from 23.2 at 0 to 13.1 at 0.5 and the
  selector's box rises from 3.6 to 9.8 over the same range: the two widgets want different phases, so one fitted phase buys about 0.35 points and not the title's halo. The two phases are read from RAM since 2026-11-24 (the bullet above).

## Pulse PSP pairs over 5 mean grey: where the grey sits (2026-11-19, `pulse-psp-pairs`)

Three pairs re-rendered at main `e51ae21d2` reproduce their recorded numbers exactly (`basilico-black` 5.08, `menu-team` 5.89, `pulse-psp-zone-talons-white-z8` 8.19).
No fix landed; this is what was measured, so the next lane starts from a ranked list. Method: copy the pair, `compare-rerender-ours.rerender` on the copy, then paste
the original's pixels over one region of ours and recompute `mean_grey` with the pair's own formula (`magick ... -colorspace Gray -compose difference`); the drop is the region's ceiling.

| Pair | Region (480 x 272 px) | mean grey with the region taken from the original | Cause |
| --- | --- | --- | --- |
| `basilico-black` 5.075 | start-line laser band, rows 146 to 156 | 4.03 | see below |
| | craft box (185, 150, 300, 250) | 3.23 | pose and livery, not isolated |
| | bottom HUD (rows 200 to 272) | 4.32 | clock and best text |
| `menu-team` 5.89 | footer (rows 205 to 272) | 3.18 | the original's ticker message and tag text; **closed for the message, see below** |
| `pulse-psp-zone-talons-white-z8` 8.19 | bottom HUD, left half | 7.75 | ours' HUD reads faded beside the original's solid one; speed bar and plume differ; not isolated |

**`menu-team`: the footer is the original's random ticker draw, now read and wired (2026-11-19, `pulse-menu-team`).** The original picks the next message by a weighted
`rand()` draw among the `PI_News` items that have data (`Ticker_PickMessage`, `docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`, "The footer ticker's
message choice"); the old frame caught "Your most listened to music track is Fenix Funk 5 (Wipeout Edit)" and ours rotated a tournament tip. Ours now draws the
same way: `oag_ui_screens::news` parses the disc's `Data\Plugins\news\Definition.xml`, draws with the original's weights and no-repeat rule through the seeded
`oag_core::Rng`, and quotes lifetime statistics `oag_game::records` keeps in the records file (`[[lifetime]]`, per title: laps and time per mode off each recorded race,
menu time, the race songs heard per track; `Lifetime::favourite` is the most-played, as `Profile_PickFavouriteMusic` is). A mode never raced still says its `TKR_NO*`
tip, because the six time items fall back to it at 0. **Not counted, so never shown:** team and circuit statistics, weapons, photos, speed pads, the unlock
announcements (`TKR_FYI*`); the original's increment sites are not read either.

The message at a grab cannot be chosen (`rand()` on the emulator's clock), so the grab picks the frame: `scripts/psp-menu-pair.py` with the plan step `tick:N` polls the
ticker widget out of PPSSPP RAM (`scripts/psp_ticker.py`; the widget is the one object with 124 rows at `+0xc8` and weight 122 at `+0x104`) with the CPU stopped, remembers
which item each text record holds (`+0xcc` is the item drawn *last*, the message waiting in the idle record), and stops on a frame with exactly one message on screen whose
item ours counts. It grabs that frame in the same stop and records the item, head x, both records, the widget address and the statistics table in
`original.footer_ticker`. Ours is drawn with `--menu-ticker ITEM@X` (the head x as the original's record has it: a message enters at the ticker's width 370 and moves left 1 px
a frame) and a `[[lifetime]]` row written from the table. Two facts the first grabs gave: the text sits **2 px right of the record's head** (`TICKER_TEXT_ORIGIN`, confidence 70:
two grabs, heads 89 and 90, matched at +2 and +3), and the presented picture **lags the CPU stop by 0 or 1 frame**, which the script settles per grab by rendering ours at the
head and at the head plus 1 and keeping the better ticker box (`original.footer_ticker.lag_frames`).

| Frame (theirs) | Ours | mean grey (whole 480 x 272) | footer rows 205 to 272 | ticker box (85, 235, 370 x 32) |
| --- | --- | ---: | ---: | ---: |
| 2026-10-29, "Your most listened ..." | before: tournament tip | 5.90 | 11.01 | 23.19 |
| | after: `TKR_FAVMUS` at x 49, 6 plays of the song seeded (**fitted**: the old grab has no RAM read) | 5.32 | 8.66 | 16.81 |
| 2026-11-19 grab A, `TKR_SLTIM` ("Why don't you try out the Speed Lap events?") | before: main's binary | 4.79 | 10.81 | 23.73 |
| | after: pinned at the head read (89) | **3.12** | **4.06** | **5.39** |
| | paste test: theirs' footer over before | 2.13 | 0.17 | 0.20 |
| 2026-11-19 grab B, `TKR_TTTIM` | before | 5.84 | 10.76 | 23.64 |
| | after: pinned at the head read plus 1 frame (91) | 4.21 | 4.17 | 5.74 |
| 2026-11-19 grab C, `TKR_TTTIM`, the final tooling | before | 11.48 | 13.30 | 23.52 |
| | after | 9.91 | 6.95 | 6.26 |

Grab C's whole-frame grey is high on both sides because the menu backdrop movie's phase differs (a bright streak in the original that ours' frame zero does not draw): that
is not the footer. The ticker box keeps 5.4 to 6.3 after the message and the position match: the silver bar's gradient and glyph antialiasing (a shift of one pixel in any
direction costs 25 to 70). **The left tag box is the profile's footer tag** (`profile + 0x457`, `AAA` on a fresh profile, read by `Ticker_Update`'s tag branch) and is blank in
ours: with theirs' tag box pasted on grab C the footer goes 6.95 to 4.90. The player tag is a planned feature (one shared name, a 3-character tag on Pulse and Pure); open, not built. (The compare route stores the original's tag as `player.tag` in ours' profile, which `menu-grid` and `menu-cell` read from RAM, see below; `menu-team` still shows a blank tag box.)
**PS2** ships the same code (`NewsItemBarText1`, `PI_News` and `NewsFrequency` are in `SCES_547.48`) and the same list less three rows (121; `pulses_news_items_are_the_discs_list`),
and ours draws it on the PS2 ship select at the PS2 viewport (113, 392, 493 x 53), inherited from PSP. **Pure** ships no ticker: its `BOOT.BIN` on both pressings has no
`TKR_`, `NewsItemBarText` or `PI_News` string and its `Data.wad` no `news` plugin; its `TextInfoIsAlwaysLast` is the description box.
**The other Pulse and Pure menu pairs, re-rendered on copies with this branch's binary** (recorded to re-rendered mean grey): `menu-track` 4.45 to 3.95, `menu-team` (the old frame)
5.89 to 5.61, PS2 `menu-team` 5.65 to 5.55; PSP `menu-cell` 4.62 to 4.64, `menu-grid` 3.59 to 3.60, `menu-title` 4.53, PS2 `menu-cell` 4.04 and `menu-grid` 2.84, and all eight Pure pairs
move by at most 0.01. The three that move are the pickers' footers: a still with no `--menu-ticker` now draws the ticker empty (a message enters at the right edge, so none is in view at
clock 0) where it drew the tournament tip at the left edge. The original's frame always holds a message, drawn at random, so the blank is closer than a wrong tip, not a match; a pair
that wants the message uses the `tick:` plan step. The live ticker is unaffected (its clock runs).

Grab A is the one to keep as the pair (ours re-rendered by `compare-rerender-ours.rerender` on a copy: 4.06 to 3.12). A pair whose ticker quotes a seeded statistic
(`TKR_FAVMUS`, `TKR_SONGS`) needs the `[[lifetime]]` row `pair.json` does not carry, so those re-render only through the pair script.

**`basilico-black`: the laser band is worth 1.05 grey, and the original's laser core is about 1.5 times brighter than ours before bloom.**
Measured on a Talon's Junction grid in the original (PPSSPP 1.20.4, `psp-ge-dump.py`, bloom removed by `psp-ge-replay.py --nop-after` the first prim drawn into
`0x110000`) because the Basilico states are not on this host:

- the laser draws (texture `startline_laser_ADD_GLOW`, 64 x 8, `CLUT8` 8888, texel alpha `0xaf` everywhere; vertices carry 4444 colours fading `ffed` to `ffa7`
  with alpha `f`; `lasersmoke` 128 x 16 with vertex alpha 0 to `f`) run `BLENDMODE 0xa2` (`SrcAlpha` / `Fix`, `BLENDFIXB 0xffffff`), `TFUNC 0x100` (the init block; the
  stream never sets it), stencil ref `0xaf`, depth test `GREATER`, no mipmap levels (`TMODE 1`) for the laser, one for the smoke;
- before the bloom the original's core pair of rows reads (205, 251, 255) over a (115, 150, 165) background: an excess of about 95 in green;
- ours at Basilico with the bloom skipped (temporary switch, not committed) reads (99, 101, 100) over (36, 37, 33): an excess of about 64, grey where the original is white-blue;
- ours' laser draw alone owns that core (skipping `startline_laser_ADD` returns the background; skipping `lasersmoke` only lowers the wide blue halo);
- `--texture-detail high|maximum` does not move a pixel, so the mip chain is not it; the same asset is right on the Talon's pair (`talons-black` 3.22, laser matched by eye).
  So the difference is specific to how this strip is seen at Basilico (the camera is 4.5 units above the ribbon, nearly edge on): ours spreads the core over four rows
  (137, 136, 112) where the original has two sharp rows (241, 251) and 88 to 141 at the shoulders.

Not established: the factor behind the ratio (a constant of about 0.686 squared against 0.686 fits the numbers, but the two sides are different circuits' files and the
test that would separate a doubled alpha from a different geometry was not run). Needed: a Basilico grid in the original (`just sync-saves <host>` brings
`time-trial-basilico-black-venom-grid.ppst`, absent from this host), one `psp-ge-dump.py dump` at the pair's stop, `census` rows for the draws with `texsize0` `0x306`,
and ours' `OAG_DRAW_DUMP` rows for the same texture; compare the strip's vertex positions and colours. A Talon's Junction grid state made on this host is
`data/saves/pulse-psp/time-trial-talons-venom-grid-lane-pairs.ppst` (not a repository file).

**`basilico-black`: the laser's difference is its lighting, named and fixed (2026-11-19, `pulse-basilico`; the pair is `pulse-psp/basilico-black`, not `pulse-psp-race/`).**
The grid state is on this host, so the "Needed" above was done: the state run on to the pair's clock (15.4487 s, bit for bit), one `psp-ge-dump.py dump` per boot (two
boots, the same lighting words in both), the strip's census row (`texsize0` `0x306`, chunk 4492) and ours' `OAG_DRAW_DUMP` rows for the same texture. **The strip's
geometry, UVs and texture agree; its colour does not.** The original's strip has no vertex colour and is drawn under a lighting list replayed just before it (the
circuit's rig, which sums to white on this strip); ours lit it with the stand-in at about 0.56 of the texel. Cause, confidence 85, with the register values and the fix in
`docs/ghidra/functions/psp-pulse-usa/scene-light.md`, "The start laser is lit by the rig too". Replaying the dump with the bloom dropped
(`psp-ge-replay.py --nop-after 4903`) and with the strip's PRIM also dropped, the strip's own pre-bloom contribution at `x` 190 reads (35, 24, 44), (133, 123, 140),
(150, 144, 154), (58, 45, 68), (6, 4, 7) in rows 149 to 153, peak 0.86 of 0.686 x 255: two near-full rows and shoulders, which is the texture's white rows 4 and 5 and its 0.45 rows
3 and 6. After the fix ours' core rows 149 to 152 are 255 in all three channels where the original's are (231, 228, 255), 255, 255, (246, 239, 255). Re-rendered on copies
(`rerender` on copies in scratch, binary of this branch; the 29 Pulse PSP scenery, Zone and race pairs): `basilico-black` 5.08 to 4.38, `basilico-white` 4.30 to 3.50, the laser
band's mean absolute error 18.6 to 9.0, every other pair within 0.01. Not reached: the region-paste ceiling 4.03 (ours is 4.38); the rest of the band is the purple tint to the strip's left, which the original's
palette carries, and the bloom's halo.

**Reaching Basilico on a fresh PPSSPP profile did not work in bounded effort.** `psp-drive.py menu --unlock` sets the dev-unlock byte but the first attempt then lands in
`MPMainMenu`; a second plain `menu` runs. `--track-down N` is relative to the profile's last selection, not to Talon's Junction (`--track-down 1` landed at
(-404.2, 4.0, -11.6), `--track-down 8` after that at Metropia White), so the circuit is chosen by reading the start position back.

## A Pulse PSP pair with both air brakes out (2026-11-19, `airbrake-pair`)

`pulse-psp-race/talons-uturn-airbrakes`: Talon's Junction (`16_Track`), Time Trial, Venom, through the circuit's flat U-turn with
both air brakes held, the original grabbed in PPSSPP and ours rendered at the recorded pose with the recorded air brake levels.
The old plan (pose the flaps on `talons-alongside`) needed a save state that is not on every workstation, so this is a new pair on a
circuit with a U-turn.

- **The circuit.** `oag-trace track` on `16_Track`: path 1 (rows 1752 to 3448, the second half of the lap) turns 88 degrees over 40
  rows (59 units) at rows 2650 to 2690, flat (`down_y` -0.97 at `y` -51). Path 0's sharper-looking rows near 1210 are the circuit's
  vertical loop, where a heading in the XZ plane means nothing. Confidence 80 (spline read, not walked).
- **The values.** `airbrake_l` and `airbrake_r` of `psp-trace.py` are `craft+0x2c4` and `craft+0x2c8`, the **ramped** level on `0..100` (a
  six-tick trace with both brakes pressed read 0, 0, 0, 15.0, 30.0, 45.0 and 100 at the seventh; released it falls 8.4 a tick), the
  scale `Race::view.flaps` holds in ours. Confidence 85 (one capture family, behaviour matches `craft-flags.md`). The pair reads
  them at the pose's own stop (row 0) and refuses a pair where either is under 90: **left 100.00, right 100.00**.
- **Ours.** `--pose-flaps` (needs `--pose-from`, `crates/game/src/race_capture/pose_flaps.rs`) reads the same two columns of the
  pose row and `Race::force_flaps` puts them on ship 0's drawn flaps after the ticks (this tick and the previous one, so a draw at any
  alpha shows them). Without it a posed capture draws stowed flaps, since no tick swings them. Tests: `forced_flaps_draw_at_their_levels_at_every_alpha`
  (through `airbrake_flaps_of`) and `race_capture::pose_flaps::tests`.
- **Making the state.** Not a grid state, so `scripts/psp-pulse-pair.py` takes `--mid-lap --layout forward` (no layout probe, no
  start-line distance check; the pose check at the rendered tick is the proof) and `--air-brakes` (the check above, `--pose-flaps`,
  `--pose-lean` and the lean row check). The craft was placed on the spline at row 2610, 130 units/s, by `psp-drive.py place` (a RAM
  write, recorded with `--ram-write`), flown 45 ticks under `psp-trace.py --script` holding thrust, right and both shoulders, the pads
  then held through the debugger (`input.buttons.send`, the shoulders are `ltrigger` and `rtrigger`) so the pause-menu save stored
  100 and 100 (a save made with the pad released stored 0 and 0 and the pair refused it). A `drive` run killed mid-way wedged the
  debugger: let it finish, or restart the instance.
- **The camera.** The close chase sat 11.70 units from the craft (11.64 at rest), inside the pair's own view check; a
  `--mid-lap` pair also accepts 6 to 12.5 units as the close view (the spring lags in a turn), since ours renders the recorded
  camera and fov (`--camera-fov` 62.513 against RAM 62.560).
- **Repeats.** The same state grabbed three times: the frame of run 1 and run 3 are byte-identical, run 2 differs by under one pixel
  of one channel level (confidence 75, three grabs of one state; a second state made the same way would be the next check).
- **The flap region's grey.** `scripts/compare-flap-region.py` (box x 185 to 275, y 148 to 210 of the 480x272 frame, both fins in both
  images; HUD rectangles excluded from the rest): mean absolute grey difference **23.27 in the flap box against 23.06 in the rest
  of the scene** (ratio 1.01; whole frame 21.83). Ours with the flaps stowed (the same flags without `--pose-flaps`) reads 23.55 in the box
  against 23.06 (ratio 1.02): the flaps are as good a match as the hull and the circuit around them and the flag moves the box by
  0.28 grey, because the hull's own pose differs more than the flap does. **Not matched:** the original's craft is rolled further
  on the banked wall than ours (the pose check holds position and heading to 0.000 and the lean to 1e-5; the residual is in the
  hull's roll about its nose, not measured here), and the HUD (ours at rest reads 0 km/h and a full cyan shield bar where the
  original reads 165 km/h and a red one).
- **Re-make.** `scripts/psp-pulse-pair.py --port P --home $PP_HOME --out D --track 16_Track --state <ppst> --layout forward --mid-lap
  --air-brakes --autopilot off --ram-write "craft_pose=..." --meta <json>`, then `scripts/compare-flap-region.py D --box 185,148,275,210`.

### The same recipe on a pack (2026-11-20, `pair-upkeep`): `talons-alongside` with both air brakes out

`scripts/psp-pulse-pair.py --field --mid-lap --air-brakes` now passes ours `--pose-flaps` too (the field path built its flags without it,
so a `--field --air-brakes` pair drew ours stowed). A pack state with the flaps out is made from `race-talons-venom-alongside.ppst`:
load it into a running instance, write the player record's Autopilot timer (`record+0x148`) to 0 (a running Autopilot flies the craft
and ignores the pad: the first attempt, pad held with the Autopilot on, read `airbrake_l/r` 0.00 and 0.00 and was refused), hold thrust
and both shoulders through one debugger connection for 0.3 s, and make the pause-menu save while that connection still holds them (the
same save made after the script's connection closed was a bad state that never resumed). Then `psp-pulse-pair.py --port P --home $PP_HOME
--out D --track 16_Track --team Assegai --state <that ppst> --field --layout forward --mid-lap --air-brakes --autopilot off
--ram-write "autopilot_timer=..."`.

- **Result.** Pack frame (place 5 of 8, 24 parity checks, `compare_provenance.py scan --strict` 0 problems): left 100.00, right 100.00 at
  the pose's stop. Whole-frame mean grey difference **4.66** (RMSE 0.062, edge NCC 0.31); the same frame with ours stowed reads 4.81.
  Hull crops (4x): the original's lower flap fins are drawn on both of ours' with `--pose-flaps` and missing with it off.
- **Repeat.** Two grabs from two instance boots: `theirs.png` RMSE 3.1 of 65535 (4.8e-5) apart, same clock bits (37.9077 s).
- **Re-captured (`alongside-airbrakes-inputs`, 2026-11-24)** from `data/saves/pulse-psp/alongside-airbrakes.ppst` with the stored
  `argv`'s flags (own PPSSPP, `psp-pulse-pair.py --field --layout forward --mid-lap --air-brakes --autopilot off --ram-write ...`),
  25 parity checks, two grabs (`--state` loaded twice through the pause menu; `trace.csv`, `field.csv`, `field.json` and `theirs.png`
  are byte-identical between them). Against the stored `theirs.png`: same race clock (37.90767), pose, camera, fov, shield (134.4671),
  air brakes 100.00 / 100.00, field and HUD record (`original.hud` unchanged); the frame differs by RMSE 8.1 of 65535 (1.2e-4), 0.09 %
  of pixels over 0.5 % and none over 2 % (largest channel step 3 of 255): no framing or pose change. Ours re-rendered from the copy
  of the pair directory: rmse 0.0618, edge NCC 0.309, mean grey 4.68 (stored 0.0617 / 0.310 / 4.66). The stored record's `meta.id`
  was the sibling's (`pulse-psp-race/talons-alongside`); the re-capture carries its own. `--field` now reads the shield bar's flash
  timer and blink (`hud+0x11c`, `+0x1dc`) into `original.hud` and passes the held pickup and flash flags like a scenery pair
  (`--hud-readout` alone did it before; the field block then overwrote `original.hud`). On this state the flash reads 0.0 and no held
  pickup, so ours' flags gain nothing.
- **Not matched.** (1) The original draws the green Autopilot-pickup hexagon top centre although the timer is 0; ours draws none
  (whole-frame grey 4.68, 3.82 with that 75x60 box masked). (2) The frame is the presented one (lag 1) and the brakes decay 8.4 a tick once the pad is
  released at the load: row 0 reads 100, the row the frame shows 91.6, and ours draws row 0's 100. (3) This is a new moment (race clock 37.91, HUD 27.99 s,
  359 km/h), not the stored pair's 38.41 s, so 4.66 is not like-for-like with that pair's 2.35.

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
| `ram_writes` | every memory write made before the grab (the state's making included), `ram_writes_complete`, and `pair_script_ram_writes` (what the pair script itself wrote for the grab: only `rpcs3-hd-menu-pair.py`'s hidden Fury backdrop, a row tint zeroed and put back; every other pair script only reads) |
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

**Where a pair's inputs live.** The files ours' recorded flags read (`--pose-from` the `trace.csv`, `--field-from` the
`field.csv`, `--input-script` a Detonator pair's `ours.inputs`; `field.json` beside them) are written by the pair script into
its `--out`, and that directory is what is copied whole into `data/reference/compare-page/<pair>/`, as in
`pulse-psp-race/talons-alongside`. `compare-rerender-ours.py` reads each of them from the pair directory by file name
(`compare_env.resolve_input_flags`, whatever path the flag records), and `check-pair-provenance` runs that same rule: an input
that is not in the pair's own directory fails the scan with `INPUT <pair>: <path>: no <file> in the pair directory`, which is
exactly when a re-render would skip the pair. A Detonator pair's `ours.inputs` and `field.csv` are made again from the record
(`ensure_detonator_inputs`), so they need not be stored. `argv` itself (`--out {scratch}/...`, `--state`) is not checked: it
names the capture's own scratch and the save state, neither of which a re-render reads. Copy `--out` whole, never `pair.json`
and `theirs.png` alone (`pulse-psp-race/talons-alongside-airbrakes` was adopted that way, with `--out {scratch}/pairs/a2`
wiped, and skipped on every re-render until it was captured again from its surviving state).

## Publishing

`just publish-compare` uploads the built `data/compare/` with `wrangler pages deploy`
to a **separate** Cloudflare Pages project (`openantigrav-compare`, never the main
site's `openantigrav`; the script refuses if the two names are ever equal). It runs
only from the workstation, never in CI, and nothing in `.github/` calls it.

It **refuses to run** unless the screenshot-policy ADR,
[ADR-0074](../architecture/adr/0074-curated-screenshots-may-be-published-no-content-may-be-shipped.md),
has a `## Status` line that **starts with** `Accepted` (markdown emphasis ignored). Its number is one
constant, `POLICY_ADR` in `scripts/compare-page.py`. A missing or non-Accepted ADR prints the reason and
exits 2. The lock is checked before `wrangler` is looked up, so the refusal is what you see.
`just publish-compare` rebuilds the page strictly from the curated manifest directory first, so a
missing `platform` refuses it before anything uploads. With no argument it publishes
`data/reference/curated/`, the maintainer's curated set; `just publish-compare <dir>` publishes another.
The default is never the seed or live manifests (`data/reference/compare-page/`): they are developer
material, and ADR-0074 wants every published image picked by a person. A missing
`data/reference/curated/` refuses by name.

**The curated set is manifests only, in the private repository** (2026-11-12). Its `*.toml` files sit in
`data/reference/curated/` and each `[page]` carries `images = "../compare-page"`, so its image paths
resolve into the reference pairs: both workstations pull one list, and no image is copied. Before this
the set was a 1.7 GB directory, `data/compare-curated/`, copied between hosts by hand, and the two
copies drifted (one host published pairs the other lacked). `[page] images` works in any manifest: a
path relative to the manifest's directory, `.` when absent.

The curated set picks the pairs and may reword them; `just compare-rerender-all` writes only into
`data/reference/compare-page/`. So with no argument, publishing first carries newer re-renders across.
For each curated entry whose id has a later `rendered_at` in the reference set, with the same `ours`
and `original` paths and a byte-identical original image, it copies `ours.png`, `side.png` and
`pair.json` and the entry's `rmse`, `edge_ncc`, `mean_grey`, `build`, `binary` and `rendered_at`
lines. Every other curated line (title, note, setting) stays as curated, and each carried entry is
printed. A directory named on the command line is published as it is.

**In one go: `just compare-publish`.** It pulls `data/reference` (the private repository,
[ADR-0085](../architecture/adr/0085-comparison-originals-live-in-a-private-repository.md)) fast-forward
only, re-renders our side of every pair at this tree (`compare-rerender-ours`), commits and pushes that
re-render there (`Comparison pairs: ours re-rendered at <sha>`), then runs `publish-compare`. It refuses
when `data/reference` has uncommitted changes, so one host's re-render is never mixed into another's;
commit or discard them first. Originals are untouched; a pair that needs a new original is still
`compare-rerender` or `compare-rerender-all`. `python3 scripts/compare-page.py --check-publish-lock` prints the verdict without publishing; with
ADR-0074 Accepted it reads `unlocked`. The maintainer runs the command.

A curated page should carry only frames the policy covers; the seed and live
manifests are developer material, not a publishing set.

## Detonator pairs (HD/Fury, 2026-11-22, `det-pairs`)

`hd/detonator-talons-{mines-ahead,cannon-volley,pad}` (manifest `compare-page/detonator.toml`, also `curated/`). Recipe:

1. Cold boot, Racebox, `RACE TYPE` six `right`s from Single Race (Arcade, Time Trial, Speed Lap, Eliminator, Zone, Zone Battle,
   **Detonator**: the Fury `DATA00` list, `head2head.md`), Talon's Junction (eight `right`s), Feisar, START RACE prompt; take the
   state with `emu-save-state.sh` (`data/saves/hd-fury/hd-detonator-talons-feisar-grid`, sidecar `mode: detonator`). The load log
   prints `GetMode()==14`.
2. Restore (`emu-restore-state.sh`, with `OAG_COMPARE_PLATFORM=hd-fury` exported so the profile turns RPCS3's pause toast off; the
   first restore after a run failed with a refused GDB proxy 3 of 5 times, one retry after a 10 s wait worked every time).
3. `scripts/rpcs3-hd-detonator-shots.py --shot T [--fire-clock F --fire-hold H]` waits 30 s (a START RACE tap before the prompt is
   held into the race and the craft leaves the track within 4 s), taps START RACE, then pauses at each lap-clock `T`, grabs through
   `grab_with_clock`, reads the pose and camera, and can hold the cannon button over a window. **The cannon appears to be TRIANGLE**
   (seen on one boot; SQUARE changed the camera view on four others and never fired). The craft does not steer itself: with no stick it holds the first straight for about
   6 s of lap clock.
4. `rpcs3-hd-pair.py --detonator --json NN.json --autopilot off --state <state> --team feisar --track 'Data\Environments\Talons_Junction\track.vex'`:
   ours runs its own countdown and clock in `--mode detonator` with an `--input-script` that holds Square (ours' cannon button) over
   the same lap-clock window, and `--pose-hold` plus `--pose-hold-ticks` keep ours' craft at rest on the original's pose after every
   tick (without the second flag ours flew 130-190 units past the pose before the frame: measured, and the first renders were redone);
   camera, fov, gantry and world clock are the original's. A held craft never meets a mine, its exhaust is cold and its rounds leave at
   500 units/s. **Ours' mine layout is its own** (the original's `Libc_Rand` is not carried), recorded as a pair caveat.

| Pair | Lap clock | RMSE | Edge corr. | Mean grey |
| --- | ---: | ---: | ---: | ---: |
| `mines-ahead` | 2.66 s | 0.297, then 0.223 | 0.08, then 0.11 | 41.5, then 20.9 |
| `cannon-volley` | 2.21 s | 0.289, then 0.205 | 0.12 | 35.9, then 20.6 |
| `pad` | 2.0 s | 0.264, then 0.159 | 0.10 | 28.3, then 12.6 |

"Then" is after `det-look` (2026-11-22) gave a Detonator race Zone's sky and dome, the shared billboard target and its own bloom arm
(`docs/ghidra/functions/ps3-hdfury-eu/detonator-look.md`, "Wired").

What differs, by region and size: the Detonator thread's "Render accuracy" section.

### A Detonator pair grabbed in the volley (`cannon-volley-hot`, 2026-11-25, `det-volley-pair`)

`hd/detonator-talons-cannon-volley-hot` sits beside `cannon-volley` (grabbed 0.75 s after its volley, so no wash). `rpcs3-hd-detonator-shots.py --fire-clock 1.84 --fire-pause 0.18 --shot 1.841` keeps the fire button held (harness `triangle`, the game's use button) and pauses 0.18 lap s later, so the frame is mid-volley: lap clock 2.067, press at 1.867, ammo 14, rounds in flight, muzzle flash on the hull. Needs the `-late` state (the plain grid state gives no scenery world clock). Frames: `--fire-pause 0.12` (clock 2.000) showed nothing yet and the same boot's next frame (2.050) showed the volley: the press takes 2 to 4 ticks to reach the game and the picture lags the RAM by 2 to 4 more, so 0.18 is the shortest that lands in the flash. One boot (confidence 70 for the numbers).

Top-left quarter mean RGB (x0..960, y0..540), original / ours, same binary (the old pair re-rendered):

| Pair | Original | Ours | Whole frame mean grey, rmse |
| --- | --- | --- | --- |
| `cannon-volley` (after the volley) | `(20.3, 10.0, 13.7)` | `(17.6, 7.6, 10.0)` | 14.18, 0.172 (stored 15.14) |
| `cannon-volley-hot` (in the volley) | `(17.6, 9.1, 23.7)` | `(18.6, 10.5, 25.0)` | 11.47, 0.139 |

Ours shows the blue wash where the original does (top-left blue 25.0 against 23.7; top-right `(24.4, 13.4, 26.6)` against `(25.9, 12.8, 26.7)`). Ours' round hit a mine (score 0000100, ammo 13) where the original's did not (0000000, ammo 14); mines in the two are ours' seeded plus carried ones. 2048 and Omega: not applicable, an HD pair.

### The volley pair's difference, split (2026-11-26, `det-cannon-hot-gap`)

`cannon-volley-hot` read 11.47 mean grey, with ours' score at 100 and ammo 13 against the original's 0 and 14, ours' round having hit a mine. The
cause was the capture state, not the layout (the 80 mines are the original's): ours ran to the HUD clock's tick (`T - 3`) with the camera at the push
buffer's frame, while the picture was about `T - 7.5`. At `T - 3` ours had fired its second volley and its first had reached a mine; at the picture's
tick it has fired once and hit nothing.

**The picture's lag without mines** (`hd_picture_lag.static_fit`). The mine fit needs two mines 8 to 50 units ahead; this frame's nearest are 74
to 81. The fallback renders ours with camera and craft `k` ticks behind the push buffer along the camera's own velocity (`ship.rig_eye`), at the
picture's tick, and takes the `k` whose edges (below the top 380 rows, less HUD, hull and bright blobs) correlate best with the original's. It
takes only an interior peak that beats every lag 1.5 ticks away by 0.05, and needs the scenery's animation clock (the floor dashes run on it).

| Grab (skew) | Mine fit `k` | Static `k` |
| --- | --- | --- |
| `mines-ahead` (2) | 3.06 | about 2.8 (offset sweep around the fit) |
| `det-pair-lag` `b7/01` (2) | 3.95 | 3.50 |
| `det-regrab-fitted` `h5` (4) | 2.15 | 1.75 |
| `det-mine-spikes` `shots3/01` (4) | 2.33 | 1.75 |
| `det-mine-spin` `shots6/00` (2) | 3.93 | refused: the best score is at the end of the range |
| `det-volley-pair` `shots2/00` (4, no world clock) | 2.94 | 0.50, why the clock is now required |

So it reads 0.25 to 0.6 ticks low where it agrees, and can fail. On the first `cannon-volley-hot` grab it gave 3.50 (score 0.54 against 0.43 to 0.47 at
0 to 2); the pad's white walls, an independent object, put it between 3.5 and 4.25 (right edge at x 1526, 1514 against the original's 1516;
1584 unshifted). Confidence 70 for 3.5 to 4, seen on one grab. "The picture is at `T - 6`" (next section but two) would give `k = 2` here and is
refused by both readings; this stop followed a fire hold, whose many short stops that section found to fit no single value.
**Two later boots of the same stop read 1.75** (peaks at 1.75 ticks, skew 4): the lag varies per grab, and each pair carries its own.

**Ours' tick is the picture's** (`compare_env.picture_tick`, `GO + round(60 T - skew - k)`, and the gantry pin with it) wherever the lag was
measured, by mines or static scene, on the first render and on every re-render (`compare-rerender-ours.py`). A pair with no measured lag keeps
the HUD clock's `T - 3`.

| Pair (same `theirs.png`) | Before | After |
| --- | --- | --- |
| `cannon-volley-hot` | 11.47 | 10.32 (lag 3.5 static, tick 393 to 388, mine ages carried) |
| `mines-ahead` (before the mine spin) | 13.17 | 13.13 (tick 429 to 427) |
| `mines-ahead` (with the mine spin and ages, `main` `68bfc33`'s build) | 12.51 | 12.46 (tick 429 to 427) |
| `cannon-volley`, `pad` (no measured lag) | 14.32, 7.61 | unchanged (both re-checked 2026-11-26, `det-cannon-regrab`) |

**What is left on `cannon-volley-hot`** (grey = that box's share of the 10.32 whole-frame mean; boxes are `x0, y0, x1, y1`):

| Gap | Box | Grey | Also on | Kind, confidence |
| --- | --- | --- | --- | --- |
| Striped metal grilles ours draws bright where the original's frame is near black (ours +22 to +37) | `215,0,520,150` and `1030,150,1360,305` | 0.95 + 0.92 | `pad` (both), `mines-ahead` and `cannon-volley` (the second) | draw, 90; **fixed 2026-11-26**: Talon's `scanlinebillboard` (slot 435), whose Zone variants draw the Zone surface, not the advert ([hd-zone-recolour.md](../rendering/hd-zone-recolour.md), "Holographic billboards in a Zone frame"), 80; -hot 10.32 to 8.96 |
| The score box's side wings are white-striped in the original, solid red in ours | `690,40,830,235` and `1090,40,1230,235` | 0.48 + 0.69 | all four | draw, 90; **fixed 2026-11-26**: the stripes are `DamageBar`, the shield fill, at another rectangle of `detonator_hud2` ([hd-hud.md](../formats/hd-hud.md), "Detonator's score box") |
| Score digits and `1/14` about 1.1 to 1.25 times the original's width | `790,100,1135,180` and `85,60,320,225` | 0.79 + 0.38 | all four | draw, 85; **fixed 2026-11-26**: the layout's `ScaleX`/`ScaleY` (0.9x1.0, 0.8x0.8) were unread ([hd-hud.md](../formats/hd-hud.md), "Text scales on two axes") |
| Hull brighter than the original (ours +7) | `825,720,1100,980` | 0.74 | all four (+5 to +8) | draw, 80 (the fuselage overlay is already open) |
| Muzzle flashes smaller and dimmer (ours -44) | `825,620,1100,720` | 0.70 | firing frame only | capture state, 55: ours presses with no input latency, so its flash is older than the original's |
| Near floor right of the hull (ours +4) | `1120,640,1920,850` | 0.48 | all four (+2 to +5) | draw or band timing, 50 |

**Mine ages.** This record has no read at the picture's clock, so each mine's age is the pause's less the measured lag (the age is a clock
running 1:1 with the lap clock): 55.950 - 7.5/60 = 55.825 on all 80 mines (`rpcs3-hd-pair.py`, `mine_flags`). It does not move this frame
(10.311 with no ages, 10.324 at the picture's, 10.326 at the pause's: the near mines are 74 to 81 units out).

Not carried on that grab: the mines' envelopes were the pause's, 7.5 ticks newer than the picture, and ours fired on the harness's press tick.
Both are carried since (next section). The craft is held at rest, and the track-name ticker's scroll is wall-clock.

### The volley clocks and the picture-clock reads (2026-11-26, `det-cannon-regrab`)

**Falsifiers written before the grabs:** a first launch on the harness's press tick would mean there is no press latency; near-mine envelopes at
the picture's clock equal to the pause's would mean the carry cannot move a frame.

**What the shot script reads now.** At every pause it already takes (it adds none, since a stop 8 or fewer ticks after another reads a different
lag) it keeps each mine's words and the cannon: the volleys left (`*(ship+0x5edc)+0x164`), the held counter (`**0x008a6814+0x6de44`) and
every `PulseBullet`'s age (`+0x4c`) and launch speed (`+0x124`) (`scripts/rpcs3_hd_volley.py`). A volley's clock is the lap clock less its
rounds' age, which is the clock before the frame that fired it. The held counter counts the game's frames, each 1 or 2 ticks, not ticks.

**The press and the volley clocks**, one RPCS3 boot each from `hd-detonator-talons-feisar-grid-late`. Lap clock less round age put the first
volley 2.0 and 2.0 ticks after the send on `-hot`, 4.0 and 3.0 on `cannon-volley`. That is not the press latency itself: the pauses show no
held frame and no round until later. On the second `-hot` boot the counter read 0 and no round lived at 1.917 and the rounds were 4 ticks old
at 1.950, so the first frame with the trigger held came 4 to 6 ticks after the send; the first `cannon-volley` boot read 0 at 1.917 and a
6-tick round at 2.017 (send 1.850). The latency is a few ticks and differs by boot (confidence 60). What is carried, and confirmed by the
frame, is each round's age at the picture. The second volley of `-hot` is 4 ticks after the first, not 6, because the original's frames
were 2 ticks long; ours fires each volley at its own clock (`ours.inputs`: one `square` tick per volley, a release between).

**Pairs** (all 80 mines, picture-tick render). Each row is one grab's `theirs.png`; "none" is the same frame with the harness's press window and
the pause's envelopes. Mean grey, 0 to 255:

| Grab | Picture lag (ticks behind the push buffer) | none | volley clocks only | envelopes at picture only | both |
| --- | --- | ---: | ---: | ---: | ---: |
| `-hot` boot 2 (**published**) | 1.75, static | 10.71 | 10.38 | 10.65 | **10.31** |
| `-hot` boot 3 | 1.75, static | 9.80 | 9.49 | 9.76 | 9.43 |
| `cannon-volley` boot 1 (**published**) | 2.35, 3 mines, 4.5 px rms | 13.65 | 13.65 | 13.35 | **13.35** |
| `cannon-volley` boot 2 | refused: mines disagree by 1.58 ticks, static peak at the range's end | 15.72 | 15.72 | 15.72 | 15.72 |

The columns are measured by removing one carry from the full pair ("volley clocks only" keeps the volleys and takes the pause's envelopes).
The previous published pairs were 10.32 and 14.32 on older `theirs.png`. On `-hot` the muzzle-flash box (`825,620,1100,720`) goes from a signed
-22.8 grey (ours dimmer) to +6.5 (boot 2) and -33.1 to -2.8 (boot 3); the mean absolute difference there hardly moves (32.5 to 31.9, and 38.0 to
39.2), because the flash's shape is rolled per tick. The envelope result on `cannon-volley` is seen on one grab.

**Not carried: the craft's speed on the rounds.** The original's round leaves at `500 + 3.6 v` on its launch tick (v read 101.8 to 102.7
units/s), from a craft that was `v x age` further back than the frame's. A scratch pin doing both moved `-hot`'s rounds (4 ticks old) by about
a pixel and the pair by +0.01 (10.312 to 10.324, 9.433 to 9.443), and `cannon-volley` by +0.002: the two terms nearly cancel at that age, and
`cannon-volley`'s rounds are gone before its frame (one retired at age 10 ticks, one on a wall at 34). It lowered no pair and was not kept.
Whether ours' round then hits the same mine at the same age is not tested.

**Not carried: the mine the original destroyed.** On `cannon-volley` the original scores 0000100 and ours 0000000, because the carried list is
the mines armed at the pause. Adding that one mine (boot 1) makes ours score 0000100 too, but the frame goes 13.35 to 14.05 (15.78 with the
speed pin): ours draws its blast as white puffs and long streaks where the original shows small shards and a `+100` (0.40 of the 0.70 added grey
is above row 450). That is a draw gap in ours' mine blast, open. (The `+100` is drawn since 2026-11-27; it stands 15 px above the
original's because ours' round hits 36 ticks before the picture and the original's about 27:
[detonator-hud.md](../ghidra/functions/ps3-hdfury-eu/detonator-hud.md), "The floating score".)

### The `cannon-volley` difference, split by region (2026-11-26, `det-volley-split`)

`cannon-volley` reads **10.5275** mean grey with the destroyed mine carried (`detonator-mine.md`, "Carrying the destroyed mine"), re-rendered
byte-for-score on `b88ed5e8f`. Question: where the 10.53 sits, and the cheapest lane under 10. Method: the scorer's own difference image
(`compare_env.overlap_score`'s `magick -colorspace Gray -compose difference`) cut into **disjoint** screen regions, a pixel going to the first
row that claims it, so the shares (region sum over `1920 x 1080`) add to 10.5275 exactly. Shifting ours by up to 4 px each way (HUD and ticker
left out) is best unshifted, so no global misalignment. `signed` is ours minus original. The last column is the same split with the floor
visualiser's sRGB decode applied (below).

| Region (boxes are `x0,y0,x1,y1`) | Area | Grey in region | Signed | Share | With decode | Cause, state |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Near mines and far cluster `690,420,885,615` `1375,440,1550,592` `1740,465,1920,632` `1240,342,1340,378` | 4.7 % | 65.7 | -10.9 | 3.11 | 3.10 | spike shape and shell follow the music state the pair carries; not a tunable gap |
| Floor, near road, red strips: polygon `0,590 700,430 850,415 1100,350 1330,335 1370,370 1300,440 1330,465 1920,545 1920,1080 0,1080` | 41.6 % | 6.2 | +2.9 | 2.58 | 1.44 | visualiser strip sampled without its sRGB decode: **read** (75), since wired ([zone-vis-gamma.md](../ghidra/functions/ps3-hdfury-eu/zone-vis-gamma.md), "Wired") |
| Right wall and white pipe `1320,300,1920,545` | 4.8 % | 21.3 | -6.6 | 1.01 | 0.93 | the window lights: **read and wired** 2026-11-27 (`tunnel_fx_noalpha`'s emissive on a plain chunk, [hd-zone-recolour.md](../rendering/hd-zone-recolour.md), "Window lights on a plain chunk"), but the row stays at 1.03: what is left is registration (ours about 10 px off at the frame's edge), the region means match within 5 levels |
| Upper-right screen `1480,0,1920,270` | 5.7 % | 16.3 | +1.3 | 0.94 | 0.97 | original: a purple dot grid with a teal band; ours: grey-white smears and no dots. Also on `mines-ahead`; **unread** (which material is not identified) |
| HUD `85,60,320,225` `690,20,1230,240` `150,850,670,980` `1250,845,1760,980` | 14.2 % | 4.9 | +2.0 | 0.70 | 0.53 | translucent panels over the scene behind (the drop with the decode is the floor under the two lower ones); no HUD gap of its own measured |
| Far scenery and sky (everything else) | 19.3 % | 3.4 | +0.3 | 0.66 | 0.64 | scenery edges, no single cause named; the sky dome is black on both sides (stage 1's horizon and zenith `0 0 0`, [detonator-look.md](../ghidra/functions/ps3-hdfury-eu/detonator-look.md)) |
| Hull `815,695,1095,940` | 3.3 % | 17.5 | +6.8 | 0.58 | 0.50 | ours lighter, rear nozzle lit blue-white; fuselage overlay (`rich_iridescent`) **unread** (Detonator thread, row 7) |
| Ribbon bands: polygons `0,20 0,90 800,400 990,470 1010,440 830,330 60,20`; `600,0 700,0 960,420 900,440`; `1170,0 1230,0 1080,420 1030,420` | 4.3 % | 7.0 | +5.3 | 0.30 | 0.31 | `Trail_Ribbon` dot chains (muted, the whole frame reads 10.22: `detonator-mine.md`, "The ribbon, open"); needs a capture |
| Blast debris shards `655,715,710,780` `550,830,620,865` `1475,278,1515,315` `505,60,550,115` | 0.3 % | 50.3 | -44.0 | 0.17 | 0.17 | rolled at random; not tunable (other shards lie unboxed in other rows) |
| White flecks only the original has `1335,195,1385,260` `1065,280,1100,310` `1375,400,1405,435` `1065,335,1085,352` | 0.3 % | 59.5 | -54.4 | 0.16 | 0.16 | white bursts with a halo, not on `mines-ahead` or `pad`, so tied to the volley; **unread** |
| Track-name ticker `400,75,640,115` | 0.5 % | 26.2 | +4.9 | 0.12 | 0.12 | wall-clock scroll; capture state |
| Destroyed mine's core `1000,460,1075,540` | 0.3 % | 26.4 | -21.7 | 0.08 | 0.08 | the original's dim pink fragment at this age; ribbon and blast timing |
| Exhaust plume below the hull `900,940,1000,1080` | 0.7 % | 11.9 | -0.6 | 0.08 | 0.11 | the craft is held at rest |
| `+100` popup `985,418,1080,448` | 0.1 % | 32.3 | -28.8 | 0.04 | 0.04 | **read**, not drawn (Detonator thread) |
| Cannon rounds (no box) | 0 % | | | 0.00 | 0.00 | both of this grab's rounds are gone before its frame (retired at 10 and 34 ticks, "Not carried: the craft's speed on the rounds" above); no bolt is on screen |

The blast subtotal, which is also the shot-specific part (ribbon, shards, flecks, core, `+100`), is 0.75; floor, screen, wall, hull, HUD and scenery are scene-wide.
`cannon-volley-hot` (7.70, which hit nothing) is the control: HUD 0.99, hull 0.83, floor **3.82** (to 1.61 with the decode), ticker 0.09,
exhaust 0.13, muzzle flashes `825,620,1100,695` 0.25, rest 1.59.

**The decode, measured on the current pairs** (scratch, reverted; `decode(...)` around the `zone_vis_tex` sample in
`crates/mesh/shaders/zone.wesl` `zone_glow`, then `compare-rerender-ours.rerender` on copies of the pairs). Falsifier written first: less than
0.53 off `cannon-volley`, or any Detonator pair rising, would rule it out. Neither happened (one render each; the unpatched binary reproduces
the stored scores):

| Pair | Mean grey | RMSE | Floor box A blue, original / ours / decode | Box B blue |
| --- | --- | --- | --- | --- |
| `cannon-volley` | 10.53 to **9.11** | 0.145 to 0.144 | 18.6 / 26.9 / 7.2 | 29.0 / 51.9 / 14.9 |
| `cannon-volley-hot` | 7.70 to 4.82 | 0.109 to 0.092 | 29.1 / 49.9 / 28.8 | 28.4 / 69.7 / 32.3 |
| `mines-ahead` | 9.68 to 9.04 | 0.143 to 0.141 | 8.0 / 18.7 / 2.5 | 14.9 / 37.1 / 5.4 |
| `pad` | 4.44 to 3.24 | 0.086 to 0.078 | 16.0 / 31.5 / 7.7 | 23.9 / 51.1 / 12.6 |

Boxes A `250,780,800,850` and B `1090,780,1230,1000` ([zone-vis-gamma.md](../ghidra/functions/ps3-hdfury-eu/zone-vis-gamma.md)). The decode
brings `-hot`'s floor to the original's level and leaves the other three at 0.31 to 0.53 of it, so the page's second, unread factor is still
open. Per-channel mean absolute difference over the whole frame also falls (R, G, B; `cannon-volley` 15.9, 8.6, 18.0 to 13.4, 7.5, 15.5;
`-hot` 13.3, 5.2, 20.0 to 6.9, 4.2, 9.0; `mines-ahead` 14.8, 8.0, 16.3 to 13.0, 7.9, 12.5; `pad` 8.6, 2.6, 13.1 to 4.7, 2.5, 8.0), so the
drop is not an effect of the luma weights. But the signed blue turns from too bright to too dark (`cannon-volley` +6.0 to -4.7, `mines-ahead`
+6.4 to -2.9, `pad` +8.3 to -4.0): a player sees a darker violet floor than the original's on three of the four. One `theirs.png` per pair; ours'
render is deterministic. The endrace pairs do not move. A Zone race cannot be shown by the current pairs: `zone-vineta-k-z5` carries no
`--zone-bands`, so the strip is zero and the decode has nothing to act on. Omega's pairs were not rendered.

**Ranked next lanes for `cannon-volley`:**

1. ~~Wire the visualiser decode~~ wired, see the next section: 10.53 to 9.11, under 10 alone. HD only, as `Title` data (2048: checked, differs, no
   `EQ` tints; Omega: open, its descriptor is unread). Show floor crops at player size: the floor goes darker than the original's on three
   pairs, and `zone-vis-gamma.md`'s second factor stays open.
2. Identify and read the upper-right screen's material (read, up to 0.94; also on `mines-ahead`).
3. ~~Right wall pipe and window lights~~ read and wired (`det-volley-wall`): the lights draw, the row does not drop (registration).
4. `Trail_Ribbon` law (capture, RPCS3, about 0.31).

### The hull's frame pose in a Detonator pair (2026-11-23, `det-camera`)

**Report (maintainer, from the pairs):** "ours' shots use the wrong camera setting/distance, the original is 'close', ours is 'far'."
**True; the camera was not the cause.** `--camera-pose` and `--camera-fov` are honoured (the original's eye is 11.7 units from its RAM
body, the close view; ours' craft attitude is within 1 degree of the original's). Two things made ours' hull small and high:

1. **The craft was fed at the RAM body, the camera at the push buffer, and the pause leaves RAM 1 to 4 ticks ahead of the frame**
   (the skew section above). The push buffer's own hull matrix read **2.06, 1.99, 2.0 and 4.05 ticks** behind the body along its
   velocity on four pauses (off-axis 0.1 to 0.4 units, `pad` and `mines-ahead` 2, `cannon-volley` 4, the cannon frame being slower to
   present). `scripts/ps3_pose.py` (`hull_matrices`, `hull_near`, `whole_ticks`, `python3 scripts/ps3_pose.py` for the self test) finds it;
   `rpcs3-hd-detonator-shots.py` records it as `ship.hull_pb` and saves the three push-buffer blobs beside each frame;
   `rpcs3-hd-pair.py --detonator` feeds ours' `--pose` and heading from it, records `original.presented_lag.craft_ticks`, and refuses a pause
   with no hull or a skew that is not within 0.1 of a whole tick.
2. **Detonator draws the hull at 0.63, ours at 0.75** (`ps3-hdfury-eu/camera.md`, "The drawn hull in Detonator is 0.63"): with the
   frame pose fed and nothing else changed ours' hull was 19 percent too wide (315 px against 266). Fixed in the engine.

Hull rim box (white rim, `min(r,g,b) > 200` inside `x 700..1220 y 600..1060`), x0,y0 to x1,y1, in a 1920x1080 frame; a new grab of each
pair on one boot (`pad` and `mines-ahead` together at lap clock 2.03 and 2.67, `cannon-volley` 2.23 after a 1.84 s volley):

| Pair | Original | Ours before (RAM body, 0.75) | Ours, frame pose only | Ours after (frame pose, 0.63) |
| --- | --- | --- | --- | --- |
| `mines-ahead` | 823,712 to 1089,960 (266x248) | 849,613 to 1078,747 (229x134) | 805,703 to 1120,1018 (315x315) | 829,710 to 1096,942 (267x232) |
| `cannon-volley` | 832,709 to 1098,959 (266x250) | 873,600 to 1051,648 (rim sparse) | 816,702 to 1131,1015 | 841,708 to 1107,939 (266x231) |
| `pad` | 829,712 to 1098,972 (269x260) | 845,615 to 1079,757 (234x142) | 811,704 to 1130,1027 | 836,711 to 1105,948 (269x237) |

Top, left and right edges are within 9 px after; the bottom is 18 to 24 px short: the original's engine nozzles and exhaust flare reach
lower than ours' and ours' hull rear is lit blue where the original's is dark metal (open, not tuned). Whole-frame mean absolute grey
difference (0-255) and RMSE (0-1) against the same new `theirs.png`, before then after: `mines-ahead` 19.81 to 19.81 and 0.174 to 0.175,
`cannon-volley` 18.60 to 18.74 and 0.167 to 0.169, `pad` 16.85 to 16.90 and 0.149 to 0.150. **The whole-frame numbers do not move**: the
hull is 6 percent of the frame and the rest (floor, sky, mines) dominates; the claim is the hull's place and size, read on the crops.
The two new pairs' mean grey of the original / ours: 18.2 / 21.7, 17.5 / 18.8, 14.5 / 17.6.

Two circuit pairs (`hd/anulpha-pass`, `hd/chenghou-project`) re-rendered with and without the engine change: **bit-identical** (maximum difference
0 over every pixel), since the factor is `1.0` outside Detonator and the pair script's new path is `--detonator` only. Those grabs were not publishable (no world clock, heading 1.4 to 1.9 degrees); "Detonator pairs re-grabbed" below explains both and replaces them.

### Detonator pairs re-grabbed (2026-11-23, `det-regrab`)

The `det-camera` grabs had no world clock and a heading check of 1.4 to 1.9 degrees (tolerance 1.0). Both are read and neither
tolerance nor filter was changed.

**World clock.** `rpcs3_hd_field.scenery_nodes` found 1,151 `MeshImporter` nodes on a restored Detonator state (560 static at 0.0, 387 and
177 on the global clock, 18 and 1 stragglers) and `_scenery_mode` kept none: it ignores a node whose time is not past `GANTRY_WINDOW_END` (13.3 s,
the gantry's own nodes sit in `[3.83, 13.3)`). The dump of one paused frame on two boots of the old state read the global clock at 10.92 and
10.96 s with the lap clock at 4.75 and 3.90 s (offset 6.2 and 7.1): the state was saved a few seconds into the START RACE prompt and the
world clock only runs on from there, so the pair's frames (lap 2.0 to 2.7 s) fall under 13.3 s. It is the guard doing its job, not a Detonator
limit: **the clock runs at about 0.4 s per wall second at the prompt** (3.1 at 17:56:15, 10.8, 18.8, 28.8, 36.2 at 17:57:43 on a cold walk, read with `scenery_nodes`
every 20 s), so **`--pre-wait` does not help on a restored state** (60 s against 30 s moved the offset only from 6.2 to 7.1: the restored game's clock hardly runs before START RACE), so the saved state is the fix: a state saved a minute into the prompt (`data/saves/hd-fury/hd-detonator-talons-feisar-grid-late`, same sidecar,
`mode: detonator`) restores to a world clock of 54 to 59 s at the pair: 24 nodes of 24 agree, offset 52.3 to 56.3 on eight boots. Recipe: `det_walk`'s walk
(`emu-hd-race-walk.sh`, circuit 8, team 0, six `right`s on RACE TYPE), poll the clock, `emu-save-state.sh` once it reads over 30 s. A
scenery clock at or under 13.3 s (a Racebox race read soon after START RACE, any mode) is refused with the same reason; widening the guard
would let a gantry node pass for scenery.

**Heading.** The check compares the original's hull forward (push buffer) with ours' trace forward at the rendered tick. `--pose` carries a
yaw only, so ours' pitch is its own: 1.6 to 2.0 degrees (forward y 0.028 to 0.035) on every pair. The original's forward y is 0.031 to 0.044
(mean 0.037, 5 frames on 3 boots, no volley fired) and 0.041 to 0.055 (mean 0.048, 7 frames on 4 boots) after a volley, so about 0.6 degrees
higher, with the ranges overlapping (confidence 50). The volley frame at lap 2.2 s read 1.06, 1.54 and 1.10
degrees (3 of 3 boots, refused); the fired frames at 2.6 and 3.0 s read 0.70 to 0.89 (4 of 4, passed). **The cause of the extra pitch at 2.2 s was undetermined here and is read in "The pitch the heading check refused" below:** unfired frames at lap 2.6 (forward y 0.040, 0.044) sit inside the fired range at the same clock, and no unfired frame exists at 2.21;
open discriminator: one boot with no fire and `--shot 2.21`. The refusal itself is pitch (`--pose` carries yaw only; ours runs under
`--pose-hold-ticks`, so its pitch is not a reading of its physics). The pair is the 2.6 s frame. Whether the original pitches from the
cannon's recoil or ours rests low is unread.

`--field` is not used by these pairs (`--detonator` only; no `--field` in any `ours.flags`). A published pair carries no `ours.inputs` (the
private repository's allowlist), so `compare_env.ensure_detonator_inputs` makes it again from `original.detonator.fire` on a re-render
(`compare-rerender-ours.py`; before it, a re-render of a published Detonator pair skipped with "no ours.inputs in the pair directory").
Pairs on `main` `3f8f627`, two boots each, mean grey (the page's score) / RMSE: `pad` 10.48 / 0.137 and 11.80 / 0.147, `mines-ahead`
18.53 / 0.195 and 19.39 / 0.212, `cannon-volley` 17.42 / 0.187 and 20.10 / 0.207; heading 0.15 to 0.85 degrees; hull rim box (white rim,
`min(r,g,b) > 200` inside `x 780..1130 y 690..1000`) within 1 to 4 px on every edge and 3 to 14 px short at the bottom
(`pad` 826,705 to 1093,943 original, 827,706 to 1094,939 ours). One boot of the volley never fired (the press landed with no tick
running: lap clock before 1.867, after 1.867) and was discarded, and one boot's camera pick failed (`no frame-unique value`) and was
retaken. The Detonator thread's "Render accuracy" holds the table and the region differences.

### The pitch the heading check refused (2026-11-24, `det-regrab-fitted`)

**Cause (confidence 80, ours): the pair held ours' craft on the spline's attitude.** `--pose x,y,z,yaw` takes its pitch and roll from the nearest
spline sample and `--pose-hold-ticks` puts that back every tick, so ours' hull never carried the nose-up pitch a Detonator craft flies at.
Discriminating test (the falsifier was written first: if ours' own flight also read about 0.030, it would be a physics difference, not the hold).
On 21 stored grabs at lap 2.4 to 2.8 s (4 boots, `det-pair-lag` and `det-regrab`; ours' attitude from `oag-game --pose` at each grab's position,
`our_attitude`): original body forward y 0.036 to 0.047, ours' spline attitude 0.030 to 0.031 at the same places (mean difference 0.0131,
sd 0.0034, about 0.75 degrees), the push buffer's hull the same offset (0.0109) with a scatter of its own (hull minus RAM body: mean
-0.0027, sd 0.0057, about 0.33 degrees, the hull's own motion between stops). Ours flown freely on the pair's inputs with no pose (full
`--mode detonator` run, 420 ticks) reads forward y 0.0406 to 0.0431 from x = 140 to 193 (flat over 30 ticks, the pairs sit at x 200 to 215),
inside the original's 0.041 to 0.047: **ours' physics pitches as the original does; only the held pose lacks it.** A lag error cannot explain
it: the hull's forward y moves about 0.001 a tick (`b3/00` to `b3/02`), so 4 ticks is under 0.25 degrees. At lap 2.2 s after a volley the
hull sits 0.007 to 0.011 above the RAM body (`b6/00`, `gD`, `gF`; unfired frames at 2.0 do not), the recoil being in the drawn hull and not the
body; that is the earlier 2.2 s refusal (1.06 to 1.54 degrees), now explained to the same confidence. One outlier, `gE/00` (hull forward y
0.023, camera pick failed), is excluded.

**Fix.** `rpcs3-hd-pair.py --detonator` writes the push buffer's hull attitude (`frame_rows`, at the position ours' craft is put, lag-fitted when the
picture lag was) as slot 0 of a one-row `field.csv` and passes `--field-from` instead of `--pose` (the exact-pose path the race pairs use;
no opponents, zero velocity as before). The record carries it as `ours.pose_row`; `compare_env.ensure_detonator_inputs` makes `field.csv`
again on a re-render and `compare-rerender-ours.py` finds it beside the `pair.json`. The heading check stays at 1.00 degrees but now reads
0.000 on every pair: it tests the plumbing (ours draws what it was fed), and the physical check is the free-flight figure above. The
`uncarried_tilt_deg` record is 0. All five stored grabs that were refused or marginal make a pair (the 2.2 s volley `gG/00` included).

**Re-grab.** `mines-ahead` was grabbed as the first stop of two boots at lap 2.70 and 2.72 (`--shot 2.67`, the `-late` state, `--flush 3`; of
five boots three read "camera null: no frame-unique value", a recall cost of `pick_camera`, retried). The fit gives 6.03 and 6.15 ticks behind RAM
(2 mines each, 1.2 and 2.5 px rms), the 6.0 of the lag section reproduced on new boots. Mean grey (the page's score) and the middle-right cell
(the 3x3 grid's row 2, column 3 of the grey difference: mean absolute, signed ours minus original):

| Pair | published, re-rendered on this branch | re-made, fit off | re-made, fit on |
| --- | --- | --- | --- |
| `mines-ahead`, boot 1 (`h1`, 2.70 s) | 16.40, cell 36.87 (-20.04) | 15.75, cell 25.51 (-8.28) | **14.65**, cell **21.81** (-10.13) |
| `mines-ahead`, boot 2 (`h5`, 2.72 s) | | 17.23, cell 37.20 (-19.30) | 15.79, cell 32.10 (-21.36) |
| `pad` (same grab, 2.0 s) | 8.82, cell 6.26 (-2.08) | 8.43, cell 6.26 (-2.09) | no mine in fitting range: same |
| `cannon-volley` (same grab, 2.63 s) | 15.33, cell 26.46 (-18.00) | 14.99, cell 26.46 (-18.00) | no mine in range: same |
| `cannon-volley` 2.23 s (`gG/00`, refused before) | not made | 12.95, cell 22.89 (-13.40) | no mine in range: same |

The brief's published 8.81, 16.23 and 14.88 are the same pairs scored at the earlier binary; the re-renders above are on this branch's. The
campaign goal (all three under 10) is **not reached**: `pad` is (8.43), `mines-ahead` is 14.65 and 15.79, `cannon-volley` 14.99. The fit
moves the near mines onto the original's pixels (crops read: two bright spiked spheres in the original, ours' two dark balls with a lavender
ring now on the same places, a little smaller) and what remains is the mine's body (dark where the original's is bright white-lavender),
the lit billboard wall and the hull's lit strip, none of them lag. Confidence 70 on the pitch cause's fix (two boots of the re-grab, five
stored grabs), 80 on the cause. Not measured: whether the hull's frame to frame pitch scatter (sd 0.33 degrees) is the original's hull sway.

### The presented lag of a Detonator grab (2026-11-24, `det-pair-lag`)

**Question:** how many ticks older is the picture RPCS3 shows than the RAM state the pair feeds ours from, and does it agree with
the two readings on record (`det-mine-look-re`: mines are drawn at their objects and the picture is 1.15 to 3.55 ticks older than the push
buffer's camera, 5.6 to 6.3 behind RAM; `hd-pair-lag-scan.py`: 0 to 3 ticks)? **Answer for Detonator: the picture is 6.0 ticks (100 ms, three
of the mode's two-tick frames) behind RAM on a first stop or a stop 16 or more ticks after the previous one, and 2 or 4 ticks behind
the push buffer's frame, which is itself 4 or 2 behind RAM. Confidence 85 for that value on those grabs (runtime measurement, 15 grabs on 11 boots,
spread 5.85 to 6.25, sd 0.11; one feature class, the mines, plus `det-mine-look-re`'s one panel edge, which is why it is not 90); 50 for what happens on a
stop taken right after another (below); the race modes are not measured here.**

**Falsifier written before the fits:** if the picture were a constant `k` behind the push buffer (the push buffer is the frame the pair
already feeds), `k` would not move with the hull's skew; if it were a constant behind RAM, `skew + k` would not. The fits show the second.

**Method (cross-track and depth only, no whole-frame score).** The mines are static (80 read in one pause, position difference 0.0 over
two stops), their RAM positions are in the record (`ship.mines`), and each draws a bright lavender blob. `scripts/hd-picture-lag-table.py`
tabulates the camera's pose per tick from every push-buffer camera of every record (tick = RAM lap tick less the hull's whole-tick skew; the
craft flies one path on every boot, the eyes at one tick agree to 0.3 units), reprojects each near mine (8 to 60 units) through the
tabulated camera at `T - L`, and takes the `L` that puts it on its blob's centroid. The residual at the best `L` is 0 to 3 px on a clean grab,
against 20 to 130 px at `L = skew` (the push buffer's own camera). No camera velocity is assumed; the single-grab fit that does
(`hd_picture_lag.py`, below), run as shipped on the six grabs that carry the rig eye (`b5/00` to `b5/03`, `b6/01`, `b7/01`, three boots), reads 4.14, 3.91, 1.20, refused, 3.86 and 3.95
against the table's `L - skew` 4.09, 4.05, 1.66, 0.81, 3.83 and 3.93: 0.02 to 0.14 ticks on four clean grabs, 0.46 on `b5/02`, and `b5/03` (the craft's speed halved, a mine hit; its nearest
mines are 6 and 11 units) is refused because one mine is left between 8 and 50 units; an earlier version without that limit read it 3.4 ticks wrong.

| Grab (boot, stop) | skew (hull behind RAM) | `L` behind RAM | `L - skew` (behind the push buffer) | rms px |
| --- | --- | --- | --- | --- |
| `b1/01`, `b4/00`, `gI`, `gJ` (skew 4) | 4 | 6.10, 6.05, 5.95, 6.00 | 2.11, 2.09, 1.95, 2.03 | 1.0 to 4.0 |
| `b2/00`, `b2/01`, `b2/02` (skew 4, nearest mines 32 to 56 units) | 4 | 5.85, 6.20, 6.25 | 1.88, 2.25, 2.21 | 2.9 to 4.7 |
| `b1/02`, `b7/01`, `gC`, `gH2`, `b5/00`, `b5/01`, `b6/01` (skew 2) | 2 | 6.00, 5.95, 5.90, 6.05, 6.10, 6.05, 5.85 | 3.98, 3.93, 3.92, 4.07, 4.09, 4.05, 3.83 | 1.3 to 5.8 (`b6/01` 12.2: one blob merged with its neighbour) |
| `b4/01`, `b4/02`, `b5/02`, `b5/03`, `b2/03` (stop 2 to 8 ticks after the previous) | 2, 2, 3, 4, 2 | 4.45, 4.75, 4.65, 4.80, 5.50 | 2.49, 2.77, 1.66, 0.81, 3.52 | 9 to 17 |
| `b3/00`, `b3/01`, `b3/02` (`--flush 0`) | 2, 3, 2 | 5.70, 5.30, 5.10 | 3.69, 2.28, 3.09 | 6 to 13 |

The hull's skew takes only 2 and 4 on the clean grabs (3 on two stops that followed another closely), which reads as a frame every two ticks
with one or two of them ahead of the picture at the pause (a reading, not located in the game). **Whatever the skew, the picture is at `T - 6.0`**:
the grabs with skew 4 sit 2.0 behind the push buffer, those with skew 2 sit 4.0 behind it, at clocks 2.4 to 4.8 (skew 4) and 2.0 to 2.7 (skew 2). The fifteen grabs
are the first three rows' and `gG`'s (one merged blob dropped); the earlier lanes' records carry no mine list, so their `L` values are fitted
with the mine positions of one RAM read (the same 25 mines), not by the tool. That is the whole reconciliation:

- `det-mine-look-re`'s 5.6 to 6.3 behind RAM stands (its 1.15 to 3.55 behind the push buffer is the same fit with the camera moved along the craft's velocity
  instead of the tabulated path; 3.55 here is 3.9 to 4.1 on the path). Its "mines are drawn at their objects" stands: 0 to 3 px residuals.
- `hd-pair-lag-scan.py`'s 0 to 3 ticks was a whole-frame score on `--field` race pairs (a different mode, with a hull skew of 1 or 2 and a
  60 Hz frame); it says nothing about Detonator and is not refuted for the race. A race frame's own value is open: the HUD clock text lags RAM by 3.3 ticks there, and
  Detonator's two-tick frame suggests a mode-dependent pipeline depth (3 frames here), not a constant of RPCS3.
- `det-camera`'s hull 2 to 4 ticks behind RAM is the skew column.

**A stop taken 8 or fewer ticks after the previous stop in the same boot reads 4.45 to 5.5**, not 6.0 (5 grabs, `b2/03`, `b4/01`, `b4/02`, `b5/02`, `b5/03`; at 9 to 10 ticks it is mixed: 6.20, 6.25, 6.05 and, on one mine, 4.95), and
`--flush 0` reads 5.1 to 5.7 where `--flush 3` and `--flush 10` read 6.0 (`b4/00`, flush 10: 6.05). Those fits are worse (rms 6 to 17 px against 1 to 4):
the picture there fits no single tick on the path, so the values are what the best tick gives, not a measured instant (confidence 50). The
recipe that gave 6.0 on every grab is the first stop of a boot or stops 16 or more ticks apart (`b1/02` 16, `b6/01` 28, `b7/01` 40), `--flush 3`.

**What the pair does (`rpcs3-hd-pair.py --detonator`, `--picture-lag fit`, the default).** `hd_picture_lag.fit` reads the same mines from the
frame and fits `k` = ticks behind the push buffer's camera with the camera moving at its own velocity (`rpcs3-hd-detonator-shots.py` now records the camera
rig's eye in RAM, `ship.rig_eye`: the push buffer's eye is the same rig `skew` ticks earlier, so `(rig_eye - eye) / skew` is the camera's velocity,
the craft's speed but not its direction). Mines nearer than 8 or farther than 50 units are not used, a mine whose own best `k` leaves 8 px is dropped, two
must remain and agree to 1 tick; otherwise the static scene is tried (`static_fit`, "The volley pair's difference, split"), and a pair that fits
neither says "not corrected" and keeps the push buffer's frame (`original.presented_lag.picture`, a caveat). A fit moves ours' camera and craft back `k` ticks of that velocity (the hull keeps its place in the frame) and records
`original.presented_lag.picture_ticks`, `picture_total_ticks` (skew + k), `picture_basis` and `picture_fit` (per mine) or `picture_static_fit`.
`--picture-lag off` keeps the push buffer's frame. Ours' tick and gantry pin are the picture's. The bands stay RAM's.

Mean grey (the page's score, 0 to 255) and RMSE of ours against the original, `off` then `fit`, on pairs made from fresh grabs on the
`hd-detonator-talons-feisar-grid-late` state (all but the second-boot mines pair are `REFUSED` by the heading check, 1.04 to 1.71 degrees against 1.00,
the open pitch difference above, so those are not publishable; the numbers compare like with like):

| Pair | `k` fitted | Mean grey off to fit | RMSE off to fit |
| --- | --- | --- | --- |
| mines-ahead (`b7/01`, clock 2.70) | 3.95 | 25.34 to 17.53 | 0.261 to 0.196 |
| mines, second boot (`b5/01`, clock 2.63) | 3.91 | 20.29 to 17.24 | 0.223 to 0.193 |
| cannon-volley, late (`b6/01`, clock 2.70, cannon fired at 1.85) | 3.86 | 23.47 to 17.05 | 0.249 to 0.192 |
| pad (`b7/00`, clock 2.05) | none (no mine blob between 8 and 50 units) | 10.98 to 10.98 | 0.141 to 0.141 |
| cannon-volley (`b6/00`, clock 2.23) | none (same) | 13.89 to 13.89 | 0.166 to 0.166 |

Read as a player would, `mines-ahead` before has two mines three times the original's size on the left; after, three mines at the original's
places and sizes (the right one is a dark spiked ball where the original has a light football: the open mine-look item). Non-Detonator HD pairs are not
corrected: the pair script's other paths are untouched (`talons` grid pair re-made from its stored capture with `main`'s script and this branch's: identical flags, mean grey 3.31217 both; the
`hd-race/talons-pack` record carries no world clock and both refuse before rendering). The 13 grid pairs are at 0 km/h (a lag changes nothing in them) and the one moving race pair (`hd-race/talons-pack`) has no static
marker to fit; its lag is the open `--field` item above.

**Not settled:** the mode-dependence (race frames), the mechanism of the 4.45 to 5.5 stops, and pad and cannon-volley frames at clock 2.0 to 2.3, whose nearest
mines are 60 units away, where one tick moves a mine 2 percent of its distance: a grab at 2.6 to 2.7 s (mines 9 to 27 units ahead) fits to about 0.1 tick.

### The original's music bands in a Detonator pair (2026-11-23, `det-pair-bands`)

The Detonator floor, its dashes, the billboards and the mine rims are music-driven, and the pairs' `--no-audio` ours is silent, so
the floor drew black. `rpcs3-hd-detonator-shots.py` now reads the original's bands in the same pause as the frame (`observe`, one
`gdb.read`): `*(0x008b7940)` must be the stage block `0x00c7dfb0` (refused otherwise), then sixteen raw floats at `+0x32b8` and sixteen
held floats at `+0x32f8` (`docs/ghidra/functions/ps3-hdfury-eu/zone-equaliser.md`). The record carries them as `ship.zone_bands`, with
the next stop's pair (`--interval` later) as `zone_bands_next_stop`. They are live under RPCS3's Null audio renderer: the game mixes
them itself, so a muted emulator still gives bands (seen on every one of 5 grabs on 4 boots).
`rpcs3-hd-pair.py --detonator` writes them to `original.zone_bands` and passes them to ours as

```sh
oag-game ... --zone-bands H0,...,H15 [--zone-bands-raw R0,...,R15]
```

`--zone-bands` is the floor's held levels (no hold applied on top: ours' `Hold` is the live path's, a capture feeds the level as it
is), `--zone-bands-raw` the levels the mine envelope reads (default: the held ones); it conflicts with `--zone-spectrum-test` and
`--zone-bands-raw` requires it (`oag_game::race_capture::spectrum::ZoneFeed`). The flags are in `ours.flags`, so
`compare-rerender-ours.py` replays them. A capture is one frame, so a mine's envelope in ours has no history (its jump rule needs a rise of
0.3) and picks its own band: mine rims are not like for like.

**The grab is new, not the old one re-fed.** The old pairs recorded no bands and the music phase depends on the wall-clock START RACE tap,
so all three were re-grabbed (two boots: `--shot 2.0 --shot 2.66`; `--fire-clock 1.84 --fire-hold 0.25 --shot 2.21`; `cannon-volley`'s frame came 0.19 s after the release with nothing in flight, so its title was rewritten), and "before" is
ours silent against the same new `theirs.png` (`previous_ours` in each `pair.json`). RMSE (0-1), box means RGB, A `x250..800 y780..850`,
B `x1090..1230 y780..1000`; held band 3 is the floor's smooth slot 164:

| Pair (boot) | Lap clock | Held band 3 | RMSE silent to fed | A theirs / silent / fed | B theirs / silent / fed |
| --- | ---: | ---: | --- | --- | --- |
| `pad` (A) | 2.03 | 0.58 | 0.154 to 0.158 | `(6.7, 0, 19.1)` / `(0.1, 0, 0)` / `(21.5, 4.3, 45.5)` | `(9.5, 0, 29.1)` / `(0.7, 0, 0)` / `(36.3, 8.0, 74.7)` |
| `mines-ahead` (A) | 2.66 | 0.51 | 0.206 to 0.200 | `(5.1, 0, 14.0)` / `(0, 0, 0)` / `(15.1, 2.9, 33.1)` | `(7.4, 0, 24.9)` / `(0.6, 0, 0)` / `(31.0, 6.6, 65.3)` |
| `cannon-volley` (B) | 2.25 | 0.92 | 0.201 to 0.198 | `(6.2, 0, 18.3)` / `(0.1, 0, 0)` / `(25.8, 3.7, 51.7)` | `(8.5, 0, 28.2)` / `(0.7, 0, 0)` / `(44.8, 7.1, 88.2)` |
| `pad` repeat (C, not published; heading 1.09 deg, over the 1.0 check) | 2.02 | 0.68 | 0.150 to 0.169 | `(4.5, 0, 9.4)` / `(0.2, 0, 0)` / `(22.8, 4.2, 48.3)` | `(5.9, 0, 15.1)` / `(0.8, 0, 0)` / `(38.6, 7.6, 79.6)` |
| `mines-ahead` repeat (C, not published) | 2.71 | 0.93 | 0.246 to 0.238 | `(7.7, 0.1, 19.3)` / `(0, 0, 0)` / `(21.1, 2.8, 43.1)` | `(8.9, 0, 30.5)` / `(0.5, 0, 0)` / `(42.5, 6.3, 85.3)` |

The floor is lit now and the hue is right (violet, no red; the billboard panels at the top right are violet in both, and ours' dashes
show faintly where the original's are bright violet: seen on the side-by-side, not measured). **Ours is 2.2 to 3.1 times too bright on blue
at the original's own band 3 (5 times on the pad repeat), and RMSE moves by at most 0.02** (0.004 worse on `pad`, 0.006 and 0.003 better on
`mines-ahead` and `cannon-volley`). The brightness is not the feed: ours' box blue against band 3 alone, pad pair, rest zero, is 0.29: 33.8,
0.58: 45.5, 1.0: 58.0 (about level^0.43, gamma-shaped; the texel is `55 x level`), so matching the original's 19 would take a presented band 3
near 0.07 in every frame, and the original's was read at 0.51 to 0.93. **Whether the original's box tracks band 3 is undetermined:** the
presented frame is 0 to 3 ticks older than the RAM read, and on three of the five frames the raw level equals the held one (band 3 was
rising at the read), so the picture may predate the rise; only `pad` (A) and `mines-ahead` (A) were falling, and those two order the way
band 3 predicts (box A blue 19.1 and 14.0 at 0.58 and 0.51). Next: grabs where raw is below held, or a lag-scanned frame
(`hd-pair-lag-scan.py`). Ours was not tuned.


### The original's mines in a Detonator pair (2026-11-23, `det-mines`)

**Claim tested:** the middle-right cell of `mines-ahead` and `cannon-volley` (original 36.4 and 26.2, ours 20.7 and 17.8 darker) is
the original's lit mines in places ours' seeded layout has none, so carrying the layout and the mine look closes it. **Placement
is carried exactly; the cell does not close, and the frames get worse, because the original draws its near mines somewhere other
than their objects (below).**

**The pair reads the mine objects.** `rpcs3-hd-detonator-shots.py` scans the heap for `DetonatorMine` objects in the same pause
(`rpcs3_hd_field.detonator_mines`; vtable `0x008642f0`, `docs/ghidra/functions/ps3-hdfury-eu/detonator-mine.md`, "Where a mine
stands") and records `ship.mines` (flags, position, entry, lane, live, age; `OAG_MINE_RAW=1` keeps the object's bytes for a survey
of its other fields). `rpcs3-hd-pair.py --detonator` writes the armed ones (`flags & 9 == 9`; a mine that left the pool keeps flag
`1`), plus a mine that left the pool during the shot itself, one a round destroyed (its age still advanced over the shot's mine reads;
`compare_env.detonator_pair_mines`, `docs/ghidra/functions/ps3-hdfury-eu/detonator-mine.md`, "Carrying the destroyed mine"), to
`original.detonator.mines` and gives ours
`--detonator-mines "X,Y,Z;X,Y,Z;..."`: ours lays the opening stage as usual (its bomb, its counter), then puts every mine on the
survey lane nearest each position across the section (`RaceSim::carry_detonator_mines`, a snap of at most 4 units across and 1.5 of the 3.0 lift in height, so a lane on another deck never takes it; a position with
no lane, or whose lane another carried mine or the bomb holds, is dropped with a warning). Positions, not the original's entry numbers: its survey starts at the player's start cursor and
ours at the spline's origin, 7 entries apart on Talon's Junction. The flag is in `ours.flags`, so `compare-rerender-ours.py` replays
it. A shot without mines (an older record) keeps ours' seeded layout and the pair says so.

**The mines' music envelopes** (`det-mine-lit`): the same read keeps each mine's football `s` (`+0xf4`) and spikes `t` (`+0xf8`) in
`ship.mines[].football_s`/`spikes_t`, and `rpcs3-hd-pair.py` gives ours `--detonator-mine-envelopes "S,T;S,T;..."` in the
`--detonator-mines` order. A carried live mine draws from its pin and does not step (`Race::pin_mine_envelopes`). Without it ours'
envelope has no history of the music before the frame, and quiet raw bands (all under 0.3) never attack it: the mine stays at
`MineBrightness` 0, a thin dark ring, where the original's `s` is about 0.5. A shot taken before this change has no envelopes
and runs as it did. One grab at lap 2.68 s, ours before and after the pins: mean grey 14.76 to 14.29; the near mines go from dark
rings to lit, spiked balls at the original's size.

**The mines' ages** (`det-mine-spin`): the same read keeps each mine's age `+0xc0` (`age_s`), and `rpcs3-hd-pair.py` records it in
`original.detonator.mines[]` and gives ours `--detonator-mine-ages "A;A;..."`, from the same read as the envelopes. A mine turns about
its up by `4 x age` ([detonator-mine-draw.md](../ghidra/functions/ps3-hdfury-eu/detonator-mine-draw.md#the-spin-wired-2026-11-26-det-mine-spin)),
and ours' own age counts from ours' laying, 53 s short of the original's on the `mines-ahead` grab. `mines-ahead` 13.17 -> 12.51.

**Evidence the carry is exact** (`detonator_ground_truth::carried_mines_stand_where_the_originals_objects_stood`, disc-backed): 80
objects, identical (position to the bit) at lap 2.7 and 3.4 s and across three boots of the same save state, so the layout is
deterministic from the state and one read stands for every frame of it; all 80 snap to a lane within 0.47 units across the
section (mean 0.06), and stand 3.00 above ours' lane point along the up axis (mean 2.92). The 3.0 is `MINE_LIFT`: ours drew a mine
on the lane point, 3 units too low, 130 px at a mine 18 units ahead.

**Placement against look, in numbers** (`mines-ahead`, middle-right cell MAD / mean grey of the frame, page scorer):

| ours | cell MAD (signed ours - original) | whole frame |
| --- | --- | --- |
| seeded layout (main `d011cf24f`) | 36.4 (-20.7) | 16.23 |
| carried, mine on its lane point | not recorded | 17.56 |
| carried, lifted 3.0 | 41.1 (-16.9) | 17.41 |
| carried, lifted, **and shifted 7 units along the view** (scratch, not committed: the fitted offset below) | 34.6 (-18.5) | 15.90 |
| carried, lifted, size breathing wired | 46.4 (-12.7) | 18.59 |

So with the mines in the original's places the cell stays at 35 to 46: **placement is not most of the cell; the look is** (the
original's mine is a lit white-violet shell with its holes and long spikes about twice ours' size; ours was a small dark rim
disc). The size breathing (`detonator-mine-material.md`, "Wired") gives ours the spikes and the size, and the cell gets worse
because ours' mines are then bigger *and* 7 units nearer than the original draws them.

**The unexplained part.** Projected through the pair's own camera, the original's near mines are drawn 30 % closer to the screen
centre than their objects (`mines-ahead`: objects at `666, 1075, 1703` across, drawn at `770, 1040, 1490`), the far mines (185 to
208 units) and all four near mines of `pad` (83 to 89 units, 2.0 s) land within 4 px, and `cannon-volley` (2.63 s) draws 0.87.
Ours draws each at its object's projection. Scenery agrees (the red strips within 8 to 25 px). Not time (the objects are static),
not a model offset (unit spheres about the origin). A shift of the object 7 units along the view reproduces `mines-ahead`
(residual 35 to 50 px) and rejects `pad`, so it is not a constant. Left open; the pairs are not published with the flag until it is
known (the lead's call).

**Per pair** (main `d011cf24f` to this branch; mean grey of the frame / RMSE; middle-right cell MAD, signed ours - original):
`pad` 8.81 to 8.80, 0.130 to 0.129, cell 6.3 (-2.1) to 6.3 (-2.1); `mines-ahead` 16.23 to 18.59, 0.187 to 0.209, 36.4 (-20.7) to
46.4 (-12.7); `cannon-volley` 14.88 to 15.24, 0.178 to 0.179, 26.2 (-17.8) to 27.6 (-17.0). The carried read for `cannon-volley` is
a boot where the volley fired (lap clock 1.900 to 1.967 over the hold): one mine had left the pool by 2.63 s.

### The picture is older than the push buffer's camera (2026-11-24, `det-mine-look-re`)

**The `det-mines` "unexplained part" is the pair's camera, not the mine.** The original draws a mine at its object's position
(`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-draw.md`, no view-dependent term on the way to `SceneNode_SetLocalMatrix`).
The camera the pair renders ours from is the push buffer's, and the picture RPCS3 shows at the pause is **1.5 to 4 ticks of travel
older** than that camera. The hull rides with the camera, so it never showed the skew; the red strips run along the travel, so a move
along it leaves them where they are. Near static things (the mines at 12 to 25 units) show it; far ones (185 units and more) hardly.

| Grab | Fitted lag `k` (ticks of craft velocity) | Near-mine rms after / before |
| --- | ---: | --- |
| `mines-ahead` pair | 3.55 | 11.9 / 135 px |
| `cannon-volley` pair | 2.30 | 9.4 / 107 px |
| `pad` pair | 1.55 | 1.4 / 5 px (85 units away: a weak lever) |
| shot A of one `det-mines` boot (lap 2.70 s, push-buffer eye `199.27, -41.48, -194.98`) | 2.15 | 11.7 / 93 px |
| shot B of the same boot (push-buffer eye `206.47`, 4 ticks on) | its picture fits shot A's camera at 1.15 (20 px), about 3 behind its own | |

Method: project the RAM mine objects through the push-buffer `viewProj` with every point moved forward by `k` ticks of the craft's
velocity (the same as the camera moved back), against the centroid of the original's lavender pixels (`b > 180`, `r > 150`,
`r - g > 12`) in a window of `0.8 * 800 * 2.5 / z` px around each mine; `k` minimises the rms. **Scenery confirms it**: ours
re-rendered from the `mines-ahead` pair's flags with camera and craft moved back `k = 0..6` ticks puts the left edge of the violet
overhead panel (top right, across the line of travel) at x `1565, 1554, 1540, 1524, 1512, 1501, 1491`, the original at `1518` (x only;
its lower edge does not match on y). **The rest does not follow:** outside the near mines and the hull the plain mean absolute grey goes
`16.67, 16.73, 17.52` at `k = 0, 2, 4` and the right wall's edge correlation peaks at `k = 0`; the eye was moved, its orientation was not.
The whole-frame plain mean moved 22.99 to 20.74 at `k = 4`, mostly from the mines (this tree's render, not the page scorer). **Confidence
70, and not reconciled** with "The skew is the pause" above (push-buffer hull and camera one frame, 1 or 2 ticks behind RAM; blind to
this, both ride the camera) or with `hd-pair-lag-scan.py`'s 0 to 3 ticks behind RAM (whole-frame edge correlation, which the regions
here show is dominated by along-track features): here the picture is `craft_ticks + k` = 5.6 to 6.3 ticks behind RAM.

**What a fix needs (not done):** first settle the disagreement above. Then, since the lag varies by grab, measure it per grab:
extend `hd-pair-lag-scan.py` with a score on static objects (the near mines in Detonator) or a cross-track edge, record `k` in the
existing `original.presented_lag` block, and have `rpcs3-hd-pair.py` move `--camera-pose`'s eye and `--pose` back by `k` ticks of the
original craft's velocity. Every HD pair takes its camera from
the push buffer, so this likely moves every HD pair with near scenery, not only Detonator's.

### The visualiser's sRGB decode, wired (2026-11-27, `det-vis-srgb`)

HD's strip decode ([zone-vis-gamma.md](../ghidra/functions/ps3-hdfury-eu/zone-vis-gamma.md), "Wired") is on
`main`'s path as `ZoneFrame::glow_vis_srgb`. All six `detonator-talons` pairs, re-rendered on copies
(`compare-rerender-ours.rerender`), main to wired:

| Pair | Mean grey | RMSE | Floor box A blue, original / before / after | Box B blue |
| --- | --- | --- | --- | --- |
| `cannon-volley` | 10.53 to **9.11** | 0.145 to 0.144 | 18.6 / 26.9 / 7.2 | 29.0 / 51.9 / 14.9 |
| `cannon-volley-hot` | 7.70 to 4.82 | 0.109 to 0.092 | 29.1 / 49.9 / 28.8 | 28.4 / 69.7 / 32.3 |
| `mines-ahead` | 9.68 to 9.04 | 0.143 to 0.141 | 8.0 / 18.7 / 2.5 | 14.9 / 37.1 / 5.4 |
| `pad` | 4.44 to 3.24 | 0.086 to 0.078 | 16.0 / 31.5 / 7.7 | 23.9 / 51.1 / 12.6 |
| `endrace-norecord` | 8.88, unchanged | 0.097 | | |
| `endrace-record` | 8.38, unchanged | 0.100 | | |

A Zone race keeps its look: `zone-vineta-k-z5` with no bands renders pixel for pixel the same; with made-up bands
fed, the near floor (box `1300,950,1600,1030`) goes from `(115, 163, 168)` to `(115, 154, 162)` (original `(128, 139, 140)`).
The numbers equal the scratch measurement above. Floor crops read at native size: `-hot` matches; the other
three are now darker than the original's (the second factor stays open).

## The four worst front-end pairs, split by region (2026-11-24, `fe-worst-residuals`)

Claim: for each of `pure-psp/menu-pause` (5.06), `pulse-ps2/menu-team` (4.60), `pulse-psp/menu-title` (4.54) and `pulse-psp/menu-pause` (4.51), the largest
residual region is named with its cause, and where the cause is a law the disc or executable authors it is wired and the region and the frame drop by more than the 0.3
grey of re-capture noise. **True for three, and the fourth is not a front-end law.** Boxes are 480 x 272 (PSP) or 1280 x 720 (PS2) pixels; every number is
`compare_env.overlap_score`'s mean grey over the box (`magick ... -colorspace Gray -compose difference`), ours re-rendered by the same recipe (`compare-rerender-ours.py`'s
`rerender`) with main's tree ("before") and this lane's ("after"), against the stored `theirs.png`.

| Pair | Largest region and cause | Box, before to after | Whole frame, before to after | Kind |
| --- | --- | --- | --- | --- |
| `pulse-psp/menu-pause` | the rows' halo: the pause draws over a parked race (`LoadOp::Load`) and the bloom ran only on a clear | rows `(40,20,150,120)` 11.84 to 5.75 | 4.01 to 2.70 (the page's 4.51 predates the music line, main's tree is 4.01) | law, wired (`docs/ui/pause-menu.md`) |
| `pulse-psp/menu-title` | `BOOT_PRESS_START` and `BOOT_LEGAL` in `Default`, the original in `Pulse_20`/`Pulse_14`; the legal line's wrap width | press `(235,212,240,32)` 28.66 to 7.81; legal `(0,244,420,28)` 17.07 to 17.88 (wrap right, glyph pitch open) | 4.54 to 3.37 | law, wired (`docs/architecture/frontend-boot.md`) |
| `pulse-ps2/menu-team` | the craft's `Anim Transform` phase (PS2 node clock unrecovered): ours drew the loop's start | hull `(100,200,500,320)` 14.74 to 1.99 | 4.60 to 2.35 | capture phase, fitted in `pcsx2-menu-pair.py` (`fit_sway`) |
| `pure-psp/menu-pause` | the race under the scrim and its HUD, not the pause: HUD corners 11.41 and 12.54, lap counter 16.75, craft 8.63, band 2.70 | unchanged | 5.06 to 5.06 | not a front-end law: `pure-psp/vineta` scores 10.93, 13.21, 16.79 on the same boxes |

**Shared cause between the two pause pairs: false.** Pulse's is the front end's bloom; Pure's skin carries no `MenuGlow` and its band already matched to the pixel row.

**Same-family pairs, before to after** (mean grey; the ones with no move are byte for byte equal in `ours.png`): `pure-psp/menu-title` 0.08 and `menu-team` 0.26, `2048/menu-title` 3.80
and `menu-team` 27.31, `pulse-psp/menu-team` 2.48: unchanged. Every other `menu-*` pair of Pulse PSP (7), Pulse PS2 (2 besides the team pair), Pure (7), HD (5), Omega (5) and 2048
(5) re-renders byte for byte equal. No pause pair exists for HD, Omega, 2048 or PS2; 2048 and Omega: checked, not applicable (neither skin carries `MenuGlow`, `small_font` or `menu_face_in_boot`).
**Pulse PS2 inherits** the boot-face rule (`menu_face_in_boot`) and the pause bloom from the PSP, unmeasured on the PS2 (no PS2 title or pause frame).

**Re-make the pairs** (the lead adopts the lane's re-made pairs):
`pulse-psp/menu-pause` and `menu-title` and `pure-psp/menu-pause` are `compare-rerender-ours.py --only <pair>` from a tree with this lane; `pulse-ps2/menu-team`
needs `--menu-model-seconds 0.575` in its recorded flags (and `original.ship_sway_fitted`; a fresh PCSX2 capture fits it itself).
