# The compare page is built; HD and Pulse PSP give pairs, four titles are partial, publishing is unlocked

`just compare-page` builds a static page of original-vs-ours pairs (split slider,
onion skin, side by side, difference) from manifests under
`data/reference/compare-page/`, and `--serve` puts it on the LAN. Usage, the manifest
and per-title recipes with their status: [`docs/tools/compare-page.md`](../../docs/tools/compare-page.md).
Landed 2026-10-20 (`compare-page` lane). Each image carries a credit (game, platform, original or engine) and the page footer carries ADR-0074's rights-holder statement, read from `legal.md` and checked against the ADR by `just check-docs` (`compare-page-credits` lane, 2026-10-20); `--publish` builds strictly and refuses an entry with no `platform`.

## Open

- **A new manifest entry needs `platform`**; the seed and live manifests under `data/` have it (`compare-seed.py` writes it).
- **`just publish-compare` is unlocked (ADR-0074 Accepted) and has never been run.** The maintainer runs it; `wrangler` is not installed on the lane host.
- ~~**HD: the craft pose is not carried.**~~ **Closed** (2026-10-20, `compare-hd-pose`): `rpcs3-drive.py capture` records the player body (`ship`: position, basis rows, velocity) in the same pause as the camera, and `scripts/rpcs3-hd-pair.py` renders ours with `--pose x,y,z,yaw --camera-pose --camera-fov --ticks 0`; 12 Time Trial states (one per circuit, Harimau classic) and an Assegai_c1 (Fury Concept) pair on Talon's, in `data/reference/compare-page/hd.toml`. Recipe in `docs/tools/compare-page.md`. **Open in it**: `--pose` takes its attitude from the spline, so pitch and roll are not carried (0.00-0.34 degrees on the 12 pairs); at `--ours-ticks 0` the gantry board is unlit (pair artifact), later ticks light it but our craft sinks about 1.9 units below the original's body (see lead A below); a campaign start would still be slot 8 against slot 1, so the states are Time Trial.
- **Pure (PSP): not reachable.** `psp-drive.py menu` is Pulse's walk; the camera hook
  `0x0883c13c` is Pulse USA's. Needs a hand walk to a grid, a save state
  (`psp-state.py save --keep`, `--serial` for Pure) and Pure's camera node located.
- **Pulse PS2: one pair, ship pose only** (2026-10-20, `compare-pairs-pulse`): Moa Therma White, Single
  Race, grid slot 8 of 8. The craft pointer is read out of the `.p2s` (`scripts/pcsx2_state.py`), the
  circuit is in `data/saves/pulse-ps2/*.p2s.json`. Ours uses our own chase camera and is letterboxed by
  design; the original's PS2 camera is not read. More PS2 circuits need a menu walk (`pcsx2-drive.py`
  has no menu route).
- **2048 (Vita3K): no race reached.** `vita3k-drive.py race`'s fixed-timing tap walk did
  not change screen and the frame was an intro cinematic.
- **Omega (shadPS4): menu only.** MAIN MENU is reached unattended in about 45 s; no
  race was walked. The pair is the original's menu against our own menu page.
- **Pulse PSP: all 12 circuits have a pair** (White entry of each, Time Trial grid after GO, Assegai/Venom;
  `data/reference/compare-page/pulse.toml`) and a state under `data/saves/pulse-psp/time-trial-<circuit>-white-venom-grid.ppst`
  (Talon's Junction's is the older file name). The Black entries are the same circuits' other layouts, not captured.

### Visible differences (`pulse-pair-diffs`, 2026-10-20)

Seen on the PSP pairs unless noted. Items 1-4 are closed; 5-8 are still leads.

1. ~~**The gantry's GO board is dark in ours**~~ **A pair artifact, closed.** `psp-pulse-pair.py` rendered ours at `--ticks 0`, the pre-countdown clock, and the state is after GO. At `--ticks 300` (now `--ours-ticks`, default 300) the board is green with GO on all 12, from the disc's own clock (`start-gantry.md`).
2. ~~**Engine glow at rest**~~ **The same artifact on the PSP, closed**: ours draws the purple glow at tick 300 on the PSP pairs. **PS2 still differs**: the original's flame is a narrow magenta plume of sprites below a white-hot nozzle, ours the PSP-style purple star. **Looked at 2026-10-20 (`hull-points-ps2`), not closed**: the original draws one narrow four-fin spike on the centre line (two passes, grey and `(155,0,124)`, fading) under a white-hot plate; ours plays the `.POB` (halo, plate) and hides the plume at rest. The spike has `shipboost.vex`'s shape and size but sits on the centre line, where that file's anchors are at `+-0.9`; forcing the plume on draws its two side flares. Not identified: [trail-ribbon.md](../../docs/rendering/trail-ribbon.md) "Pulse PS2 at rest". Next: try the `engine_flare` locator as the anchor, and read the draw's colour source.
3. ~~**Hull colour under strong coloured light**~~ **Fixed**: the GE's fourth hull light is the circuit's nearest `wopoint`/`wospot`, not modelled before. Mean wing colour on Tech de Ra now `(87,106,115)` against the original's `(87,105,115)`, The Amphiseum `(107,129,144)` against `(108,131,145)`. The law, read live and reproduced on three circuits' lists: [`scene-light.md`](../../docs/ghidra/functions/psp-pulse-usa/scene-light.md) "The point and spot lights". De Konstruct is `(102,106,89)` against `(94,98,78)`: a residual of about 8 levels that these lights do not explain (ours takes none there; whether the original's list holds one was not read live). **Part-closed 2026-10-20 (`hull-points-ps2`)**: the live list matches ours except the scale bytes, `(179,179,250,250)` against our nearest-ring-point `187`; `Course::light_scale_at` now blends at the craft's own place and the residual falls by 2 levels. The rest (`(5,5,13)` on the wing, `(8,7,6)` low on the hull) is not the list and was not isolated (bloom or blur suspected): [`scene-light.md`](../../docs/ghidra/functions/psp-pulse-usa/scene-light.md) "De Konstruct".
4. ~~**Record panel**~~ **Matches, closed.** The panel is a countdown: the original's `1:46.3` on The Amphiseum at a clock of `0:20.7` is the authored `<RaceTimes>` figure `2:07.0`, ours reads `2:07.0` at clock 0. Every circuit agrees to 0.1 s once the clock is added (Arc Prime `1:24.7 + 0:32.2 = 1:56.9`, ours `1:56.5 + 0:00.4`).
4a. **PS2 hull lights (`hull-points-ps2`, 2026-10-20)**: the PS2 builds the PSP's `wopoint` vector and colour per craft, but our PS2 hull ignores the whole hull-light rig (coloured vertices read as lit), so nothing of it reaches the picture; the PS2's own composition (floor `0.5`, a second light of unknown source) is unread. Pure ships the nodes (seven circuits) and its builder is unlocated. [`hull-lights.md`](../../docs/ghidra/functions/ps2-pulse-eu/hull-lights.md). Next: the VU1 program (`vu1MicroMem.bin` of the saved state) and the block's writer.
5. **Outpost 7**: a white panel at the top left of ours where the original shows blue sky; Platinum Rush and Vertica: a distant sign is yellow or red in ours against red or orange in the original (animated billboards, phase not matched).
6. **Tech de Ra**: a lit box at the top left that the original does not draw.
7. **Race clock**: not matched by design (the original has run since GO).
8. **PS2**: the original's hull fills about 1.35x the width ours does (our PSP-shape fit and our own camera), the original shows "pos 8/8" where ours shows no position, and a faint `(UT EDIT)` string sits at the bottom left of the original (a debug string).

### HD visible differences (`compare-hd-pose`, 2026-10-20)

Seen on the 13 HD pairs (12 circuits, Harimau classic, Time Trial just after GO; Assegai_c1 on Talon's), the original at rest (`rpcs3-hd-pair.py` holds no pad button; `rpcs3-drive.py capture` defaults to holding `cross`, which is throttle once a Time Trial runs, and a first set of pairs taken that way read 5-7 km/h and a 96-98% thrust bar: that bar is closed as an artifact, at rest it is 0%). Leads only, nothing here was fixed. Pair artifacts, not differences: the unlit gantry board and the race clock (tick 0 of ours is before GO). Ours is the main checkout's `target/debug/oag-game` of 13:18 on 2026-10-20, after the HD Zone material merge `4097e9a44`.

**Caveat (lead, 2026-10-20): the 13 HD pairs behind A-I were captured with RPCS3's `Write Color Buffers` off** (the lane's private `config.yml` and `custom_configs/config_BCES00664.yml` both read `false`), which reads the original about 1.74x too bright (`docs/reverse-engineering/rpcs3-capture.md`), and ours was a build from before the HD probe-map sRGB fix. B (exposure and bloom), D (hull colour), F (washed-out sky) and I (engine glow) are suspect until re-shot; A, C, E, G and H do not depend on brightness. `rpcs3-drive.py` turns the setting on by default since `a5470adfe`.

A. **Ride height after the pose.** Ours placed at the original's body position (`y = -50.078` at Talon's) settles to `y = -51.93` by tick 60 and stays: 1.9 units lower than the original's resting body, on the same spline. Measured on Talon's only (`oag-game --pose ... --ticks 300 --trace-out`); other circuits not run. Either our hover rests lower than HD's, or the body origin differs between the engines.
B. **Exposure and bloom.** The original is brighter and high-key everywhere: Moa Therma, Ubermall and Sol 2 blow out to white (sky, track), Metropia and Chenghou carry a warm bloom; ours is darker, more contrasted, tonemapped. Largest single difference on every pair.
C. **The hover glow under the craft.** At rest the original draws a faint blue ring and chevrons under the hull on the road (brighter under throttle, in the first pairs); ours draws a small blue spot at the nozzle and nothing on the road. The `.pob`/ground-glow trigger was not identified here.
D. **Hull colour.** On Modesto Heights and Tech de Ra the original craft reads cream-yellow and ours orange and blue (Amphiseum: original yellow-green, ours pink-cream); on the other circuits ours has a pearl, rainbow sheen the original does not. The hull takes the circuit's probe/lights differently.
E. **The scan line.** The original draws a thin horizontal line across the track at the craft's height (red on Metropia, Chenghou and Ubermall, green on Sebenco Climb); ours draws none.
F. **Sky.** Ours draws a textured, cloudy sky where the original is white or washed out (Talon's, Moa Therma, Sol 2); Sebenco's snow and Modesto's sky agree in hue.
G. **Sprites and lights.** Pink flare sprites top right on Metropia and the harimau banner's glow on Vineta K are absent in ours; Metropia's left-hand stair rail and Chenghou's ribbed steps are lit tan in ours and dark in the original (a lighting or shadow difference on those meshes).
H. **HUD.** The original reads `RECORD 9:50.x` where ours reads `TOTAL 0:00.0`; the original's speed and thrust panel is translucent where ours is solid (the thrust bar reads 0% on both at rest); the position hexagon matches.
I. **Concept-craft engine.** Assegai_c1 at rest: the original's engine plate is a bright pink-red glow, ours a dim orange.

States: `data/saves/hd-fury/hd-timetrial-<circuit>-harimau-classic-start.SAVESTAT.zst` for `vineta-k`, `anulpha-pass`, `moa-therma`, `chenghou-project`, `metropia`, `sebenco-climb`, `ubermall`, `sol-2`, `talons`, `the-amphiseum`, `modesto-heights`, `tech-de-ra` (all twelve; the table in `docs/reverse-engineering/emulator-recipes.md` lists them). `emu-restore-state.sh <lane> <state> --restart` then `rpcs3-hd-pair.py` rebuilds a pair in about 1 minute (a restart that does not reload in 120 s happened once in 14: run it again).

## Next Steps

0. Pulse PS2: more circuits need a menu route for `pcsx2-drive.py` (it only boots and loads states); the original camera's pose (rig eyes at craft-rig `+0x850`/`+0x860`) could be carried like PSP's `--camera`.

1. Make a Time Trial state for each title the maintainer wants reviewed and capture
   one pair per circuit with the recipes in the doc.
2. ~~HD ship pose~~ done (`rpcs3-hd-pair.py`); the next HD step is lead A above (ride height) and, for a pair that shows the lit board, carrying the original's clock into ours.
