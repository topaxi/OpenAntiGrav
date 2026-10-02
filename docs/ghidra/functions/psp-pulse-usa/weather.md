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

## Not done, and why

- **The mist overlay** (`FUN_088fa0a0`, built from `Tex`, `Alpha`, `Drift*`, `MistInside`).
  Unread.
- **Outpost 7's open-air snow does not match.** Ours draws visible white flakes in the sky.
  Three original frames at an open section (`07_Track`, section 18, `env_flags 0x2013`, `64`
  flakes live) show none. `WO_SNOW`'s size channel is `Random`, `12`/`12`, and its draw is
  unread, so ours is probably too big or too bright. Open.
- **The noise clock** is the manager's `0x08ab224c` (seconds, measured: it ran `0.57` per
  wall second under a slow emulator); ours is the race clock and a seeded table.
- **The original updates at its frame rate**: the wind step is not scaled by `dt`, so on the
  PSP's 30 fps the fall is half as fast in time. Ours is 60 Hz.
- **Other titles.** The PS2 disc authors `<Weather>` on `07`, `11` (not on the PSP disc) and
  `14`; none is played. Wipeout HD carries neither effect.
