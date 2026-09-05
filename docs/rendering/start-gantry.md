# The start gantry: how a `3`, `2`, `1`, `GO` is drawn

The object standing over the start line, not the HUD graphic drawn on the
screen. Two different things have both been called "the countdown display" in
this tree; [`ui/hud.md`](../ui/hud.md) covers the on-screen `<Mode3D>` overlay
(`Pulse_Ready_Go` / `Cockpit_321GO`, identical across every mode on every title
checked), and this page covers the physical gantry, which arrives through
`TrackStartup.xml`'s **billboard slot 8** ([`formats/README.md`](../formats/README.md)'s
Track startup row).

**Read on Wipeout Pulse (`pulse-psp-usa.chd`, `UCUS-98712`) only.** Pure, HD/Fury
and 2048 ship the same family of assets and are unmeasured here; the closing
section says what looks title-wide and what does not.

> **Nothing on this page places the gantry.** Where slot 8's transform comes
> from is still unrecovered - see
> [`ghidra/functions/psp-pulse-usa/billboards.md`](../ghidra/functions/psp-pulse-usa/billboards.md),
> whose highest-priority open item it remains. The model is loaded by nothing
> and the load report names the file it wanted instead. What is recovered here
> is what the model *does* once something draws it.

## The asset

`Data\Environments\321_Go\321Go_StartFinish.vex`, 54,032 bytes in `Data.wad`.
Named by slot 8 of every one of the eleven Pulse circuits that author a
manifest, and it is the **only** gantry model on the whole disc (below).

28 nodes: one `World`, two `Transform` (`camera1`, and a bare childless
`start_lights` locator), nine `Anim Transform` (`0x3c0`) each carrying one
`Mesh`, and six `Texture`. 1,728 triangles, radius 21.44.

```text
world
  camera1 -> cameraShape1                      (a Maya camera, not drawn)
  start_lights                                 (locator, no children)
  start_light_background      -> Mesh            17 tri
  Text                        -> Mesh           436 tri
  Arrow                       -> Mesh           123 tri
  Board                       -> Mesh            17 tri
  Honey_Board                 -> Mesh            48 tri
  polySurface7                -> Mesh            68 tri
  polySurface17               -> Mesh            49 tri
  Jons321go:Jons321go:start_light_321go -> Mesh 474 tri
  JonsFinalLap:Final_Lap      -> Mesh           496 tri
  6 x Texture: 321go_NOMIP.tga (16x32), Fx400_nomip.tga (128x32),
               Honey_LightBlue.tga, Honey3.tga, Checkered.tga, Honey2_KEY.tga
```

**All six textures are colour palettes, not glyph atlases.** Every piece of
lettering the gantry shows - the digits, `GO`, `FINAL LAP` - is geometry. That
is the fact the rest of this page turns on, and it is what makes a 16x32 texture
enough for a whole countdown.

## The mechanism: four UV cells walked across a palette staircase

**Confidence 90.** Read straight off the file and confirmed by rendering it;
nothing here is decompiled or inferred.

`start_light_321goShape` carries **exactly four distinct per-vertex `(u, v)`
pairs**, and each one is a glyph. Their model-space `x` spans say which:

| glyph | `u` | u in texels | `v` | v in texels | vertices | x span |
| --- | --- | --- | --- | --- | --- | --- |
| `3` | 0.054 | 0.86 | 0.984 | 31.49 | 87 | -17.11 .. -4.66 |
| `2` | 0.187 | 2.99 | 0.984 | 31.49 | 102 | -5.33 .. 6.59 |
| `1` | 0.304 | 4.86 | 0.984 | 31.49 | 61 | 5.72 .. 16.87 |
| `GO` | 0.492 | 7.87 | 0.953 | 30.50 | 230 | -12.22 .. 11.59 |

Left, middle, right, then one wide centred shape - which is what "3 2 1" side by
side and a `GO` across the board looks like.

**Order them by `u`, not by where they sit.** `u` is the axis the offset track
walks, so it is the mechanism's own order - and it is the only one that gets the
labels right: `GO` spans the whole board, so its left edge sorts it *second* by
x, between `3` and `2`. A first pass at the ground-truth test sorted by x and
mislabelled two of the four while still looking like it measured something.

`321go_NOMIP.tga` is a **staircase**, not a ramp. Its 16x32 texels, by region:

```text
rows  0..21   col 8 opaque white; everything left of it alpha 0
rows 16..22   cols 0..5 dark red (100,49,49) but alpha 0
rows 23..29   cols 0..5 dark red, alpha 255 - except three white pairs:
                rows 24..25  cols 4..5   opaque white   -> lights `1`
                rows 26..27  cols 2..3   opaque white   -> lights `2`
                rows 28..29  cols 0..1   opaque white   -> lights `3`
rows 30..31   alpha 0 left of col 8
cols 12, 14   full-height red and green columns, unused by this node
```

The material's own texture-transform block - the `TEXOFFSET` keyframe track
[`formats/vex.md`](../formats/vex.md) documents and `oag_formats::vex::mesh_tex_transforms`
already parses - then slides one shared offset across it:

```text
loop 6.000 s    step false    seconds_per_key 1/60
TEXOFFSET, (u, v) in 1/256 units:
    frame   0 -> (  0,    0)
    frame 180 -> (  0,  -64)     v -0.25   =  -8 texels
    frame 181 -> (  9,  -64)
    frame 182 -> ( 12,  -64)     u +0.047  = +0.75 texel
    frame 358 -> ( 12, -127)     v -0.496  = -15.875 texels
TEXSCALE constant (256, 256)
```

`v` slides from texel 31.49 to 23.49 over the first three seconds while `u`
holds, so **each glyph's own column crosses its own white pair at its own
moment**. The windows fall out of the grid arithmetic alone:

| glyph | palette columns | white rows | lit from | to |
| --- | --- | --- | --- | --- |
| `3` | 0-1 | 28-29 | 0.93 s | 1.31 s |
| `2` | 2-3 | 26-27 | 1.68 s | 2.06 s |
| `1` | 4-5 | 24-25 | 2.43 s | 2.81 s |
| `GO` | 8 | 0-21 | ~3.6 s | 5.97 s |

Outside its window a digit is the dark red the same columns hold at rows 23-29,
and above row 23 (past 3.18 s) or below row 29 (before ~0.56 s) it is alpha 0 -
gone rather than dimmed. `GO` reaches its own opaque column only after the
`u` step at frames 181-182, which is what hands the board over from the digits.

**`GO`'s column is not solid, and that is a real difference, not noise.**
Columns 0-5 are opaque across the digits' whole row band, but columns 8-9
*alternate* opaque and alpha-0 row by row over rows 0-21 (opaque on 0, 1, 2, 4,
6, 8, 10, 12, 14, 15, 16, 18, 20, 21; transparent on 3, 5, 7, 9, 11, 13, 17,
19). Scrolling through that makes the word **flash** rather than fade in, while
the digits do not. Over the 3.60-5.95 s span `GO` owns, sampling every 50 ms
finds it lit on 33 of 47 samples and dark on 14. A test that pinned `GO` to one
instant the way the digits are pinned would answer by luck; the ground-truth
test asserts the strobe instead.

**Verified headlessly**, one capture per phase, with
`oag-view --mesh ... --only start_light_321go --anim-seconds T --yaw 1.5708
--pitch 0 --screenshot`:

| `T` | what renders |
| --- | --- |
| 0.0 | nothing (alpha 0) |
| 1.0 | `3` white, `2` and `1` absent |
| 2.0 | `2` white, `3` fading dark red, `1` dark |
| 2.6 | `1` white, `3` and `2` dark red |
| 3.0 | all three dark red together - `v` parks at row 23.49 |
| 4.0 | `GO`, part-lit |
| 5.9 | `GO` white |

The `T = 2.6` frame was **predicted from the texel grid and then rendered**,
which is what takes this from a plausible reading to a measurement.

So: not node animation, not per-node visibility, not a material swap, and not an
atlas-cell swap. A shared UV offset against a per-glyph palette column. The
project already implements both halves - `vex::mesh_tex_transforms` parses the
track and `oag_render`'s `TexAnims` replays it per frame - so **no new rendering
mechanism is needed to play this once something places the model.**

## What is not proven: that the engine plays the authored track

**Confidence 55, and deliberately kept apart from the 90 above.**

The asset authors the sequence. Whether the *engine* runs that authored track,
or writes the offset itself per countdown state, is unrecovered. There is a live
competing candidate:
[`billboards.md`](../ghidra/functions/psp-pulse-usa/billboards.md) found slot 8's
material object holding **four** wrapper sub-objects at `+0x88`, `+0x90`,
`+0x98` and `+0xa0`, plus a per-frame accumulator at `0x0890cf34` gated by a
flag byte:

```text
lbu   a1, 0xc0(a0)         ; gate flag
bne   a1, zero, +0x1c      ; skip the accumulation while it is set
lwc1  f13, 0x40(a0)
add.s f12, f13, f12        ; += elapsed time
swc1  f12, 0x40(a0)
```

Four state groups for four states is exactly the shape a per-state write would
have. Neither candidate has been traced.

These two claims are kept separate on purpose. `billboards.md` merged a pair of
claims this shape once - "construction writes no transform" became "nothing
overrides what is drawn" - and the project owner caught it from playing the
original. The scores here are 90 for what the file authors and 55 for what the
engine does with it, and they are not the same sentence.

## Timing against the measured countdown

The numbers, without a reconciliation:

- `GO` first lights **3.03-3.6 s** into the asset's own clock.
- The measured thrust gate is **272 ticks = 4.533 s**
  (`oag_race::COUNTDOWN_TICKS`, three live captures - see
  [`gameplay/race-modes.md`](../gameplay/race-modes.md#the-countdown-is-measured)).
- The countdown panel **exits at frame 360/361 = 6.000 s**, 2.97 s after `GO`,
  precisely as the 6.000 s texture loop closes.

That leaves roughly **1.5 s, or 90 ticks, of unexplained pre-roll** between the
asset's `GO` and the measured green. Nothing measures it, and no wiring should
assume it. The gated accumulator above is where the alignment has to live: the
clock the renderer plays `Anim Transform` from today is `g_ingame->0x40`
([`anim-transform.md`](../ghidra/functions/psp-pulse-usa/anim-transform.md#what-advances-g_ingame-0x40)),
a free-running global advanced by `InGame_Update`, and a free-running clock
cannot drive a countdown - which is a second, independent reason a per-object
gate must exist.

## One asset, the whole race

The same file carries every state the gantry ever shows, as non-overlapping
windows on one 60 Hz timeline. From the nine `Anim Transform` nodes' own keys:

| frames | seconds | what happens |
| --- | --- | --- |
| 0..358 | 0..5.967 | the countdown panel plays `3`, `2`, `1`, `GO` |
| 350/351 | 5.833/5.850 | `Board`, `Text` and `Arrow` teleport in from y = -34 to y = 0 |
| 351..407 | 5.850..6.783 | `Arrow` scales 2.5 -> 1.0 |
| 360/361 | 6.000/6.017 | `start_light_background` **and** the digit panel teleport +9.99 in y - out of the aperture, as the texture loop closes |
| 407..435 | 6.783..7.250 | `Text` slides x 10.46 -> 0 |
| 560/561 | 9.333/9.350 | `Board` drops away; `Honey_Board` and `Final_Lap` slide in, x -33 -> +33 |
| 740/741 | 12.333/12.350 | `Final_Lap` slides out; `polySurface7` (chequered) and `polySurface17` slide in, x +42.7 -> -42.7 |

`Text` and `Arrow` author `LoopEnd` = 9.333 s; every other node takes the
6,000 s default, so they are one-shots. `polySurfaceShape7` has a
texture-transform of its own - loop 13.333 s, keys 741..800, `v` 0 -> -2.0 -
which is the chequered scroll.

This is why `14_Track`'s manifest carries two `#`-prefixed `<Billboard num="8">`
lines after its root close naming `Checkered_StartFinish.vex` and
`Final_Lap_StartFinish.vex`: neither file is on the disc, and the content they
would have held is in the timeline above. They are the fossil of a three-file
design that got folded into one, and reading them made `14_Track` look like the
one Pulse circuit whose `num` is not unique. It is not - see
`oag_formats::trackstartup`.

## Which model a mode selects on Pulse: none. Confidence 85

**Pulse ships one gantry model and has no per-mode selection to find.** A
`strings` sweep of all three WADs (`Data.wad`, `BEData.wad`, `FEData.wad`) and
of `PSP_GAME/SYSDIR/BOOT.BIN` turns up `321_Go`, `321go_NOMIP.tga`, the Maya
authoring path `321Go_StartFinish.mb`, and the HUD's separate `Cockpit321Go` -
and nothing else. There is no `321Go_Zone`, no mode variant of any spelling.
`BOOT.BIN` names no gantry model at all: the model reaches the engine only
through slot 8 of a circuit's manifest.

Zone on Pulse runs `26_Track`, `25_Track`'s Zone variant, which **authors no
`TrackStartup.xml` of its own** - so it has no slot 8, and no gantry from this
path at all.

**Do not inherit HD's shape.** HD/Fury ships four distinct `321Go_*.vex`
(`StartFinish`, `Zone`, `HD_Zone_Battle`, `hd_detonator`) with four different
byte sizes and a runtime mode-descriptor pointer choosing between them
([`ps3-hdfury-eu/billboards.md`](../ghidra/functions/ps3-hdfury-eu/billboards.md));
2048 ships those four plus two of its own. Pulse has one file and no chooser.
The per-mode axis is real on the later titles and absent on this one.

## What a later title should expect to reuse

The **mechanism** is the part that should generalise: a palette texture, one UV
cell per state, and the material's own `TEXOFFSET` track walking between them.
Both halves are already implemented and title-agnostic -
`vex::mesh_tex_transforms` parses the block big-endian for HD as readily as
little-endian for Pulse, and the renderer replays it per frame.

The **packaging** is what looks Pulse-specific: one model holding the whole
race's gantry states on one timeline, where HD splits them across four files.
Pulse's own commented-out lines show it used to split them too, so the
difference is a late authoring decision rather than an engine one - a reader on
Pure or 2048 should check the packaging before assuming either shape.
