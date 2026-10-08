# Does HD have a motion blur or a speed blur? `FunkLayerZoom`, measured (2026-10-08, `hd-motion-blur`)

**Short answer.** HD/Fury has **no motion blur**: no pass of its post chain samples a
previous frame's picture or a velocity, at rest, at full speed, or through a Turbo. What
it has is one **boost-driven zoom pass**, `FunkLayerZoom`, that draws a warped, slightly
enlarged copy of the quarter-resolution scene over the scene for about a second after a
boost starts. It is a pulse that fires on an event, not a function of speed: at 440
km/h with no boost the pass does not run. Everything below is the evidence, then what
is still unread.

Confidence per claim is in "Confidences" at the end; the rubric is
[confidence-rubric.md](../../../reverse-engineering/confidence-rubric.md). The capture
method is [rpcs3-capture.md](../../../reverse-engineering/rpcs3-capture.md), "Capturing
one frame's draws" and the `hd-motion-blur` section there.

## What was captured

`scripts/rpcs3-hd-postchain.py` boots HD on RPCS3, walks to a race on Talon's Junction
(Feisar `concept1`, the same walk every HD capture here uses), taps through the flyby and
countdown, and dumps one paused frame of RSX draws per state with
`scripts/rpcs3_draw_hook.py`. At the same pause it reads the live `FunkLayer` object
(`0x00c50ee0`) so the pass's two inputs are known for the frame, not inferred from
timing. A frame counts only if it is **complete**: a draw on the bloom chain's last target
(`0x02240000`) followed by a draw on a screen buffer (`0x00010000` or `0x00394000`).
A paused frame can be cut mid-write and then lacks exactly the passes read here; two
early boot frames (`run3/boost3`, `run3/boost4`) were such frames and were discarded.
`HOOK_NEED_SCREEN=1` makes the hook retry until the frame is complete.

States: at the grid with nothing held; throttle held for 6 s (436 to 438 km/h on the
straight, read off the HUD); and `state 4` (Turbo) written to the pickup slot then
triangle, as in [weapons.md](weapons.md)'s recipe, with the dump 0.15 to 0.7 s after the
press (the wall clock around a retried dump is not exact, which is why the live input is
read instead). Raw captures: `data/scratch/hd-motion-blur/run{1,2,3,4,5}` (gitignored).

## Which programs ran

Names are `scripts/ps3-registry.py`'s registry resolution (each name to its `SHO` block,
exact), and each live fragment program was matched to its executable block by
normalised microcode (`data/scratch/hd-motion-blur/fpmap.py`), each vertex program by
its uploaded microcode (`vpmap.py`). Both matches are byte-for-byte on the opcode
stream, constants and patch slots aside.

| Pass | Block | Ran at rest | at 438 km/h | in a Turbo (E > 0) |
| --- | --- | --- | --- | --- |
| `FunkLayerBloomDownsample_fp` | `0x92d780` | yes | yes | yes |
| `FunkLayerBloomGate_fp` | `0x92d580` | yes | yes | yes |
| `FunkLayerBloomBlurVertical_fp` / `Horizontal_fp` | `0x92d880` / `0x92dc80` | yes | yes | yes |
| `FunkLayerZoom_vp` + `_fp` | `0x92e580` + `0x92b180` | **no** | **no** | **yes** |
| `FunkLayerBloomRadial_vp/_fp`, `RadialGate_fp` | `0x92eb80`, `0x92d000`, `0x92ce80` | no | no | no |
| `FunkLayerBlendBuffer_vp/_fp` | `0x92ed80`, `0x92e080` | no | no | no |
| `FunkLayerCopy`, `CopyAlpha`, `CopyBlend`, the seven `FunkLayerDof*` | `0x92cd80`.. `0x92c000` | no | no | no |

Complete frames, with the live `FunkLayer` inputs read at the pause (`E` is `+0x64`,
`P` is `+0x58`, viewport 0; the Zoom column is the vertex constant `size` of the Zoom
draw, `1 + the pass's jitter`):

| Frame | State | `P` | `E` | Zoom pass | `size` (x, y) |
| --- | --- | ---: | ---: | --- | --- |
| `run3/rest0` | grid | 0 | 0 | absent | |
| `run4/rest0` | grid | 0 | 0 | absent | |
| `run3/speed1` | 438 km/h | 0 | 0 | absent | |
| `run4/speed1` | 436 km/h | 0 | 0 | absent | |
| `run3/boost5` | after a Turbo, over | 0 | 0 | absent | |
| `run4/boost4` | after a Turbo, over | 0 | 0 | absent | |
| `run3/boost2` | Turbo, early | 0 | 0.895 | **present** | 1.05532, 1.03984 |
| `run4/boost2` | Turbo, early | 0 | 1.000 | **present** | 1.03178, 1.03700 |
| `run4/boost3` | Turbo, early | 0 | 0.965 | **present** | 1.04144, 1.03176 |

Every complete frame with `E > 0` has the Zoom pass and every complete frame with
`E = 0` and `P = 0` lacks it: nine of nine, three boots. The earlier boots' complete frames
without the live read agree (`run1/boost4`, `run2/boost3` have it, at 1.026/1.025 and
1.011/1.011; their at-rest, at-speed and late-boost frames lack it), and so do the
whiteout lane's captures of a rival being hit, which hold only the four bloom programs.

**No previous-frame sample.** Across every complete frame no draw binds a screen buffer
(`0x00010000`, `0x00394000`) as a texture, so the picture is never read back a frame
later; `data/scratch/hd-motion-blur/texscan.py` is the check. The Zoom pass's source,
`0x02300000`, decodes to a 320x180 copy of the **current** scene (the dump of
`run4/boost2` shows the same picture as the frame, one quarter of the size).

## What the Zoom pass is

One draw, after the bloom blurs and before the HUD, into the scene copy
(`0x00cc0000`):

- **96 vertices**: a CPU-built mesh of 24 quads, rewritten every frame
  (`FUN_003b4690`, the loop over 24 records of `0xa0` bytes, four vertices of ten floats:
  position, two UV pairs, colour). `idx96` in the draw list is this draw's count.
- **Programs**: `FunkLayerZoom_vp` (`0x92e580`: `pos = v.xy * size.xy + size.zw`,
  `TC0 = (uv0, uv1) * uvSize.xyxy + uvSize.zwzw`, colour passed through) and
  `FunkLayerZoom_fp` (`0x92b180`, below).
- **Constants of the live draw**: `size = (1.0x, 1.0x, 0, 0)`, `uvSize = (1, 1, 0, 0)`,
  `clampUvs = (1/640, 1/360, 1 - 1/640, 1 - 1/360)`, the half-texel border of a 320x180
  texture.
- **Blend**: RGB source alpha over one-minus-source-alpha, the draw's alpha written
  as `one` (register words `0x10302` / `0x303`).
- **Fragment program**, read from the microcode:

```text
tap0 = tex(clamp(TC0.xy))              tap1 = tex(clamp(TC0.zw))
rgb  = 1 - col.rgb * (1 - (tap0 + tap1))
a    = 1 - (1 - col.a)^2               ; col = the vertex colour
```

- **Vertex colour, per the CPU loop** (`FUN_003b4690`, the block that writes record
  `+0x40..+0x4c`): with `P` the damage pulse and `E` the engine-glow input,
  `col = (1 - 0.9 P, 1 - 0.08 P, 1, max(E, P) * k)`, the constants `0.1` and
  `0.92` being TOC words `0x8b745c` and `0x8b7460`, `k` the tuning struct's
  per-viewport `1.0` (`0x008c2b70`, initial value in the ELF). With `P = 0`, the boost case,
  the colour is white and the pass adds `tap0 + tap1` at alpha `1 - (1 - E)^2`.
- **UV warp**: the two UV pairs are `(1 - F) * screen + F * template`, `F` being
  `E * 0.15` across and `E * 0.95` down (tuning struct `+0x64` and `+0x68`, initial
  `0.15` and `0.95`). The template is the mesh's own coordinates; **the 24-quad
  template is not decoded here** (open, below), so the shape of the warp is not
  reproduced by this page.
- **Size jitter**: `size.x = 1 + A2`, `size.y = 1 + A3`, each updated every frame by
  `A = 0.75 * A + (rand() % 100) * 0.0002 * E` (`FUN_0029f070`). The mean is
  `0.0396 E`, the bound `0.079 E`. The measured sizes for `E` of 0.9 to 1.0 are
  0.032 to 0.055, all inside the bound, mean 0.038 (six draws). With `E = 0` the size
  is exactly 1.0 (a bomb frame, below).

What it looks like: `data/scratch/hd-motion-blur/run4/boost2.png` (E = 1.0, 647 km/h)
against `run4/speed1.png` (436 km/h). At the boost the world on the right edge and the
ceiling stretches outward from the screen centre, the HUD fades toward the middle, and
the scene reads as smeared along the radius. That is the look the stored
`talons-matched/01.png` capture ("529 km/h, heavy motion blur") has. It is a warped
duplicate of the scene laid over the scene with an alpha that rises to nearly 1 within
a quarter of a second, not a blur of the previous frames.

## What drives it

`FunkLayer` (`0x00c50ee0`) keeps two per-viewport floats the pass reads: `+0x58`, the
damage pulse `P`, and `+0x64`, the engine-glow input `E`. The runner draws the pass
when `E > 1e-4` or `P > 1e-4`.

**`E` is a boost-start pulse** (`FUN_0029ef40` fires it, `FUN_0029f070` shapes it; both
reached from `EngineFlare_Update` at `0x002a3100`), an array at `0x00ad7880`:

```text
trigger:  A0 = 0.8                       (TOC word 0x8b2eb0; one write)
each update, dt in game seconds:
          A0 = max(0, A0 - dt * 8/7)     (0x8b2ed4 = -1.142857)
          while A0 > 0 and 1 - 1.25 A0 > 0.075:
                A1 = clamp(5 * ((1 - 1.25 A0) - 0.075), 0, 1)
          once A0 <= 0:  A1 = max(0, A1 - 0.035)       (per update, not per second)
E = A1
```

At 60 updates a second that is: nothing for 0.05 s, a ramp to 1.0 at 0.26 s, a hold
until 0.70 s, a decay over about 0.48 s, so **about 1.2 s end to end**, and a Turbo
press measured 0.2 s in reads `E = 0.895` to `1.0`. The three early reads agree with the ramp
(`0.895` at about 0.19 s); a pause in the hold reads `1.0` and the frame after the
pulse reads `0`. The decay runs per update, so on a slow frame rate it lasts longer in
game time.

**`P` is a damage pulse.** `Ship_ApplyDamage` (`0x000e7760`) passes `1.8 * damage`
(TOC word `0x8a8ad0`, `0x3fe66666`) to `Hud_RaiseDamagePulse` (`0x000849b0`), which
keeps `HUD+0x67c = max(old, clamp(amount, 0, 1))`; `Hud_Update` (`0x0009e3d0`)
decays it by `1.6667 * dt` (TOC word `0x8a7bf0`) and hands it to `FunkLayer_SetDamagePulse`
(`0x003af820`, through the cross-TOC stub `0x006774d8`). The pass then draws with
`E = 0`, so `size = 1` and the colour is the red-tinted one above. Static reading only so
far; the bomb frames of `run5` test it (below).

## What is not there

- **`FunkLayerBlendBuffer_fp`** (`0x92e080`) is a noise-driven UV displacement
  (`uv += (noise(uv * (16, 9) * a).xy - 0.5) * b`), not a frame blend, and it never
  draws: its two handles (`FunkLayer+0xa0`, `+0xa4`) are read in the runner only as the
  early-out test at the top (`if either is null, return`). **Confidence 80.**
- **`FunkLayerBloomRadial_fp`** (`0x92d000`) is a 14-tap radial blur toward `screenOrigin`
  with weights summing to 10.4, and the runner binds only the radial *gate*
  (`+0xc4`), never the blur (`+0xbc`, `+0xc0` are not bound anywhere in
  `FUN_003b4690`). The radial branch is gated by `Radial bloom Enabled`, `0` on every
  circuit but `zone_1`, so it could not appear on Talon's. Its strength term looks like
  a view-direction dot product cubed, a sun glare, but the AltiVec decompile does not
  read cleanly: **below 70, not named**.
- **`FunkLayerDof*`, `Copy*`** never ran in any race frame. DoF is not chased here.

## Port decision

Not ported this lane. The pass is a boost effect, and a contained one, but three pieces
of its law are unread: the 24-quad template that shapes the warp, what fires
`FUN_0029ef40` (the call site is a virtual call; its vtable was not found, and the
OPD `0x881ae8` has no static reference), and the exact blend of the history in
`0x02300000`. Wiring it from a guess would draw a different picture than the original's.
The open work is a handover thread, with the capture script already reading the mesh.

It is **not a motion blur**, so the motion blur setting's `original` value
(maintainer's rule, 2026-10-05) is not offered on HD.

## Other titles

- **Omega**: **not checkable here** (no PS4 emulator in the toolchain). Its settings
  registrar names a `MotionBlur` group beside Vignette and DepthOfField
  ([lightmap-prelit.md](../ps4-omega-eu/lightmap-prelit.md)), so HD's answer does not
  carry over; checked, differs, by strings only.
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
| The Zoom pass runs in every complete frame with `E > 0` and in none with `E = 0, P = 0` (nine frames, three boots) | 90 |
| No pass samples a previous screen buffer; no blur, radial, BlendBuffer, DoF or Copy program ran (the same frames) | 85 |
| Block names (`ps3-registry.py` and exact microcode matches) | 92 |
| The Zoom pass draws the quarter-res current scene over the scene, as described | 85 |
| `E` follows the pulse law above | 70 (static, agrees with one ramp read) |
| `P` is the damage pulse | 75 (static chain, test below) |
| The vertex colour and UV warp formulas | 70 (decompile of `FUN_003b4690`, scalar parts) |
| BlendBuffer and Radial blur never draw | 75 |

## Open

1. The 24-quad template: `scripts/rpcs3-hd-postchain.py` now reads it live
   (`<stem>-zoommesh<v>.bin`).
2. What fires `FUN_0029ef40`: a Turbo does; pads, rolls, the start boost are untested.
3. Whether the history blend in `0x02300000` changes the Zoom pass's picture.
4. Radial: read it on `zone_1`, where it is enabled.
