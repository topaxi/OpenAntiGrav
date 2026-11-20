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

The reset lands at **picture frame 4.3 to 5.0** (game frame 1.3 to 2.0 at lag 3) rather than frame 3
(game frame 0): the first ink at 8 and the line fit say the ramp starts one to two frames after the
selection moved. The old logo is gone at frame 3 (the texture swap) as the bare panel shows. The one to
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

## Omega and 2048

- **Omega: checked, applies in part.** `eboot-ps4-omega-eu-v107.bin` has the same `Image` `<Values>` parser
  (`FUN_01462af0`, `strcasecmp` key match; `transitiontype` at `0x01463323`, the `vwipe` string referenced at
  `0x0146376e`), and its post-loop sets `+0x1c4` to `vwipe` = 2 and `hscale` = 4, exactly HD's two modes, plus
  four HD's parser lacks: `hgrow_right` = 8, `hgrow_left` = 0x10, `vgrow_down` = 0x20, `vgrow_up` = 0x40, and
  `none` = 0. The strings `EnableTransitionType` and `DisableTransitionType` are in the binary (`0x0188368c`,
  `0x018836a1`); their consumer is not read. The Omega draw, the alpha ramp and the reset are **not read**
  and the `2.25`/`4.5` constants were not searched for in the x86 code, so the three-phase law is HD's;
  Omega's `vwipe` geometry is a lineage inheritance, unmeasured.
- **2048: checked, shares the key.** `vwipe` is a string in `eboot-vita-2048-eu-v104.elf` (`0x8143b1d4`);
  parser and draw not read.
