# The `Image` widget and its `vwipe` transition: what the Team Selection banner does on a step

2026-11-20 (`hd-banner-transition`). The banner logo on Team Selection (`<Image name="Logo" ...
transitiontype="vwipe">`) draws nothing for a few frames after a team step, then a thin slab sweeps
down it, back up it, and the logo grows in from the top. That sequence is one geometry function of one
number (the widget's alpha, `+0x68`), driven by a linear ramp (`Widget_UpdateTransition`). Everything
here is read statically from `/hdfury/EBOOT-ps3-hdfury-eu.elf` (GUI bridge, `program=` by path) and
checked against the 84 RPCS3 grabs of `hd-step-banner` / `hd-settle-step` (frame k = game frames after
the press, picture lag 3).

All addresses read TOC `0x008ad4d8` (`scripts/ps3-toc.py toc` says `exact` for each function below).

## Correction to the earlier reading

`team-selection-model.md` ("Where the banner lives", `hd-step-banner`) put the transition at "the
attribute-name table `0x008ad680` registered by `FUN_0052ea7c`". That is wrong in both halves:
`0x008ad680` is TOC slot `+0x1a8` of the main module, holding the name pointer of `Color3`; the
attribute names of the `Image` `<Values>` element are the TOC run `0x008ad658..0x008ad6ec`, read by
`Image_ParseXml`. `FUN_0052ea7c` uses another TOC (`0x008bd3c4`) and has nothing to do with `Image`.
The `Image` class is found the usual way: `scripts/ps3-toc.py map` gives `Image_Item.cpp` ->
constructors `0x001b5c90`, `0x001b6270`, `0x001b7c18`; the vtable is `0x00866210`.

## The names

Scores follow the rubric's decompilation-plus-agreement band: the reading is unambiguous and consistent with
84 emulator grabs, but no watchpoint ran, so nothing here is above 85 (what would raise it: a breakpoint on
`Widget_ResetTransition` at a team step, and `+0x68` read frame by frame).

| Address | Name | Confidence |
| --- | --- | --- |
| `0x001b5c90` | `Image_Construct` | 85 |
| `0x001b7c18` | `Image_ParseXml` | 85 |
| `0x001b6398` | `Image_Render` | 82 |
| `0x001b7720` | `Image_Update` | 72 |
| `0x00164bd0` | `Widget_UpdateTransition` | 85 |
| `0x00164df8` | `Widget_ResetTransition` | 85 |
| `0x00165018` | `Widget_ArmTransition` | 85 |
| `0x0016e580` | `Widget_ParseChildXml` | 75 |

Vtable `0x00866210` (word index -> OPD -> code; the parser is called through `+0x48` by
`Widget_ParseChildXml`, which fixes its slot):

| Slot | OPD | Code |
| --- | --- | --- |
| 3 | `0x0087ac28` | `0x001b7720` `Image_Update` (calls `Widget_UpdateTransition`, then releases/adopts the texture) |
| 5 | `0x0087abe8` | `0x001b5e78` (fills the quad's width/height from the texture when unset; unread beyond that) |
| 7 | `0x0087ac00` | `0x001b6398` `Image_Render` |
| 12 | `0x0087ac30` | `0x001b7898` (unread) |
| 13 | `0x0087ac40` | `0x001b7ae8` (unread) |
| 14 | `0x0087ac50` | `0x001b8d78` (unread) |
| 18 | `0x0087ac48` | `0x001b7c18` `Image_ParseXml` |

## How the XML selects the transition

`Image_ParseXml` (`0x001b7c18`) runs over a `<Values ...>` element with a case-insensitive key match
(`FUN_00676328` -> `FUN_00346708` -> `Str_CompareNoCase`). The keys, in the order of the TOC run
`+0x180..+0x208`: `X Y Width Height U V TxtrWidth TxtrHeight Color1..Color4 RealGlow DeltaWidth
DeltaHeight RotationTheta SlantedOffset RotationCentreOffsetX/Y Vertices BackgroundLayer Async Centred
AutoLoad Zclear Color Src SrcFury localised langualised HiddenInSD ScaleForSD OnlyScaleSD
transitiontype UseParentAlpha`. After the loop (`0x001b7ef8..`):

```
Image.transition (+0x120) = default 1                      // ctor 0x001b5c90: param_1[0x48] = 1
if transitiontype == "vwipe"  : +0x120 = 2                 // string at TOC +0x218
if transitiontype == "hscale" : +0x120 = 4                 // string at TOC +0x21c
```

The mode is a bit set: 1 (the default) means "the colour alpha is multiplied by the widget alpha"
(`Image_Render` does `colour.a *= parent.alpha * alpha` when bit 0 is set), 2 is `vwipe`, 4 is `hscale`.
`+0x124 = 4` and `+0x128 = int(4 * 0.5) - 1` (the float at `0x008ad60c`) (the quad's vertex count and last index, `Vertices`
default 4).

Two further keys come from the generic child parser `Widget_ParseChildXml` (`0x0016e580`, the one
that creates the widget from its `<Image ...>` tag; keys compared case-insensitively against the TOC run
`-0x1a20..-0x19cc` = `Type Name Global Focus Save Default Transition EnableTransition DisableTransition
OffsetX OffsetY OffsetZ Delay StartEnabled PushWidth PushHeight ...`). It stores, into the new widget:

```
+0x6c  transition-in  seconds = EnableTransition, else Transition, else the class default
+0x70  transition-out seconds = DisableTransition, else Transition, else the class default
+0x74  Delay                       // stack 0x7c(r1); stfs at 0x0016e9e8
+0x40/+0x44/+0x48 OffsetX/Y/Z
```

(`0x0016e960..0x0016e9e8`: `fcmpu` against the unset sentinel, the float at `0x008c3678` = `0x00800000`
= FLT_MIN.) **An authored `0` is therefore "set"**: `+0x6c = 0` means no ramp, alpha 1. The four panel
images of `LogoOutline` author `transition="0"` and the grabs show that panel at a constant 30.10 in all 28
bare frames, which is a second, independent corroboration of this reading of the parser. The class
default for `Image` is **0.7 s**: the constructor stores the float at `0x008ad624` (`0x3f333333`) into
`+0x6c` and 0 into `+0x70`.

The Team Selection `Logo` tag is `delay="0.5" enebletransition="0.5" disabletransition="0"`. `delay` and
`disabletransition` match; **`enebletransition` matches nothing** (the key is `EnableTransition`; one
letter off), so `+0x6c` stays at the class default 0.7 s. The XML's 0.5 s is dead; the half-second
`Delay` (`+0x74`) is the page-entry delay, not part of a team step (see "On a step" below).

## The alpha ramp: `Widget_UpdateTransition` (`0x00164bd0`)

Called once a frame with `f1 = dt` (seconds) and the widget; vtable slot 3 of `Image`. State: `+0x68`
alpha, `+0x6c` in-time T, `+0x70` out-time, `+0x9c` elapsed timer, `+0xa0` delay countdown, `+0x64` the
parent. Constants: `0x008ab800` = 0.0, `0x008ab848` = `0x3a83126f` = 0.001, `0x008ab84c` = `0x38d1b717`
= 1e-4.

```
if +0x6c == 0:                              // no transition authored
    delay branch (+0xa0) ; alpha = parent.in ? 1 : 0
else if parent.+0xc4 == 0:                  // parent leaving: ramp OUT
    +0x9c = clamp(+0x9c - dt, 0, +0x70) ; alpha = +0x9c / +0x70   (0 if +0x70 == 0)
else:                                       // parent present: ramp IN
    if +0xa0 > 0 { +0xa0 -= dt ; if +0xa0 > 0 { alpha = 0.001 ; return } }
    +0x9c = clamp(+0x9c + dt, 0, +0x6c) ; alpha = +0x9c / +0x6c
```

So `alpha` is a **linear ramp** over `+0x6c` seconds after the delay: `alpha = clamp(t, 0, T) / T`.
While the delay is still counting, alpha is 0.001 (and `Image_Render` draws nothing, see below).

## What is drawn: `Image_Render` (`0x001b6398`)

Early out: `if (0.0 < +0xa0) return` (still in the delay: nothing drawn at all). Then it builds the
quad (position `x = +0xa4 + parent.+0x4c`, `y = +0xa8 + parent.+0x50`, size `w = +0xac*scale + alpha*DeltaWidth`,
`h = +0xb0*scale + alpha*DeltaHeight`, UV window `(u0,v0,du,dv) = (+0xcc,+0xd0,+0xd4,+0xd8)`) and, **if
`+0x120 & 2` and `alpha != 1.0`**, replaces the height, y and v of that quad by the `vwipe` terms below.
With `alpha == 1.0` (`0x008ad5f8`) it is the plain quad. Let `s = alpha` (`+0x68`, as `param_1[0x1a]`),
`H = h`, `Y = y`, `V0 = v0`, `DV = dv`, constants `0x008ad5e8..0x008ad60c`:

| Address (TOC slot) | value | role |
| --- | --- | --- |
| `0x008ad5e8` | 0.4 | end of phase A |
| `0x008ad5ec` | 0.1 | slab thickness as a fraction of H |
| `0x008ad5f0` | 0.8 | end of phase B |
| `0x008ad5f4` | 0.9 | apex position |
| `0x008ad5f8` | 1.0 | "no wipe" |
| `0x008ad5fc` | 2.25 | phase A speed (0.9 / 0.4) |
| `0x008ad600` | -0.5 | centring (y) |
| `0x008ad604` | -2.25 | phase B speed |
| `0x008ad608` | 4.5 | phase C speed (0.9 / 0.2) |
| `0x008ad60c` | 0.5 | centring (y, phase C) |

The branch tests are at `0x001b688c` (`+0x120 & 2`, `beq 0x001b7308` when not vwipe), `0x001b68d4..0x001b68e0`
(`fcmpu alpha, 1.0`; `bne 0x001b73f8`), and the vwipe block is **`0x001b73f8..0x001b74a0`**: `0x001b7400`
`fcmpu s, 0.4`; phase A `0x001b7408..0x001b7434` (`fmuls f13 = s * 2.25`; `fmadds y = H*f13 + y`;
`fmadds v0 = DV*f13 + v0`; `H *= 0.1`; `DV *= 0.1`); phase B `0x001b7438..0x001b7460` (`fcmpu s, 0.8`;
`f12 = s - 0.4`; `f13 = f12 * -2.25 + 0.9`; then the same tail as A); phase C `0x001b7464..0x001b74a0`
(`f12 = (s - 0.8) * 4.5`; `H' = H*f12 + H*0.1`; `DV' = DV*f12 + DV*0.1`; `y` and `v0` untouched, `y` recentred by
`0.5 H' - 0.5 H` when `Centred`).

```
phase A  0.0 <= s < 0.4 : f = 2.25 s              ; slab at fraction f, thickness 0.1 H, sweeping DOWN
phase B  0.4 <= s < 0.8 : f = 0.9 + (s-0.4)*-2.25 ; same slab, sweeping back UP (0.9 -> 0.0)
   quad:  Y' = Y + H*f ; H' = 0.1 H ; V0' = V0 + DV*f ; DV' = 0.1 DV      // texture rows pass through unscaled
phase C  0.8 <= s < 1.0 : g = (s-0.8)*4.5 + 0.1  ; the logo grows DOWN from its top edge
   quad:  Y' = Y ; H' = H*g ; V0' = V0 ; DV' = DV*g      // g = 0.1 at s=0.8, 1.0 at s=1.0
alpha == 1.0            : plain quad
```

When `Centred` (`+0x11c & 1`) is set the quad is shifted by `-0.5 * size` afterwards; the Logo is not centred.
**`vwipe` replaces mode 1**: with `+0x120 = 2` the vertex colours are not scaled by the widget alpha
(`Image_Render` takes its plain-colour path when `+0x120 & 1` is clear), so the slab and the growing quad draw
at full opacity; only the geometry follows `s`.

So what a viewer sees on a step (the texture has ink only in the middle of its rows): the new logo's quad
is reset to `s = 0`; a 0.1 H slab at the top shows blank texture rows (bare panel), slides down through the
ink rows (the 12 px slab), runs off the bottom, comes back up through the ink rows (the second sweep), runs
off the top, and from `s = 0.8` the quad grows from the top until it covers the ink rows (whole logo)
at `s` of about 0.93. **That is the "two sweeps and a gap, not a plain wipe" shape of
`team-selection-model.md`.**

## On a step: `Widget_ResetTransition` (`0x00164df8`)

`TeamSelectionScreen_Update` (`0x00254348`, around `0x00255698..0x002557bc`): when the selection moved it
creates the new team's logo texture (`FUN_002de5c8`), stores it in the widget named by TOC `+0x4784`
(`0x008b1c5c` -> `0x00799348` = `"Logo"`) at `+0xdc/+0xe0` (and resets its UV scale `+0xec/+0xf0 = 1.0`,
`+0xf4/+0xf8 = 0`), then calls `Widget_ResetTransition(Logo)` at `0x002557bc`. That function sets `+0x9c
= 0` and `+0x68 = 0` on the widget and on every descendant (a recursive walk through `+0x14` children /
`+0x10` siblings), and does not touch the delay countdown `+0xa0`. `+0xa0` was spent when the page came
in, so after a step the ramp restarts with **no delay**: `alpha` runs 0 -> 1 over `+0x6c = 0.7 s` and the
`vwipe` replays. Other callers of the same reset: `RaceBox_UpdateTrialPadlocks` (`0x00249b40`), `TrackSelectionScreen_SelectCircuit`
(`0x0025cb8c`), and, unattributed, the functions at `0x001f2bd8` (call `0x001f2ed0`), `0x002308a0`
(`0x00230ad0`), `0x00257998` (`0x00257e38`) and `0x0025e960` (`0x0025eb2c`, `0x0025eb38`, `0x0025ec5c`,
`0x0025ec68`): all the same "restart this widget's transition" call.

## Check against the 84 grabs

`s = (p - p0) / N` with `N = 60 T`, `p` the picture frame, `p0` the picture frame at which `s` is 0. The
ink extents of the slab are the rows of the 470x130 banner crop at (200, 200) whose difference to the
settled logo exceeds 12 on more than 3 pixels. Slab top row, phase A: 9: 33, 10: 39, 11: 45, 13:
55, 15: 68.5; phase B: 25: 82, 27: 69, 29: 55. A least-squares line gives +5.8 px/frame in A and -6.75 in
B, mean **k = 6.28 px/frame**, and the two lines cross at frame 21.5 at row 105.7 (the apex).

The model says `k = 2.25 H / N` and an apex of `0.9 H` below the quad top. Solving for `N` from `k`:

| H (px) | N (frames) | T (s) | p0 | apex vs 0.9 H |
| --- | --- | --- | --- | --- |
| 115.2 (128 at the 90 % safe area) | 41.3 | 0.688 | 5.0 | 105.7 vs 103.7 |
| 120 | 43.0 | 0.717 | 4.3 | 105.7 vs 108.0 |
| 128 | 45.9 | 0.765 | 3.2 | 105.7 vs 115.2 |

With the quad height at the safe-area-scaled 115 to 120 px the measured speed gives 0.69 to 0.72 s, the
code's 0.7 s, from an independent number. The slab thickness agrees too: 11 to 12 ink rows (0.1 H =
11.5 to 12). Phase C and the end: `g` reaches 0.69 (ink rows 35 to 85 of a 115 px quad) at `s = 0.93`,
frame about 43 to 44; the grabs show a 30-row band at 41 and the whole logo at 45.

The offset between the press and the ramp's start depends on the unpinned quad height H: **picture frame 3.2
(H = 128, T = 0.77 s: no offset) to 5.0 (H = 115, T = 0.69 s)**, i.e. 0 to 2 game frames after the press at
lag 3. Pinning the Logo quad's height at 1080p from the safe-area layout would close it (a wiring lane has
ours at rest, which matches the original's). The first ink at 8 is consistent with all three rows. The old logo is gone at frame 3 (the texture swap) as the bare panel shows. The one to
two frame offset is **unread**: it is either the screen detecting the step a frame or two after the
settle starts, or the new texture's async load (`Async`, `FUN_002de5c8`) making the widget skip its first
frames; one of the unread vtable slots (`0x001b7898`, `0x001b79c0`, `0x001b7ae8`) may hold it.

## What is still unread

- Who arms the delay countdown `+0xa0` from `+0x74` on page entry (the Logo's authored `delay="0.5"` plays
  once per page entry, then a 0.7 s ramp); the page-entry timeline is not measured here.
- The exact frame at which Team Selection resets the Logo relative to the press (see above).
- `hscale` (`+0x120 = 4`, used by the thin lines of `Track Select`): only `dVar43 = width * alpha` is read.
- Image vtable slots 12 to 14 (`0x001b7898`, `0x001b7ae8`, `0x001b8d78`).
- The lattice slide over frames 4 to 10 and the pulsing ring's displacement do **not** fall out of these
  functions (`HexSelection`'s code is a different class, not touched here).

## Wired: the Team Selection logo (2026-11-20, `hd-banner-wipe`)

`oag_ui::screen::Vwipe` (`crates/ui/src/screen/wipe.rs`) is the law above: `Screens::vwipe_from_node` reads
`transitiontype="vwipe"` (case-insensitive) and the in-time (`EnableTransition`, else `Transition`, else the 0.7 s
class default, so `enebletransition` is not read), `Vwipe::alpha` is the linear ramp, `Vwipe::window` the three phases as
fractions of the quad's height. `picker::hd::logo_draw` scales the quad and the texture's rows by the window and draws
at full opacity, from `Picker::seconds_since_step` (the clock `land_on` zeroes on a team step). Tests: the phase boundaries
(0, 0.39, 0.4, 0.79, 0.8, 0.99, 1) in `screen/wipe/tests.rs`, a pad step through `Picker::update` to a 12.8-row slab in
`picker::hd::tests::a_team_step_replays_the_logos_vwipe`, and the disc's `Logo` reading 0.7 s in
`selection_screens_ground_truth::hd_reads_its_own_team_selection`.

Banner box `190,185,680,335`, mean absolute grey against the original, ours at picture lag 3, the seven step pairs
(frame k): k04 29.66 to 16.07, k06 19.27 to 16.07, k08 18.76 to 16.07, k10 20.54 to 16.83, k15 17.15 to 16.22, k19 25.97 to
16.07, k29 15.41 to 16.02. 16.07 is the floor when both sides show the bare panel (ours fills it 50 against 31, the open
`HD_LightGrey` item: the 13.6 to 14.6 of a whole logo had ink matching on both sides). Slab rows and phases read by eye match
(bare at k04, k06, k08, k19; the slab at k10 near the top, k15 and k29 near the apex). **The unread start offset shows**: starting
the ramp 0, 1 or 2 frames after the press gives k10 16.83, 15.61, 16.69; k15 16.22, 16.03, 15.48; k29 16.02, 15.80, 15.98; k19
16.07, 16.07, 16.23; no single offset is best for all, so none is applied (the offset is 0 to 2 frames, unpinned, above).
Not played: the screen's opening (the original waits the authored `Delay`, then ramps; the arming is unread), so a
fresh screen draws the finished logo; the other `vwipe` images (below).

**Other screens that author `vwipe`** (DATA06 and the patch archives, `Image` tags): Team Selection `Logo` (played), Track
Selection's circuit emblem (192x192; `TrackSelectionScreen_SelectCircuit` restarts it), `cellmode_definition` (9), `racebox` (5),
`recordgrid` (6), `showunlocks` (3), `controls_definition_ps3` (4), `credits` (2), `additional` (1), `network` (44). `hscale`:
Track Selection's two lines, `endrace` (16), `network` (11). None of these plays yet: each needs the page's own entry clock.

## Page entry: who arms the `Delay`, and when each page's wipe starts (2026-11-20, `hd-vwipe-pages`)

Measured in RPCS3 RAM (cold boot, own instance, `scripts/rpcs3-hd-vwipe.py`; game not paused: the widgets are polled
every 1 to 3 ms, 2 to 5 polls a frame). **Every front-end page's widget tree exists from boot**, so each `Image` widget is
one heap address for the whole session (`scan` finds the 71 `vwipe` widgets by the vtable word `0x866210` and a `+0x120`
with bit 2: Controls 2, ControlsP2 2, Credits 2, Race Records 6, Show Unlocks Root 3, Single Player 2, MP Select 2, Grid
Selection 2, Grid Selection Fury 2, Cell Selection 5, Team Selection 1, TrackSelectionTopLevel 1, SoundTest 1, Edit
Settings 2, Game List 18, GameLobby 18, FilterList 2). At rest every one reads `s = 0`, `+0x9c = 0`, `+0xa0 = 0` (or a
few frames' leftover below 0).

### The arming: `Widget_ArmTransition` (`0x00165018`)

```
Widget_ArmTransition(w):                 // recursive over +0x14 children / +0x10 siblings (unrolled eight deep)
    w+0xa0 = w+0x74                       // the delay countdown := the authored Delay
    w+0x9c = 0 ; w+0x68 = 0               // timer and alpha restart
    for each child: Widget_ArmTransition(child)
```

It is `Widget_ResetTransition` (`0x00164df8`, above) plus the one store that function leaves out, `+0xa0 = +0x74`. The
`0x00878208` data reference is a vtable slot. Callers (`get_xrefs_to`, 55 call sites in 22 functions): `TrackSelectionScreen_OnEnter_q`
(`0x0025d454`), `0x00252c10` (`0x00253440`, the function before `TeamSelectionScreen_Update`), `ShowUnlocks_Screen`
(`0x0024d010`, `0x0024d5b4`), the cell screen's refresh `0x002181a8` (six sites, one per widget it re-textures) and
`0x002452f0`, `0x002587a0`, `0x00225490` (7), `0x0021a570` (6), `0x0021f488`, `0x0021f9e8` (4), `0x0021c9d8`,
`0x001c5278` (16), `0x001c4a10`, `0x001c2e00`, `0x001c2ef8`, `0x001a3d80`, `0x001a6460`, `0x001a7db8`, `0x001a8250` and
`0x00202d28` (an unnamed helper the cell refresh calls with a widget and a string, so probably a text setter). Which
of these is the hook of Controls, Credits, Race Records, Racebox, Grid Selection and Team Selection is unread (only
`TrackSelectionScreen_OnEnter_q` and `ShowUnlocks_Screen` are named); the RAM reads above do not depend on it. `Image_Update` has no arming.
`Widget_UpdateTransition` then does `+0xa0 -= dt` each frame and ramps `+0x9c` the frame the countdown reaches 0
(the `+0x9c += dt` is in the same call, so the first ramp frame is the frame the countdown crosses 0).

### What RAM shows (8 pages, 2 to 4 entries each, two boots for four of them)

At the frame a page is entered its root's present flag (`+0xc4`) rises and, in the same frame (same poll or the next
2 ms), every `vwipe` widget under it reads `+0xa0` = its authored `Delay` (`+0x74`) exactly, `s = 0.01` for that first
poll and 0.001 after, then `+0xa0` falls by `dt` a frame. The ramp (`+0x9c` above 0) starts the frame it crosses 0 and
runs the in-time (`+0x6c`) to `s = 1`. **So the wipe starts `Delay` game-seconds after the page-entry frame and runs `+0x6c`
seconds**; at 60 Hz (`dt` = 1/60) that is `ceil(Delay * 60)` frames of nothing and `0.5 * 60` = 30 frames of ramp. The
emulator's `dt` was 16.7 ms on Racebox, Cell Selection and Grid Selection, 17.7 to 21 ms on Credits, Race Records and
Team Selection and 16.7 to 18.5 on Track Selection (the game's timestep is variable), so frame counts below are at 60 Hz and
the game-second figures are the measurement.

| Page (root, TTY screen) | Widget (authored `Delay`, in-time) | Start after entry (60 Hz frames), ramp | Seen |
| --- | --- | --- | --- |
| Controls (`Controls Menu` then `Controls`) | `Item 1957` (0.25, 0.5), `Item 1958` (0.7, 0.5) | 15 and 42 frames, 30 | 1 clean entry + 1 `discover` entry; armed value 0.25 / 0.70 exactly |
| Credits (Extras tab, `Credits`) | `Item 5349` (0.6, 0.5), `Logo` (0.8, 0.5) | 36 and 48, 30 | 2 entries |
| Race Records (Records tab, `Race Records`) | `Item 1830` 0.6, `TaskLogo` 0.8, `Item 1834` 0.8, `TrackLogo` 1.0, `Item 1837` 1.0, `ClassLogo` 1.2 (in 0.5) | 36, 48, 48, 60, 60, 72; 30 | 3 entries, first-sight countdown = authored in all 18 |
| Racebox (`Single Player`) | `Item 315` (0.45, 0.5), `BigIcon` (0.8, 0.5) | 27 and 48, 30 | 4 entries (3 timed + 1 `discover`) |
| Track Select (`Track Creation` in the TTY, root `TrackSelectionTopLevel`) | `Emblem` (0.6, 0.5) | 36, 30 | 3 timed entries + 1 `discover`; `+0xa0` 0.6 at the flag's frame |
| Team Selection (the `Logo`) | `Logo` (0.5, **0.7**) | 30, 42 | 3 entries; the flag rises 0.76 to 0.80 s after the press (the page switch), the countdown starts there |
| Campaign grid (`Grid Selection Fury`) | `unlockboxbrackets` (0.45, 0.5), `flyerlogo` (0.65, 0.5) | 27 and 39, 30 | 3 entries + 1 `discover` |
| Cell Selection (`Cell Selection`) | `flyerlogo` (0.8, 0.5) | 48, 30 | 3 entries |
| Cell Selection emblems | `Event Emblem` 0.05, `Track Emblem` 0.125, `Speed Class Emblem` 0.2, `Weapons Emblem` 0.275 (in 0.3) | **22 frames (0.37 s hold) plus 3, 8, 12, 17**; 18 | 3 entries + 2 on a second boot |

**Stopped at aimed countdowns** (`rpcs3-hd-vwipe.py stop`, Racebox, game stopped through the GDB stub, read in the stop and again
0.3 s later, equal every time): stopping when `Item 315`'s countdown read at or below 0.6, 0.3, 0.1 and at its first ramp frame
gave `+0xa0` = 0.4333, 0.2995, 0.0835 and -0.0 (authored 0.45), the beside `BigIcon` (authored 0.8) at 0.7833, 0.6495, 0.4335,
0.35, so both count the same game-seconds from one arming frame (0.45 s at the last stop for both). At the last stop
`Item 315` read `+0x9c` = 0.0164 (one `dt`) and `s` = 0.0328 = `+0x9c / 0.5`, while `BigIcon` was still at `s` 0.001: the ramp
starts on the frame the countdown crosses 0, with one `dt` already added. Confidence for the law (start = `Delay` after
the present flag rises, ramp = in-time, linear): 80 (eight pages, 2 to 4 entries each; a second boot repeated Racebox, Track Select, Grid Selection Fury and Cell Selection with the same
numbers and, at other addresses, the same 71 widgets; what would raise it: a page-hook breakpoint).

**Cell Selection's emblems wait a hold of about 0.37 s before their own delay.** Their `+0xa0` reads the authored value minus one
frame for the first 0.39 to 0.42 game-seconds after the page's arming (the last re-arm, from the `flyerlogo` countdown as the clock:
0.416, 0.389, 0.398 and 0.40 in four entries): the cell screen's refresh (`0x002181a8`, which re-textures the four emblems and calls
`Widget_ArmTransition` on each) runs every frame until then, re-arming them each time. The `flyerlogo` beside them is armed once and
starts at its own 0.8 s. The ramp start of each emblem, measured as game-seconds on the `flyerlogo` clock less its own delay, is 0.349
to 0.399 (mean 0.372) over 20 starts in 5 entries on two boots, so the emblems start `0.37 + Delay` after entry (0.42, 0.50, 0.57, 0.65 s),
about 22 frames of hold at 60 Hz. What ends the hold is unread (it is not an asset load: the first entry and the later ones
hold the same time). Confidence for the 0.37 s: 65.

### Not reached

- **Network** (`MP Select`, `Game List`, `GameLobby`, `Edit Settings`, `FilterList`, 44 widgets): the ONLINE tab opens
  `NetLoginPage` and falls back to `Main Menu` without a PSN session, so no network page was entered. The authored
  `Delay` and in-time of all 44 are in RAM at rest (`scan`): GameLobby 0.5 to 0.95 (in 0.3), Game List 0.35 to 0.9 (in
  0.3), `MP Select` 0.45 and 0.8 (in 0.5), `Edit Settings` 0.45 and 0.8, `FilterList` 0 (in 0.5 and 0.7). By the same
  arming, they start `Delay` after entry; unmeasured.
- **Show Unlocks** (`dots`, `closed`, `open`: `Delay` 0.1, in 0.5): the screen flashed through after a quit-race and left
  by itself, so its entry timeline was not polled. Its arming caller is `0x0024d5b4`; by the same function it starts 0.1 s
  after entry, unmeasured.
- **`additional`**: the options page itself (`Additional`, the tab bar) has no `vwipe` that moves (`discover` saw 0 of 71
  move on entry, twice); the one `additional_definition.xml` `vwipe` is `SoundTest`'s `Item 1150` (`Delay` 0.2, in 0.5), a
  page this lane did not reach.

### What to wire

Per `Image`: read `delay` (the XML key, matched case-insensitively by `Widget_ParseChildXml`; an unset one is 0) as well as
the in-time, and hold the wipe at `s = 0` (nothing drawn: `Image_Render` returns while `+0xa0 > 0`) until `delay` seconds after
the frame the page becomes the current one, then ramp. Today `Screens::vwipe_from_node` reads only the in-time. The page-entry
clock starts when the page's present flag rises, not at the button press (Team Selection's is 0.76 to 0.80 s after it,
Racebox's within a frame). Two page-specific rules: Cell Selection's four emblems add the 0.37 s hold; a changed
cell (`TrackSelectionScreen_SelectCircuit`, the cell screen's refresh) re-arms with the delay again where the caller is
`Widget_ArmTransition`, and without it where the caller is `Widget_ResetTransition` (the circuit emblem's restart).

## Omega and 2048

- **Omega: checked, applies in part; wired by lineage, unmeasured.** Omega's Ship Select runs the same `picker::hd` layout and
  its `team_selection_definition.xml` authors the same `Logo`, so a step plays the wipe there (`--menu-page ship-select
  --menu-picker-step-seconds 0.15` shows the slab, 0.45 the slab on its way up, 5 the finished logo). The draw law below is not read in Omega. `eboot-ps4-omega-eu-v107.bin` has the same `Image` `<Values>` parser
  (`FUN_01462af0`, `strcasecmp` key match; `transitiontype` at `0x01463323`, the `vwipe` string referenced at
  `0x0146376e`), and its post-loop sets `+0x1c4` to `vwipe` = 2 and `hscale` = 4, exactly HD's two modes, plus
  four HD's parser lacks: `hgrow_right` = 8, `hgrow_left` = 0x10, `vgrow_down` = 0x20, `vgrow_up` = 0x40, and
  `none` = 0. The strings `EnableTransitionType` and `DisableTransitionType` are in the binary (`0x0188368c`,
  `0x018836a1`); their consumer is not read. The Omega draw, the alpha ramp and the reset are **not read**
  and the `2.25`/`4.5` constants were not searched for in the x86 code, so the three-phase law is HD's;
  Omega's `vwipe` geometry is a lineage inheritance, unmeasured.
- **2048: checked, shares the key.** `vwipe` is a string in `eboot-vita-2048-eu-v104.elf` (`0x8143b1d4`);
  parser and draw not read.
