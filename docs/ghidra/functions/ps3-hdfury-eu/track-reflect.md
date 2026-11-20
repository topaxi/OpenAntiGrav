# HD's planar reflection: who fills the target, the mirror, the doubled lights

2026-11-20, lane `planar-water`. Static read of the PS3 `EBOOT.elf` (disc 02.00,
`BCES00664`, TOC `r2 = 0x008bd3c4` for every function below, `scripts/ps3-toc.py toc`),
checked against a new live frame of Moa Therma on RPCS3 0.0.43. The live read of the target,
its camera and the sea's sampling is [water.md](../../../rendering/water.md), "The planar
reflection, read off live frames"; this page is the code behind it. What ours draws from it:
water.md, "The planar reflection, drawn".

## The law

```text
Scene_PrepareFrame, once a frame (0x003aa888):
    bind the shared screen-space target, clear colour 0xff000000, depth/stencil 0xffff, mask 0xf3
    h = env.WaterPlaneHeight                    ; env + 0x528, copied to frame + 0x7d14
    if eye.y <= h: run RenderTrackRefract       ; eye = frame + 0x100, engine slot 4
    else:          run RenderTrackReflect

TrackReflect_RenderJob (0x006cf358):
    alpha test, blend and cull off (methods 0x304, 0x310, 0x183c = 0)
    Sky_DrawCube with the sky matrix mirrored
    M = mirror about h:  diag(1, -1, 1) with translation -env[+0x528]
    viewProj   (slot 1)  = main view-projection composed with M   ; frame + 0x7de0
    prelitScaleSpecular (slot 12)    = frame + 0x7c00 = 2.0 * prelit scale (xyz)
    directionalLight0Colour (slot 15) = frame + 0x7bc0 = 2.0 * sun colour (xyz)
    cull face FRONT (0x404)
    draw the reflect chunk list (0x00403a30)
    restore viewProj (+0xc0), slot 12 (+0x7bf0), slot 15 (+0x7bb0), cull face BACK (0x405)
```

`eyePositionWorldSpace` is not rebound: the reflected scenery's fog and specular see the main
eye.

## Evidence

**The two jobs and the switch** (`Scene_PrepareFrame`, `0x003aa888`; confidence 88). The
only reader of `"Job RenderTrackReflect"` (`0x007b10a0`, TOC slot `-0x612c`) is this function,
and `"Job RenderTrackRefract"` (`0x007b10b8`, slot `-0x6124`) sits beside it. After
`Rsx_SetColorClearValue(0xff000000)`, `Rsx_SetZStencilClearValue(0xffff)` and
`Rsx_ClearSurface(0xf3)` (the clear water.md measured on the live target), the code at
`0x003ac010`..`0x003ac030` loads word 1 of `frame + 0x100` and `frame + 0x7d14`, `fcmpu`, and
`ble` to the refract branch (`0x003acc20`); the fall-through initialises the job at
`frame + 0x6e80` with vtable `0x0086c8c8` and the reflect name. `frame + 0x100` is bound to
engine parameter slot 4, `eyePositionWorldSpace` (`stw r4, 0x98(r11)` at `0x003ab34c`: entry
`(0x98 - 0x18) / 0x20 = 4`, renderer.md's parameter table). `frame + 0x7d14` is written once,
at `0x003ab13c`, from `env + 0x528` (`lfs f13, 0x528(r22)` at `0x003ab120`).

**`env + 0x528` is `Water plane height`** (confidence 92). `Environment_RegisterLightingSchema`
(`0x003a83d8`) registers `env + 0x528` with TOC word `-0x18d6`, slot `0x008b706c`, which holds
`"Water.Water plane height"` (`0x007b0690`), and `env + 0x520` with `"Water.Water distortion"`
(`0x007b0678`); their defaults are `0xc2480000` = -50.0 and `0x42200000` = 40.0. Moa Therma
authors -50 and 40. Vineta K authors 100, so its eye is below `h` and the refract job runs there,
which is the behind-the-glass target Vineta K's glass reads.

**The reflect job** (`TrackReflect_RenderJob`, `0x006cf358`; confidence 85). The vtable
`0x0086c8c8`'s fourth entry is the descriptor `0x0088ba00`, whose code is `0x006cf358`. In it:

- `0x006cf410`..`0x006cf438`: `Rsx_SetMethod` `0x304`, `0x310`, `0x183c` to 0; then
  `Sky_DrawCube` (`0x005ecc28`) with a matrix built from the frame's sky matrix (water.md: the
  target's sky draws carry the main sky rotation with its `y` row negated).
- `0x006cf9b0`..`0x006cfa18`: `lfs f0, 0x7d14(r31)`, `fneg`, stored at `0xa4(r1)`, beside
  `(1.0, -1.0, 1.0, 1.0)` at `0x80(r1)` and `(0, -h, 0, 1)` at `0xa0(r1)`; the two vectors are
  masked into a 4x4 and multiplied into the frame's view-projection, stored at
  `frame + 0x7de0..0x7e1f`. The AltiVec composition was not multiplied out by hand; the live fit
  on two frames, two months apart, gives `y' = K - y` with `K = -50.0000` (residual 0.00004 and
  0.00007), so the plane is at `h / 2` and the offset is `h` itself.
- `0x006cfa28`: `Rsx_SetCullFace(0x404)`, then the slot writes at `0x006cfc90`..`0x006cfcd8`:
  `+0x38` (slot 1, `viewProj`, 4 vectors) = `frame + 0x7de0`; `+0x198` (slot 12,
  `prelitScaleSpecular`) = `frame + 0x7c00`; `+0x1f8` (slot 15, `directionalLight0Colour`) =
  `frame + 0x7bc0`; then `bl 0x00403a30`; then the restores at `0x006cfd0c`..`0x006cfd54`
  (`frame + 0xc0`, `+0x7bf0`, `+0x7bb0`) and `Rsx_SetCullFace(0x405)`.

**The doubled lights** (confidence 85 static, live partial). `Scene_PrepareFrame` fills
`frame + 0x7bb0` from `env + 0x430` (`"Lighting.Sun color"`, slot `-0x18f4`) and `frame + 0x7bf0`
from `env + 0x4c0` (`"Lighting.Prelit ambient colour scale"`, slot `-0x18e8`), then at
`0x003ab074`..`0x003ab118` loads the vector at TOC `-0x61d4` (`0x007b0da0` =
`{2.0, 0, 0, 0}`), splats word 0, multiplies the `x`, `y`, `z` of both by it and selects the
original `w` back (`vsel` with a lane mask), storing `frame + 0x7bc0` and `frame + 0x7c00`. The
main view binds `+0x7bf0` to slot 12 (`stw r3, 0x198(r11)` at `0x003ab36c`) and `+0x7bb0` to
slot 15 (`0x003ab410`).
**Live:** in the new Moa Therma frame's fragment-program memory, 15 programs carry the sun
colour doubled, `(8.0, 7.686, 6.620)`, and 174 the authored `(4.0, 3.843, 3.310)`; no main-view
sea or scenery program does. Program addresses are reused within a frame, so which draw wrote
each is not attributable: this supports the law and does not prove it per draw.

**The reflect list** (`TrackReflect_DrawList`, `0x00403a30`; confidence 72). The reflect job's
only chunk draw. It walks 6-byte records `(model, chunk, flags << 1)` from `scene + 0x371ec` to
the count at `scene + 0x3d1ec` and picks each chunk's fog as the behind-the-glass target does:
bit 5 `Scene_GetAlternateFog`, else `Scene_GetPrimaryFog`, bit 0 in a Zone race the Zone fog.
Which chunks the list holds is read live, not here: the scenery in the target is the render
flag bit 3 chunks (water.md), 257 on Moa Therma and 73 on Ubermall
(`hd_water_ground_truth::the_reflection_target_draws_the_bit_3_chunks`). Who fills
`scene + 0x371ec` is not read.

**The refract job** (`TrackRefract_RenderJob`, `0x006ce6e0`; confidence 75). Vtable
`0x0086c8e0` (initialised with `"Job RenderTrackRefract"`), fourth entry `0x0088b9f8` ->
`0x006ce6e0`. Same frame as the reflect job (the three `Rsx_SetMethod` 0 calls, `Sky_DrawCube`,
a matrix into the `viewProj` slot), and it draws its list through `0x003fc140`, not
`0x00403a30`. Not read further; `0x003fc140` is left unnamed (hypothesis: the behind-the-glass
bucket's drawer).

## Live corroboration (2026-11-20)

RPCS3 0.0.43-20574, `Write Color Buffers` on, audio `"Null"`, `OAG_COMPARE_PLATFORM=hd-fury`,
the window at 1920x1080. A new Time Trial state on Moa Therma (`scripts/emu-hd-timetrial-state.sh
<lane> 2 9 0 hd-timetrial-moa-therma-harimau-std-start-2611`, standard Harimau; the October state
is refused by this RPCS3), restored with `--restart`, the craft placed at `210 38 -380` facing `-z`
(`SETTLE=0.4`; 0.8 s let a craft still at race speed fall off) by `scripts/hd-sea-place-dump.py`
with `HOOK_LIGHT=1 HOOK_NO_HEADS=1 HOOK_FP_LEN=0x900`. `scripts/hd-sea-reflect-read.py`:

- the target `0x02a20000`, format `0x123`, clip `0x3c0 x 0x21c`, clear `0xff000000` / `0xffff`
  mask `0xf3`, 14 draws (6 sky, 8 scenery); 5 sea draws sample it on unit 3, and no other draw;
- the mirror `K = -50.0000`, residual 0.00007; `distortion` 40 on all five sea programs;
- unit 3 is fetched with gamma nibble 0 (`SET_TEXTURE_ADDRESS` `0x60030303`), raw, on every sea
  draw, where the sea's `DiffuseTexture` units are 0 or 7 by material.

## Confidence

| Claim | Address | Confidence |
| --- | --- | ---: |
| Reflect or refract, by `eye.y > Water plane height` | `0x003ac010`..`0x003ac030` | 88 |
| `env + 0x528` is `Water plane height`, default -50; `+0x520` `Water distortion`, 40 | `0x003a83d8` | 92 |
| The reflect job: sky, mirrored `viewProj`, cull front, the list, restore | `0x006cf358` | 85 |
| The mirror is `y' = h - y` (plane `h / 2`) | static `fneg` of `+0x7d14`, live fit | 90 |
| Sun colour and prelit scale bound doubled in the reflect pass, `w` kept | `0x003ab074`..`0x003ab118`, `0x006cfc90`.. | 85 |
| `0x00403a30` is the reflect pass's chunk-list drawer, fogged by bit 5 | `0x006cfce8` | 72 |
| `0x006ce6e0` is the refract job | vtable `0x0086c8e0` | 75 |
| The sea reads the target raw (no gamma decode) | live, unit 3 address word | 85 |

## Not read

- What the refract job puts in the target on a circuit whose sea samples it, when the eye drops
  below `h` (ours clears the target to black and draws nothing).
- Who fills `scene + 0x371ec` (the reflect list), and whether it is culled against a mirrored
  frustum, the main one or none (ours keeps the visible set and skips the frustum).
- `0x003fc140`, the refract job's list drawer.
