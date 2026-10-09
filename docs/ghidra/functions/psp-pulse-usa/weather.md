# Pulse's weather: what `Weather_Update` does with the rain, the snow and the lens

2026-10-02. Read off the decompile and checked against PPSSPP 1.20.4 (software
renderer) on Fort Gale White and Outpost 7 White. Ported in
`oag_raceplay::scenery_fx::{weather, lens}`, `oag_vex::weather` and
`oag_fx::psys::field`. The node and trigger are in
[placed-particle-systems.md](placed-particle-systems.md).

## Corrections to earlier pages

- **`weatherPos` (`0x3da`) IS the weather's placement anchor.** An earlier reading
  said it does not place rain or snow. It does not name the effect, but
  `WeatherPos_Init` stores each node at its section's `+0x64`, and `Weather_Update`
  reads that node's world matrix when the camera is in a covered section.
- `FUN_08912930` is a per-section lookup of that anchor, not a camera-matrix source.
- The effect is **not hidden** under cover. Instance flag `0x200000` selects a
  world-anchored frame instead of the view-space one.

## The functions

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x0892c528` | `WeatherPos_Init` | 85 | The class's `+0x7c` slot. Walks up to the nearest ancestor that parents a `section` node (`ancestor+0x54`), collects that section node, writes the `weatherPos` to `section+0x64` and its world matrix to `weatherPos+0x60`. |
| `0x0887866c` | `Section_IsCovered` | 85 | A section `< 0`, or a bit set in the 64-bit mask `DAT_08b3bfd0`/`d4`. `Vex_LoadModel` sets bit `i` for each `section` whose `+0x64` is filled. Live mask on Fort Gale `0x43fc78001c`; `oag_vex::weather` reproduces it from the file. |
| `0x08878644` | `Camera_PublishSection` | 80 | `DAT_08ab10a4 = DAT_08ab10a0; DAT_08ab10a0 = section`: last frame's and this frame's. `Weather_Update` does nothing unless they differ. |
| `0x088f1570` | `Weather_UpdateWind` | 85 | Two filtered Perlin noises: heading (`2 pi` swing, `0.06` Hz) and strength (`WindRange`, `0.15` Hz) on `WindBase`. Wind `= (cos a, 0, sin a) * strength`, `DriftY` added to `y`. Matches live: heading `-0.156`, strength `5 - 4.02`, `(0.9678, -10, -0.1525)`. Also `node+0x420 = atan(-wx / wy)` of the wind in the camera frame, the lens azimuth (live `0.0155` against `0.0152` predicted). |
| `0x088fb11c` | `Modifier_WrapBox` | 80 | Modifier type `0x13`, authored by `WO_RAIN` and `WO_SNOW`. `params[0..3]` is the wind (`Weather_Update` writes `0.3 *` the wind every frame), `params[4] = 50` the box half-extent. Per particle: hold the world still against the camera's move (view-space instance), add the wind, wrap a whole `2e` on any axis that left. With flag `0x200000` the wind is added raw and the camera is ignored. |
| `0x088fbcec` | `ParticleSystem_EmitRect` | 85 | Shape 2: `(U(-1,1) * ex, 0, U(-1,1) * ez)`, `ex`, `ez` the scaled `+0x34`, `+0x38`; velocity from `ParticleSystem_AimedVelocity` with a fixed `(0,0,1)`. |
| `0x088f2918` | `Perlin_Noise1` | 85 | Perlin's one-dimensional gradient noise on a 256-entry permutation. |
| `0x088f2aa4` | `Perlin_Fractal` | 85 | `sum 0.5^(k-1) * noise(k * (clock * freq + phase))`, times `amp`. The table is filled from `rand()` on first use. |
| `0x088fa0a0` | `WeatherMist_Construct` | 85 | Built by `Weather_Construct` on a `0x2c0`-byte node with `[Alpha, AspectRatio, TexScale, DisplayScale, Tex]` (the config words `0x40`, `0x43`, `0x41`, `0x42` and the `Tex` string, read off the caller's copy loop). Stores them at `+0x94`, `+0x84`, `+0xb0`, `+0x90`; `+0x88 = 0.02`, `+0x8c = 1.0`; the two layers' phases `0` and `0.5`; four `Psys_RandFloatRange(0, 0.99)` UV offsets; two four-vertex quads at clip `(1,-1) (1,1) (-1,-1) (-1,1)`; a texture node (`FUN_089277ac`) for `Tex` at `+0x190`. Vtable `0x08ad0f44`. |
| `0x088f99f8` | `WeatherMist_Update` | 80 | Vtable `+0x24`. Derives `+0xc4 = TexScale`, `+0xc8 = TexScale * AspectRatio`, `+0xc0 = 1`, `+0xbc = AspectRatio`, samples the camera's motion (`CameraMotion_Sample_q`), and when that reports motion and `+0x94 > 0` steps both layers. |
| `0x088f9a88` | `WeatherMist_Enqueue` | 75 | Vtable `+0x34`: `Gfx_Enqueue(g_display, this, 0x4f000000)`. |
| `0x088f9ab4` | `WeatherMist_Draw` | 85 | Vtable `+0x44`. Nothing when `+0x94 <= 0`. Identity projection and view, texture `+0x190` bound with repeat wrap, depth test, culling and lighting off, blend `Gu_BlendFunc(ADD, SRC_ALPHA, FIX, 0, 0xffffff)` (additive), then two `Gu_DrawArray(TRIANGLE_STRIP, 0x183, 4)` quads (float UV, float XYZ), each in the colour at `+0x170`/`+0x180`. |
| `0x088fa38c` | `WeatherMist_UpdateLayer` | 85 | One layer's step, below. Live-checked: UV extents and colour alpha match the formula to four places. |
| `0x088f959c` | `CameraMotion_Sample` | 88 | Read to the instruction and recomputed live (2026-10-04, below). `f12` is the update virtual's `dt` in seconds (`WeatherMist_Update` never writes it before the call; live `1/60`). Copies the view matrix (`0x08b32d00`) and the camera node's (`DAT_08ab10b0`) matrix `+0x40` (axes as columns, `-eye` in the fourth row). Out: the move `cur - prev` of `-eye`, minus `drift * dt`, put in view axes by `vtfm3.t C000,E100,C200` on the view rows (`y` then negated) at `+0xf0..`; the yaw and pitch steps of the back axis at `+0x110`/`+0x114`; `g_camera_roll` at `+0x118`. Returns 0 only when the view or the back axis is zero. |
| `0x0897ed98` | `acosf` | 90 | libm wrapper; its error path names `"acosf"` (`0x08a91830`). |
| `0x08ab10a8` | `g_camera_roll` (data) | 88 | Written by `Camera_SubmitScene` (`0x08878fe8`): `acosf` of the `y` of the camera's up with its horizontal-back part removed, negated when that vector leans against the horizontal right. Live: matches the value recomputed from the camera matrix to `0.004` rad. `CloudGroup_Draw` turns its sprites by `g_camera_roll - phase`, checked live on 28 sprites (`clouds.md`, 2026-10-04). |
| `0x088f9494` | `Quad_RotateScaleUv` | 80 | Rotates a vertex's `(u, v)` about a centre by an angle and scales it about the same centre. |
| `0x0897f388` | `fmodf` | 90 | libm wrapper; its error path names `"fmodf"` (`0x08a91880`). |
| `0x0897eed0` | `asinf` | 90 | Same shape, names `"asinf"`. |
| `0x0897f008` | `atan2f` | 90 | Same shape, names `"atan2f"`. |

## What a player sees

- **In the open** the env effect's `32` rain drops (8 a tick, 4 ticks of life, read live)
  fill a 100-unit cube `60` units ahead of the camera (`res+0x98`). The world holds still
  under it, so drops streak with the camera's own motion plus a fall of `0.3 * DriftY` a
  tick (`3` on Fort Gale, `0.45` on Outpost 7).
- **Under cover** (a section with a `weatherPos`) the same cube sits at that anchor in the
  world. Fort Gale's anchors cover 16 sections, Outpost 7's 20.
- **The lens** (Fort Gale): droplets on the glass, `6` units ahead, a `10 x 5.625`
  rectangle, running down the screen along the wind. See
  [`lens.rs`](../../../../crates/raceplay/src/scenery_fx/lens.rs) for the cover edge's
  lifetimes (`80/40`), drag (`0.97`) and speed factor (`0.3`). The original puts the
  lifetime back only when its `0.3` s timer expires and the speed factor never; ported as read.
- The switch is edge-triggered on the published section, and both sections start at `-1`
  (covered).

## The snow's draw (2026-10-04)

Read off the pool draw and measured on PPSSPP 1.20.4 (software renderer), Outpost 7
White, Venom. Ported: `oag_fx::psys` now honours flag `0x800`.

- **The flakes never die.** `WO_SNOW`'s emitter authors flags `0x5001805` (the loaded
  resource reads `0x5101805`), a one-tick duration, one burst of `64`, and a
  `5`-tick life. Flag `0x800` makes `ParticleSystem_InitParticle` store `FLT_MAX`
  instead (see [particle-system.md](particle-system.md)). Read live: every one of the
  `64` pool particles carries `FLT_MAX` at particle `+0x88`/`+0x8c`, and the pool holds
  `64` throughout. Ours ignored the flag, so its flakes died five ticks in and the
  pool stayed empty for the rest of the race. That was the whole of the 2026-10-03
  mismatch. Confidence **90**.
- **How they are drawn** (`ParticleSystem_DrawEmitterPool` `0x08918bf8`, then
  `ParticleSystem_DrawRolledQuads` `0x089178c0`, render index `res+0xb8 = 2`). Under
  resource flag `0x1000000` and in the open (instance flag `0x200000` clear), the
  particle matrix is identity with translation `(0, 0, -res+0x98)`, so a flake at box
  position `p` is drawn at view-space `p - (0, 0, 45)`. Under cover it is the instance
  matrix times the view. Each flake is a rolled square of half-size `12` (the size
  channel, `x` times `res+0x4c8`), colour `0x64ffffff` (white at `100/255`),
  depth-tested, additive. A breakpoint on the `Gu_DrawArray` at `0x08917c5c` read
  `384` vertices (`64 x 6`) a frame, view `z` in `[-93, 4]`.
- **Why they are faint.** The sprite is a `128 x 64` sheet, a 2 x 2 atlas of opaque
  black cells with small grey specks (brightest texel `147`). Additive at `100/255`,
  the most a speck can add is about `58`. Each flake is a scatter of faint specks, not a
  white blob.
- **The 2026-10-03 reading was wrong.** "The original shows no flakes" missed this faint
  additive signal. Measured A/B at one stationary pose: section-mask forced open at the
  start line, frames with `res+0xb8` set to `2` and to `9` (a draw index that draws
  nothing). The difference is a field of faint specks over the upper half of the screen,
  invisible in the full frame at player size. Ours at the same pose and mode gives the
  same picture: its largest per-pixel difference is `58`, the bound above. Scratch frames:
  `t3-*.png`, `ours2/start-*.png`, `cmp-start*.png`.

## The mist overlay (2026-10-04)

`Weather_Construct` builds it from `Tex`, `Alpha`, `TexScale`, `DisplayScale` and
`AspectRatio`. Both circuits author it (`Data\Tex\ScreenFX\Mist.mip`; `Alpha 0.3`,
`TexScale 3.1`, `DisplayScale 0.8`, `AspectRatio 0.5625`, `DriftMistMult 6` on Fort Gale; Outpost 7's
`Alpha` read live as `0.3`). Two layers of the same texture, additive over the
whole screen, zooming towards the camera as it moves forward and crossfading so the
zoom never visibly restarts.

**The trigger.** The weather node holds the overlay's opacity at `+0x450` and a
target at `+0x454`. `Weather_UpdateWind` eases it `+= (target - it) * 0.1` once per
whole `1/60` s in the frame. On the edge into the open the target becomes `Alpha` and
on the edge into cover `0`, both only while `MistInside` (an int, `Xml_AttributeAsInt`)
is `0`. A non-zero `MistInside` leaves the target alone on both edges. Being an int,
Outpost 7's authored `MistInside="0.2"` reads `0`: live on its start line (section 1,
covered, mask `0x400c79f47f`) the target is `0` and the opacity `4e-8`. Each frame
`Weather_Update` writes `+0x450` into the overlay's `+0x94` and
`-(0.3 * wind) * DriftMistMult` into its drift `+0xa0`. Live on Outpost 7: `+0x450`
climbs to `0.30` in open section 18 and falls toward `0` in covered sections 2 and 16.
Confidence **85**.

**One layer's step** (`WeatherMist_UpdateLayer`, layer `L` = 0 or 1, `s = 0.02`):

```text
phase_L = fmodf(phase_L + dz * s, 1)  (+1 if negative)      ; dz: camera's forward move, view axes
t = 1 - phase_L                     ; a wrap of more than 0.5 re-rolls the layer's UV offsets
scroll_u += dx * t * s * TexScale;  scroll_v += dy * t * s * TexScale * Aspect
yaw_u    += dyaw * Aspect * t * TexScale; pitch_v += dpitch * t * TexScale * Aspect
alpha_L = (t < 0.5 ? 2t : 2 - 2t) * opacity
u0 = rand_u + (1 - t) * TexScale / 2 + fmodf(yaw_u + scroll_u, 1)
v0 = rand_v + (1 - t) * TexScale * Aspect / 2 + fmodf(pitch_v + scroll_v, 1)
quad UVs (u0, v0) (u0, v1) (u1, v0) (u1, v1), u1 = u0 + t * TexScale, v1 = v0 + t * TexScale * Aspect
each UV rotated by the camera roll about the quad's UV centre and scaled by DisplayScale * tan_half_fov
```

The quad's corners are clip-space `(1,-1) (1,1) (-1,-1) (-1,1)`: `u0` is on the right.
Layer 1 starts half a phase on, so one layer fades in as the other fades out. Live
check, Outpost 7 (`TexScale 3.1`, `Aspect 0.5625`, `DisplayScale 0.8`,
`tan_half_fov 0.6249`): at `t = 0.684` the predicted `u` extent `1.060` and `v` extent
`0.596` match the vertices read, and at `t = 0.184` `0.285`. The colour alpha matched
`(2 - 2t) * opacity` (`0.00515` read, `0.00515` predicted). Confidence **85** for the
layer law; the camera-motion terms **88**, checked against a live turn below.

### The camera motion, read to the instruction and checked live (2026-10-04, `pulse-mist`)

`CameraMotion_Sample` (`0x088f959c`) keeps its state at `mist+0x1a0` (`S`): the camera
node at `S+0`, the view matrix at `S+0x10`, this frame's node matrix at `S+0x50` and
last frame's at `S+0x90`. The node matrix stores the camera's axes as **columns** and
`-eye` as its fourth row ([camera.md](camera.md)). So `S+0xd0..d8` = `(m02, m12, m22)`
is the camera's **back** axis in the world, and `S+0xdc..e4` is last frame's.

```text
yaw_c   = -atan2f(back.x, back.z)            S+0x104 ; prev at S+0x10c
pitch_c = -asinf(back.y)                     S+0x100 ; prev at S+0x108
dyaw    = -wrap(yaw_c - yaw_p)               S+0x110 ; wrap folds into (-pi, pi]
dpitch  =  wrap(pitch_c - pitch_p)           S+0x114
m       = (-eye_c) - (-eye_p) - drift * dt  ; drift = mist+0xa0, dt = f12, seconds
(dx, dy', dz) = view rotation applied to m  ; vtfm3.t C000,E100,C200 at 0x088f9964
dy      = -dy'                               S+0xf0, S+0xf4, S+0xf8
roll    = g_camera_roll (0x08ab10a8)         S+0x118
```

The view rotation is settled by a second call site: `Camera_SubmitScene` builds the
view matrix's translation from `-eye` with the **same** instruction on the same rows
(`vtfm3.t C000,E100,C200` at `0x08878b50`), so `m` lands in exactly the space the eye
does. In our terms, with `right`, `up`, `back` the camera's axes:
`dx = m . right`, `dy = -(m . up)`, `dz = m . back`. Driving forward, `m` is
`+back * speed`, so `dz > 0`.

**Live, Fort Gale White, Assegai, Time Trial, PPSSPP software renderer, 23 pauses**
(a throwaway script, not kept, `check.py`, `fg-turn1.json`):

- **Yaw and pitch recompute exactly** (to five places, every sample) from the back axes
  left in `S+0xd0..e4`. Holding **left**, `dyaw` is **positive** (`+0.008` to
  `+0.025` a frame) while `atan2(back.x, back.z)` grows; holding **right** it is
  negative (`-0.014` to `-0.024`). A climb was not driven; `dpitch` is checked by the
  recomputation only, on a small natural slope (`-0.001` to `+0.002`).
- **The drift and `dt`**: at the start line, before the craft moves,
  `(dx, dy, dz) = (-0.087, 0.302, 0.041)` against `view(drift) = (5.30, 17.81, 2.84)`:
  `dy = (up . drift) * dt` with `dt = 0.0170`, `1/60`. `dx` has the sign
  `-(right . drift) * dt` predicts. Fort Gale's drift is `-(0.3 * wind) * 6`, so the
  `DriftY` fall reads `17.8` units a second in the view's up.
- **The roll** matches `g_camera_roll` recomputed from the node matrix to `0.004` rad.
- Confidence **88** for the sampler. Not 94: a climb or a dive was not driven, and the
  first call's previous state (the node's initial bytes) was not read.

**The first call.** The sampler compares against whatever `S+0x90..` holds, and nothing
seeds it, so the first step is a one-off jump. It only moves the layers' phases and
scrolls by an arbitrary amount.

### Where the mist draws, and with what (2026-10-04, `pulse-mist`)

- **Under the HUD.** A live dump of the render queue (`display+0x16a0`, count at
  `+0x5520`, `g_display = *0x08abf5d4`) on Fort Gale holds the mist node
  (vtable `0x08ad0f44`) at key `0x4f000000`. The HUD's `Image` widgets (vtable
  `0x08acce34`, [head2head.md](head2head.md)) sit at `0x58......`, and the 3-D widget
  views (`Mode3D_EnterView`, `0x52000001`/`0x60000001`/`0x65000001`) above that. The
  bloom (`0x08ad148c`) is last, at `0x70......`. The queue sorts ascending
  (`Gfx_CompareQueueKeys`), so the mist draws after every world layer
  (`0x30`..`0x4d`), in the screen flash's own layer, and before the HUD. Read live, **88**.
- **The glow mask is protected.** `WeatherMist_Draw` calls
  `Bloom_SetGlowMaskWritable(g_bloom, 0)` (`0x08907828`) before drawing, as
  `ScreenFlash_Draw` does: the mist never feeds the bloom.
- **The texture.** `Data\Tex\ScreenFX\Mist.mip` is a `128 x 64` 8-bit paletted image
  whose palette is white with a ramped alpha. The frame's texture function is
  `GU_TFX_MODULATE`/`GU_TCC_RGBA` ([mesh-draw.md](mesh-draw.md)) and its filter is
  bilinear on magnification, so a layer adds `texel alpha * vertex alpha` of white.
  The vertex colour is the float colour truncated to bytes.
- **The opacity's start.** `Weather_Construct` sets both `+0x450` and its target
  `+0x454` to `Alpha`, and the ease is `n = (int)(dt / 0.016666668)` steps of
  `+= (target - it) * 0.1`.

### Played, and checked against the original (2026-10-04, `pulse-mist`)

`oag_fx::mist` (the law and the pipeline) and `oag_raceplay::scenery_fx::mist`
(the opacity, the drift and the edges). Off the disc's own `Mist.mip` and `<Weather>`
values; a circuit whose `Tex` does not decode draws no mist and says so in the load report.

**The draw law against the original's pixels.** At a stationary pose, the original's
opacity was poked to `0` and back, and its
added light was predicted from the quad UVs and colours dumped in the same pause, the
unswizzled `Mist.mip` alpha, bilinear repeat sampling and the byte-truncated alpha
(`predict.py`). Masked to pixels the rest of the scene left alone:

| Pose | Predicted mean | Measured mean | Median residual | Correlation |
| --- | ---: | ---: | ---: | ---: |
| Fort Gale White start line | 29.7 | 30.2 | 3.0 | 0.84 |
| Outpost 7 White, section 2 forced open | 27.8 | 25.4 | 1.0 | 0.92 |
| Ours, Fort Gale start, from our own dumped UVs | 39.5 | 37.2 | 0.4 | 0.96 |

The same check with the quad mirrored in `u`, `v` or both gives correlations of
`-0.36`, `0.03` and `-0.28`, so the corner mapping is measured rather than assumed.
The original's residual includes the drift between the dump and the frame grab.
Ours draws a different random offset and phase, so its mean differs per frame (texel
sampling, not a gain): `37` at our start against the original's `30`.

**Frames at player size**: `cmp-start.png` (original
left, ours right) and `cmp-start-diff.png` (each one's on/off difference, times 3);
`cmp-fg-run.png` (Fort Gale driving: open, tunnel, a left turn, original left);
`cmp-o7-and-turn.png`: Outpost 7's start line, a matched stationary pose, both
without mist (covered section 1); then the original forced open beside ours in open
sections 7 and 11 (`--autopilot`, opacity `0.30` and `0.29`, mean add `32` and `28`);
then a held left turn. In both games the open reads as a soft white haze over the whole
world and none of the HUD, and a covered section (Fort Gale's tunnel, Outpost 7's start
and sections 1 to 6) shows none.

**Not matched, and why.** The frames are not a matched pose while driving: the
original's craft hit walls and its random offsets differ, so the haze's pattern differs
frame to frame. The first sampler step and the layers' starting `t` are chosen (see
`oag_fx::mist`). Not compared: a climb or a dive, and Fort Gale's rain lens over
the mist.

## Not done, and why

- **The noise clock** is the manager's `0x08ab224c` (seconds, measured: it ran `0.57` per
  wall second under a slow emulator); ours is the race clock and a seeded table.
- **The original updates at its frame rate**: the wind step is not scaled by `dt`, so on the
  PSP's 30 fps the fall is half as fast in time. Ours is 60 Hz.
- **Other titles.** The PS2 disc authors `<Weather>` on `07`, `11` (not on the PSP disc) and
  `14`; none is played. Wipeout HD carries neither effect.
- **The PS2 executable has the same law** (2026-10-04, static, confidence **75**). In
  `SCES_547.48` the `TrackStartup` parser `FUN_00146668` reads the same attribute list
  (`MistInside` at `0x002a46f8`, referenced at `0x00146870`). Under the same gate
  (`DAT_002dabd0 < 0xe || DAT_002dab08 < 5`, and not mode `6`), it builds a `0x890`-byte
  node through `FUN_001c4b58`. That node has the PSP `Weather_Construct`'s shape: a
  `0x2c0`-byte mist node (`FUN_001ccb28`), the `EnvPsys` spawned as `FXW1` and the
  `ScreenPsys` as `FXW2` in mode `2`. PS2 `WO_SNOW` sets `0x800` as well (flags
  `0x5041805`). Nothing PS2-specific was run live. PS2 is corroboration only, so if the
  PS2 weather is wired, it plays the PSP law. Not wired: the PS2 port's circuits
  load no weather here.
