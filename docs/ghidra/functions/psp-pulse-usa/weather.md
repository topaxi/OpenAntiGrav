# Pulse's weather: what `Weather_Update` does with the rain, the snow and the lens

2026-10-02. Read off the decompile and checked against PPSSPP 1.20.4 (software
renderer) on Fort Gale White and Outpost 7 White. Ported in
`oag_game::race::scenery_fx::{weather, lens}`, `oag_vex::weather` and
`oag_render::psys::field`. The node and trigger are in
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
| `0x088f959c` | `CameraMotion_Sample` | 65 | Copies the view matrix (`0x08b32d00`) and the camera node's (`DAT_08ab10b0`) world matrix, and against last frame's gives the position delta minus `drift * dt` in view axes (`y` negated) at `+0xf0..`, the yaw and pitch deltas (`atan2f`/`asinf` of the forward, wrapped to `+-pi`) and `DAT_08ab10a8` (the camera roll). Returns 0 on its first frame. Read, values live-plausible, the sign of each term not checked against a turn: `_q`. |
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
  [`lens.rs`](../../../../crates/game/src/race/scenery_fx/lens.rs) for the cover edge's
  lifetimes (`80/40`), drag (`0.97`) and speed factor (`0.3`). The original puts the
  lifetime back only when its `0.3` s timer expires and the speed factor never; ported as read.
- The switch is edge-triggered on the published section, and both sections start at `-1`
  (covered).

## The snow's draw (2026-10-04)

Read off the pool draw and measured on PPSSPP 1.20.4 (software renderer), Outpost 7
White, Venom. Ported: `oag_render::psys` now honours flag `0x800`.

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
  `data/scratch/pulse-weather/orig/t3-*.png`, `ours2/start-*.png`, `cmp-start*.png`.

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
is `0`. A non-zero `MistInside` leaves the target alone on both edges. Each frame
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
layer law; the camera-motion terms' signs **65** (not checked against a turn).

## Not done, and why

- **The mist overlay is recovered, not played.** Ours has no screen pass for it yet.
- **The noise clock** is the manager's `0x08ab224c` (seconds, measured: it ran `0.57` per
  wall second under a slow emulator); ours is the race clock and a seeded table.
- **The original updates at its frame rate**: the wind step is not scaled by `dt`, so on the
  PSP's 30 fps the fall is half as fast in time. Ours is 60 Hz.
- **Other titles.** The PS2 disc authors `<Weather>` on `07`, `11` (not on the PSP disc) and
  `14`; none is played. Wipeout HD carries neither effect.
