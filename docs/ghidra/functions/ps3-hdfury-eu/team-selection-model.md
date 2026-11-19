# Team Selection's craft: the screen poses the `ShipModel` widget itself, and fits its depth

2026-11-11. `Team_Selection_Definition.xml` authors one pose for the craft
(`<Model name="ShipModel">`: `OriginX=1220 OriginY=412 z=-24 RotX=0.4 RotY=-0.5 nearZ=1.0
ObeySafeZone=true`), and **the original draws none of it but `nearZ` and `ObeySafeZone`**.
`TeamSelection_Screen`'s update writes the widget's rotation, depth, field of view and origin
every frame, and a helper steps the depth until a box the size of the selected model just fits a
window on screen. This page is that law, read statically (GUI bridge, program
`/hdfury/EBOOT-ps3-hdfury-eu.elf`) and checked against one RPCS3 frame.

All addresses below read TOC `0x008ad4d8` (the main module; [memory.md](memory.md) for the
per-function TOC defect). **Trap**: Ghidra's own string references for `RotX`, `OriginX`,
`Length` and the rest point into `0x005853a0` and `Render_ClassifyBoxAgainstPlanes`, which use
another module's TOC; the readers are found by scanning for the `off(r2)` operand instead.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00254348` | `TeamSelectionScreen_Update` | 85 |
| `0x00250e18` | `TeamSelectionScreen_FitModelDepth` | 90 |
| `0x00250a30` | `TeamSelectionScreen_ProjectFitCorner` | 90 |
| `0x00253458` | `TeamSelectionScreen_Init` | 55 (below 70, carries `_q`) |
| `0x001e7bc8` | `TeamModelItem_ParseElement` | 88 |
| `0x001d5fa8` | `ModelItem_ParseValues` | 85 |
| `0x001d3150` | `ModelItem_Construct` | 78 |
| `0x001d3f08` | `ModelItem_BuildPose` | 75 |
| `0x003be688` | `FeModel_DrawAtOrigin` | 65 (below 70, carries `_q`) |

`0x001d35e8` is `ModelItem_RenderScene` ([menu-backdrop-scene.md](menu-backdrop-scene.md)), the
same widget class: this page adds its lens.

## The widget: what `<Values>` sets and what the lens is

`ModelItem_ParseValues` (`0x001d5fa8`, `Model_Item.cpp`) reads the `<Values>` attributes into
the item, compared against the names at TOC slots `0x008ae35c..0x008ae3c0` in this order:

| Attribute | Field | Constructor default (`0x001d3150`) |
| --- | --- | --- |
| `X`, `Y`, `Z` | `+0xac`, `+0xb0`, `+0xb4` | 0 |
| `RotX`, `RotY`, `RotZ` | `+0xb8`, `+0xbc`, `+0xc0` | 0 |
| `RotationCentreOffsetX`, `Y`, `Z` | `+0xe0`, `+0xe4`, `+0xe8` | 0 |
| `orthoScale`, copied over the three below unless it is the default | `+0xc4` | |
| `orthoScaleX`, `Y`, `Z` | `+0xc8`, `+0xcc`, `+0xd0` | |
| `colour` | `+0xd4` | |
| `startPaused`, `Ztest`, `UseModelCamera` | `+0xd8`, `+0xd9`, `+0xda` (bytes) | 0, 1, 0 |
| `OriginX`, `OriginY` | `+0x10c`, `+0x110` | 0 |
| `nearZ`, `farZ` | `+0x114`, `+0x118` | `20.0`, `100.0` |
| `mirror`, `ObeySafeZone` | `+0x109`, `+0x11c` (bytes) | 0 |
| `mode="orthographic"` | `+0x104 = 1` | 0 (perspective) |
| `FirstPass="yes"` | `+0x108 = 1` | 0 |

**No attribute sets the field of view.** `+0xdc` is the constructor's `57.3` (degrees; the
literal `0x42653333` at `0x008ae2e8`) and only code writes it afterwards. Confidence 85.

`ModelItem_BuildPose` (`0x001d3f08`) builds the model matrix in column form as
`T(x, y, z) * Rx(RotX) * Ry(RotY) * Rz(RotZ)`. Confidence 75: read off the decompile's matrix
products, not re-derived per element; the frame check below would not survive another order.

`ModelItem_RenderScene` (`0x001d35e8`, at `0x001d37a0..0x001d3848`) builds a standard
perspective projection: `f = 1 / tanf(fov * 0.008726646)` (the literal at `0x008ae304` is
`pi / 360`, so the tangent is of half the angle), `x = f / aspect` with the aspect the global at
`0x008c1518` (`1.7777778` in the image), `y = f`, and the depth terms `(n + f) / (n - f)`,
`2nf / (n - f)`, `-1` from `+0x114` and `+0x118`. It then calls `FeModel_DrawAtOrigin`
(`0x003be688`) with `(OriginX + parent x, OriginY + parent y)` and the `ObeySafeZone` byte.
Confidence 85.

`FeModel_DrawAtOrigin` slides the finished picture so the model's origin lands on the grid
point: a device-space offset of `s * ((ox - 960) / 960, (540 - oy) / 540)`, with the picture
itself scaled by `s`, the Safe Area as a fraction, when the byte is set. Confidence 65 on the
static read (one pass over a 2,984-byte function); **the frame check below confirms both
halves**, which is why the law is wired while the name stays below 70.

## What the update writes over the widget

`TeamSelectionScreen_Update` (`0x00254348`), at `0x002546f8..0x002547e8`, with `st` the screen
and `item = *(st + 0x188)`:

| Item field | Written | Constants (TOC slot) |
| --- | --- | --- |
| `RotX` `+0xb8` | `st[0xfe8] + 0.25` | `0x008b1ba0` |
| `RotY` `+0xbc` | `st[0xfec] - 0.6` | `0x008b1b7c` |
| `z` `+0xb4` | `4.0 * st[0xff0] - st[0x1020]` | `0x008b1d34` |
| `x` `+0xac`, `y` `+0xb0` | `-st[0xff4]`, `-st[0xff8]` | |
| `fov` `+0xdc` | `57.3 - 60.0 * st[0x1024]` | `0x008b1ce0`, `0x008b1d3c` |
| `OriginX` `+0x10c` | `1220.0 + st[0x1018]` | `0x008b1ce4` |
| `OriginY` `+0x110` | `500.0 + st[0x101c]` | `0x008b1d40` |

`st[0xfe8]`, `[0xfec]`, `[0xff0]`, `[0xff4]`, `[0xff8]` and `[0x1024]` are the player's own
turn, push, pan and zoom of the craft (pad axes read through `FUN_006778b8`, eased at `0.125`
and `0.25` a frame, the turn wrapped at `+-3.1459`, and stepped back to zero at `0.05` a frame
once `st[0xffc]` has counted 181 idle frames). **All six are zero at rest**, so the rest pose is
`RotX 0.25`, `RotY -0.6`, `fov 57.3`, `z = -st[0x1020]`. Confidence 85 on the rest pose, 65 on
the input half (read, not exercised).

The three eased values follow the fit's targets every frame:

```
st[0x1018] += (st[0x100c] - st[0x1018]) * 0.0625      // origin slide, across
st[0x101c] += (st[0x1010] - st[0x101c]) * 0.0625      // origin slide, down
st[0x1020] += (st[0x1014] - st[0x1020]) * 0.125       // depth
```

(`0x008b1d2c`, `0x008b1d30`.) This is the "new craft cuts in slightly off its rest size and
eases back" that `docs/ui/selection-screens.md` measured off film: the depth target is not reset
when the selection moves, so the next model starts at the last one's depth and eases to its
own. `TeamSelectionScreen_Init` (`0x00253458`, the only other writer, at `0x002538a0`) sets the
target to `13.0`.

## The fit

`TeamSelectionScreen_FitModelDepth(length, st)` (`0x00250e18`) is called once a frame with
`length = *(*(st + 0x194) + 0x1cc)`: the selected `PI_TeamModel`'s `<FE Length>`.
`TeamModelItem_ParseElement` (`0x001e7bc8`, `PI_TeamModel_Item.cpp`) reads `<FE>`'s `Speed`,
`Thrust`, `Handling`, `Shield` and `Length` into `+0x1bc`, `+0x1c0`, `+0x1c4`, `+0x1c8` and
`+0x1cc` (the attribute names are TOC slots `0x008aebdc..0x008aebec`). The disc authors eight
lengths over its 84 rated models: `3.2`, `3.8`, `3.9`, `4.0`, `4.1`, `4.2`, `4.4`, `4.5`.

1. Eight corners `(+-1.125, +-0.75, z)` (the words `0xbf900000`, `0x3f900000`, `0xbf400000`,
   `0x3f400000` at `0x00250e9c..0x00250f00`), with `z` one of two values chosen by the game mode
   at `st + 0xf0`: `-4.2` and `4.2` in Zone (mode 6; `0x008b1bd0`, `0x008b1bcc`), `-2.0` and `0`
   in Detonator (mode 14), else `-length` and `length`.
2. Each corner goes through `TeamSelectionScreen_ProjectFitCorner` (`0x00250a30`) and a running
   minimum and maximum, **both starting at `0.0`**, are kept across and down
   (`0x00251308..0x00251324`).
3. The slide targets are minus the box's centre: `st[0x100c] = -0.5 * (min_x + max_x)`,
   `st[0x1010] = -0.5 * (min_y + max_y)` (`0x008b1bec` is `-0.5`).
4. If the centred box is inside `+-535` across and `+-389` down (`0x008b1bd8..0x008b1be4`) the
   depth target `st[0x1014]` **drops** by `0.1`, else it **rises** by `0.1` (`0x008b1be8`); it
   is then floored at `11.0` (`0x008b1bf0`).

So the target never rests: it alternates between the nearest depth the box fits at and one step
nearer, and the eased depth sits between the two. Confidence 90.

### The projector, and the unit of the window

`TeamSelectionScreen_ProjectFitCorner` (`0x00250a30`), instruction by instruction:

- `Matrix_RotationX(st[0xfe8] + 0.25)` and `Matrix_RotationY(st[0xfec] - 0.6)`
  (`0x00250a80..0x00250af4`), multiplied as `Rx * Ry` and applied to the corner
  (`0x00250b50..0x00250c04`);
- the depth target is subtracted from the result's `z` (`0x00250c08..0x00250c28`,
  `st[0x1014]`, the target and not the eased value);
- a projection is built with `1 / tanf(0.5)` down and that over the aspect global across
  (`0x00250c2c..0x00250ca8`), its last two columns the constants at `0x00799190` and
  `0x007991a0`: `(0, 0, -1.0202, -1)` and `(0, 0, -2.0202, 0)`, a near of 1 and a far of 100;
- the corner is projected, and the result is `x / w * 1920.0` and `y / w * -1080.0`
  (`0x00250d54..0x00250de8`; the literals at `0x008b1bc4` and `0x008b1bc8`).

`x / w` is a normalised device coordinate, `-1..1` across the whole screen, so **the projector's
unit is half a pixel of a 1920 by 1080 frame**: the window of `+-535` by `+-389` is `+-267.5` by
`+-194.5` pixels. Two consequences, both of which the original draws:

- **The hull overhangs the box.** A hull is 12 to 15 units long and its `<FE Length>`, half the
  box's length, is 3.2 to 4.5; Feisar's box rests about 530 pixels wide where its hull is 887.
- **The slide over-corrects.** Step 3 is in the projector's unit and is added to `OriginX` and
  `OriginY`, which are pixels of the grid, so the box's centre ends as far on the other side of
  the origin as it started.

The other reading (the window is pixels, the `1920` a slip for `960`) was tested and fails: it
rests every model on the disc at the floor of `11.0`, which draws Feisar about 9% larger than
the frame, and the fit would then never act.

### Rest depths

From the law, at 16:9 (`oag_game::preview::ship_model::rest`):

| `<FE Length>` | Rest pair | Drawn depth | Slide (across, down) |
| --- | --- | --- | --- |
| 3.2 | 11.0 (floor) | 11.0 | +93, -81 |
| 3.8 | 12.0, 12.1 | 12.05 | +109, -85 |
| 4.5 | 13.6, 13.7 | 13.65 | +118, -84 |

## The frame check

`data/reference/compare-page/hd/menu-ship-select/theirs.png`: RPCS3, 1920 by 1080, Feisar's
classic hull at rest, Safe Area 96 (a RAM read, `docs/formats/hd-hud.md`). The hull's silhouette
(any channel above 24 inside the `SHIP MODEL` frame) against Feisar's `ship.vex` vertices
projected through the law (`hd_ship_preview_ground_truth::the_screens_own_fit_lands_feisar_where_the_original_draws_it`,
which holds each edge to 5 pixels):

| Edge | Original | This law |
| --- | --- | --- |
| Left | 750 | 753.0 |
| Right | 1637 | 1636.0 |
| Top | 256 | 255.9 |
| Bottom | 639 | 638.9 |

All
four edges within 3 pixels fixes the rest rotation, the lens, the near plane's irrelevance, the
depth, the origin, the slide's sign and size, and the Safe Area's scale of picture and origin
together: no one of them can be off and the four still agree.

**Four more teams, four more lengths.** `data/reference/hd-capture/ship-framing/rpcs3-rest-t*.png`
are rest frames of the concept hulls of Feisar, Qirex, Piranha, Assegai and Icaras (2000 by 1200
window grabs; the screen's two rules put the grid at `px = 1058.5 + 0.9406 * (x - 960)`,
`535.7 + 0.9406 * (y - 540)`, Safe Area folded in). Edges in the unscaled grid, original against
the law, where the frame shows the edge against black (the rest are behind backdrop embers or
past the frame's brackets):

| Hull | `<FE Length>` | Rest depth | Left | Right | Top | Bottom |
| --- | --- | --- | --- | --- | --- | --- |
| `Qirex_c1` | 4.5 | 13.65 | 786 / 787 | | 286 / 287 | 706 / 705 |
| `Assegai_c1` | 4.4 | 13.45 | 755 / 751 | 1627 / 1624 | 283 / 285 | |
| `piranha_c1` | 3.9 | 12.25 | | 1665 / 1661 | 268 / 269 | |
| `Feisar_c1` | 3.8 | 12.05 | | 1663 / 1660 | 253 / 257 | |
| `Icaras_c1` | 3.2 | 11.0 (floor) | | 1594 / 1591 | 263 / 263 | |

Every held edge is within 4 pixels, across a depth range of 11.0 to 13.65, so the length is the
fit's argument and the floor is real. The same test holds these (Feisar's concept hull aside).

On the compare page's measure (sigma 7 blur, mean absolute RGB), the hull box
`716,150 1012x650` went from 20.62 to 14.33 and the frame from 9.35 to 7.35. What is left in the
box is not placement: the hull is darker than the original (mean bias R -7.6, G -11.7, B -11.1)
and the Fury backdrop's embers sit behind it where the original's frame has them over the
honeycomb.

## What is wired, and what is not

- **Wired** (HD): the rest pose, the lens, the fit, the slide and `ObeySafeZone`, as
  `oag_title::ShipFit` (`oag_hd::frontend::SHIP_FIT`), drawn by
  `oag_game::preview::ship_model::fitted`. The Zone box is used where the screen draws the Zone
  hull.
- **Wired 2026-11-12** (`hd-ship-swap-settle`): the easing between two models' depths, as
  `oag_game::preview::ship_model::FitMotion` (target, drawn depth, target slide, drawn slide; the
  fit measured at the target and stepped, then both eased at `ShipFit::depth_ease` 0.125 and
  `slide_ease` 0.0625). `PickerStage` holds it as a `FitPlayer`, ticked at 60 Hz beside the
  picker's clock with whichever model is selected, so a step (a press, a pointer pick) resets
  nothing and the new craft is drawn from the last one's depth. It runs while the craft is hidden
  (the entry's 0.9 s, the step's 60 ms gap), as the original's update does; the drawn depth's
  value at open is unread and placed on the target, 54 frames before the craft is shown. The
  order inside a frame (fit, then ease) is chosen. The old measured curve
  (`hull_swap::SETTLE`) and its chosen 1.06 start are removed; `hull_swap` keeps the two timings.
  Captures: `--menu-picker-step-seconds T` plays `round(T * 60)` frames of a step from the entry
  above the selected one (chosen).
- **Not wired**: the player's own turn, pan and zoom; the Detonator box (the preview does not
  draw the Detonator hull).

### The swap against the 60 fps film

`data/reference/hd-capture/ship-select-anim/steps60-hull-series.json` (RPCS3, 60 fps; right,
left, right presses at 3, 8 and 13 s; columns `frame, pixels, x at 1 %, x at 99 %`). The hull's
width over its settled width, by frames since the press (the first visible frame is the fourth to
fifth):

| Frames | 4-5 | 10 | 20 | 30 |
| --- | --- | --- | --- | --- |
| Film, Feisar to the next team | 1.084 | 1.081 | 1.027 | 1.007 |
| Film, back to Feisar | 0.952 | 0.964 | 0.988 | 0.996 |
| Law, Feisar to Auricom (4.2) | 1.078 | 1.041 | 1.011 | 1.003 |
| Law, Auricom to Feisar | 0.920 | 0.951 | 0.987 | 0.997 |
| Law, Feisar to Qirex (4.5, the roster's next team) | 1.168 | 1.121 | 1.038 | 1.010 |
| Law, Qirex to Feisar | 0.858 | 0.892 | 0.964 | 0.990 |

(Law rows: hull vertices of the destination team projected through `fitted` at
`FitMotion::framing` after `stepped` frames, the 1st to 99th percentile of x.) The film does not
say which team Feisar steps to, and the roster's own neighbour (Qirex) overshoots by twice over;
a team of about 4.2 fits the tails (frames 20 and 30 agree to 0.004 both ways) and the first
frame to a few per cent. **The film holds its first size for about six frames longer than the
law**, and then decays by 0.875 a frame, the ease's own rate. One candidate, not adopted because
it fits one direction and worsens the other: the fit's model pointer (`st + 0x194`) changes when
the new craft cuts in, so the fit keeps the old box through the 4-frame gap (Feisar to Auricom
1.087, 1.068, 1.018, 1.005; back 0.908, 0.925, 0.978, 0.994) - **falsified by the RAM read below**
(the pointer moves on the frame the selection does and the fit has the new box one frame later; the
film's excess is not in the screen's law). Open: **the live screen keeps the previous team's hull until the new mesh's worker read lands**
(a `--drive` frame 4 ticks after a press shows Assegai's hull under the MIRAGE label), where the
film cuts the craft for 3-4 frames and shows the new one; the same read would be the moment the
fit's box should change if the candidate above is right (it is not: see below). Open: which team the film stepped
to, and a second film of a pair whose lengths are known. Confidence 60 (tails and shape), 40 (the
first six frames).

## The step against RAM (2026-11-19, `hd-settle-step`)

The question: does ours play the step the way the original does, frame by frame, and if not which term
(settle start, craft turn, zoom, the gap) differs. **Written before the capture, what would falsify
"ours matches"**: ours' drawn depth or slide differing from the original's by more than 0.02 depth or
1 grid unit at any of frames 1 to 30 after the selection moved; a non-zero turn, pan or zoom term during a step; the
craft absent for a different count of frames.

**Method** (`scripts/hd_ship_step.py`, `scripts/rpcs3-hd-ship-step.py`; RPCS3 0.0.43, 1920x1080, scale 100, Fury style, a cold boot
walked to Ship Select with the pad):

- **The screen object `st` is found by a heap scan** for the pattern the fit leaves (depth target on the 0.1 grid at or above 11.0,
  drawn depth within 3 of it, slides under 600, `st+0x194` pointing at a `PI_TeamModel` whose `+0x1cc` is one of the eight authored
  lengths): exactly one hit on each of two boots (`0x3292cf30`, and another address on the second). `st+0x100` is the selection
  index (Feisar 0, Qirex 1, Piranha 2, AG Systems 3, ... wrapping at 12), the model's hull entry name (`Feisar_c1`, ...) sits in
  its first `0x400` bytes.
- **A frame is a change of `(st+0x1014, st+0x1020, st+0x1018, st+0x101c)`**: the update rewrites them every frame, and a reader polling
  `0x1040` bytes of the object takes about 0.3 ms a read, so it sees every one (131 frames in 2.2 s, 59.5 a second). The frames
  since `st+0x100` moved are the age of a step in game frames, whatever the emulator's speed.
- **The picture at a known frame**: the game is stopped (the stub, as `hd_flyer_clock.grab_paused`) once the count reaches an
  aimed frame (the stop lands on the aim or one after it), the state is read in the stop, the window is given 0.6 s to show the frame the RSX last
  got, grabbed through `grab_with_clock`, and the state is read again (equal in every grab). The Fury backdrop is hidden on both sides
  (the rest pair's recipe). After one lap of every team (a team's first visit loads its hull: on a cold first visit the craft stayed
  absent past frame 10 in two of two grabs, not measured further), 32 grabs on boot 1 (frames 0 to 19), 32 on boot 2 (1 to 29) and a
  rest grab of each of the twelve teams on each.

**1. The settle law is exact. Settle start, ease, fit order: true.** Fourteen steps (seven on each boot, `right` and `left`,
the lengths 3.2, 3.8, 3.9, 4.4 and 4.5, one from the floor at 11.0), thirty frames each: ours'
`FitMotion` (the model's box measured at the target, the target stepped, depth ease 0.125, slide ease 0.0625) gives the same depth target
and drawn depth to under 1e-4 and the slide to under 0.06 of a grid unit, once ours is on the same side of the rest pair's alternation as
the original was when the selection moved (which frame the player's press fell on is nobody's law; a wrong side is a 0.1 shift of the
target and 0.08 of depth at most). Held by `ship_model::tests::a_step_follows_the_screens_own_frames_in_ram` (boot 1's seven steps,
nine samples each; boot 2's seven pass the same test). Qirex to Piranha, frames since the selection moved (original / ours): target
13.600 / 13.600 at 1, 12.500 / 12.500 at 12, 12.300 / 12.300 at 24; depth 13.6467 / 13.6467, 13.0496 / 13.0496, 12.4195 / 12.4195; slide
across 116.75 / 116.79, 107.06 / 107.09, 109.07 / 109.10. Confidence 90.

**2. The order inside the frame**: `st+0x100` and `st+0x194` change on the same frame (frame 0), the fit that frame still ran on the old
model (its target and slide target are the old model's alternation), and the slide target takes the new box on frame 1. Ours'
`stepped` applies the new box from its first frame, which is that frame 1. So the candidate "the fit keeps the old box through the gap" is false:
the fit has the new box a frame after the selection, while the craft is still cut. Confidence 90.

**3. The craft's own turn, pan and zoom are not in a step: true.** `st+0xfe8`, `+0xfec`, `+0xff0`, `+0xff4`, `+0xff8`, `+0x1024` read 0 and the idle
counter `+0xffc` read 181 on every one of the fourteen steps and every grab (the right stick untouched). The turn and zoom terms of the claim
cannot differ on a step. Confidence 90 for a step with no stick input; the input half stays at 65 (read, not exercised).

**4. The picture, in game frames since the selection moved (RAM time)**. Old craft at its rest through frame 2, **absent at frames 3, 4 and
5, new craft from frame 6**: twelve grabs at frames 3 to 5 (six per boot) showed no craft; at frame 6 three of four grabs showed the new craft and
one (a `Feisar` to `Qirex` step) was still empty, and all four at frame 7 showed it, so the gap was 3 frames in three of four and 4 in one. The
screen's header dots (the selected team's marker) are on the old team through frame 2 and **on the new one from frame 3** in 71 of 72
grabs on two boots (one grab at frame 3 still showed the old one; every one from frame 4 on the new). The press
reaches the selection 0.032 s (about 2 frames; 0.025 to 0.041 over 14 presses) after the pad is pressed, so a film counted from the press sees every
number above 2 frames later, which is part of "the film holds its first size about six frames longer than the law".
Ours (`hull_swap::GAP_SECONDS` 0.06): absent while the step is under 0.06 s, so frames 0 to 3 of its step, 4 frames. **The one term that
differs is the gap: 3 frames in the original (once 4), ours 4.** Confidence 70.

**5. The picture's lag behind RAM** is not pinned: the dots read 3 frames (if they follow the selection the frame it moves; 3 or 4 by the one late grab), and of seven
fast steps the frame of ours with the lowest craft-box error is the original's less 3, 5, 4, 5, 3, 1 (flat) and 4, a minimum half a unit deep: 4 frames
(0.07 s), the card clock's own fitted lag. The pairs below use 4. Confidence 45 (a frame either way is not told apart by the craft box;
the gap's alignment to the selection depends on it: at 3 the gap runs from the selection to two frames after it, at 4 it starts a frame
before it).

**What a player sees: the craft box (716,150, 1012 by 650, 5 px blur, mean absolute RGB) of ours against the original** (boot 2's grabs,
`pairs-new/hd/menu-ship-step-kNN`; the rest pair of the same teams on the same boot, the floor, is 3.70 over twelve teams: 2.7 to 5.1,
colour not placement, the lighting item):

| Frames since the step | Team stepped to | Ours at | Craft box | Whole frame |
| --- | --- | --- | --- | --- |
| 4 | Assegai | 0 (both absent) | 0.50 | 3.64 |
| 6 | Harimau | 2 (ours absent, the original's new craft present) | **30.52** | 13.58 |
| 8 | Icaras | 4 | 3.69 | 6.05 |
| 10 | Qirex | 6 | 4.59 | 6.53 |
| 15 | Triakis | 11 | 4.04 | 2.70 |
| 19 | Goteki | 15 | 3.95 | 5.34 |
| 29 | EGX | 25 | 4.49 | 2.79 |

Across the 14 grabs of boot 1 after the gap (ours at the original's frame less 4, own-team rest as the floor): the craft box is 3.85 against a
floor of 3.82, +0.04 (frames 7 to 9 -0.12, 10 to 12 +0.51, 13 to 16 -0.14, 19 -0.24): **the settle itself adds nothing to the craft box
beyond the rest floor**. Where ours is absent and the original shows the new craft the box is 20 to 30, which is the gap term alone: a scratch build
with the gap at 0.045 s (3 frames) draws the new craft at the original's frame 6 less 3 and brings the two boot-1 grabs at frame 6 from 20.5 and
27.2 to 3.4 and 3.7 (at a lag of 3; at a lag of 4 the original's craft is back a frame earlier still and the gap would be 2 frames after the selection).

**Verdict for the claim**: the settle start per team, the turn and the zoom match (the law is exact in RAM, the turn and zoom terms are zero on a
step); the craft box during the settle is at the rest pair's floor; the single term that differs is the cut: 3 frames (once 4) against ours' 4,
a one-to-two-frame difference (17 to 33 ms) on the first visible frame. Open: the picture lag (3 or 4) that decides which; the cold
first visit of a team (craft absent past frame 10); `FitMotion`'s alternation phase after the cut (ours fixed, the original's the
press's frame). **2048: checked, not checkable** (no Team Selection fit in v1.04; the constant run is absent). **Omega: checked, differs**: the
eased fields are never read there (above), so a step has no settle to compare.

## Omega and 2048

- **Omega: checked, differs; ported on its own constants (2026-11-19, `hd-ship-craft`).**
  `eboot-ps4-omega-eu-v107.bin` carries the fit inside its own screen update, `FUN_015a3b80`
  (`0x015a3b80`): the same constant run (`0.25`, `-0.6`, `1920.0`, `-1080.0`, `-0.5`, the window
  `-389`, `535`, `-535`, `389`, the floor `11.0`), the same `pos%d` debug name, targets at
  `st[0x7b4..0x7b6]` eased into `st[0x7bb..0x7bd]`, with a projector of another lens (a 100 degree
  field against HD's 57.3). **Nothing reads the eased values**, so Omega has no depth fit on the
  craft and no settle on a step; the item is written with rest rotation `+0.4307`, `-0.4438`,
  position `(0.4, 0.1, -2.05)`, scale `0.1` and lens `100`, wired as `oag_omega`'s
  `ship_preview_pose`. The page with the evidence, the readers searched and the one fitted factor:
  [../ps4-omega-eu/team-selection-pose.md](../ps4-omega-eu/team-selection-pose.md). Omega's teams
  definition authors the same `<FE length>` (Feisar `3.8`), which the reader carries and the screen
  does not use.
- **Omega, the swap's settle: checked, differs (an earlier reading is withdrawn).** An earlier
  version of this page said Omega "eases the same targets without resetting them, so a step plays
  HD's law", and `ship_preview_settle` carried HD's fit as **inherited from HD**. The eased fields
  are never read (the page above), so Omega's craft cuts in at its rest pose: the field and the
  scale-of-pose path are removed.
- **2048: checked, differs.** The window's `535.0` occurs nowhere in the v1.04 `eboot.elf` as a
  literal (a byte search for `00 c0 05 44`; an ARM `movw`/`movt` pair would not show, so this is
  absence of the HD constant run, not proof of no fit), its `<FE>` authors `length` beside
  `firepower` and `description`, and its Team Selection is drawn by this build through 2048's
  own path, which this change does not touch.
