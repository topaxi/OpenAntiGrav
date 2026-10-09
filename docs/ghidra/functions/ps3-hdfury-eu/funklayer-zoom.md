# Does HD have a motion blur or a speed blur? `FunkLayerZoom`, measured (2026-10-08, `hd-motion-blur`)

**Short answer.** HD/Fury has **no motion blur** in the usual sense: no per-object or camera
velocity pass, and nothing that depends on speed. What it has is `FunkLayerZoom`, a
**radial zoom-streak ring** drawn over the screen's periphery by a short pulse: the ship
boosting (`E`, about 1.2 s from the start of a Turbo) or taking damage (`P`, 0.6 s from a
hit). It re-draws a quarter-resolution copy of the scene with its texture coordinates
pulled toward the centre, so the edges of the picture streak along the radius, strongest at
the corners and fading to nothing 62 % of the way from the centre to the edge. At 438 km/h with no boost and
no damage the pass does not run. The stored 529 km/h capture ("heavy motion blur") shows the signs
of this pass (the periphery smeared along the radius, the centre sharp) and no draw list
exists for it, so it is consistent with the pass and not proved to be it. Everything below is the evidence, then
what is still unread.

Confidence per claim is in "Confidences" at the end; the rubric is
[confidence-rubric.md](../../../reverse-engineering/confidence-rubric.md). The capture
method is [rpcs3-capture.md](../../../reverse-engineering/rpcs3-capture.md), "Reading which
post-chain programs ran, state by state".

## What was captured

`scripts/rpcs3-hd-postchain.py` boots HD on RPCS3, walks to a race on Talon's Junction
(Feisar `concept1`, the walk every HD capture here uses), taps through the flyby and the
countdown, and dumps one paused frame of RSX draws per state with
`scripts/rpcs3_draw_hook.py`. At the same pause it reads the live `FunkLayer` object
(`0x00c50ee0`) and the Zoom mesh it builds, so the pass's inputs are known for the frame
and not inferred from timing. A frame counts only if it is **complete** (a draw on the
bloom chain's last target `0x02240000` followed by a draw on a screen buffer,
`0x00010000` or `0x00394000`): a paused frame can be cut mid-write and then lacks exactly the
passes read here. Several early frames were such halves and were dropped.
`scripts/rsx-frame-census.py` applies the test and prints each frame's row below.

States: the grid with nothing held; throttle held for 6 s (436 to 438 km/h on the straight,
read off the HUD); `state 4` (Turbo) written to the pickup slot and triangle pressed, the
recipe of [weapons.md](weapons.md); and a bomb fired standing still. Raw captures:
`run{1..7}` (gitignored).

## Which programs ran

Names are `scripts/ps3-registry.py`'s registry resolution (each registered name to its `SHO`
block, exact). Each live fragment program was matched to its executable block by its
opcode stream with constants dropped (`scripts/rsx-fp-names.py`), and each vertex program by
its uploaded microcode (`scripts/rsx-vp-names.py`); both matches are byte for byte on the
opcodes.

| Pass | Block | at rest | at 438 km/h | in a Turbo or a hit |
| --- | --- | --- | --- | --- |
| `FunkLayerBloomDownsample_fp` | `0x92d780` | yes | yes | yes |
| `FunkLayerBloomGate_fp` | `0x92d580` | yes | yes | yes |
| `FunkLayerBloomBlurVertical_fp` / `Horizontal_fp` | `0x92d880` / `0x92dc80` | yes | yes | yes |
| `FunkLayerZoom_vp` + `_fp` | `0x92e580` + `0x92b180` | **no** | **no** | **yes** |
| `FunkLayerBloomRadial_vp/_fp`, `RadialGate_fp` | `0x92eb80`, `0x92d000`, `0x92ce80` | no | no | no |
| `FunkLayerBlendBuffer_vp/_fp` | `0x92ed80`, `0x92e080` | no | no | no |
| `FunkLayerCopy`, `CopyAlpha`, `CopyBlend`, the seven `FunkLayerDof*` | `0x92cd80` to `0x92c000` | no | no | no |

Complete frames with the live `FunkLayer` inputs read at the pause (`E` is `+0x64`, `P` is
`+0x58`, viewport 0; `size` is the vertex constant the Zoom draw used, `1 + jitter`):

| Frame | State | `P` | `E` | Zoom pass | `size` (x, y) |
| --- | --- | ---: | ---: | --- | --- |
| `run3/rest0`, `run4/rest0`, `run5/rest0`, `run6/rest0` | grid | 0 | 0 | absent | |
| `run3/speed1`, `run4/speed1`, `run6/speed4` | 436 to 438 km/h | 0 | 0 | absent | |
| `run3/boost5`, `run4/boost4` | after a Turbo, over | 0 | 0 | absent | |
| `run3/boost2` | Turbo | 0 | 0.895 | **present** | 1.05532, 1.03984 |
| `run4/boost2` | Turbo | 0 | 1.000 | **present** | 1.03178, 1.03700 |
| `run4/boost3` | Turbo | 0 | 0.965 | **present** | 1.04144, 1.03176 |
| `run6/boost1` | Turbo | 0 | 0.790 | **present** | 1.03400, 1.03714 |
| `run6/boost2` | Turbo | 0 | 0.685 | **present** | 1.03246, 1.03569 |
| `run6/boost3` | Turbo, wall hit | 0.210 | 0 | **present** | 1.00011, 1.00015 |
| `run5/bomb1` | standing, bomb blast | 0.276 | 0 | **present** | 1.0, 1.0 |

Every complete frame with `E > 0` or `P > 0` has the pass and every complete frame with
both zero lacks it: 16 of 16, four boots. The first two boots' complete frames without the
live read agree (a pass at 1.026 and 1.011 on boost frames, none at rest or at speed), and
so do the whiteout lane's frames of a rival being hit, which hold only the four bloom
programs.

**No frame samples the previous picture of the screen.** No complete frame binds a screen
buffer (`0x00010000`, `0x00394000`) as a texture, and no pass reads a velocity. One buffer
pair does carry the previous frame: see "The scene copy the pass draws" below.

## What the Zoom pass is

One draw per frame while it runs, after the bloom blurs and before the HUD, into the scene
copy (`0x00cc0000`), 96 vertices:

- **A ring of 24 quads, rebuilt every frame by the CPU** (`FUN_003b4690`: 24 records of
  `0xa0` bytes, four vertices of ten floats each: position `xy`, two UV pairs, colour `rgba`).
  The geometry is read live from the object (`FunkLayer + 0x130 + 4 * viewport`, `+0x10`;
  `<stem>-zoommesh0.bin`) and is constant: inner vertices at radius **0.62**, outer vertices
  at radius **1.5**, in 15 degree steps, positions in NDC. The inner vertices have vertex alpha
  0, the outer vertices `max(E, P)`, so the ring is invisible at its inner edge and full at the
  rim; the screen corners (radius 1.41) lie inside it.
- **Texture coordinates**: inner vertices sample the scene copy at their own screen position,
  `u = x / 2 + 0.5`, `v = 0.5 - y / 2`. Outer vertices take their own screen UV and pull it
  toward the inner vertex of the same angle by a fixed factor: **`F0 = 0.15` for the first
  tap and `F1 = 0.95` for the second**. Measured on `run6/boost1` and `boost2` (the outer
  vertex of record 0: `v0 = -0.1840` and `v1 = 0.1680`, from `-0.25` own and `0.19` inner)
  and unchanged at `E = 0` in `run6/boost3`. So the first tap is a mild magnification, and
  the second samples almost only the inner ring's colour along each radius: a radial
  smear. The CPU's own formula scales these factors by `E` when a flag byte
  (`FunkLayer + 0x70 + viewport`) is clear; in every measured frame it is set and the
  factors are the constants `0.15` and `0.95` (the tuning struct's initial data,
  `0x008c2b70 + 0x64` and `+ 0x68`).
- **Colour**: `rgb = (1 - 0.9 P, 1 - 0.08 P, 1)`, alpha `max(E, P)`. White for a boost
  (`P = 0`, measured: `(1, 1, 1)`); the damage tint is read from the CPU loop and not yet
  seen on a frame with a large `P` (open). The vertex alpha equals `max(E, P)` exactly:
  `0.825` against a live `E` of `0.79` (the live `E` is one update later and the decay is
  `0.035` an update), `0.72` against `0.685`, `0.2102` against `P = 0.2102`.
- **Programs**: `FunkLayerZoom_vp` (`0x92e580`: `pos = v.xy * size.xy + size.zw`,
  `TC0 = (uv0, uv1) * uvSize.xyxy + uvSize.zwzw`, colour passed through) and
  `FunkLayerZoom_fp` (`0x92b180`). Constants of the live draw: `size = (1 + jx, 1 + jy, 0, 0)`,
  `uvSize = (1, 1, 0, 0)`, `clampUvs = (1/640, 1/360, 1 - 1/640, 1 - 1/360)`, the half-texel
  border of a 320x180 texture. Blend: source alpha over one minus source alpha for RGB.
- **Fragment program**, read from the microcode:

```text
tap0 = tex(clamp(TC0.xy))              tap1 = tex(clamp(TC0.zw))
rgb  = 1 - col.rgb * (1 - (tap0 + tap1))
a    = 1 - (1 - col.a)^2               ; col = the interpolated vertex colour
```

- **Size jitter**: `size.x = 1 + A2`, `size.y = 1 + A3`, each updated every frame by
  `A = 0.75 * A + (rand() % 100) * 0.0002 * E` (`FUN_0029f070`): mean `0.0396 E`, bound
  `0.079 E`. Measured `E` of 0.69 to 1.0 give 0.032 to 0.055 (ten values, all inside the
  bound, mean 0.038). With `E = 0` it is 1.0 (a bomb frame) or `1.0001` (the frame after a
  boost, the decay of `A`).

What it looks like: `boost2.png` (E = 1.0, 647 km/h)
against `run4/speed1.png` (436 km/h). At the boost the world on the right edge and the
ceiling stretches outward from the screen centre, the HUD fades toward the middle, and
the periphery reads as smeared along the radius while the ship at the centre is sharp.

### The scene copy the pass draws

The pass samples `0x02300000`, a 320x180 buffer. It holds the scene: `run4/boost2`'s dump of
it (`<stem>-vram-02300000.bin`, pitch `0x500`) is the same picture as the frame. In **every**
frame, boost or not, the bloom chain keeps a **ping-pong pair**, `0x02300000` and
`0x022c0000`, one frame to each: it draws the quarter-resolution scene into one (blend off),
then draws the *other* buffer over it with blend on
(`tgt X <- tex(0x02280000)`, then `tgt X <- tex(Y)`, source alpha over one minus source
alpha, `X` and `Y` swapping every frame), and `Y` is never written earlier in that frame
(`history_reads` in `scripts/rsx-frame-census.py`). So the buffer is an accumulation of
earlier quarter-resolution scenes, an exponential moving average with a weight set by the
second draw's alpha. At rest the two buffers match (correlation 0.9997, `run6/rest0`). The
weight was solved on dumped buffers (half-resolution scene `0x02150000` box-filtered to
quarter size as `cur`, then `X = (1 - a) * cur + a * Y`, `run7`): **`a = 0` at 438 km/h
(`E = 0`; rms 0.29, exact) and `a = 0.048` at `E = 0.339`** (rms 0.71 against 1.46 at
`a = 0`). Two points, one frame at `E = 0` and one on the decay; a second speed frame was
unusable (the half-resolution buffer belonged to another frame). It looks tied to the pulse,
about `0.15 * E`, so the boost smear carries a little previous-frame ghosting, but two
points do not fix the law: open item 3. The program constants are patched in place per draw,
so a dump cannot give the alpha directly.

## What drives it

`FunkLayer` keeps two per-viewport floats the pass reads: `+0x58`, the damage pulse `P`, and
`+0x64`, the engine-glow input `E`. The runner draws the pass when `E > 1e-4` or
`P > 1e-4`, with alpha `max(E, P)`.

**`E` is a boost-start pulse** (`FUN_0029ef40` fires it, `FUN_0029f070` shapes it; both
reached from `EngineFlare_Update`, `0x002a3100`), an array at `0x00ad7880`:

```text
trigger:  A0 = 0.8                       (TOC word 0x8b2eb0; one write)
each update, dt in game seconds:
          A0 = max(0, A0 - dt * 8/7)     (0x8b2ed4 = -1.142857)
          while A0 > 0 and 1 - 1.25 A0 > 0.075:
                A1 = clamp(5 * ((1 - 1.25 A0) - 0.075), 0, 1)
          once A0 <= 0:  A1 = max(0, A1 - 0.035)       (per update, not per second)
E = A1
```

At 60 updates a second: nothing for 0.05 s, a ramp to 1.0 at 0.26 s, a hold until 0.70 s,
a decay over about 0.48 s, so **about 1.2 s end to end**. The live reads agree with the
shape: `E` of 0.895 on the ramp, exactly 1.000 in the hold, then 0.965, 0.790 and 0.685 on
the decay in steps of `0.035`, and 0 afterwards. The decay runs per update, so on a slow
frame rate it lasts longer in game time.

**`P` is a damage pulse.** `Ship_ApplyDamage` (`0x000e7760`) passes `1.8 * damage` (TOC word
`0x8a8ad0`) to `Hud_RaiseDamagePulse` (`0x000849b0`), which keeps
`HUD+0x67c = max(old, clamp(amount, 0, 1))`; `Hud_Update` (`0x0009e3d0`) decays it by
`1.6667 * dt` (TOC word `0x8a7bf0`) and hands it to `FunkLayer_SetDamagePulse`
(`0x003af820`, through the stub `0x006774d8`). A bomb frame read `P = 0.276` and a wall
hit during a Turbo `P = 0.210`; both drew the pass with `E = 0` and `size` 1.

**What fires `FUN_0029ef40` is unread** (a virtual call; its vtable was not found, and the
function descriptor `0x881ae8` has no static reference). A Turbo does; pads, rolls and the
start boost are untested.

## What is not there

- **`FunkLayerBlendBuffer_fp`** (`0x92e080`) is a noise-driven UV displacement
  (`uv += (noise(uv * (16, 9) * a).xy - 0.5) * b`), not a frame blend, and the runner never
  draws it: its two handles (`FunkLayer+0xa0`, `+0xa4`) are read only as the early-out test at
  the top of `FUN_003b4690` (if either is null, return). **Confidence 80.**
- **`FunkLayerBloomRadial_fp`** (`0x92d000`) is a 14-tap radial blur toward `screenOrigin`
  with weights summing to 10.4, and the runner binds only the radial *gate* (`+0xc4`), never
  the blur (`+0xbc` and `+0xc0` are not bound anywhere in `FUN_003b4690`). The radial branch is
  gated by `Radial bloom Enabled`, `0` on every circuit but `zone_1`, so it could not appear on
  Talon's. Its strength term looks like a view-direction dot product cubed, a sun glare, but
  the AltiVec decompile does not read cleanly: **below 70, not named.**
- **`FunkLayerDof*`, `Copy*`** ran in no race frame. DoF is not chased here.

## Port decision

Not ported this lane. The pass is a boost and damage effect, and the geometry, the UV
factors, the alpha law, the pulse laws and the fragment arithmetic are read and measured.
Three things stop a faithful port: the weight of the accumulation in `0x02300000` (open
item 3), what fires the boost pulse beyond a Turbo (open item 2), and the damage tint on a
frame with a large `P` (open item 4). Wiring those from a guess would draw a different
picture than the original's. The follow-up is a ready-to-wire handover thread, with the
capture script already reading the mesh, the accumulation buffers and the tuning struct.

It is **not a motion blur**, so the motion blur setting's `original` value (the maintainer's
rule of 2026-10-05) is not offered on HD.

## Other titles

- **Omega**: **not checkable here** (no PS4 emulator in the toolchain). Its settings
  registrar names a `MotionBlur` group beside Vignette and DepthOfField
  ([lightmap-prelit.md](../ps4-omega-eu/lightmap-prelit.md)), so HD's answer does not carry
  over; checked, differs, by strings only.
- **2048, Pure, Pulse**: HD's `FunkLayer*` names are HD's. Pulse's answer is in
  [bloom.md](../psp-pulse-usa/bloom.md).

## Names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x003b4690` | `FunkLayer_RunBloomChain` | 90 |
| `0x003af820` | `FunkLayer_SetDamagePulse` | 80 |
| `0x003af858` | `FunkLayer_SetEngineGlow` | 75 |
| `0x000849b0` | `Hud_RaiseDamagePulse` | 80 |
| `0x0029f070` | `EngineFlare_UpdateZoomGlow` | 68 |
| `0x0029ef40` | `EngineFlare_TriggerZoomGlow` | 66 |
| `0x00c50ee0` | `g_FunkLayer` | 85 |
| `0x00ad7880` | `g_EngineFlareGlowState` | 70 |
| `0x008c2b70` | `g_FunkLayerTuning` | 70 |

## Confidences

| Claim | Confidence |
| --- | ---: |
| The Zoom pass runs in every complete frame with `E > 0` or `P > 0` and in none with both zero (16 frames, four boots) | 90 |
| No velocity pass and no screen-buffer read-back; no blur, radial, BlendBuffer, DoF or Copy program ran | 85 |
| Block names (`ps3-registry.py` and exact microcode matches) | 92 |
| The ring mesh geometry, `F0 = 0.15`, `F1 = 0.95`, alpha `max(E, P)` (read live, four frames) | 90 |
| The fragment arithmetic and the blend, as read from the microcode | 80 |
| `E` follows the pulse law above | 80 (static law, five live reads agree with its ramp, hold and decay) |
| `P` is the damage pulse (static chain, two live reads) | 80 |
| The damage tint `(1 - 0.9 P, 1 - 0.08 P, 1)` | 60 (CPU loop only) |
| BlendBuffer and the Radial blur never draw | 75 |
| The scene copy is an accumulation of earlier frames, weight unread | 80 on the existence, 0 on the weight |

## What stalled the captures

Each state costs one boot, about 10 minutes to the first frame, then 3 to 8 minutes per dumped
frame: the hook retries a half frame after resuming for 0.5 s (3 to 15 retries was normal, 30
not enough twice), and the head-span stage reads about 800 small spans through the GDB stub.
A Turbo's pulse lasts 1.2 s, so a state is hit or missed by the retry clock; the live
`FunkLayer` read replaced timing for that reason. Two attempts (`speed` at attempt `None`,
`bomb:1.0`) never completed a frame in 30 retries. The lane stopped here at the maintainer's
request, to resume with faster tooling.

## Open

1. A frame with a large `P` and its mesh, to confirm the damage tint.
2. What fires `FUN_0029ef40`: a Turbo does; pads, rolls and the start boost are untested.
3. The accumulation weight in `0x02300000` (run `rpcs3-hd-postchain.py`, which now reads the
   half-resolution scene `0x02150000` and both quarter buffers at each pause, and solve
   `X_k = (1 - a) * cur + a * X_{k-1}`).
4. Radial: read it on `zone_1`, where it is enabled.
