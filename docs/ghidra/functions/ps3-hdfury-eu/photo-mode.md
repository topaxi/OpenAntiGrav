# HD/Fury: photo mode

Read 2026-11-06, lane `photo-read`, `/hdfury/EBOOT-ps3-hdfury-eu.elf`, TOC `0x008ad4d8` (every
function here is below `0x32d5e0`, so Ghidra's default TOC is right, `memory.md`). Player-facing
law: [docs/ui/photo-mode.md](../../../ui/photo-mode.md).

`FUN_00241940`'s caller question from `renderer.md` is closed: it is the screen's own update, a
vtable entry of `PhotoMode_Screen.cpp`'s class.

## Functions

| Address | Name | Conf. | What it does |
| --- | --- | ---: | --- |
| `0x00241940` | `PhotoMode_ScreenUpdate` | 90 | `this+0xfc += dt`, then on `g_GamePaused`, dispatch on screen name, both the state machine's and the screen's own |
| `0x0023e570` | `PhotoMode_ScreenEnter` | 90 | Saves stick settings, rebuilds the effect list, sets exposure and saturation |
| `0x0023f568` | `PhotoMode_Update` | 90 | `InGame Photo`, per frame |
| `0x0023f4c0` | `PhotoMode_EnterTransition` | 72 | `InGame Taking Photo Transition Screen` |
| `0x0023ed48` | `PhotoMode_TakeShot` | 90 | `InGame Taking Photo`: name, then one of the two save paths |
| `0x0023f170` | `PhotoMode_WaitForSave` | 90 | `InGame Taking Photo Saving`: flashes the icon, exports, removes the temp file |
| `0x002d5118` | `Renderer_SaveFrameJpeg` | 90 | Single frame: JPEG writer at colour target size, quality 90 |
| `0x002d59d0` | `Renderer_SaveTiledJpeg` | 72 | Stitched: width and height times a tile count, tiles `photomodeTile_` |
| `0x000a9a50` | `Camera_AddPhotoOrbitA` | 78 | `+0x294 += stick * 0.05`, wrapped at `+-2 pi` |
| `0x000a9a90` | `Camera_AddPhotoOrbitB` | 78 | `+0x298 += stick * 0.05`, wrapped at `+-2 pi` |
| `0x000a9ad0` | `Camera_AddPhotoTilt` | 78 | `+0x29c += stick * 0.05` (unclamped) |
| `0x000a9a20` | `Camera_AddPhotoPanA` | 78 | `+0x2ac = clamp(+0x2ac + stick * 0.2, -5, 5)` |
| `0x000aa6e0` | `Camera_AddPhotoPanB` | 78 | `+0x2a8 = clamp(+0x2a8 + stick * 0.2, -5, 5)` |
| `0x000aa740` | `Camera_AddPhotoFov` | 78 | `+0x2b0 = clamp(+0x2b0 + stick, 10, 100)` |

Constants `0x008a7dd8` to `0x008a7dec`: `0.05`, `2 pi`, `-2 pi`, `0.2`, `-5`, `5`; `0x008a7e10` is
`10`, `0x008a7e54` is `100`. All read out of the ELF.

## Entry and defaults

Sliders come from `ingame_definition.xml`, screen `InGame Photo`. **`DATA00`'s copy is the live one**
(`docs/formats/hd-frontend.md`, confidence 92). Its `Photo Mode Top Level` subtree equals `DATA06`'s
with `delay=` and whitespace dropped, and `DATA00`, `DATA03`, `DATA05` and `DATA06` differ in layout
only. `DATA02`, the base game, has an earlier form (`Photo Use DOF` offers `FE_OFF`/`FE_ON`).

**The authored defaults of `Exposure` and `Saturation` do not survive entry.**
`PhotoMode_ScreenEnter`, on entry from anywhere but `InGame Photo Message`, writes both sliders
(`0x0023e9bc` and before):

```text
Exposure slider = "%d%%" of (int)(g * -33.3 + 33.3)      g = table at *(r2+0x3e20)[max(0, *(r2+0x3e24))]
Saturation slider = "20%"
```

`g` is the current grade bias and `-33.3 / 33.3` is the exact inverse of the per-frame law below,
so the sliders open on the picture already on screen; the same line stands at `0x8112e6bc` in 2048
with the same `"20%"`. `g` lives in BSS (`0x00c51134`); it read `0.3452` on Vineta K (Live, below).

The effect list is **rebuilt in code**, not read from the XML (`_opd_FUN_0015a278` clears it, then
`_opd_FUN_0015b298` adds): `FE_OFF`, `MSC_PHOTO_DEPTH`, and `MSC_PHOTO_MBT`, `MSC_PHOTO_MBS` only
when `RaceManager +0x1970` is 2 or 3 (`gantry-clock.md`: phase 2 is racing). The `<aEntry>` lines are
dead markup. So motion blur is offered during a race and after the flag, and not on the grid.

## `PhotoMode_Update`

Per frame, in order. Slider values reach the code as strings read with `"%f"`:

| Row | Range, step (XML) | Law | Target |
| --- | --- | --- | --- |
| `Photo Camera Mode` | `internal`, `close`, `track` | below | camera mode 3, 4, 5 of `_opd_FUN_000abc20` |
| `Photo Hud Display` | `FE_OFF`, `FE_ON`, default off | per local player, HUD object `+0x115 = (row == FE_ON)` | HUD |
| `Photo Blur` (`MSC_PHOTO_SPEED`) | 0 to 100, step 5, default 0 | `value * 0.01` to `FUN_00678f18` | speed blur |
| `Exposure` | 0 to 100, step 1 | `(value - 33.3) * -0.03003` (f1) | `Renderer_SetColourCorrection` |
| `Saturation` | 0 to 100, step 1 | `value * 0.05` (f2) | same, so 20% is `1.0` |
| `Photo Use DOF` (`MSC_PHOTO_PEFF`) | see above | selects which of the next three show | |
| `DOF Aperture` | 0 to 100, default 40 | `value * 0.01` | `FUN_00678f38` |
| `DOF Focus Distance` | 0 to 100, default 20 | below | `FUN_00678f28` |
| `Shutter Speed` | 0 to 100, default: two in the file, inner `20%` | `(value * 0.01)^2 * 0.25` to `+0x114` | motion blur shot |

**This closes `renderer.md`'s "one float at `[sp+0x7c]`".** The three colour values do not derive
from one slider: `0x00676968` (a `sscanf`) rereads the same stack slot once per row. `0x0023fee4`
to `0x0023ff14` show it: `f31 = ([sp+0x7c] - k) * k2` before the Saturation read, `f2 = [sp+0x7c] * 0.05`
after it. `finalScale` is `r28` splatted, `finalBias` is `f30` splatted, both constants of the
function, and `r3 = 1`.

**Depth of field** (focus `f = value * 0.01`):

```text
d = 40 f + 3                                       f <= 0.5
d = u (450 u^2 + 40 f) + 40 f (1 - u) + 3          f > 0.5, u = 2 f - 1      d runs 3 to 493
c = d^0.55    e = c^1.7
near edge d - (c + e), near focus d - c, far focus d + 2 c, far edge d + 2 (c + e)
```

The same formula stands in 2048 (`FUN_8112ea02`, `pow(x, 0.55)`, `pow(x, 1.7)`). When the focus row
has focus, `FUN_00678eb8(2 d)` draws the focus plane; otherwise `-1`.

**Camera.** `close` and `track` set up as in the table; in all modes the subject can be changed:

| Input index | Legend | `close` (mode 3) | `track` (mode 4) | `internal` (mode 5) |
| --- | --- | --- | --- | --- |
| stick `0x14`, `0x13` | `MSC_PHOTO_ROT` | orbit A, B | none | none |
| `7` held plus stick | `MSC_PHOTO_PAN`, `ά` | pan B, A, `+-5` | none | none |
| `0xb`, `0xa` | `MSC_PHOTO_TILT`, `Κ/Μ` | tilt | tilt | none |
| `0x12` | `MSC_PHOTO_ZOOM`, `Λ/Ν` | FOV `[10, 100]`, starts 65 | `+0x290` `[3, 40]`, starts 20 | none |
| `5` | `MSC_PHOTO_RESET`, `Ϊ` | pan, orbit, tilt 0; FOV 65; `+0x2f0 = 0.25` | `+0x290 = 20`, tilt 0 | none |
| `8`, `9` | `MSC_PHOTO_SHIP` | previous, next craft, skipping any with `+0x6a64 & 0x1000` | same | same |
| `0xe` | `MSC_PHOTO_HELP` | toggle `Photo Toggle Help Display` | same | same |

Entering `close` zeroes all five; the label bar moves with the mode. With one craft in the race
`Cycle` hides, and on top of that `internal` hides the whole bar. **No function here tests the eye against the track**: orbit, pan
and FOV are plain clamps.

Other race state: `FUN_00678e48(0.3, 0.04, pad)` sets stick response on entry; the old pair at
`this+0xf4/+0xf8` is restored on leaving. `this+0x100` and `this+0x10c` are the accumulation counts:
**depth of field accumulates 7 further frames, motion blur `0x78` = 120**, and `uVar42` above holds
the controls off while either runs. A shot is refused for `0.15` s after entry (`+0x3ee8`).

`FUN_00678ef8`, `FUN_00678f58`, `FUN_00678f78`, `FUN_006765e8` and `FUN_006765f8` gate the shot
(capture busy states): not named here.

## Saving

`wohd_%d%02d%02d_%02d%02d%02d.jpg`, under `/dev_hdd0` free space, written to a temp path, then
`cellPhotoExportFromFile` on a thread (`exportPhotoThread`), then `cellFsUnlink`. The manual string
`MAN_2_PM`: "saved in the JPEG format to the Photo folder ... in the resolution that the game is
currently being displayed at". Reads with it: the writer is sized from the colour target, `0x5a`
quality (90) on both paths. `MSC_PHOTO_INSUF`, `ER_PHOTO_RESOURCE` and `MSC_PHOTO_CANCELLED` are
returns 1, 2, 3. Success adds 1 to the stat `+0x238` (`uNumPhotosTaken`).

Tiled capture is `*(0x008c1f04 + 4) >= 2`. `0x008c1f08` reads `1` in the static image and live, so
the single path ran. No writer was read; recipe: `get_xrefs_to 0x008c1f08`.

## Live, RPCS3

2026-11-06, Fury EU, cold boot, Single Race on Vineta K, `Resolution: 1920x1080`, audio renderer Null.
`scripts/emu-env.sh`, `rpcs3-drive.py serve`, guest memory through `RemoteMem`.

- **The race stops.** Craft body position (`body+0x200`): moved 7.3 units in 0.5 s racing, then
  unchanged to the bit across `InGame Pause SP`, entry, and 4 s of `InGame Photo`. CIRCLE returns to
  `InGame Pause SP`, Continue to `InGame`, and the craft moves again.
- **Sliders open on the race's grade.** The screen reads Exposure `21%`, Saturation `20%`, both not
  the XML's. Table value `g = 0.34520742` at `0x00c51134`: `(int)(33.3 - 33.3 g) = 21`, and
  `(21 - 33.3) * -0.03003 = 0.36937` is what `g_RenderGlobals+0x3e4` held, `0.36936936`. The
  correction switch `g_RenderGlobals+0x3e0` reads `01` in photo mode and `00` after leaving.
- **Modes.** `camera+0x40`: `5` on entry (`internal`), `3` on `close`.
- **Stick.** Left Y drives `+0x294`, left X `+0x298`, unclamped (`a` read 6.03 and `-3.55`; `2 pi`
  wrap at `+-6.28`). Right Y drives `+0x2b0`, held to **100.0** at the top; 65 after reset. L2 and R2
  move `+0x29c`. The rate is the same `0.05` per frame (emulated at about 11 frames a second here).
- **Pan was not seen.** With the virtual pad's `square` held, `+0x2a8` and `+0x2ac` stayed `0.0` and
  the stick kept orbiting, though the legend shows stick plus square. Index 7 is `FUN_00677308`, a
  different query from `FUN_006770c8`: unresolved. Recipe: break in `Camera_AddPhotoPanA`.
- **Track collision does not limit it.** At `+0x294 = 1.34` the eye is under the road: the road's back
  face is culled and the frame looks up at the sky and the buildings. `-1.36` is overhead.
- **The shot.** Button to file: `InGame Taking Photo Transition Screen`, 1.8 s, `InGame Taking Photo`,
  `InGame Taking Photo Saving`, `InGame Photo Message`. Temp `wohd_20261106_153859.jpg` in
  `USRDIR`, exported as `dev_hdd0/photo/2026/11/06/...`, 495,667 bytes: **1920 x 1080**, baseline,
  `0x11` three times, the same luma table start as Pulse's, `2, 1, 1, 1, 1, 1, 2, 1`. So quality 90
  holds in two binaries. UI and HUD absent. The first shot printed `TROPHY: Unlocking trophy 12`.
- Audio not measured (Null renderer). Recipe as on the Pulse page: voice state over `InGame Photo`.

## Split screen and the finish

`Block name="Photo"` is on `InGame Pause Split Player 1`, `Split Player 2` (with `InputPad="1"`), both
`Tournament Split` and `Zone P1`/`P2` screens. So **HD offers photo mode in split screen**, from either
pad; `PhotoMode_ScreenEnter` picks the pad from `g_GameState+0xe4/+0xe5`. On leaving,
`0x0023fae4`'s byte is set for two local players, unless the phase is 3 or 4.

`Race End Photo` carries `PhotoModeButton`/`PhotoModeText` and `Race End Photo Redirect` with
`forward="square"`: **square here, SELECT on Pulse.**

## History

- 2026-11-06: `PhotoMode_Update` 78 -> 84. Sole caller read, its rows and laws read
  (`renderer.md` gave it 78 from strings alone).
- 2026-11-06: 84 -> 90 on the RPCS3 run above. `PhotoMode_ScreenEnter`, `PhotoMode_TakeShot`,
  `PhotoMode_WaitForSave`, `Renderer_SaveFrameJpeg` move to 90 with it.
