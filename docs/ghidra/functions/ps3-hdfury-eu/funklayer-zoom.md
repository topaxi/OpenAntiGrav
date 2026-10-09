# Does HD have a motion blur or a speed blur? `FunkLayerZoom`, measured (2026-10-08, `hd-motion-blur`; ported 2026-10-09, `hd-zoom-ring`)

**Ported (2026-10-09, lane `hd-zoom-ring`).** The pass is `oag_post::hd_zoom`, HD's numbers are
`oag_hd::race::ZOOM_RING` (`Title` data, `RaceDefaults::zoom_ring`), the pulses are stepped per
simulation tick in `oag_raceplay::zoom` and fired by the player's speed pad and Turbo. What is
**chosen, not measured**, and what is left unwired, is in "Port" below. The accumulation weight, the
history crop, the tint and the trigger laws were measured in that lane and are in "Measured on
2026-10-09".

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
  (`P = 0`, measured: `(1, 1, 1)`). **The tint is confirmed on the live mesh** (2026-10-09, a Turbo
  into a wall, `P` up to 0.97): the outer vertices read `(0.690, 0.972, 1.0)` at `P = 0.3445`,
  `(0.259, 0.934, 1.0)` at `P = 0.8238`, `(0.334, 0.941, 1.0)` at 0.7395, `(0.409, 0.947, 1.0)` at
  0.6572, `(0.484, 0.954, 1.0)` at 0.5729: all of `1 - 0.9 P` and `1 - 0.08 P` to three places,
  alpha `max(E, P)` with `E` as low as 0.055. The inner vertices stay `(1, 1, 1, 0)` throughout.
  The `P` decay read off the same series is `1.67` a second. The vertex alpha equals `max(E, P)` exactly:
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

What it looks like (a frame not kept in the repository; E = 1.0, 647 km/h)
against `run4/speed1.png` (436 km/h). At the boost the world on the right edge and the
ceiling stretches outward from the screen centre, the HUD fades toward the middle, and
the periphery reads as smeared along the radius while the ship at the centre is sharp.

### The scene copy the pass draws

The pass samples `0x02300000`, a 320x180 buffer. It holds the scene: `run4/boost2`'s dump of
it (`<stem>-vram-02300000.bin`, pitch `0x500`) is the same picture as the frame. In **every**
frame, boost or not, the bloom chain keeps a **ping-pong pair**, `0x02300000` and
`0x022c0000`, one frame to each: draw 512 copies the quarter-resolution scene `0x02280000` into
one (blend off), then draw 513 draws the *other* over it with `SRC_ALPHA, ONE_MINUS_SRC_ALPHA`
(`tgt X <- tex(0x02280000)`, then `tgt X <- tex(Y)`, `X` and `Y` swapping every frame), and `Y`
is never written earlier in that frame (`history_reads` in `scripts/rsx-frame-census.py`). Both
draws run `FunkLayerBloomDownsample_fp`, whose microcode is
`rgb = tex(uv * C.z + C.x).rgb; a = tex.a * C.y + C.w` with `C` patched per draw, so **a dump
cannot give the alpha and the weight is read from the code instead**: see the next section. Draw
526, the ring, then draws into `0x00cc0000` - the scene the bloom ladder read at draw 510, **before**
the exposure resolve (draws 527 on, into the screen buffer) and the HUD.

### Measured on 2026-10-09: the history draw is a zooming feedback

`FunkLayer_RunBloomChain` (`0x003b4690`), the block gated by the byte `FunkLayer + 0xa87` and
ending in the history draw, builds the patched `float4` of draw 513 as `(x, y, z, w) = (c, 0,
1 - 2c, weight)`:

```text
weight = min(0.95, E * 10 * F[0] + 10 * F[0x250] + Env[0x54c])       (0x8b7518 = 10.0, 0x8b74e8 = 0.95)
c      = F[0x18 + 4 * viewport]
history sample = uv * (1 - 2c) + c           (UV = uv * C.z + C.x in the microcode)
history alpha  = tex.a * 0 + weight = weight (so the draw's alpha is the weight, whatever the texture holds)
X = (1 - weight) * scene_quarter + weight * Y(uv * (1 - 2c) + c)
```

Read live on RPCS3 (`scripts/rpcs3-hd-zoom-probe.py`, no pause): **`F[0] = 0.025`,
`F[0x250] = 0`, `Env[0x54c] = 0`, and `F[0x18] = 0.05 * E` exactly** (E = 0.2155 -> 0.01077, 1.0 ->
0.05, 0.615 -> 0.03075, 0.16 -> 0.008) on Talon's Junction and on Metropia. So
**`weight = 0.25 E` and `c = 0.05 E`**: the history is read cropped, `c` of each edge, so it
**zooms in by up to 10 % a frame while the pulse runs**, and each frame's copy of it is blended
back at up to a quarter. That is the streak: a recursive zoom, with the ring only deciding where
it shows. **It is a feedback, not a one-frame ghost**, which the earlier fit missed: it assumed an
unzoomed history (`c = 0`) and so read `a = 0.048` at `E = 0.339`.

Fitted on dumped buffers (`scripts/hd-zoom-history-fit.py`: half-resolution scene `0x02150000`
box-filtered to quarter size, then a scan of `w` and `c` for the least residual of
`X = (1 - w) * cur + w * warp(Y, c)`; `E_pause` is the live `E` at the pause, `E_eff` what the
fit's `w / 0.25` and `c / 0.05` say it was when the frame was drawn):

| `E` at the pause | fitted `w` | fitted `c` | `E_eff` from `w`, from `c` | rms fit, rms at `w = 0` |
| --- | ---: | ---: | --- | --- |
| 0 | 0 | 0 | 0 | 0.29, 0.29 (exact, two frames) |
| 0.30 | 0.090 | 0.0175 | 0.36, 0.35 | 2.26, 4.55 |
| 0.339 (on the ramp) | 0.055 | 0.0100 | 0.22, 0.20 | 0.37, 1.46 |
| 0.65 | 0.175 | 0.0350 | 0.70, 0.70 | 1.49, 9.13 |
| 0.685 | 0.185 | 0.0350 | 0.74, 0.70 | 2.20, 9.31 |
| 0.79 | 0.205 | 0.0425 | 0.82, 0.85 | 0.44, 4.00 |
| 0.86 | 0.230 | 0.0450 | 0.92, 0.90 | 1.98, 10.27 |
| 1.0 (hold, five frames, two circuits) | 0.265 | 0.0500 | 1.06, 1.00 | 1.5 to 2.6, 6.2 to 10.1 |

Eight levels of `E`, two circuits, and `w` and `c` name the **same** `E_eff` in every row, which is
the structure the law predicts and a free fit would not give. `E_eff` is one update off `E` at the
pause (0.30 -> 0.335, 0.65 -> 0.685, 0.79 -> 0.825, 0.86 -> 0.895; the 0.339 row is on the ramp,
where one update is `0.119` of `E`, so `E_eff = 0.22`): the chain draws what the previous update
set, the original's CPU/GPU pipelining, **not modelled** here. The fitted `w` at the hold reads
`0.265` against the law's `0.25`, a constant 0.005 to 0.015 high, which is the fit's own bias from
the box-filtered stand-in for the downsample (the same bias is on every row); `c`, which has no
such bias, is exact (`0.0500`). The rms floor at `E = 0` is 0.29 (the stand-in's own error).
**Confidence 85** on `weight = 0.25 E`, `c = 0.05 E` (static read, `F[0]`, `F[0x18]` read live on
two circuits, eight fitted levels).

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

At 60 updates a second: nothing for 0.05 s, a ramp to 1.0 at **0.19 s** (this page said 0.26 s
before the series below was read), a hold until 0.70 s, a decay over about 0.48 s, so **about 1.2 s
end to end**. The decay runs per update, so on a slow frame rate it lasts longer in game time.

**Read at 2 ms steps on 2026-10-09** (`A0` from the glow array at `0x00ad7880`, `E` from
`FunkLayer + 0x64`, one Turbo): `A0` falls `0.019048` an update (`8/7 / 60`), `E` follows
`5 * ((1 - 1.25 A0) - 0.075)` exactly (`A0 = 0.7238 -> E = 0.1011`, `0.7048 -> 0.2202`,
`0.6857 -> 0.3394`, `0.5714 -> 1.0`), holds at 1.0 for `A0` from 0.5714 to 0.019, and on `A0 = 0`
falls to 0.965, then by 0.035 each. Confidence 90 on the law (a static read and a series of 68
changes). `oag_post::hd_zoom::Pulse` steps it and its tests pin the ramp points.

**`P` is a damage pulse.** `Ship_ApplyDamage` (`0x000e7760`) passes `1.8 * damage` (TOC word
`0x8a8ad0`) to `Hud_RaiseDamagePulse` (`0x000849b0`), which keeps
`HUD+0x67c = max(old, clamp(amount, 0, 1))`; `Hud_Update` (`0x0009e3d0`) decays it by
`1.6667 * dt` (TOC word `0x8a7bf0`) and hands it to `FunkLayer_SetDamagePulse`
(`0x003af820`, through the stub `0x006774d8`). A bomb frame read `P = 0.276` and a wall
hit during a Turbo `P = 0.210`; both drew the pass with `E = 0` and `size` 1.

**What fires `FUN_0029ef40`** (a virtual call; its descriptor `0x881ae8` is not referenced by any
word in the image, so no vtable names it statically): measured by watching `A0` at 10 ms while the
race ran on Talon's Junction (`scripts/rpcs3-hd-zoom-probe.py --onsets`):

| Source | Fires the pulse? | Evidence |
| --- | --- | --- |
| A Turbo (state 4 written to the pickup slot, triangle) | yes, every time | dozens of pulses, `A0 = 0.8` |
| A speed pad | yes | onsets at race clock 3.6 s and 6.5 s, repeated on a second boot at the same positions (15.0/17.3 s and 15.6/17.9 s of the probe, i.e. 3.5 s apart from the race start); the screenshot 0.25 s after the first shows the pad chevron under the craft at 579 km/h (`data/reference/hd-capture/zoom-ring/pad-onset-0.png`) |
| The start boost | **not seen** | three starts with throttle held from 1.5 s, 3.5 s and 6 s after START RACE: no onset at `GO`. **Not conclusive**: none of the runs was shown to have had a start boost at all |
| A barrel roll | untested | |
| Damage | no (it is `P`, not `E`) | |

A write watch on `0x00ad7880`, the planned way to name the caller, was tried for a bounded 35
minutes and **did not resolve**: `Z0` fires only under the PPU interpreter (the state was restored
with `emu-restore-state.sh --interpreter`, which the second attempt applied), a breakpoint on
`0x0029ef40` took no hit in two 120 s runs (the race ran at about 20 % speed and reached 0:22, the
start pad possibly not crossed), so the caller stays unnamed. The Pulse flare has the same
shape (`Turbo_Fire`, the pad and a perfect start all call one flare boost, `exhaust.md`), which
makes a shared HD caller likely and a start-boost trigger plausible, but that is Pulse's evidence.

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

## Port

**Ported 2026-10-09** as HD's own pass, in this order of reading:

- `oag_post::hd_zoom` (`crates/post`): the pulses (`Pulse`, per tick), the ring's 96 vertices
  (`ring`), the history draw and the ring pass, `crates/post/shaders/hd_zoom.wesl`. The history
  draw runs right after the chain's quarter-resolution downsample (the blur passes overwrite that
  buffer) and the ring after the blurs and before the resolve, both inside `hd_bloom::Chain::run`
  (`hd_bloom/zoom.rs`), as the original orders them.
- `oag_hd::race::ZOOM_RING` (`oag_title::ZoomRing`, `RaceDefaults::zoom_ring`): every number above,
  `None` on every other title (ADR-0058).
- `oag_raceplay::zoom`: the pulse lives in the race's view state, steps once a tick, is fired by
  the **player's** speed pad (`pads.rs`) and Turbo (`weapons.rs`), is posed by `--pose-boost AGE`,
  and is read once a frame as `Race::hd_zoom_frame()` (the one read-only accessor added to
  raceplay).

**Chosen, not measured** (no confidence score):

- **Per-tick stepping** of `E`, `P`, the size jitter and the history draw. The original steps per
  update; the decay `0.035` is per update and the crop compounds per update, so a per-frame step at
  144 fps would shorten the pulse and lengthen the streak.
- **The space the three buffers blend in.** The original's scene, ladder and history are all
  8-bit surfaces in one space; here they are all in this chain's scene space with the clamp
  applied where the ring reads and writes. A blend that is a blend in the original's space is a
  blend here; the 8-bit rounding is not reproduced.
- **The history runs every tick the chain has a pulse object**, including `E = 0` (weight 0, a
  plain copy), so a pulse always finds a current `Y`. The original's gate for the block (the byte
  at `FunkLayer + 0xa87`) is not read.
- **The size jitter's random** is a render-side xorshift, not the original's `rand()`.
- **The damage tint's other branch** (`FunkLayer + 0x50` above zero uses the tuning struct's
  `0x70..0x78` constants) is not drawn; the field read 0 in every capture.

**Left unwired, and why:**

- **`P` has no trigger.** The pass draws it (tint, decay) and tests pin it, but the unit
  `Ship_ApplyDamage` hands `Hud_RaiseDamagePulse` (`1.8 * damage`) is not related to one of our
  hits by any page, so nothing raises it.
- **The start boost and a barrel roll** do not fire it (see the table above).
- **An opponent's boost** does not: the original's pulse is the viewing craft's.
- **The one-update lag** between the pulse and the frame the original shows is its CPU/GPU
  pipelining.

**Compared as a player would** (`data/reference/hd-capture/zoom-ring/`, four RPCS3 frames of one
Turbo at `E = 0.577` on the ramp, `1.000` in the hold, `0.195` and `0.103` on the decay, paused at
the stated `E`; ours at the matching poses `--pose-boost 0.133 / 0.4 / 1.083 / 1.15`, side by side in
`sheet-ours-vs-original.png`): the periphery smears along the radius while the craft and the centre
stay sharp, strongest in the hold, gone by `E = 0.1`, and the HUD is drawn after the ring in both.
The scene differs (ours is the grid on Talon's with the default craft, the originals are racing
at speed), so the comparison is of the ring's character and its envelope, not of pixels.

**It is not a blur**, so the motion blur setting's `original` value (the maintainer's rule of
2026-10-05) is not offered on HD, and there is no setting for the ring: it is the title's own.

## Other titles

- **Omega**: **checked, differs, by strings only** (2026-10-09). `eboot.bin` has no string
  containing `Zoom` and its `FunkLayer` strings are `FunkLayerReplayBar_vp/_fp`, the
  `FunkLayer.zone.cpp` and `FunkLayer.PS4/Fp/FpXml/FpXmlParser.cpp` paths: none of HD's
  `FunkLayerBloom*` or `FunkLayerZoom*` program names, and `FunkLayerZoom` and `FunkLayerBloom`
  occur in no extracted Omega file. Its settings registrar names a `MotionBlur` group beside
  Vignette and DepthOfField ([lightmap-prelit.md](../ps4-omega-eu/lightmap-prelit.md)) and it
  authors a `Tonemap` block, not HD's bloom. **Not proved absent**: its shader packs are not
  unpacked here and there is no PS4 emulator to run it. So not wired (`zoom_ring` is `None`).
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
| The damage tint `(1 - 0.9 P, 1 - 0.08 P, 1)` | 90 (CPU loop, then read on the live mesh at five values of `P`) |
| BlendBuffer and the Radial blur never draw | 75 |
| The history draw is a feedback, `weight = 0.25 E` (cap 0.95), crop `c = 0.05 E` | 85 (static read, `F[0] = 0.025` and `F[0x18] = 0.05 E` live on two circuits, eight fitted levels of `E`) |
| The boost pulse follows the formula, ramp at 0.19 s | 90 (read at 2 ms steps against the static law) |
| A speed pad and a Turbo fire the pulse | 85 (repeated onsets at fixed track positions; a Turbo every time) |
| The start boost and a roll do or do not fire it | 0 (not seen, not conclusive; untested) |
| The ring draws into the scene before the exposure resolve and the bloom add, and before the HUD | 85 (the draw list: 526 into `0x00cc0000`, 527 on into the screen buffer) |

## What stalled the captures (the 2026-10-08 attempt) and what replaced it

Each state cost one boot, about 10 minutes to the first frame, then 3 to 8 minutes per dumped
frame (the hook retried half frames and read about 800 small spans). **Replaced** on 2026-10-09 by
restoring a save state (about 10 s, `emu-restore-state.sh`) and reading only the three buffers
the fit needs through the GDB stub (`scripts/rpcs3-hd-zoom-capture.py`, about 25 s a frame, with
`Write Color Buffers` on in the private config - off, the buffers read as stale noise). The
inputs (`E`, `P`, `F[0]`, `F[0x18]`, the ring mesh) are read through `/proc/<pid>/mem` without a
pause (`scripts/rpcs3-hd-zoom-probe.py`), so a pulse's value is read, not timed.

## Open

1. What fires `FUN_0029ef40` beyond a pad and a Turbo: the start boost (first show that the
   original grants one in the run), a roll, and the caller itself (a write watch on `0x00ad7880`
   needs the interpreter and a pad crossing within its slow run).
2. `P`'s trigger: how `Ship_ApplyDamage`'s `damage` relates to a hit of ours.
3. The gate byte `FunkLayer + 0xa87` of the history block and the `+0x50` branch of the ring's colour.
4. Radial: read it on `zone_1`, where it is enabled.
