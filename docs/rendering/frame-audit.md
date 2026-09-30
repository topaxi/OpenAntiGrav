# Pulse frame audit: our race frame against the original's, same state

Read 2026-09-30, PPSSPP v1.20.4 (`pulse-psp-usa.chd`) against `oag-game --race`.
The question was which visible differences a player would notice first, and
whether their cause could be traced in the data or the decompile. **One was fixed
(texture level selection, below); everything else measured close, and the
remainder is ranked with what was ruled out.**

## How the two sides were matched

Every comparison is one frame of each, from one state, at the PSP's own size,
with nothing either side adds:

| | Original | Ours |
| --- | --- | --- |
| Size | 480x272, `InternalResolution = 1`, window 480x272 | `--size 480x272 --render-scale 100` |
| Filters | no post shader, `TexScalingLevel = 1`, `AnisotropyLevel = 0`, FPS counter off | `--msaa off --motion-blur off --screen-filter off --anisotropy off` |
| State | the craft's pose and camera read per tick by `psp-trace.py --camera --shot-every 1` | `--pose-from` that CSV, `--pose-tick`, **`--ticks 1`** |
| Config | own `HOME`/`XDG_CONFIG_HOME` under `data/scratch/pulse-frame-audit/` | an isolated `XDG_CONFIG_HOME` (English, bloom on) so the maintainer's `settings.toml` (200 % render scale, 4x MSAA, `psp-3000` filter) never reaches a screenshot |

Circuits: Talon's Junction (`16_Track`, bright outdoor), Metropia (`03_Track`),
Tech De Ra (`04_Track`, indoor). Same team (Assegai), same Venom time trial.
Moments: on the grid, mid-straight, at 41 units/s on a straight, mid-corner. The
corner moment was unusable - `place` left the original's craft tumbling in a
tube - and is not cited.

Three traps that each produced a false difference before they were understood:

1. **`--ticks 0` skips the PVS.** The visibility set is built from the craft's
   spline position, which does not exist before the first tick, so a
   `--pose-from ... --ticks 0` frame draws all 2,051 draws and a whole-circuit
   perimeter wall (`fw_wall01_pr`, `Building_05`, radius 1,270) covers the
   horizon. It reads as a large geometry bug and is not one. **Use `--ticks 1`.**
2. **The exhaust flare is state, not geometry.** At `--ticks 0` intensity is 0 and
   no flare draws; the original's idle craft shows one. Ours shows it from about
   tick 30 (`Exhaust_Update`'s ramp). A comparison of the hull's brightness at
   tick 0 reads the missing flare's bloom as a 15-35 % darker spine.
3. **The gantry and the strip lights are time.** The original was captured 0.1-2 s
   after GO; ours is a fresh state. The GO banner, the countdown board and the
   pulsing cyan wall strip differ for that reason alone.

## Ranked differences

Ranked by how much a player would notice. "Doc" names where it is already
recorded.

| # | Difference | Size | Cause | Status |
| ---: | --- | --- | --- | --- |
| 1 | **Scenery, trees, mountains and track surface drawn soft** | Laplacian standard deviation in four regions of one frame: original 13.4 / 6.1 / 14.9 / 7.5, ours before 10.8 / 4.4 / 11.3 / 6.3, ours after 12.7 / 6.2 / 15.5 / 8.2 | our box-filtered mip chain, where the original resolves the base level | **fixed** (`PSP_SAMPLED_LEVELS`) |
| 2 | Hull sheen: a whiter, glossier spine and canopy in the original | spine mean 143,139,113 against 91,77,53 at tick 0, but 191,193,143 against 143,144,76 at tick 90 and the close crops read nearly equal once the flare's bloom is in | part flare bloom (state), part unconfirmed: `Ship.vex` has one transparent batch (`Glass_ADD.tga`, 8x8) that the GE draws under environment-mapped UVs and `oag_render::texgen` has no caller | open, see below |
| 3 | Dark bowl-shaped objects on the grass left of Talon's straight at (161,-47,-185) | a few percent of the frame | draws 1381-1391 of the track (`factory_floor_01_rp_shinemap`, `Building_05`, `Building_07`, `energy_GLOW`, `piston_end_shinemap`) are factory roofs at 37-100 units; the original's terrain hides them, ours does not | open |
| 4 | Track neon strips and the start gantry | animated | timing (trap 3), not measured as a defect | not a defect on this evidence |
| 5 | Sky, fog and tone | regional means agree within 5 % on all four frames (e.g. Talon mid, 18 cells: 101,129,146 against 108,137,156 at worst) | - | matches |

Camera height and FOV were not re-measured: `--pose-from` carries the original's
own camera, so this audit cannot see a camera difference. That question is the
race-camera FOV thread's (see `HANDOVER.md`'s open threads) and stays there.

## 1. Texture level selection (fixed)

The original **never minifies a scenery texture** at the distances a race frame
shows. Measured four ways against PPSSPP on Talon's Junction:

| Region (px) | Original | Chain (before) | Base level (after) |
| --- | ---: | ---: | ---: |
| left trees and mountain, 130x100 | 13.45 | 10.78 | 12.73 |
| road right, 100x60 | 6.13 | 4.38 | 6.24 |
| horizon, 200x60 | 14.94 | 11.32 | 15.47 |
| skyline, 120x50 | 7.54 | 6.26 | 8.19 |

(Laplacian standard deviation of the luma channel, 8-bit; higher is sharper.) The
same holds on the Talon grid frame's far track, where a chain would have cost the
most: 16.0 against the original's 15.9, and 12.8 for the chain. `AnisotropyLevel`
was 4 in the copied profile for the first pass and 0 for the confirming one; the
result did not move, so it is not the emulator sharpening oblique floor.

**What the binary says, and how far it goes.** `Gfx_FlushRenderManager` programs
the texture unit once per frame: `Gu_TexFilter(7, 1)` (trilinear minify,
bilinear magnify), `Gu_TexLevelMode(2, 1.0)` (slope mode, bias 1.0) and one call
to a `0xd0` `TEXLODSLOPE` emitter, **`Gu_TexLodSlope`** (`0x08811694`), with the
float `0x3b800000` = `1/256`. That emitter is the only one in the binary, and this
is its only call. The GE's slope law is `level = log2(|z| * slope) + bias`, which
depends on **depth, not on texel density**: with a slope of `1/256` a texture stays
at level 0 until about 128 world units from the eye and reaches level 1 near 256.
So the original's picture is not "no mips", it is a very coarse depth-driven
selection that a 480x272 race frame barely leaves level 0 for. **PPSSPP's GPU
backends do not implement slope mode as the GE does** - whether real PSP hardware
mips distant scenery beyond about 128 units is unmeasured here, and the reference
this project compares against is the emulator. `PSP_SAMPLED_LEVELS = 1` reproduces
what that reference draws. The mode-0 (auto) branch `Texture_BuildBindList` takes
on the `+0x06 & 0x18` bits was not separated out; every texture in these frames
matched the base level.

The change is one line, `mesh::PSP_SAMPLED_LEVELS`, passed where the `.vex` loader
used to pass the authored `mip_count`. It also halves the GPU memory those
textures held. It applies to every title that loads a `.vex` with embedded
textures: **Wipeout Pure is unmeasured** against its own capture and shares the
line. `crates/render/tests/psp_texture_levels_ground_truth.rs` fails if a circuit's
textures are uploaded with more levels again.

## 2. The hull's sheen (open)

What was ruled out: the hull's light list (it is the recovered `AmbientLight` and
three `DirectionalLight` nodes, and the wings match to 1 %), bloom (toggling it
changes neither the trees nor the spine), and the idle flare (a state artefact,
trap 2). What remains is a gloss the original has on the yellow spine and the
inner wings. **Lead**: `Ship.vex`'s single transparent batch is `Glass_ADD.tga`,
8x8, 111 indices on Assegai; `Mesh_BeginTransparentPass` draws such batches with
`TEXMAPMODE` uvgen 2 (environment mapping from the normal and two light
directions), which `oag_render::texgen` implements and nothing calls. A local
experiment that env-mapped **every** transparent batch in `fs_main_blend` left the
hull unchanged and erased the track's bright glass floor, so the naive wiring is
wrong. What decides it, unread: whether the hull takes the fixed-basis branch
(`model+0x1a8 != 0`) or the view-matrix matcap, and whether the track's glass
takes the other. A live read of the hull model's `+0x1a8` and `+0x48`/`+0x70`
light lists would settle it; the craft pointer this session read
(`craft+0x8b4`) did not resolve to a model, so the read needs the trace's entity
pointer instead.

## 3. The factory roofs (open)

At the Talon straight pose, the original hides a factory complex behind terrain
that ours draws over. Ours renders the buildings' dark roofs as bowls on the
horizon. Not traced: whether the original's terrain is a layer that draws after
the factory, whether the factory belongs to a PVS section the original culls at
this eye, or whether our section placement differs. The draws are
`1381..1391` (`--ticks 1`, PVS on).

## Reproducing

```sh
# original: own HOME and XDG_CONFIG_HOME, own Xvfb, ppsspp.ini with
# InternalResolution=1 TextureFiltering=1 TexScalingLevel=1 AnisotropyLevel=0 iShowStatusFlags=0
uv run --with websocket-client scripts/psp-drive.py --port 45093 menu
uv run --with websocket-client scripts/psp-drive.py --port 45093 place --pos=X,Y,Z --tangent=X,Y,Z --up=X,Y,Z --speed 0
uv run --with websocket-client scripts/psp-trace.py --port 45093 --ticks 3 --camera --out t.csv --shot-every 1 --shot-dir shots
# ours
oag-game --race --no-audio --size 480x272 --render-scale 100 --msaa off --motion-blur off \
    --screen-filter off --anisotropy off --ticks 1 --pose-from t.csv --pose-tick 2 --screenshot ours.png
```

`psp-drive.py place` takes `--tangent=-0.9,...` with an equals sign: a value that
starts with `-` is otherwise read as a flag. Other circuits than Talon's Junction
need the dev-unlock byte (see the debugger page) written once `Main Menu` is up,
and `menu --track-down N --any-track`. Crops and montages from this pass are under
`data/shots/frame-audit/` (derived game data, not committed).
