# Pulse frame audit: our race frame against the original's, same state

Read 2026-09-30, PPSSPP v1.20.4 (`pulse-psp-usa.chd`) against `oag-game --race`.
The question was which visible differences a player would notice first, and
whether their cause could be traced in the data or the decompile. **One was fixed
(texture level selection, below, later given the hardware's own law); everything else measured close, and the
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
| 1 | **Scenery, trees, mountains and track surface drawn soft** | Laplacian standard deviation in four regions of one frame: original 13.4 / 6.1 / 14.9 / 7.5, ours before 10.8 / 4.4 / 11.3 / 6.3, ours after 12.7 / 6.2 / 15.5 / 8.2 | our box-filtered mip chain, where the original resolves the base level | **fixed** (the disc's levels, selected by the GE's slope law) |
| 2 | Hull sheen: a whiter, glossier spine and canopy in the original | spine mean 143,139,113 against 91,77,53 at tick 0, but 191,193,143 against 143,144,76 at tick 90 and the close crops read nearly equal once the flare's bloom is in | part flare bloom (state), part the hull's `0x2000` extra pass: six batches (not the one glass batch) drawn a second time under environment mapping with `envtest4bit.tga` | **drawn** (`oag_render::shine`); closes roughly a third to a half of the measured gap, the rest is bloom and pose, see below |
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

**What the binary says.** `Gfx_FlushRenderManager` programs the texture unit
once per frame: `Gu_TexFilter(7, 1)` (trilinear minify, bilinear magnify),
`Gu_TexLevelMode(2, 1.0)` (slope mode, bias 1.0) and one call to a `0xd0`
`TEXLODSLOPE` emitter, **`Gu_TexLodSlope`** (`0x08811694`), with the float
`0x3b800000` = `1/256`. That emitter is the only one in the binary, and this is
its only call. The GE's slope law is `level = log2(|z| * slope) + bias`, which
depends on **depth, not on texel density**: level 0 up to a view depth of 128,
level 1 at 256, level 2 at 512. That is why a 480x272 race frame barely leaves
level 0, and why PPSSPP, whose GPU backends do not implement slope mode, draws
level 0 everywhere.

**Implemented as the game's rule, 2026-09-30 follow-up.** The first fix capped
every PSP `.vex` texture at one level, which is PPSSPP's behaviour. The law is
now in the shader:

- **The levels are the disc's, not ours.** The texel block of a `.vex` texture
  carries every declared level after the base, at the padded stride the sum-check
  on `texture_row_stride` establishes. `oag_vex::vex::EmbeddedTexture::levels` reads
  them (all 135 textures of `16_Track`, and the level 1 of most differs from a box
  filter of the base, which is asserted), and `Texels::Chain` carries them to the
  GPU in place of the box-filtered chain this project used to make. A pre-swizzled
  Pure texture keeps a synthesised chain capped at its declared depth.
- **The selection is the game's**: `mesh.wgsl` takes
  `textureSampleLevel(..., max(log2(view_depth / 256) + 1, 0))` when the model
  carries a chain (`texlod_slope`/`texlod_bias` pipeline constants, from
  `mesh_render::PSP_TEXLOD_SLOPE` and `PSP_TEXLOD_BIAS`), clamped to the levels
  that exist by the sampler. The fractional part blends two levels, as the
  trilinear filter asks. **Chosen, not measured**: `|z|` is the vertex's view
  depth (`clip.w`) - an unverified reading of the GE's own `z` - and
  `Texture_BuildBindList` emits its own mode and bias per texture, unread, so a
  texture on its mode-0 branch may not follow this rule. The two constants are
  recovered (confidence 85 on the emitter, and the values are read off the
  call), the mapping to our depth is not.
- **Proof.** `crates/render/tests/psp_slope_lod.rs` draws one quad with a
  red/green/blue three-level chain at view depths 64, 128, 256, 512 and 100,000
  and reads levels 0, 0, 1, 2, 2; a model without `Texels::Chain` stays on level
  0. Disabling the constants fails it at depth 256.

**Near field kept, far field softens.** Same four Talon-mid regions as above,
luma Laplacian standard deviation: original 13.45 / 6.13 / 14.94 / 7.54, base
level only 12.73 / 6.24 / 15.47 / 8.19, slope law 12.73 / 6.24 / 15.08 / 8.19.
Three regions sit under 128 units and are byte-for-byte the base level; the
horizon strip, which reaches past it, moves from 15.47 to 15.08 - nearer the
original's 14.94. A long-straight frame (`--pose -297.96,-50.5,-172.83`, Talon,
`--ticks 1`, no original: PPSSPP draws level 0 here by construction) reads, base
level against slope law, 24.51 to 22.77 and 28.36 to 26.40 in two strips past
about 256 units (whole frame 16.74 to 16.56, 74 pixels differing by more than
1 %). That gap is the level 1 the hardware rule selects and the emulator does
not; whether real hardware shows it is still unmeasured.

**TEXTURE DETAIL** (`[render_profiles.<title>] texture_detail`, `--texture-detail`,
the graphics page's row beside MODEL DETAIL) is one multiplier on that law, as
levels: `original` (default) is the recovered slope law exactly; `high` doubles
the depth of every level step (the level shifted by -1); `maximum` never steps
(level 0 at every depth). `high` and `maximum` are **chosen, not measured**. It is
`Fog.texlod_shift` in the scene uniform (`mesh_render::TextureDetail::level_shift`,
zero for `original` and for any buffer nothing writes), added to the level in
`mesh.wgsl`, and the race hands the preset over every frame, so the row applies
live. Proof: `psp_slope_lod.rs` reads the level each preset selects at depths
64 to 100,000 (a preset that stops reaching the shader fails its own test), and
`texture_detail_ground_truth.rs` runs the binary per preset on the Talon straight
and asserts the three pictures differ pairwise.

Talon long straight (`--pose -297.96,-50.5,-172.83`, `--ticks 1`, 480x272),
luma Laplacian standard deviation, same metric as above (`magick ... -colorspace
Gray -define convolve:scale='!' -morphology Convolve Laplacian:0`, sd x 255):

| Region | original | high | maximum (= the old base level) |
| --- | --- | --- | --- |
| Whole frame | 16.56 | 16.70 | 16.73 |
| Strip 120x40+120+80 | 21.37 | 22.49 | 22.80 |
| Strip 120x40+120+0 | 32.52 | 32.92 | 32.92 |

`original` is softest and `maximum` sharpest, and the whole frame reproduces the
previous pass's 16.56 (slope law) and 16.74 (base level). The strips are the two
with the largest change. The previous pass's own boxes were not recorded, and its
24.51/22.77 and 28.36/26.40 cannot be recovered from the saved frames
(`data/shots/frame-audit/far/ours-l0.png` and `ours-slope.png` reproduce its whole-frame
16.74 and 16.57, but hundreds of boxes match each pair to two decimals), so they are
not directly comparable. Only 74 pixels differ
between `original` and `maximum` at this size: the far field is small at 480x272.

**`--anisotropy` works again on a PSP `.vex` model.** The slope law first sampled
with `textureSampleLevel`, an explicit level, and a GPU takes no anisotropic
footprint from one: the same pose (Talon straight) with `--anisotropy off` against
`16x` differed in 602 pixels over 1 % with the derivative path (`textureSample`
over the same chain) and 5.6 with the slope law. `mesh.wgsl`'s
`sample_at_slope_level` now takes `textureSampleGrad` with the screen-space
derivatives reshaped so the level the hardware picks is still the law's (plus
TEXTURE DETAIL's shift). With `w = 2^level` the width of one texel of that level
and `pmax` the footprint's long axis in base texels, `probes = clamp(ceil(pmax / w),
1, clamp)`; the long gradient is set to `probes * w`, the short one to `w`, each
along its own direction, so the hardware reads a ratio of `probes` and a level of
`log2(probes * w / probes)`. The sampler's clamp reaches the shader as the
`aniso_max` pipeline constant. Anisotropy off, or a footprint no wider than a
texel of the level, is one probe and the plain explicit level.

Two things this is not. It **cannot sharpen**: the level is the slope law's and
stays so, so anisotropy only antialiases along the long axis of a foreshortened
surface (less shimmer and moire on far girders and floor), and TEXTURE DETAIL is
the control that shows finer levels. And a first version that set the probe count
from the surface's own ratio, ignoring how big the footprint was, blurred near
walls by up to 16x the footprint; sizing it from `pmax / w` leaves everything a
texel or narrower alone. The same pose, `--anisotropy off` against `16x`, now
differs in 22156 of 130560 pixels at 480x272 (72975 of 921600 at 1280x720),
concentrated on far edges and grazing floor; near surfaces are unchanged.
`crates/render/tests/psp_slope_lod.rs` asserts the level under every anisotropy
setting with a stretched footprint (so a level that drifts fails), that 16x moves
a foreshortened striped surface, and that it moves nothing when the footprint is
under a texel.

**A consequence to watch**: a coarse authored level of a cutout texture has alpha
above the reference nearly everywhere, so a cutout batch (trees, fences, pad
glows) reads fatter at depth than it did with a base level. Wipeout Pure's
speed-pad guard (`pad_alpha_test_ground_truth`, an orbit framing hundreds of
units out) stopped discriminating for exactly that reason and now compares at the
base level, which is what it is about; the discard operator's own test is
unchanged. Whether the original's distant cutouts thicken the same way is part of
what a hardware capture would settle.

The change applies to every title that loads a `.vex` with embedded textures:
**Wipeout Pure is unmeasured** against its own capture and shares the rule.
`crates/render/tests/psp_texture_levels_ground_truth.rs` fails if a circuit's
textures stop arriving as the disc's own chain.

## 2. The hull's sheen (drawn, gap partly closed)

What was ruled out: the hull's light list (it is the recovered `AmbientLight` and
three `DirectionalLight` nodes, and the wings match to 1 %), bloom (toggling it
changes neither the trees nor the spine), and the idle flare (a state artefact,
trap 2). What remained was a gloss the original has on the yellow spine and the
inner wings.

**Cause, read live 2026-09-30**: the original draws six of the hull's fourteen
batches **twice** - `Ship.vex`'s `0x2000` batches (`shipShape`'s three blended
ones, both airbrakes' and the canopy glass) - first under environment mapping
with their material's **second** texture, `envtest4bit.tga` (a 64x64 black map
with three white glints), then their ordinary pass added on top. The lead this
page used to carry, the single `Glass_ADD` batch, was one of the six and not the
cause alone; the "naive wiring" that left the hull unchanged drew the wrong
texture (the glass's own) on the wrong batches. The hull model's `+0x1a8` is **1**
(read live), so the coordinates come from the fixed world-space basis and follow
the ship, not the camera. Evidence, the recorded GE list and the loader rule that
makes five of the six *replace* what is under them: `mesh-draw.md`, "The hull's
extra pass". Implementation: `oag_render::shine`, drawn after the hulls through
the absorb overlay's plumbing; `crates/game/tests/shine_ground_truth.rs`.

Matched poses (Talon's Junction, Assegai, 480x272, no filters; RGB means, spine
box of `talon-mid2`, ship box of `talon-fast`):

| Frame | Original | Ours without | Ours with |
| --- | --- | --- | --- |
| `talon-mid2` spine | 153, 154, 95 | 135, 139, 85 | 143, 148, 94 |
| `talon-mid2` whole ship | 142.5, 161.0, 161.4 | 134.6, 154.1, 155.2 | 137.1, 156.6, 157.8 |
| `talon-fast` ship box | 155.6, 173.2, 170.5 | 149.1, 161.7, 158.9 | 150.8, 163.4, 160.8 |
| `talon-corner` ship box | 14.4, 26.5, 27.8 | 35.0, 36.8, 60.2 | 35.4, 37.1, 60.5 |

The pass moves every measured region toward the original and never past it, and
the glint reads as a soft sheen on the inner wings and canopy rather than a flat
lift. **It does not close the gap**: a third to a half of it on the first two
frames, nothing on `talon-corner`, where the ship is mostly out of frame and the
difference is framing. What remains is the flare's bloom and small pose and
camera differences, and the pass's own unmeasured parts: one additive redraw
stands in for the original's replace-then-add, there is no fog on it, and the
airbrakes' batches are not deflected with their flaps.

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
