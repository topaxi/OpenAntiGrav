# The PS2 release is anamorphic, and its ASPECT RATIO option only moves the camera

Source: `pulse-ps2-eu.chd` (SCES-54748) and `pulse-psp-eu.chd`/`pulse-psp-usa.chd`,
read with `oag-wad cat --expand`, `ffprobe`/`ffmpeg` and Ghidra.

**Short version.** The PS2 port draws its 2D layer by stretching the PSP's
480x272 artwork into a 640x448 frame - `x` by `640/480`, `y` by `448/272`, a
*non-uniform* stretch that is only undistorted when that frame is shown at
**480/272 ≈ 16:9**. There is one asset set, no 4:3 variant of anything. The
disc's own `Aspect Ratio` menu does not touch the 2D layer at all: it multiplies
the **3D camera's** aspect by `4/3` and nothing else. So the picture the artwork
was built for is the 16:9 one, and the 4:3 setting is a 25% horizontal squash of
everything except the world.

| Claim | Confidence |
| --- | --- |
| The option's two values are `4:3`/`0` and `16:9`/`1`, from the disc's own XML | 98 |
| Setting `16:9` widens the 3D camera's aspect by `4/3` and changes nothing else | 95 |
| The 2D layer is the PSP's art stretched by `(640/480, 448/272)` | 95 |
| That stretch is undistorted only at a display aspect of `480/272` | 95 |
| The `.PSS` intro is anamorphic to the same frame, not to the 4:3 it declares | 88 |
| There is no second, aspect-keyed asset set anywhere on the disc | 92 |
| An `Image` with no `width`/`height` is scaled by the same factors | 70 |

## The option

`Data\Plugins\PI001\GUI\Additional_Definition.xml` in `WADS2.WAD`, reached from
OPTIONS through the `Additional` screen, whose two entries are `OPT_SCR_OFFSET`
and `OPT_ASPECT`:

```xml
<Screen type="Settings" name="Aspect Ratio">
  ...
  <List name="AspectList" focus="true" save="true" default="100">
    <Entry string="4:3"  Value="0"></Entry>
    <Entry string="16:9" Value="1"></Entry>
  </List>
</Screen>
```

Reproduce with:

```sh
just wad cat data/images/pulse-ps2-eu.chd:54748/WADS2.WAD \
  'Data\Plugins\PI001\GUI\Additional_Definition.xml' --expand
```

`default="100"` is not one of the two values and is not explained; what ships as
the out-of-box setting is **not resolved here**.

The settings screen controller is three functions, none of which is renamed
(`docs/reverse-engineering/confidence-rubric.md`: a screen dispatcher read once
is not a name yet):

| Address | What it does |
| --- | --- |
| `0x001bc6f0` | `OnEnter`. `strcasecmp`s the screen's name against `Display`, `Screen Offset` and `Aspect Ratio` and stores 1, 2 or 3 |
| `0x001bc380` | Per-tick. In case 3, `DAT_00284fe8 = atoi(AspectList->selected->text)` |
| `0x001bc8f0` | On leaving. In case 3, writes that global to `g_settings + 0x2d8` |
| `0x00106cf0` | Settings load. Reads `g_settings + 0x2d8` back into the global |

So the global at `0x00284fe8` is `0` for `4:3` and `1` for `16:9`, straight out
of the XML's `Value`.

## What the option changes: one multiply, in the camera

`Camera_SubmitScene` (`0x0013e280`, named 2026-09-05 - see
[`docs/ghidra/functions/ps2-pulse-eu/camera.md`](../ghidra/functions/ps2-pulse-eu/camera.md))
is the per-frame camera update. At `0x0013e6ec`:

```
0013e604  lui at,0x3fb6 / ori 0xdb6e     ; f21 = 1.4285714 = 640/448
0013e694  lw  v0,0x1d8(s2)               ; split-screen viewport count
          ...                            ; 2.857143 or 0.714286 for a split
0013e6ec  lui v0,0x28 / lw v0,0x4fe8(v0) ; DAT_00284fe8, the setting
0013e6f4  beql v0,zero,0x0013e714        ; 4:3 -> skip
0013e6fc  lui at,0x3faa / ori 0xaaab     ; 1.3333334 = 4/3
0013e70c  mul.S f21,f21,f0               ; aspect *= 4/3
0013e738  jal 0x001e9420                 ; build the projection
```

`FUN_001e9420(fov_rad, aspect, near, far, 1.0)` is an ordinary perspective
build - `m00 = (1/tan(fov/2)) / aspect`, `m11 = 1/tan(fov/2)` - so the option
**widens the horizontal field of view by a third**. That is anamorphic
widescreen done properly: at `16:9` the world is not stretched, there is a third
more of it across the screen.

Every reference to `DAT_00284fe8` in the program is accounted for: the four
functions above and this one read. The aspect it computes is also stored to
`DAT_0027e7e4`, which **nothing reads**. No 2D path, no HUD path and no GS
display path sees the setting. (The sweep is exhaustive for the *global*; the
`g_settings + 0x2d8` field it is copied from is reached through a pointer
parameter, so Ghidra lists no references to it. The global is the only way the
field propagates.)

## What the option does not change: the 2D layer

### The same artwork, drawn stretched

`Data\Plugins\PI001\GUI\Skin.xml` and `Data\XML\Arcade_HUD.xml` exist on both
discs, and the images in them name the same texture and the same source
rectangle in it. Only the destination size differs:

| Element | PSP draw | PS2 draw | source texels | x | y |
| --- | --- | --- | --- | --- | --- |
| `Skin.xml` logo strip | 224x24 | 299x40 | 224x24 | 1.335 | 1.667 |
| `Skin.xml` news bar | 454x14 | 605x23 | 454x14 | 1.333 | 1.643 |
| `Arcade_HUD.xml` `SpeedBarBg` | 168x26 | 224x43 | 168x26 | 1.333 | 1.654 |
| `Arcade_HUD.xml` `ShieldBarBg` | 140x12 | 187x20 | 140x12 | 1.336 | 1.667 |

`640/480 = 1.3333` and `448/272 = 1.6471`: the PSP's screen scaled to the PS2's
frame, on each axis independently. **Every image the two discs share follows
it** - 34 in `Arcade_HUD.xml` and 4 in `Skin.xml`, each to within a pixel of the
prediction, with an identical source rectangle on both discs. That is a stronger
result than the coordinate half of the same comparison, where 13 of 43 shared
coordinates were re-placed by hand;
`crates/game/tests/frontend_grid_ground_truth.rs` measures both against the
discs, so neither has to be believed.

### The stretch is in the executable too

`FUN_001e9370` takes four `short`s and scales them:

```
001e939c  lui at,0x3faa / ori 0xaaab     ; 1.3333334 = 640/480
001e93b4  lui at,0x3fd2 / ori 0xd2d3     ; 1.6470588 = 448/272
```

`x0` and `x1` by the first, `y0` and `y1` by the second. One of its four callers
is `Loading_Show` (`0x001da0a0`) - the loading screen, whose numbers the PS2
build kept in the PSP's grid (see `oag_pulse::loading`). **The port's own answer
to "PSP coordinates on a PS2 screen" is a non-uniform stretch, in code**, and
the XML above is the same stretch applied ahead of time by whoever prepared the
data.

Nobody stretches a layout by two different factors unless the destination frame
is displayed at the source's shape. A port that meant its 640x448 frame to be
seen as 4:3 would have scaled uniformly and left bars.

### The arithmetic

A `w`x`h` texel rectangle drawn `1.3333w` by `1.6471h` grid units, in a 640x448
grid shown at display aspect `A`, appears with the shape

```
(w/h) x A x (1.3333 x 448) / (1.6471 x 640) = (w/h) x A x 0.56667
```

so it is undistorted at `A = 1/0.56667 = 1.7647 = 480/272`, the PSP's own panel.
16:9 is `1.7778`, wider by exactly `(16/9)/(480/272) = 1.0074`, which is why the
console's option can call it 16:9 and be right to within three quarters of a
percent. 4:3 is `1.3333`, **narrower by 24.4%**.

**One frame cannot hold both answers, and this build was holding both.**
`oag_race::AUTHORED_ASPECT` is `480/272` - the original's field of view is only
defined at the PSP's shape - so a PS2 disc drew its *world* at 1.765 and its 2D
layer at 1.333 in the same picture, 24% apart. They agree now.

### Measured, not only computed

`--screen "Show Logo"` renders the same `pulse_logo.mip` from both discs
(512x128 on each - PSP header `00 02 80 00`, PS2 header `79` packed log2 with
`+0x04` height 128 and `+0x06` width 512, per [ps2-texture](../formats/ps2-texture.md)).
Measuring the ink in each render:

| | ink width/height |
| --- | --- |
| PSP, square pixels | 4.20 |
| PS2, drawn in the 640x448 grid and shown as 4:3 | 3.92 |

Convert the PS2 figure back into grid units - divide by `A x 448/640` - and both
discs give **4.20**. Same art, same texture, same size: nothing was redrawn for
the PS2.

## The intro movie is anamorphic to the same frame

`INTRO512.PSS` (PAL) is 512x512 at 25 fps and declares `SAR 4:3`, so `ffprobe`
reports a 4:3 display aspect; `INTRO640.PSS` (NTSC) is 640x448 and declares the
same. **That tag does not match the picture.** The film's closing card is the
`wipEout PULSE` logo - the same artwork as `pulse_logo.mip`, which the PSP front
end draws at square pixels. Measuring both words' ink boxes:

| | `wipEout` w/h | `PULSE` w/h |
| --- | --- | --- |
| PSP front end (ground truth) | 7.56 | 8.76 |
| `INTRO512.PSS`, decoded square | 4.40 | 4.56 |
| stretch needed | **1.72** | **1.92** |

4:3 would need `1.333`; the frame's own `480/272` is `1.765` and 16:9 is
`1.778`. Both bands bracket that and neither is anywhere near 4:3.

The whole-logo ink box says the same thing in one number: the PSP front end
draws it at **4.20** wide-to-tall, and the film's card scaled to 910x512 (16:9)
measures **4.23** - eight tenths of a percent apart. At 4:3 it measures 3.14.
Reproduce:

```sh
just unpack extract data/images/pulse-ps2-eu.chd '*INTRO512.PSS' --out /tmp/pss
ffmpeg -i /tmp/pss/54748/DATA/MOVIES/INTRO512.PSS -vf "select=eq(n\,940),scale=910:512" -frames:v 1 logo.png
```

**This supersedes the reading recorded in
[pulse-disc-layout.md](pulse-disc-layout.md#video) and
[ipf.md](../formats/ipf.md)**, which took the declared 4:3 at face value on the
strength of a symmetric UI mark looking right when corrected to it. The mark is
a stylised helix whose "correct" proportions were assumed rather than compared
against anything; the logo can be compared against the very texture it is drawn
from, on the other disc, and it says the frame is the wide one. Confidence 88:
one film, one measurable element in it, measured twice.

An `IPUF` backdrop declares no aspect at all and inherits this reading with it.

## What this project does about it

`oag_display::space::Space::PS2` now carries `display_aspect = 480/272` rather
than `4/3`, so the front end, the menus and the in-race HUD are all fitted the
way the artwork was built. **There is no setting for it**: the original's option
exists because a PS2 could be plugged into either kind of television, and this
build draws into a window whose shape it already knows.

Consequences, all of them in `crates/game`:

- `Space::PS2.display_aspect` is `SCREEN.0 / SCREEN.1`, and the grid stays
  640x448. The two numbers still disagree - by 24% now instead of 7% - which is
  the whole reason [`Space`](../architecture/frontend-boot.md) carries both.
- `Space::texture_scale` is `(640/480, 448/272)` for the PS2 and `(1, 1)`
  elsewhere. An `<Image>` with no `width`/`height` is the one case where the
  data gives no size, and drawing such an image at its texel size in a 640x448
  grid would make the boot logo 62% of the screen where the PSP's is 80% of it,
  as well as the wrong shape. Applying the executable's own two factors puts it
  exactly where the PSP has it. **Confidence 70, and it is a reimplementation
  choice rather than a reading**: `FUN_001e9370` is evidence that the port
  scales PSP-grid numbers this way, not that it scales *this* number this way,
  and the original's own default-size path has not been read.
- A PS2 movie reports the frame's aspect rather than its container's, so it
  fills the screen instead of being pillarboxed inside it. **The transcode is
  untouched**, deliberately: the cache is keyed by the *source* bytes, so
  resizing frames on the way in would have silently reused every stale file
  until someone passed `--refresh-video`, and it would have baked one answer
  into the cache. The decoded frames stay the decoded picture; how it is shown
  is a draw-time question.
- One visible regression, unfixed: `widthlimited="true"` is parsed and ignored,
  so `Show Logo`'s PS2 legal line - which always overflowed the 640-wide grid -
  now runs off the edge where the 4:3 letterbox used to leave it somewhere to
  spill. Wrapping it needs a line-breaking rule and a leading this project has
  no evidence for, so it is recorded rather than invented.

### What the port did not stretch

Two things on the PS2 disc were left in PSP proportions while everything around
them was scaled, and both are the disc's own doing rather than a decision here:

- **The fonts.** `Data\FE\Fonts\Pulse_20.fnt` has a line height of 24 on the PS2
  against the PSP's 22, and `pulse_text.fnt` 14 against 13 - roughly 1.09 where
  the grid grew by 1.33 and 1.65. So PS2 text covers about two thirds of the
  screen height the PSP's does, and in an anamorphic frame its glyphs come out
  about a quarter shorter than they are wide compared with the PSP's. Measured
  on `START-Taste drücken` in `Show Logo`: 396x24 pixels on the PSP against
  394x18 on the PS2, at the same window size.
- **`Show Logo`'s logo**, the one `<Image>` in the PS2 front-end XML with no
  `width`/`height` - see `Space::texture_scale` above for what this build does
  with it and at what confidence.

## Open

- **What the PAL frame does to a real raster.** The DISPLAY registers are built
  in `FUN_00238de8` and patched with the Screen Offset in `FUN_00216ff8`;
  `g_refresh_mode` (`0x0027a85c`, also what picks `Intro512` over `Intro640`)
  selects `DX/DY` defaults of `(0x280, 0x34)` or `(0x2a8, 0x48)`. Whether the
  448 lines fill a 576-line PAL raster or sit inside it changes how much of the
  24% is visible on real hardware, not which way it goes. A PCSX2 frame at each
  setting would settle it; nothing here needs it. **This page called
  `0x0027a85c` "the region index" until 2026-09-08 and that was wrong** - it is
  the 50/60 Hz refresh mode, and the player picks it on a first-boot screen; see
  [refresh-mode.md](../ghidra/functions/ps2-pulse-eu/refresh-mode.md). The
  reading above is unaffected: the mode moves the picture in the raster and
  corrects its pixels, and resizes nothing.
- **`default="100"`**, and therefore which setting a fresh memory card gets.
  **2026-10-01:** `DAT_00284fe8` reads `0` (`4:3`) on a cold boot with an empty
  profile at every poll from 15 s to 150 s, and on the race savestate. Not a
  resolution: the settings load had not necessarily run at those polls.
- **The race camera's aspect is `10/7` at `4:3`, read live.** The matrix the race
  renders with has `m11 / m00 = 1.42857` and a vertical field of the authored
  60 degrees; see [camera.md](../ghidra/functions/ps2-pulse-eu/camera.md), "The
  projection the race renders with". This build now fits a PS2 source's field to
  that shape. The anamorphic part is **not** reproduced: the original's frame is
  `640/448` shown at whatever the display is (the 2D art assumes ~16:9), so at the
  `4:3` option a 16:9 display stretches the world 1.24x wide, and ours presents
  the projection at the viewport's own shape instead.
- **The original's default size for an `<Image>` with no `width`/`height`.**
  Reading it in the PS2 front-end widget code would turn the 70 above into a
  measurement.

## See also

- [PS2 disc layout](pulse-disc-layout.md) - the files these come out of
- [Front-end boot](../architecture/frontend-boot.md) - `Space`, and the grid
  measurement this page's stretch factors come from
- [`ipf.md`](../formats/ipf.md) - the backdrop container
- [PSP vs PS2](../comparisons/pulse-psp-vs-ps2.md)
