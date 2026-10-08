# The start gantry: how a `3`, `2`, `1`, `GO` is drawn

The object standing over the start line, not the HUD graphic drawn on the
screen. Two different things have both been called "the countdown display" in
this tree; [`ui/hud.md`](../ui/hud.md) covers the on-screen `<Mode3D>` overlay
(`Pulse_Ready_Go` / `Cockpit_321GO`, identical across every mode on every title
checked), and this page covers the physical gantry, which arrives through
`TrackStartup.xml`'s **billboard slot 8** ([`formats/README.md`](../formats/README.md)'s
Track startup row).

> **2026-10-06, billboards lane: the original draws this model to a texture.**
> A live PPSSPP GE dump of Talon's Junction shows the gantry (pass B: 8 prims, the
> model's own camera at aspect 4.0 and 61.93 degrees, a 128 x 128 target) and the
> circuit's slot-7 advert (pass A) rendered offscreen and **sampled on the quads the
> track textures `billboard8.tga` and `billboard7.tga`**
> ([billboards.md](../ghidra/functions/psp-pulse-usa/billboards.md), 2026-10-06). So
> the mount this page measures is the quad the original samples the gantry on, and
> `oag_raceplay::gantry` still stands the model on it directly - a placement measured
> against the original and kept; the render-to-texture path the other six slots now use
> (`oag_raceplay::adverts`) was not applied to slot 8. Checked, applies, not wired.

**Read on Wipeout Pulse (`pulse-psp-usa.chd`, `UCUS-98712`; and
`pulse-ps2-eu.chd`, `SCES-54748`), Wipeout Pure (`pure-psp-usa.chd`,
`UCUS-98612`), Wipeout HD/Fury (`hdfury-ps3-eu-dec.iso`) and Wipeout 2048
(`data/extracted/vita/PCSF00007`, the decrypted EU Vita package).** The
closing section says what looks title-wide across all four and what is
per-title. Pure ships no track-side gantry at all; PS2 Pulse ships the
identical asset PSP does, under an external skin (below); HD ships four
files, and its own section is where the packaging question the Pulse pass
left open gets an answer; 2048 ships those same four plus four of its own
(three real glyph files and one empty stub), and turns out to author a third
countdown mechanism entirely - a manifest reaches only two of the eight.

> **The gantry is placed, and it is drawn - on every platform that ships a
> slot 8 now.** This page used to open by saying nothing on it placed the
> gantry. That changed in two steps on 2026-09-06: HD/Fury's equivalent code
> path was read end to end and provably reads no position either - it
> reaches the world only by binding to geometry the **track model** authors,
> keyed by the name `billboard<num>` - and Pulse's own track files turned out
> to author the identical surface, at coordinates this project's parser
> reads straight off the disc. `oag_render::gantry` measures that surface
> per circuit and `oag_raceplay::gantry` stands `321Go_StartFinish.vex` on
> it, so a race now plays the `3`, `2`, `1`, `GO` on the object over the
> start line rather than as a screen overlay - on Pulse, and, since
> 2026-09-13, on HD/Fury too, once the same texture match was widened to
> HD's own full-path label shape and the PS3 model's own sibling-`.rcsmodel`
> load path was wired in. HD's own board plays the same authored 6.000 s
> teleport Pulse's does, confirmed through this project's pipeline, and,
> since 2026-09-17, its own glyph *reveal* plays too: one Edge Animation
> curve, on the digit board's own material, walking across a shared texture
> the same way Pulse's own shared offset does - wired generically
> (`oag_mesh::mesh::rcs::curve_track`) and confirmed by direct capture to
> read `3`, `2`, `1`, `GO` in order. **PS2 Pulse joined them on 2026-09-25**:
> reported from play as entirely absent, it was in fact placed and animated
> correctly and only the palette texture was missing - the PS2 disc's
> `Texture` nodes carry no pixels by design, the same directory-position gap
> already fixed for the hull, the plume and the shield models - and skinning
> it the same way makes it read pixel-for-pixel identical to PSP's own
> countdown at every phase checked. See [the placement section
> below](#where-the-placement-actually-comes-from-hdfury-answers-it-for-pulse),
> [what it took to draw it](#what-it-took-to-actually-draw-it),
> [the HD implementation](#implemented-on-hd-the-same-mechanism-on-a-mount-that-is-not-flat),
> [the PS2 implementation](#implemented-on-ps2-the-identical-asset-under-an-external-skin),
> and
> [`ghidra/functions/ps3-hdfury-eu/billboards.md`](../ghidra/functions/ps3-hdfury-eu/billboards.md).

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
[`formats/vex.md`](../formats/vex.md) documents and `oag_vex::vex::mesh_tex_transforms`
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

**Which of these numbers are measured, and which are derived.** The window
*boundaries* in the table above are computed from the texel grid and the key
times - nothing samples them. What the ground-truth test pins is the
*midpoints*: it evaluates the real track against the real palette at 1.10 s,
1.85 s and 2.60 s and asserts exactly one digit is lit each time, in order. So
a decoder change that shifted every window by a fraction of a second would keep
the tests green and leave this table wrong. Re-derive the boundaries rather
than trusting them if the offset evaluator ever moves.

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

**Measured 2026-09-29 (confidence 85): the timeline starts at race tick 92, and
its `GO` lands on the thrust release.** The numbers:

- `GO` first lights **3.03-3.6 s** into the asset's own clock, and the `u` step
  that hands the board from the digits to `GO` is at frame 181.
- The measured thrust gate is **272 ticks = 4.533 s**
  (`oag_race::COUNTDOWN_TICKS` - see
  [`gameplay/race-modes.md`](../gameplay/race-modes.md#the-countdown-is-measured)).
- The asset's countdown panel **exits at frame 360/361 = 6.000 s**, 2.97 s after
  `GO`, precisely as the 6.000 s texture loop closes. **That is what the file
  authors, not what the original was seen to do**: it never happens on a
  stationary craft in 21 s (next section).

The capture is in
[`gameplay/race-modes.md`](../gameplay/race-modes.md#the-race-clock-starts-at-the-release):
one screenshot per tick through a Time Trial's countdown in PPSSPP, the ticks
numbered from the thrust release (tick 272), on four runs. The board is a blank
dark-red panel until tick 132 (`3` reads white), tick 178 (`2`) and tick 222
(`1`) - 46 and 44 ticks apart against the asset's 45 - and turns **green with
`GO` on tick 273**, one tick after the release. Two things pin frame 0:

- **The green step, tick 92.** The `u` step at frame 181 is the one sharp edge
  and lands on tick 273, so frame 0 is tick 92 = `272 - 180`. Confidence 85.
- **The digit windows, ticks ~85-96.** Their midpoints (152, 197, 242) against
  this page's lit windows (0.93-1.31 s, 1.68-2.06 s, 2.43-2.81 s) give frame 0 at
  about 85, and against the texel rows (`3` rows 28-29, `2` rows 26-27, `1` rows
  24-25, the `v` offset walking 8 texels in 180 frames) at about 96. These are
  soft edges (a threshold on white pixels) and the two window models disagree by
  11 ticks, so they bracket the green step rather than confirm it to a tick.

The clock uses 92, and the digits may sit up to ~7 ticks early or late against
the original; only `GO` on the release is pinned. **The 92 is Pulse only**; the
rule is not. HD's and 2048's gantry files are different timelines (2048's `GO`
slides in at frame 200), so their start tick is not 92 - see
[the inherited rule](#every-other-title-pulses-rule-on-its-own-go-edge).

`crates/raceplay/src/gantry.rs::CLOCK_START_TICK` is that measurement, and
Pulse's gantry clock is `(tick - 92) / 60`.

### Every other title: Pulse's rule on its own `GO` edge

**HD's start tick is bounded by its capture (2026-10-04, confidence 75; see
[HD's countdown on RPCS3](#hds-countdown-on-rpcs3-measured-2026-10-04)), and
what it does after the release is read from its EBOOT (the "After the release"
bullet below). Everything else here is
inherited from Pulse, unmeasured (2026-10-03, maintainer's rule: a title
with no measured rule runs the known one). No confidence score.** The rule is
that the board's `GO` edge lands on the tick after the thrust gate (273) and
the clock is held on `GO` from there. `race::gantry::clock::go_edge` reads the
edge off the title's own asset: **the first frame at which a drawn vertex
samples the authored green texel** of the board's texture, walking the model's
own texture tracks (`TexAnims`) at 60 frames a second, and
`Clock::inherited` sets `start_tick = 273 - go_frame`.

- **Check against the one measured title.** Run on Pulse's own
  `321Go_StartFinish.vex`, the rule finds **frame 181** - the `u` step the
  capture was pinned on - so it gives `273 - 181 = 92` with no Pulse constant in
  it (`start_gantry_clock_inherit_ground_truth.rs`). The same walk finds the
  first frame a drawn node leaves its place after the edge at **350** (the
  `Board` teleport), one frame past the chosen loop end of 349.
- **HD**: the edge is **frame 203**. `321_go_64.gtf`'s backdrop column steps
  from its red texels to the green marker column (cols 57-59) as the Edge
  Animation curve's `u` ramps 0.861 -> 0.901 over frames 198-204, so
  frame 0 is **tick 70**. The digits then read `3` about tick 110, `2` about 160,
  `1` about 210, against Pulse's captured 132, 178 and 222: the asset's own
  spacing, not a fitted one. `GO`'s letters reach full opacity a few frames
  after the green (alpha 72 at frames 207-210, 255 from 211), so on HD the
  board would be green on the release tick and `GO` fully lit about 8 ticks
  later. **On HD that ramp is never seen**: the release jump below lands past
  it, which is what the 2026-10-04 capture shows (`GO` bright on the step
  frame). Since the jump turns the board green on the release for any start
  tick that keeps the free run short of frame 203 there, the capture bounds 70
  from below rather than measuring it; the digits reading ~13 ticks late in the
  capture point later.
- **After the release, HD's own law, read from its EBOOT (2026-10-04,
  confidence 85; [gantry-clock.md](../ghidra/functions/ps3-hdfury-eu/gantry-clock.md)).**
  The billboard's curve time is the gantry node's animation time, and HD's race
  manager (`RaceManager_Update`, `0x0005e948`) keeps it inside a window picked
  by the player's lap, calling `SetTime(from)` whenever it reads outside. Before
  the first line crossing the window is **`[3.83, 5.25)` s**. On the release the
  free-running time (frame ~202, red board) is outside it, so the clock **jumps
  to frame 229.8, where `GO` is already lit and the digits long gone, and loops
  every 86 ticks**: `Clock::hd`, `HD_PRE_LAP_WINDOW`. This replaces the chosen
  221..359 hold. The later windows, by the lap counter `ship+0x7810` (1 on the
  first lap): `[6.017, 9.3)` otherwise, `[9.5, 9.9)` on `lap == total - 1`,
  `[12.35, 13.3)` on `lap == total` (where the `FINAL_LAP` announcer cue also
  fires). Which board state each shows on HD's own timeline is not read.
  **Played since 2026-10-04**: the original, forced into each window on
  RPCS3, shows the `FX-350` art between laps, a strobing `FINAL LAP` on the
  lap before the last and a chequered flag on the last; ours plays the same
  frames and culls per frame to the panel (`race::gantry::BoardWindow`,
  `PanelCull`; [gantry-clock.md](../ghidra/functions/ps3-hdfury-eu/gantry-clock.md#the-lap-windows-played-2026-10-04-hd-gantry-laps-lane)).
- **2048 and Omega place no gantry**, so there is no clock to set. Loaded
  through `race::load`: all ten native 2048 circuits, **thirteen of the sixteen
  HD-ported 2048 circuits** (the base package's four - `Anulpha_Pass`,
  `Chenghou_Project`, `Moa_Therma`, `Vineta_K` - and nine of the DLC's, under
  `Data\art\published\DLC1\environments\`), and on Omega the default circuit,
  Tech De Ra (HD's circuit) and Altima (2048's) all report `no start gantry:
  this circuit's track authors no 321backplate/billboard8 surface`
  (`start_gantry_clock_inherit_ground_truth.rs`). The three HD-ported circuits
  not loaded are `zone_2`-`zone_4`. Whether 2048's psp2 copy of HD's
  `321Go_StartFinish.vex` carries the Edge curve `go_edge` walks was not tested,
  since no circuit stands one; 2048's native `321Go_2048.vex` (`GO` node sliding
  in at frame 200) has no texture track at all, so `go_edge` would report none
  and the clock would run from the race start, chosen.
- **A title whose edge is not found** keeps the timeline running from the race
  start, chosen, and the loader says why (`start gantry clock: no GO edge
  found in ...`).

Played on HD (`oag-game --race --track /data/environments/talons_junction/track.vex
--ticks N --screenshot`, `hdfury-ps3-eu-dec.iso`), since the release window (2026-10-04): the `3 2 1`
strip at tick 271, a fully lit `GO` on tick 273 with no digit left, dark at
+18..+30, +59..+67 and +104..+116 ticks, and the same 86-tick loop on. The side
by side against the capture is in the gantry-clock evidence page.

### HD's countdown on RPCS3, measured 2026-10-04

**Question: at which race tick does HD's board turn green and `GO` light,
against the frame the craft can first move?** Answer: **on the release, to
within one 30 fps video frame (2 ticks). Confidence 75.** The inherited start
tick 70 (frame 203 on tick 273) stands, now measured; the held span does not
match the original, below.

Method. `scripts/rpcs3-drive.py countdown` boots HD on a private RPCS3 (own
`XDG_CONFIG_HOME`/`XDG_CACHE_HOME`, display :91, GDB port 2391, silent), walks
`Team Selection -> InGame`, starts RPCS3's own recording on `Team Selection` so
the load and the whole countdown are in the file, taps cross once to skip the
track fly-over (the race opens on a fly-over with a `START RACE` prompt) and
then holds thrust. `scripts/hd-countdown-frames.py` reads the recording
(Talon's Junction entered through the Campaign path the harness walks - `Campaign Selection -> Grid Selection Fury -> Cell Selection -> Team Selection` - not a Single Race, which was not captured; the default grid ship, 1280x720; every video frame is a new image, consecutive-frame road differences never fall below 9, so it is a true 30 fps and a frame is 2 ticks).
Three things share one clock - the video:

- **The board.** Red `3`, `2`, `1` strip on a dark teal board, then in **one
  video frame** the red is gone and a bright `GO` is on the teal board: frame 223
  on both boots (red pixels in the board crop 271 -> 16 and 237 -> 4, teal 2 ->
  580 and 11 -> 594).
- **The race clock.** The HUD lap timer reads `0.00.0` until frame 226 (boot 2) /
  227 (boot 3), `0.00.1`, then +0.1 every 3.0 frames out to `0.01.3`: the video is
  real time and the clock zero is frame 223.0 (boot 2) / 224.0 (boot 3), +-1.
- **The craft.** The road crop's difference against a held frame sits at the idle
  noise (4-5) until frame 222 (boot 2) / 223 (boot 3) and then climbs 11, 19, 26,
  31 (boot 2) / 4, 12, 20, 27 (boot 3): first movement at frame 223-224.

So the green step is at the clock's zero and the craft's first movement, 223 vs
223-224 vs 223-224. Two ticks per frame puts the step at tick 272 +-2 where the
inherited rule says 273. First `3` sliding in: frame 146 / 144, i.e. about 150
ticks before the step (inherited: `3` about tick 110, 163 before; so the digits
read up to ~13 ticks later than the asset-walk estimate on this capture).

Where it **does not** match the inherited build:

- **`GO` arrives at once.** The original's `GO` is bright on the step frame
  (223) and the `1` is gone with the red. Ours at tick 273 is the teal board
  with a ghost of the `2 1` digits and no `GO`; the letters reach half at 279 and
  full at ~286, the 8 ticks of alpha ramp the asset authors (72 at frames
  207-210, 255 from 211). One 30 fps frame cannot show a ramp, so the original
  either skips it or runs it inside 2 ticks; **not resolved**.
- **Both are resolved by HD's race-manager window** (2026-10-04, same day,
  [gantry-clock.md](../ghidra/functions/ps3-hdfury-eu/gantry-clock.md)): the
  clock jumps to 3.83 s on the release and loops `[3.83, 5.25)`, 86 ticks. Its
  dark centres off the asset are +24, +63 and +110 against the capture's +23,
  +61 and +109 (`start_gantry_clock_inherit_ground_truth.rs`). The two bullets
  below are the mismatch as captured, kept as the evidence.
- **After `GO` the original pulses, and not in our phase.** `GO` goes bright ->
  empty -> bright repeatedly (so "held" was wrong in that it is not steady;
  ours pulses too, from the asset's own glyph walk). Dark centres, in ticks
  from the step: original +23, +61, +109 (video frames 235, 254, 278; both
  boots agree to a frame); ours +50, +89, +129 (ticks 323, 362, 402). As asset
  frames under start tick 70 the original's dips fall at about 226, 264 and 312
  against ours at 253, 292 and 332; the spacing (38 then 48 ticks) is not yet a
  loop period. The
  original's first bright stretch is ~14 ticks (frames 223-230), ours ~26.
  Shifting the start tick cannot fix both: moving it 25 ticks earlier would
  put the green step 25 ticks before the release, which is the one thing that
  was measured sharply. Unresolved: the pulse is probably driven by a curve
  with its own phase, or the hold loop the original runs starts elsewhere than
  frame 221.

Reproduce, from the repo root:

```sh
S=data/scratch/<lane>; export XDG_CONFIG_HOME=$S/xdg/config XDG_CACHE_HOME=$S/xdg/cache
export OAG_RPCS3_DISPLAY=91 OAG_RPCS3_GDB=127.0.0.1:2391
python3 scripts/rpcs3-drive.py display
uv run --with evdev python3 scripts/rpcs3-drive.py --log-dir $S/boot --image data/images/hdfury-ps3-eu-dec.iso countdown
python3 scripts/hd-countdown-frames.py <recording.mp4> --out $S/frames
```

(Copy `~/.config/rpcs3` into `$S/xdg/config` first, minus `savestates`.) The
side-by-side: original frames `f` against ours at tick `273 + 2 (f - 223)`, from
`oag-game --race --track /data/environments/talons_junction/track.vex --ticks N
--screenshot`. Captures: `data/scratch/hd-countdown-capture/` (`boot2.mp4`,
`boot3.mp4`, `compare_strip_a_digits_to_go.png`,
`compare_strip_b_go_pulse.png`, `b2_timer_sheet.png`, `b2_go_perframe.png`,
`b3_go_perframe.png`, `ours_go_perframe.png`). Limits: 2 ticks per frame, the
video is of a ~30 fps presentation, the lap timer is read by eye, and the
"release" is taken as the clock's zero and first road motion (no thrust-gate
read; a `Z2` watchpoint on the gate would give the tick exactly).

### What the original does after `GO`: it holds `GO`

**Measured 2026-09-30.** Reported from play: past `GO`, a full `3 2 1 GO` strip
stayed visible above the gantry, beside the banner. Reproduced first: on
`16_Track`, `--race --ticks 460/700`, `start_light_background` (node 5) and
`start_light_321go` (node 19) sit 9.99 units above the aperture (`--draws`
lists both, `oag-view --anim-seconds 7.0` shows them stacked over `Board`),
and the 6.000 s texture loop replays `3`, `2`, `1`, `GO` on them. The mount
measures 45.5 x 10.4, so +9.99 is one aperture height up: the file authors
"out of the aperture" and relies on the track's own structure to hide it, which
this project's open-air draw does not have. Candidates (a) and (b) of the lane
brief are ruled out (the transform track is played correctly, the UVs are the
authored four cells); (c) is out too, the nodes are the countdown itself.

Then the original, PPSSPP, `pulse-psp-usa.chd`, Talon's Junction, Time Trial,
fresh profile, **craft not driven** (no thrust, so the gantry stays in view),
one screenshot per ~0.4 s wall for 46 s, which is about 21 s of HUD race clock
at the emulator's roughly half speed. Two runs agree:

- The board goes green and reads `GO` on the release, and **keeps showing
  `GO`, strobing between lit, dim and dark, through the whole capture.**
- It never replays `3 2 1`, the panel never moves up out of the frame, nothing
  sits above the banner, and the `Board`/`Text`/`Arrow` dressing (which the
  file brings in at 5.85 s) never appears. The authored teleport at frame 360
  is therefore not played in the original, at least on a stationary craft.

**What holds it, read from `BOOT.BIN` (2026-10-04, confidence 85;
[gantry-clock.md](../ghidra/functions/psp-pulse-usa/gantry-clock.md)).** The
same law HD's race manager runs, with Pulse's own numbers. The intro sets the
gantry mesh's time to 0 beside `ready`, the release sets it to 3.0, and
`RaceManager_Update` (`0x08829778`) then keeps it in a window picked by the
player's crossing count, resetting it to the window's start whenever it reads
outside: `[3.2, 5.5)` before the first crossing, `[6.0, 9.0)` between laps,
`[9.5, 12.0)` on the lap before the last, `[12.4, 13.3)` on the last. A craft
that never crosses loops frames 192..330 every 138 ticks: `GO` green one tick
after the release, strobing, and never the `Board` teleport at 350. That is
both measurements above, and the 2026-09-30 contact sheet happens to contain
the loop's seam: one frame at 2.3 s of race clock (exactly one period) shows
the dim `3 2 1` ghost that frame 192 carries.

**Ours since 2026-10-04**: `race::gantry::Clock::PULSE` carries
`PULSE_PRE_LAP_WINDOW` and `PULSE_LAP_WINDOWS`, the HD lane's per-frame
`PanelCull` plays the `Board`, `FINAL LAP` and chequered states by lap, and
the chosen 216..349 `GO_LOOP` is gone. Played (autopilot, `16_Track`): `GO`
strobing on a stationary craft at tick 460; the teal `Board` with its arrow
approaching the end of lap 1; `FINAL LAP` approaching the end of lap 2; the
chequered flag on the crossing into lap 3. Only the first is compared with
the original; the lap states were not captured on PPSSPP. **PS2 Pulse runs
the same clock** (every non-PS3 source takes `Clock::PULSE`), inherited from
the PSP read and unread on its own executable; its between-laps `Board` was
seen once at player size, matching the PSP's.
`crates/game/tests/start_gantry_go_hold_ground_truth.rs` pins that the panel
never leaves the aperture on Pulse's clock and does on the raw one.

**The hexagon is answered, and it was not the gantry.** The green chevron
hexagon over the banner's left end was this project's Time Trial `TurboIcon`,
granted on the release. The original grants the free Turbo on the line
crossing ([pickups.md](../gameplay/pickups.md)), so a stationary craft holds
none; fixed the same day.

### What this retires: the two do not share a zero, and ours drew `GO` 1.5 s early

This page used to argue at confidence 82 that the asset's clock and the thrust
gate **share a zero to within three frames**, from `g_ingame->0x40` being zeroed
once per race in `InGame_Construct` and read by every in-race animation path, and
concluded that the original lights `GO` about 0.93 s before the craft can move
and holds it for 1.4 s after. **Both halves are wrong for the gantry.**

- The board is not driven by `g_ingame->0x40`, or not by it alone: a restart does
  not reset that field (it read 41.9 s and 77.0 s at the first tick of two
  captures, on a race started from the pause menu), yet the board still starts
  from blank on every run and plays 3, 2, 1, `GO` in the same ticks.
- The original's `GO` is drawn **on** the release, not 0.93 s before it. The
  strobing `GO` column the asset authors is what the board shows from there on.

The consequence the maintainer reported from play was real and was ours: the
gantry ran off `world.tick / 60` from tick 0, so `GO` lit at tick ~182 and the
board was green for the last ~90 ticks of the countdown, before the craft could
move. It is fixed by starting the gantry's own clock at tick 92; nothing else on
the track is shifted (the scenery's animation is still `world.tick / 60`).

What is *not* settled is what starts the timeline at tick 92 in the original: a
gated accumulator (`0x0890cf34`, above) or a per-state write are both still
candidates, and the fit only shows that the *result* is a 1:1 replay of the
authored track that starts 180 frames before the release.

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
`oag_tables::trackstartup`.

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
byte sizes, and a code-confirmed substitution site exists at runtime - see
[the HD section below](#wipeout-hdfury-four-files-one-mechanism-concept-two-encodings)
and
[`ps3-hdfury-eu/billboards.md`](../ghidra/functions/ps3-hdfury-eu/billboards.md)
for exactly what it does and does not settle; 2048 ships those four plus four
of its own (three real glyph files and one empty stub) - see
[the 2048 section below](#wipeout-2048-eight-files-plus-fx350-two-reached-by-any-of-26-manifests-a-third-countdown-mechanism).
Pulse has one file and no chooser. The per-mode axis is real on the later
titles and absent on this one.

## Wipeout Pure: no track-side gantry at all. Confidence 85

**Read on `pure-psp-usa.chd` (`UCUS-98612`).** Pure's `TrackStartup.xml` was
never read for content before this pass - `docs/formats/pure-status.md`
reached the plugin id and stopped - and the race-start handover thread
recorded `psp-pure-usa` as unreachable through `ghidra-mcp` in an earlier
session. Neither mattered: everything below came off the disc with
`oag-wad cat` and `oag-view`, the same two tools the Pulse pass used, no
Ghidra and no emulator.

**Every one of Pure's 16 circuits' own `<TrackStartup>` was read, exhaustively,
and none of them authors a slot 8.** `Data\Plugins\PI001\Definition.xml` names
eight race circuits, four Classics and four Zone circuits; each race circuit's
`TrackStartup.xml` carries `<Billboard>` entries and each Classics/Zone one
carries none at all. Across the eight that do, slot numbers run 1 to 7 -
`12_Sol_2`'s manifest is the one that reaches 7 - and never 8, model or
colour. This is the exact schema Pulse's own `14_Track` debris pointed at, and
`crates/tables/tests/start_gantry_pure_ground_truth.rs` reproduces the count:
42 billboards over 16 circuits, 0 naming a `location`.

**Pure's `<Billboard>` schema is not Pulse's schema, either.** Every one of
those 42 slots carries `type` and `color` and no `location` attribute at all -
`<Billboard num="1" type="landscape" color="red"/>`. Pulse's own
`trackstartup.rs` module already knows this shape exists (`Fill::Colour`,
14 of Pulse's own 118 slots) as a live but unresolved lead of its own -
`Data\Plugins\PI004\Definition.xml`'s colour-tagged sponsor-logo catalogue -
and that catalogue is confirmed, on Pure too, to be about advertising
hoardings (`Data\Billboards\3DVexWindow\*\logo.vex`) and nothing gantry-shaped.
So even under the most generous reading - that a colour slot resolves through
a pool the way Pulse's might - **there is still no slot 8 for it to resolve
from** on any Pure circuit.

**Nothing else on the disc names a gantry either.** A guessed-path sweep
(`Data\Environments\321_Go\321Go_StartFinish.vex` and half a dozen spelling
variants) and a `strings` pass over `PSP_GAME/SYSDIR/BOOT.BIN` both come back
empty - no `321`, no `StartFinish`, no `Countdown`, no `Gantry`. The one hit
either search turns up is `ReadyGo`, which is the on-screen HUD widget
`ui/hud.md` already names (`Data\HUD\Ready_GO.vex`), not a track-side object.

**The static "starting line" architecture baked into each circuit's own
`track.vex` is not it either.** `01_Vineta_K\track.vex` authors over thirty
`starting_line_Shape*` mesh nodes as part of the environment, some of them
carrying their own `TEXOFFSET` tracks - but every one measured is a short,
2-key linear scroll (0.8-8 s loops, `u` or `v` sliding once with no
intermediate keys), the shape of a chasing light strip or a scrolling banner
accent, not a multi-state countdown. None comes close to the gantry's own
5-key, ~6-second track. So the physical start-line structure is there, static
lighting flourishes and all, but nothing on it walks through `3`, `2`, `1`,
`GO`.

**Conclusion: Wipeout Pure ships no track-side start-line gantry object, on
any circuit, reachable by any lead tried.** This is a negative claim over an
open-ended search rather than a confirmed positive, which is why it sits at
85 rather than the 90 the Pulse mechanism carries - but the one part of it
that *is* exhaustive (every circuit's own manifest schema) is tested, not just
observed, in `start_gantry_pure_ground_truth.rs`.

**The nearest analogous asset is the on-screen HUD widget, and it uses a
different mechanism from Pulse's track-side one.** `ui/hud.md` already names
`Data\HUD\Ready_GO.vex` as Pure's shared `Arcade_HUD.xml`/`Zone_HUD.xml`
`<Mode3D>` overlay, with no separate cockpit variant; measuring it here for
the first time, since it is the only `3`-`2`-`1`-`GO`-adjacent asset Pure
ships:

- **3 static mesh nodes** (`loftedSurfaceShape1/2/3`, 54 triangles each), not
  Pulse's one node carrying every glyph.
- **One embedded 32x64 8bpp texture**, 256 palette entries, **every one of
  them pure white** (`255, 255, 255`) with only the alpha channel varying -
  not Pulse's multi-colour staircase.
- Each mesh node carries **its own** `TEXOFFSET` track (per-node, not
  per-vertex-cell), and all three are **identical**: 2 keys, `v` sliding
  0 -> 251/256 over a 0.983 s loop, interpolated rather than stepped. Nothing
  like Pulse's 5-key, ~6 s, multi-phase track.
- Per-vertex UVs are **continuous strip coordinates** - `u` pinned to 0.0 or
  1.0 (a tube's two edges) with `v` running the full 0..1 range and beyond -
  not Pulse's four discrete per-glyph cells. Rendered, the three nodes are
  nested ring/loop shapes, not digit outlines.

**What this does and does not settle.** It settles that the UV-cell-against-a-
palette mechanism does **not** generalise to whatever Pure's on-screen widget
is doing - a scrolling alpha gradient along ribbon geometry is a different
technique entirely. It does **not** settle what actually draws Pure's `3`,
`2`, `1` and `GO` glyphs, since none of the three ring shapes reads as a digit
outline; that is unresolved and left for whoever picks this up next, along
with everything else this page already keeps separate: which of the two is
even the physical countdown Pure's players see is itself unmeasured, since
`Ready_GO.vex` is `ui/hud.md`'s subject, not this page's, and the two must not
be conflated the way this project's own notes once did for one session before
catching it (see `ui/hud.md`'s own warning).

**Pulse's own on-screen `ReadyGo` widget, measured 2026-09-06, is the same
shape family, not a digit display either.** `Data\HUD\Pulse_Ready_Go.vex`'s
one mesh (`thingieShape`, 285 vertices) renders, standalone off the disc, as
a nested ring/swoosh - the same "loop, not glyph" reading as Pure's three
nodes above - and its own material `TEXOFFSET` track is a 2-key, 0.983 s
loop, the identical period to Pure's. Pulse still ships the literal digits
too, on the *other* `<Mode3D>` model in the same widget pair
(`Data\HUD\Cockpit_321GO.vex`, four glyph nodes, see `ui/hud.md`) - so on
Pulse the two mechanisms sit side by side in one HUD block rather than one
title using one and another using the other. What blocks drawing the ring is
not its shape but its envelope: see
[`countdown-widgets.md`](../ghidra/functions/psp-pulse-usa/countdown-widgets.md#readygos-gate-not-a-digit-sync-problem-but-a-genuinely-unrecovered-one).

## Wipeout HD/Fury: four files, one mechanism concept, two encodings

**Read on `hdfury-ps3-eu-dec.iso`.** HD is the interesting case the Pulse and
Pure passes both flagged: it ships **four** distinct `321Go_*.vex` files under
`/data/billboards/hd_adverts/321go/` rather than Pulse's one, confirmed as
genuinely different content by rendering them in the prior scoping session -
see [`ps3-hdfury-eu/billboards.md`](../ghidra/functions/ps3-hdfury-eu/billboards.md).
This pass measured `321go_startfinish.vex` (the one slot 8 actually names on
every circuit) the same way the Pulse and Pure passes measured their own
assets: `oag-view --nodes`/`--draws`, then the geometry and material tables
directly through `oag_formats`, no Ghidra and no emulator.

**The four files are not all in one archive, and not even self-consistent
under one name.** `python3 scripts/psarc.py list` over the whole disc finds
`321go_startfinish.vex` (12,544 B) and `fx350.vex` only in `DATA02.PSARC`;
`321go_hd_detonator.vex` and `321go_hd_zone_battle.vex` only in `DATA00.PSARC`;
and `321go_zone.vex` in **both**, at two different sizes - 8,368 B in
`DATA00.PSARC`, 2,672 B in `DATA02.PSARC`, each with its own distinct texture
(`321_go_zone.gtf`, 2048x1024, beside the `DATA00` copy;
`321_go_64_zone2.gtf`, 64x128, beside the `DATA02` copy). **Measured, not
traced**: which archive set a given mode actually loads from is unread, so
this is recorded as a fact about the disc's own packaging rather than
resolved to a loader. Confidence 90 for the listing itself, and explicitly
open which copy (if either alone) a race reaches.

### The geometry moved out of the `.vex`; the mechanism has to be looked for beside it

Every HD `Mesh` node's own payload is a bounding box and a hash, nothing else
- `oag_rcs::rcsmodel` is where the real vertices, indices, texture
coordinates and material table for a PS3 model live (see that module's own
doc comment, and `docs/rendering/scenery-animation.md`). `321go_startfinish.vex`
is exactly that shape: 43 nodes, 19 of them `Mesh`, and reading any of their
288-byte-or-smaller payloads finds a bounding box, a hash, and zeroes - no
embedded palette, no `TEXOFFSET` block, because there is nowhere in this
payload shape for either to live. **This is the first fact that has to be
priced in before comparing HD to Pulse at all**: Pulse's whole mechanism lived
inside the `.vex`; HD's geometry-bearing half of the same story lives in
`321go_startfinish.rcsmodel` (51,796 B), beside it in the same `DATA02.PSARC`
entry list.

The glyph node itself is named `pasted__Go_HD_start_light_321go` - an
`Anim Transform` node carrying one `Mesh` child, the same two-node shape
Pulse's own `start_light_321go` takes. Its payload's hash (`0x81f409f5`)
resolves to one chunk of the `.rcsmodel`, material index 2:
`data/materials/billboards/simpletextureandtexturealphauvoffsetscale.rcsmaterial`,
texture `data/billboards/hd_adverts/321go/321_go_64.gtf`.

### The texture is Pulse's staircase, scaled up and DXT-compressed - confidence 88

`321_go_64.gtf` decodes (`oag_texture::gtf`, format `Dxt45`) to **64x128** -
exactly 4x Pulse's 16x32 in both dimensions - and a full-texel opaque-white
scan (every texel with alpha > 200 and RGB > 200) finds:

```text
rows  7..14  cols  0..7    opaque white  (band 1)
rows 15..22  cols  8..15   opaque white  (band 2)
rows 23..30  cols 16..23   opaque white  (band 3)
cols 32..39, rows 40..127  alternating opaque/dark 8-row bands
cols 48..54                red   (255,0,0), alpha ~128 (half), rows 32..127 only
cols 57..59                green (0,169,25), alpha ~128 (half), full height
```

That is Pulse's own staircase shape, at 4x scale and moved onto a diagonal:
Pulse held `u` fixed per digit and walked a shared `v` down three row-pairs at
one `u`; HD's three white bands step **both** row and column together, three
8x8-texel blocks descending a diagonal. The red and green columns at the far
right - half-alpha rather than Pulse's fully opaque markers, but otherwise the
same idea - are the same "authored, unused by this node" markers Pulse's own
`321go_NOMIP.tga` carries at columns 12 and 14 - present at both resolutions,
on both platforms, doing nothing either time.

**The glyph's own five per-vertex UV cells align to these bands once the
material's own static `u` offset is added**, which is the strongest evidence
this is the same authored concept: reading `pasted__Go_HD_start_light_321goShape`'s
`.rcsmodel` chunk (stride 14, 200 vertices) finds five distinct `(u, v)`
cells, not Pulse's four:

| raw `u` | raw `v` | `x` span | `+0.04` `u` in texels | falls in band |
| --- | --- | --- | --- | --- |
| 0.0547 | 0.0128 | -16.60..-4.95 | 6.06 | 1 (cols 0-7) |
| 0.1831 | 0.0128 | -5.05..5.35 | 14.26 | 2 (cols 8-15) |
| 0.1920 | 0.0128 | -4.10..6.30 | 14.86 | 2 (cols 8-15) |
| 0.3079 | 0.0128 | 6.12..16.59 | 22.26 | 3 (cols 16-23) |
| 0.4968 | 0.0451 | -11.94..11.30 | 34.4 | the alternating column |

`0.04` is exactly the material's own authored `uvOffset.x` (below). Adding it
to each cell's raw `u` lands the leftmost cell (`x` -16.60..-4.95, reading-order
"3") in band 1's own column range, the two near-identical middle cells (`x`
-5.05..6.30, "2"'s position) in band 2's, and the rightmost cell (`x`
6.12..16.59, "1"'s position) in band 3's - the same reading-order-by-`x`
outcome Pulse's own four cells give, and not a coincidence four independent
column ranges land where four independent digit positions do. The fifth cell
is wide (`x` -11.94..11.30, spanning the whole board like Pulse's `GO`) and,
after the same offset, sits in the alternating column rather than any of the
three staircase bands.

**Two things are reported rather than storied, because the evidence does not
resolve them and a tidy label would look measured without being it** - the
same trap a first pass at Pulse's own test fell into sorting by `x` instead of
`u`:

- **Five cells for four states.** The two near-`u` cells at `x`
  -5.05..5.35 and -4.10..6.30 are not identical - a real, if small,
  `x`-shifted twin - and nothing here explains the second one. It is left
  unexplained rather than folded into "two overlapping quads for the 2" on a
  guess.
- **The alternating column is not confirmed to be a "GO" flash.** It reads as
  the same *shape* of thing Pulse's `GO` column does (alternating opaque and
  transparent 8-row bands over a much taller span than any digit needs), but
  nothing here traces it to the word "GO" the way Pulse's own strobe test
  samples a known span and asserts exactly one label. Confidence 60 on "this
  is HD's `GO` region" alone, separate from the 88 above on the staircase
  correspondence.

### What is not proven: whether anything ever changes it - confidence 40, and this is where HD's answer genuinely differs from Pulse's

**The material's `uvOffset`/`uvScale` are a static value, not a keyframe
track, and this is a fact about the format, not an inference from one
file.** `oag_rcs::rcsmodel::material::parameters` decodes a material
record's parameter table as a name-hashed array of `{value: [f32; 4], quads:
u32}` entries with no time axis at all - a `quads` above one would be a longer
vector the shader indexes, not a sequence of keys, and every parameter this
material carries reports `quads == 1`. Feeding this exact node's payload
through `oag_vex::vex::mesh_tex_transforms` - the same parser that
recovers Pulse's `TEXOFFSET` track and is proven title-agnostic on HD's own
byte order (`docs/rendering/scenery-animation.md`) - returns `None` for every
material on the node: there is no texture-transform block here for it to
find, because a PS3 `Mesh` payload carries no batches at all past its
bounding box and hash. **So if HD's countdown swaps between states, no track
on disk drives it, on either the geometry or the material side - the same
conclusion Pulse's own competing "does the engine write it per state"
candidate reached for different reasons, except here it is not a competing
candidate, it is the only mechanism the file format has room for.**

**That does not mean the countdown swaps at all, and the material's own
disc-wide use argues against reading too much into one static value.**
`simpletextureandtexturealphauvoffsetscale.rcsmaterial` is not
countdown-specific: the same material, by name, backs at least nine other
advert boards surveyed across both archives - `aftermath_board`,
`blitzed_board`, `corruption_board`, `impact_board`, `nuked_board`,
`turbulance_board`, `voltage_board`, `vortex_board` and `arial_landscape_01` -
each with its own baked, unrelated `uvOffset` (`(0, 1)`, `(0, 0)`, `(0,
0.958)`, ...) and every one of them a single static advert, not a
multi-state display. Read plainly, this is a generic "sample one fixed
window of a shared texture" shader used disc-wide, and `321go_startfinish`'s
own instance - `uvOffset = (0.04, 0.4189558)` - is one more static value in
that same family. Sampled at rest, none of the five glyph cells lands on
opaque white: the four digit-shaped cells land at texel row ~55, well below
every staircase band (rows 7-30), and the wide cell lands in a gap between
two of the alternating column's opaque bands (`rgba = [255, 255, 255, 71]`,
near-white but 28% alpha) - so as authored, at rest, nothing reads as lit.

**Both readings are left standing rather than one being picked**: the
geometry's own column alignment against the texture's diagonal staircase (88
above) is real evidence an artist laid this out as a walkable countdown the
same conceptual way Pulse's palette is; the material system's disc-wide,
single-static-value usage is real evidence this specific instance is nothing
more than one crop window like every other advert's. If the display does
change at runtime, this project's own reading of the format says the write
has to land in this exact shader parameter (or an equivalent uniform this
static value seeds) once per state, since there is nowhere else in either
file for a change to come from - but nothing here observed that write happen,
and settling it needs the same live-tracing step
[`ps3-hdfury-eu/billboards.md`](../ghidra/functions/ps3-hdfury-eu/billboards.md)
already named for `mode_descriptor`. Confidence 40, and explicitly the
softest claim on this page - lower than Pulse's already-open 55, because
Pulse has an authored track to weigh against a competing write and HD does
not even have that much to weigh.

**Superseded, 2026-09-17: it does change, and the mechanism is a third thing
this section did not have a slot for.** `mesh_tex_transforms` correctly finds
no `TEXOFFSET`-shaped block, because HD's own equivalent is not that format -
it is a material's `+0x20`-then-`+0xc` pointer into Sony's own Edge Animation
Tools clip format, decoded in full by `lane/hd-edgeanim` and
`lane/hd-gantry-wire` (`docs/formats/edge-animation.md`). That resolves this
section's own "if HD's countdown swaps between states, no track on disk
drives it" the other way: a track does drive it, in a place this pass had no
name for yet. See ["What is still open"](#what-is-still-open) below for the
resolved finding and real captures.

### The 6.000-second loop close generalises - the best result of the three-title sequence

**Confidence 85.** `pasted__Go_HD_start_light_321go`'s own `Anim Transform`
carries a translation channel with exactly two keys, at frames 359 and 360 -
5.983 s and 6.000 s, `seconds_per_key` confirmed 1/60 the same way every other
HD node's is. Sampling it (`AnimTransform::sample`, `step` false) holds the
node at its rest position through 5.9 s, then moves it **+10.004 in world Y**
by the time the second key lands at exactly **6.000 s**. That is the same
shape and the same instant as Pulse's own finding: `start_light_background`
and the digit panel "teleport +9.99 in y - out of the aperture, as the
texture loop closes" at Pulse's frames 360/361 - also 6.000 s on Pulse's own
60 Hz clock. The frame indices differ by one (359/360 against 360/361) and the
magnitudes differ (+10.004 against +9.99, off different quanta), so this is
not claimed as the identical authored value - it is claimed as what it is:
**both titles move their countdown glyph roughly ten units up and out of view
at the same 6.000-second loop-closing instant**, independently authored, on
two different platforms, in two different file formats. A sibling node,
`Go_HD_start_light_background`, does the same kind of teleport (+11.149 in Y)
at a different, earlier instant (3.35 s) - the same "something moves away
before the loop resets" shape Pulse's `start_light_background`/`Board`/`Text`/
`Arrow` group also carries, just at a different node boundary.

**This finding grew a great deal once the 2048 pass checked it against HD's
own disc directly rather than stopping at this one node**: `polySurface7`
and `pasted__Final_Lap` both carry the same 9.333 s/12.333 s beats Pulse's
own file authors, confirmed by `start_gantry_hd_ground_truth.rs`'s own sixth
test - see the 2048 section below, "The master timeline is three beats, not
one", for the full three-title account.

### Which model a mode selects: a substitution site exists, at slot 7 - not slot 8

**Confidence capped at 84**, the whole page's own static-reading ceiling,
inherited from [`ps3-hdfury-eu/billboards.md`](../ghidra/functions/ps3-hdfury-eu/billboards.md).
That page traced `TrackStartup_Load` end to end and found a real,
code-confirmed substitution: when `num == 7`, and `mode_descriptor`'s own
`field_0x90` and `field_0x4c` are both non-null, the engine throws away
whatever `fx350.vex` slot 7's own manifest authored and substitutes a
computed pointer instead. **This fires on slot 7, not slot 8** -
`321Go_StartFinish.vex` (slot 8) is not special-cased anywhere in that
function and loads exactly as authored, on every circuit, in every mode.

Four strings that look like the four `321Go_*.vex` names sit in the same TOC
window as the attribute names that substitution reads, which is a real lead -
the mode-free single race/time trial, Zone, Zone Battle and Detonator each
have exactly one - but the computed pointer was never traced to one of those
four addresses, so **which of the four names slot 7 actually resolves to in
which mode remains unread**, and stays a lead rather than a finding per that
page's own account. No new Ghidra time was spent chasing it this pass: the
mechanism question above was the cheap one, per this thread's own priority
order, and answering it did not make the chooser question moot, but it does
mean **the countdown asset itself is mode-independent as authored** - if
Zone's gantry really does show a track icon instead of `3 2 1 GO`, per the
player's own memory `billboards.md` already recorded, that difference has to
come from slot 7's own substituted content standing in for or beside slot 8's,
not from slot 8 switching files. That tension is unresolved and is exactly
where the next Ghidra or live-tracing pass on this subsystem should start.

### Is the gantry placed on HD? No - a second, independent witness

**Confidence 90, agreeing with `ps3-hdfury-eu/billboards.md`'s own reading by
a completely different route.** That page found nothing in `TrackStartup_Load`
or either billboard constructor that reads a position, rotation or scale.
This pass's own geometry-side reading agrees: `pasted__Go_HD_start_light_321goShape`'s
`.rcsmodel` chunk carries `bias = [-0.0078, -0.0078, 0.047]`, `scale =
[0.0078, 0.0078, 0.0078]` - centred on the origin, the same "node-local with a
bias near the origin" shape `docs/formats/README.md`'s Track startup row
already records for HD's ordinary advert billboards. Nothing on this page's
own HD section places the gantry either - it is the **track**'s own
`billboard8.gtf`-bound geometry that does, the same convention Pulse's
`321backplate`/`billboard8` binding is, found by extending the mechanism
already recovered for Pulse below rather than by anything new on this
model's own side. See ["Implemented on HD"](#implemented-on-hd-the-same-mechanism-on-a-mount-that-is-not-flat).

## Wipeout HD: the gantry is a card, and the mode picks the file (2026-10-08, `hd-gantry`)

**The maintainer's two reports from play, both measured on RPCS3** (private instance,
muted, 1280x720 at render scale 100; Talon's Junction, Vineta K). Captures under
`data/scratch/hd-gantry/` (`pl1`, `pl2`, `zp` RSX dumps; `cd1`, `zcd2`, `zb`, `det`,
`elim-cd` recordings). Not a new binary read: the mode numbers below agree with the
PS4 branch in [`ps4-omega-eu/billboards.md`](../ghidra/functions/ps4-omega-eu/billboards.md).

### "The countdown is very blurry": a bloom halo on a white board, because it was not a card

**What the original does, read off a hooked frame (`scripts/rpcs3_draw_hook.py`,
`scripts/rsx-draw-list.py`; one boot, texture named against `321_go_64.gtf` by its first
bytes and three further samples; confidence 85, one boot):**

- the main target is **A8R8G8B8 with 2x MSAA** (surface word `0x3148`: colour format 8,
  depth `0x40`, pitch `0x2800` = 2560 x 4 B), not fp16 as a first reading said;
- the frame draws **two 512 x 256 colour cards** before the scene (surface `0x123`,
  offsets `0x02aa0000` and `0x02ae0000`, pitch `0x400`). The second holds slot 8: 14
  draws of `321_go_64.gtf` (64 x 128 DXT5, 8 mips, min filter trilinear, mag linear) through
  `TEX; MUL H0.xyz, H0, {1,1,1,1}`. The first is slot 7 (`fx350`);
- the scene then samples the card on the track's `billboard8` quad (draw 128, program
  `0075f501`: `(f[TC0..] + ambient {0.404, 0.392, 0.51} + sun * sat(N.L)) * card`, then fog), so the board
  is **lit like the road under it**. A lit `GO` reads **(132, 131, 112)** on screen, not white.

**What ours did:** drew `321Go_StartFinish.vex` as a model stood on the mount, straight
into the scene. Its `GO` came out full white, above the HD bloom gate's knee, and
`post::hd_bloom` smeared it into a halo wider than the letter strokes. **A/B, same
camera and tick** (`data/scratch/hd-gantry/ours/cmp-bloom.png`): with the bloom term removed
the letters are crisp but still white; with the card route they are cream, crisp, and carry no halo,
matching the original frame. So the "blur" is not the texture (64 x 128 with the disc's
own 8 mips is what the original samples too), not the sampler and not the render scale.

**Fix:** `oag_title::adverts::Adverts::gantry_card` (HD `true`, measured; Pulse `false`).
On HD slot 8 goes through the advert card path (`oag_raceplay::gantry::place_card`): the
model is drawn through its own camera into the 512 x 256 target and the track's `billboard8`
quad samples it. The gantry's clock, per-window cull and `GO` edge are the ones
[`gantry-clock.md`](../ghidra/functions/ps3-hdfury-eu/gantry-clock.md) measured; they now drive the
card. **Not reproduced:** the card's R5G6B5 format (ours is RGBA8, chosen). Pulse's frame is
byte-identical before and after (`data/scratch/hd-gantry/pulse/`, sha256 `6318e31c...` at tick
150, `72fc7de6...` at tick 273).

### "Zone levels have a different gantry animation": the mode picks the file

Measured live, one boot per mode, slot 8 on Vineta K, from the Racebox `RACE TYPE` row
(`rpcs3-drive.py countdown --nav "Single Player=right,..."`):

| Race type | `GetMode()` (TTY) | Gantry before the release | File (inferred from the picture and the names) |
| --- | ---: | --- | --- |
| Single Race | 3 | `3 2 1 GO` board | `321Go_StartFinish.vex` |
| Eliminator | 8 | the same `GO` board | `321Go_StartFinish.vex` |
| Zone | **6** | dark panel; a loop icon assembles from light pieces | `321Go_Zone.vex` |
| Zone Battle | **13** | dark panel with arrow glyphs | `321Go_HD_Zone_Battle.vex` |
| Detonator | **14** | dark panel with scrolling glyph shapes | `321go_hd_detonator.vex` |

**The mode numbers are the ones the PS4 binary branches on** (`ps4-omega-eu/billboards.md`: allowlist
`{6, 13, 14, 21}`; 13 and 21 select Zone Battle, 14 the detonator, 6 `321Go_Zone.vex`), so the
rule HD follows is Omega's, by mode id. Confidence 85 for "the mode picks a different file", 80 for the
name per mode (the picture, the file shapes and Omega's table agree; HD's own code site was
not read: the four names are only referenced by a name-to-hash registration loop at
`0x003f1300`, entries in the registry at `+0x4a20...`, which is not a chooser).
The Zone icon starts about 3.5 s (210 ticks) before the release and is complete about 10 frames before it
(`zcd2`, release at video frame 1066 of 30 fps).

**Not wired, and why.** `oag_race::Mode` has Zone but no Zone Battle or Detonator.
Substituting `321Go_Zone.vex` for Zone was tried and **reverted**: loaded as a card, the panel
draws blank white on both archive copies (`DATA00` 8.4 KB / bars, `DATA02` 2.6 KB /
`cf_321_zone` + `321_go_64_zone2.gtf`; `DATA00` is mounted last by the game, which would make it
the live copy, but the original's picture has light pieces on a dark panel that the `DATA00`
copy's textures - `black_fadeout`, an 8x8 - cannot make). A wrong picture is not shipped in place of the known one. Open: which
copy the game loads, and why ours draws that model empty. A Zone race therefore still shows `3 2 1 GO`.

**Per title** (the rule is a single name table by mode id): **Omega** - ported in the binary's own code
(`ps4-omega-eu/billboards.md`), checked, not wired (no PS4 capture path); **2048** - checked, differs: no manifest
of the twenty-six names any of the three files (see the census above), so the rule has nothing to
substitute there; **Pulse** - checked, differs: one gantry file, no chooser, and its gantry
stays a placed model (the card route is measured on HD only).

## Wipeout 2048: eight files plus `fx350`, two reached by any of 26 manifests, a third countdown mechanism

**Read on `data/extracted/vita/PCSF00007`** (the decrypted EU package: base,
the 1.04 patch, and both DLC packages), with
`crates/rcs/tests/start_gantry_2048_ground_truth.rs` (9 tests) and
`crates/hd/tests/start_gantry_hd_ground_truth.rs`'s own added sixth test as
the executable form of every claim below. No Ghidra and no emulator - the
same `oag-wad`-equivalent reading (`scripts/psarc.py` for the archive census,
then `oag_formats::{trackstartup, vex, rcsmodel::psp2}` directly) the first
three passes used, plus two throwaway probes kept in the tree:
`crates/game/examples/gantry_2048_probe.rs` (node trees and `.rcsmodel`
submesh dumps) and `gantry_2048_anim_probe.rs` (sampled transforms, generalised
to take an archive spec and a node list so the same tool checks a claim
against HD's own disc, not only 2048's copy of it - see below).

### The archive census: eight `321Go_*`/`321Fight_*` files, plus `fx350.vex`

**Confidence 95** - a real PSARC listing, the same standard the HD pass held
its own four to before this pass, closing the "six or seven, string-search
only" gap the handover thread opened with. `base/PSP2/data.psarc` carries all
four of HD's own glyph files under `Data\Billboards\HD_Adverts\321Go\` -
identical names, `321Go_StartFinish.vex`, `321Go_Zone.vex`,
`321Go_HD_Zone_Battle.vex`, `321Go_HD_Detonator.vex` - HD's separate
substitution-target `fx350.vex` alongside them, plus **four** of 2048's own:
`321Go_2048.vex`, `321Go_2048_Combat.vex`, `321Fight_2048.vex` and
**`321Go_HD.vex`**, a 752-byte stub scene (a camera, a `start_lights` locator
and a `start_light_background1` locator, all static `Transform` nodes, no
`Mesh`, no `Anim Transform`, no sibling `.rcsmodel` anywhere in the package) -
easy to miss on a first pass precisely because it carries nothing to find.
Every one of the seven glyph files that *does* have geometry, plus `fx350.vex`
itself, has a sibling `.rcsmodel`, and every sibling opens as
`oag_rcs::rcsmodel::psp2`'s container (`0 unpaired_pointers` on all eight)
- **2048 re-exported HD's own four glyph files (and `fx350.vex`) into its own
Vita container rather than shipping the PS3 bytes verbatim**, a measured fact
about the pipeline, not an assumption.

**The 1.04 patch carries an eighth copy, not an eighth file**:
`patch-v104/PSP2/data2.psarc` ships its own `321go_startfinish.rcsmodel`, 160
bytes shorter than the base copy (51,183 against 51,343) and independently
confirmed as a valid `psp2` container by its own magic. Which of the two a
patched retail unit actually loads is unread - `oag_2048::open` deliberately
does not mount the 1.04 patch archives at all (see that module's own doc
comment: a patch shadows the base and this project's `Archives` searches the
base first, so mounting the patch behind it would be worse than not mounting
it), so **this whole page's own reading, like the rest of this project's 2048
support, is the base copy**, on the same terms every other unread-patch axis
in `docs/formats/2048-status.md` already is.

### Which of the eight a manifest actually names: two, across all twenty-six circuits

**Confidence 92**, from reading every one of the **twenty-six** circuits'
own `TrackStartup.xml` the whole package ships across all three
archives - ten native, four HD-ported and re-shipped in the base package, four
more HD-ported in `dlc1.psarc` alone, and eight more HD-ported in `dlc2.psarc`
alone (including all four `Zone` circuits) - not just the base package's
fourteen, the same exhaustive standard the Pure pass held itself to for its
own negative:

| file | reached by a manifest? |
| --- | --- |
| `321Go_2048.vex` | **yes** - slot 8, all ten native circuits (`altima`, `arena`, `bridge`, `cathedral`, `mall`, `park`, `sol`, `square`, `subway`, `tower`) |
| `321Go_StartFinish.vex` | **yes** - slot 8, all sixteen HD-ported circuits, across all three archives |
| `fx350.vex` | **yes** - slot 7, the same sixteen HD-ported circuits, the identical pair HD's own manifests carry |
| `321Go_2048_Combat.vex` | no |
| `321Fight_2048.vex` | no |
| `321Go_Zone.vex` | no |
| `321Go_HD_Zone_Battle.vex` | no |
| `321Go_HD_Detonator.vex` | no |
| `321Go_HD.vex` | no |

**Not even the four `Zone` circuits (`zone_1`-`zone_4`) reach `321Go_Zone.vex`**
- every one of them keeps HD's ordinary `fx350`/`321Go_StartFinish` pair, the
same as every other ported circuit. **None of the ten native circuits authors
a slot 7 at all** - unlike every HD-ported one, which keeps HD's exact
substitution-site pair unchanged. So 2048's own circuits have no
manifest-level equivalent of HD's slot-7 substitution to feed, and whether the
six unreached files load through some other, engine-side selection - the same
kind of site
[`ps3-hdfury-eu/billboards.md`](../ghidra/functions/ps3-hdfury-eu/billboards.md)
found for HD - is **unattempted for 2048**: no Ghidra time was spent on this
pass, per the thread's own priority order, so this is a manifest-level
negative only, not a runtime one - but it is now an exhaustive one, checked
against every circuit the package ships rather than only the base fourteen.

### The mechanism: four separate sliding nodes, not a UV-cell walk - a third encoding

**Confidence 90**, read directly off `321Go_2048.vex`'s node tree and its
`Anim Transform` decode - independent of the `.rcsmodel` correlation problem
below, which this finding does not need.

`321Go_2048.vex` authors `Three`, `Two`, `One` and `GO` as **four separate
`Anim Transform` nodes**, each carrying its own single `Mesh` child and its
own keyframe schedule - not Pulse's one node with four per-vertex UV cells,
not HD's one node with five. Sampling each node's translation:

| node | starts | slides on-screen at | slides off-screen at |
| --- | --- | --- | --- |
| `Three` | on screen (`x ≈ -0.1`) | - | frame 62 (1.033 s), to `x ≈ -33.6` |
| `Two` | off screen (`x ≈ -38.9`) | frame 79 (1.317 s), to `x ≈ 0.1` | frame 122 (2.033 s), to `x ≈ -27.3` |
| `One` | off screen (`x ≈ -38.9`) | frame 139 (2.317 s), to `x ≈ -0.4` | frame 182 (3.033 s), to `x ≈ -27.3` |
| `GO` | off screen (`x ≈ -43.2`) | frame 200 (3.333 s), to `x ≈ 5.8` | frame 360/361 (6.000/6.017 s), to `x ≈ -34.2` |

So the state a viewer sees is **which node is parked at `x ≈ 0` this
instant**, not which UV cell a shared offset has reached. `Three` is the one
exception to the "starts off screen" shape because it is first - its own
track only ever moves it *away*. Three `Stripe_0N` decorative nodes and a
static (non-animated, class `Transform`, not `Anim Transform`)
`start_light_background2`/`start_light_background3` pair round out the panel;
the background halves never move at all on this title, unlike Pulse's own
`start_light_background` node.

**Position selects the state; scale is polish within it, not a second
selector.** Every digit also carries a `scale` channel, sampled alongside
translation with `gantry_2048_anim_probe`: each one opens at roughly its rest
scale (`Three`, `GO` at exactly `1.0`; `Two`, `One` already at `0.988`, having
inherited whatever `Two`/`One` last held), eases down to `0.5` partway through
its own *visible* window (`Three`: `1.0 -> 0.5` between frames 0 and 40, while
still parked on screen; `GO`: the same shape between frames 200 and 220), and
holds at `0.5` afterwards, on or off screen, until the file's own end. So the
on/off state is entirely the translation channel's doing - a digit at `x ≈ 0`
and one shrunk to half size are both "shown," just at different points in its
own settle - and scale is an entrance flourish (pop large, settle to half
size) layered on top, not a fifth encoding needing its own row in the
cross-title table.

**The digit shape is geometry, not a texture lookup - checked, not assumed.**
Positionally matching each node to its `.rcsmodel` submesh by node-declaration
order (see the next section's own caveat on why this is positional rather
than addressed) finds the `Three`/`Two`/`One`/`GO` submeshes each carry **as
many distinct UV cells as they have vertices** - a fully unique unwrap, the
shape a hand-modelled glyph outline takes, not a shared-atlas sample. `GO`'s
own submesh (88 triangles) is visibly wider (`x` span ±17.4) than any single
digit's (`Three` ±6.8, `Two` ±6.7, `One` ±2.9), the same "GO spans the whole
board" shape Pulse's and HD's own wide cell take. The same positional mapping
puts `polySurface7Shape` (the chequered-flag mesh, see below) at submesh 0,
bound to material 0 - `2048_checkered.gxt` - whose own UVs run outside
`[0, 1]` (e.g. `(1.500, -0.501)`), exactly the tiling an artist authors for a
texture meant to scroll rather than sample once; a third, independent
agreement with the node-name match below, on the same capped 75 confidence
this whole positional mapping carries.

### Why node-to-submesh correlation is positional here, not addressed

**This container has no hash, no name, nothing to address a chunk by at
all** - `oag_rcs::rcsmodel::psp2`'s own module doc states this plainly:
every HD chunk is found through a `Mesh` node's `+0x30` hash, and 2048's
format simply has no equivalent field. So unlike
`start_gantry_hd_ground_truth.rs`, which resolves `pasted__Go_HD_start_light_321go`
to its exact `.rcsmodel` chunk by that hash, nothing here can prove which
submesh is `ThreeShape` versus guess from the order both the `.vex`'s 40
`Mesh` nodes and the `.rcsmodel`'s 40 submeshes are declared in. The count
matching exactly (40 and 40) and the shapes lining up (triangle counts, `x`
spans, the "GO is widest" property) is corroborating, not proof - carried at
a capped confidence (75) separate from the 90 above, which needs none of it.

### `321Fight_2048.vex`: the same asset, `GO` replaced by `FIGHT`

**Confidence 80.** `321Fight_2048.vex` is otherwise node-for-node identical to
`321Go_2048.vex` - same `Three`/`Two`/`One`, same `Stripe_*`, same
`Main_Logo` sponsor-reveal subtree - except the `GO` node is replaced by a
`FIGHT` node (its own `Anim Transform`, key times `[199..220, 360, 361]`,
the same 6.000 s loop-close pair `GO` carries) and the tail overlay node is
renamed `Hex_Overlay_Hex_Overlay` rather than `start_light_background3`.
**Unreached by any of the twenty-six circuits' own manifests, across all
three archives** (previous section), so this is a real, measured asset with
no confirmed trigger - the same "authored but unwired" state `HANDOVER.md`
already tracks for several particle effects, now joined by a countdown
variant. `321Go_2048_Combat.vex` is a smaller variant again: identical to the
base file except its own tail overlay swaps to `Hex_Overlay` while keeping
`GO` - also unreached by any manifest measured here.

### The master timeline is three beats, not one - 6.000 / 9.333 / 12.333 s, and it holds on HD's own disc too

**Confidence 88, the strongest single finding of the whole four-title
sequence.** The first pass at this section read Pulse's, HD's and 2048's
6.000 s teleports as one isolated coincidence. They are not isolated: Pulse's
own file (already on this page, above) authors a `start_light_background`/
`Board`/`Text`/`Arrow` teleport at 6.000 s, a `Board`/`Honey_Board`/
`Final_Lap` entrance at 9.333 s and a `Final_Lap`-out/`polySurface7`-in swap
at 12.333 s - **three beats, on one authored clock**, not three separate
findings. Checking 2048's own inherited copy of `321go_startfinish.vex`
found the same three beats on its `pasted__Go_HD_start_light_321go` (6.000 s),
`pasted__Final_Lap` (9.333 s entrance) and `polySurface7` (12.333 s) nodes -
and rather than trust a title's *re-export* of another title's file,
`gantry_2048_anim_probe` was pointed straight at
`hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC`'s own
`321go_startfinish.vex` to check HD's own disc directly. It carries the
identical keys: `pasted__Final_Lap`'s translation times are exactly
`[560, 561, 740, 741]` - entering at 9.333/9.350 s and exiting at
12.333/12.350 s - and `polySurface7`'s are exactly `[740, 741]`, both now
pinned by `crates/hd/tests/start_gantry_hd_ground_truth.rs`'s own sixth test,
added this pass specifically to check HD's disc rather than lean on 2048's
copy of it.

`321Go_2048.vex`'s own **native** gantry - not the inherited file - carries
the same three beats again, independently authored a third time in a
completely different node graph:

- **6.000 s**: `GO`'s own key times (`[198, 199, 200, 360, 361]`) land a
  teleport exactly on frame 360/361, the same frame numbers Pulse's and HD's
  own files use (HD's is one frame earlier, at 359/360). The delta is real
  but large (`x` moves from `5.777` to `-34.159`, about 40 units) and on a
  different axis than Pulse's and HD's own `y` moves (~10 units) - the
  *instant* matches exactly across all three; the axis and the magnitude do
  not, and are reported as measured rather than smoothed into agreement.
  Three `Stripe_0N` nodes teleport at the same 360/361 pair alongside `GO`,
  the same "more than one node moves together" shape Pulse's own
  `Board`/`Text`/`Arrow` group takes at its own 360/361.
- **~9.333 s**: `321Go_2048.vex` spells `FINAL` and `LAP` **letter by
  letter** - `Final_F`/`Final_I`/`Final_N`/`Final_A`/`Final_L`,
  `Lap_L`/`Lap_A`/`Lap_p` - each its own node, going a step further than
  Pulse's and HD's shared single `Final_Lap` node. Every one of those eight
  nodes' own first translation key lands at frame 558-566 (9.300-9.433 s) -
  the same instant, to within one frame in sixty, Pulse's and HD's own
  `Final_Lap` node enters at (frame 560/561, 9.333/9.350 s exactly).
- **12.333 s**: `321Go_2048.vex` carries a node named **`polySurface7`** -
  the identical name Pulse's and HD's own chequered-flag nodes carry - whose
  own translation keys are exactly `[740, 741]`, the exact frame pair both
  other titles use for the same chequered-flag entrance.

One shared node name and one exact frame-pair match would be a coincidence
worth a sentence; a **three-beat clock**, independently authored on three
platforms in at least three genuinely different geometry encodings, with one
node name surviving byte-for-byte across all three, is the sequence's actual
headline result.

### Placement: untried, on the same terms as the other three

Nothing in this pass looked at slot 8's transform for 2048 - the same
question every earlier title's own pass left open, now simply not attempted
here rather than measured absent. A reader who solves it once solves it for
all four titles at once, per the standing note at the top of this page.

## What four titles now say, and what a fifth should expect

**Nothing generalised the way the Pulse-only version of this page guessed,
and the things that did generalise were not predicted going in.** Four titles
in, all four gantries a disc actually ships have been read with no Ghidra and
no emulator - only Pure's own absence needed neither. This is the actual
state:

| | Pulse | Pure | HD/Fury | 2048 |
| --- | --- | --- | --- | --- |
| Slot 8 exists | yes, on 11/12 circuits | **no**, on any of 16 | yes, on all 16 | yes, on all 26 |
| Files behind it | one, `321Go_StartFinish.vex` | none | four, plus `fx350` | eight, plus `fx350`; a manifest reaches two, plus `fx350` |
| Geometry lives in | the `.vex` itself | n/a | a sibling `.rcsmodel` | a sibling `.rcsmodel` |
| State encoding | 4 UV cells + an **animated** material track | n/a (nearest asset uses a different technique) | 5 UV cells + a **static** material parameter | **4 separate sliding nodes**, no UV walk at all |
| Palette/geometry shape | 16x32 staircase texture, `u` fixed per digit | n/a | 64x128 (4x) diagonal staircase texture, DXT4/5 | each digit its own fully-unwrapped mesh, no shared atlas |
| Unused marker columns | yes (2, in the palette) | n/a | yes (2, wider, same idea) | n/a - no shared palette to carry one |
| Master timeline (6.000/9.333/12.333 s beats) | authored reference: the panel teleports, FINAL LAP enters, chequered flag enters | n/a | **confirmed on HD's own disc**, all three beats, `polySurface7`/`pasted__Final_Lap` named identically to Pulse's | **confirmed on 2048's own native file**, all three beats, on a different axis/magnitude for the first one |
| Per-mode chooser | none - one file | n/a - no slot to choose into | a substitution site exists, at slot 7, untraced to a result | none authored on slot 8 itself; six of eight glyph files unreached by any of 26 manifests |
| Placed at the start line | no | n/a | no | untried |

**What holds across every title that has a slot 8 at all** - Pulse, HD and
2048: the model name family (`321Go_StartFinish`/`321go_startfinish`/
`321Go_2048`), node names surviving verbatim or near-verbatim across three
completely different pipelines (`polySurface7` is byte-for-byte the same
string on Pulse's PSP export, HD's own PS3 disc and 2048's Vita one, checked
directly on all three rather than assumed from one re-export of another;
`Final_Lap`/`pasted__Final_Lap` becomes 2048's own `Final_*`/`Lap_*` letter
set, the same word broken down further rather than renamed), the "one asset
carries the whole race's states, including the chequered board and FINAL LAP"
design, and - the strongest result across the whole sequence - **a shared
three-beat master timeline**, not a single shared instant: something moves at
6.000 s, `FINAL`/`LAP` enters at 9.333 s and the chequered flag enters at
12.333 s, on all three titles that ship a gantry, independently authored in
three genuinely different geometry encodings. Three platforms, three
geometry formats, three completely different state-selection mechanisms, and
the same authored clock underneath all of them.

**What is per-title, not shared, is everything about *how* a state is
selected and drawn** - and this is where 2048 breaks the pattern the first
three titles' own comparison predicted, rather than extending it. Pulse walks
one shared texture offset across per-vertex UV cells; HD's file format has no
room for a texture-offset track at all, so its own five cells sit against a
static material parameter with the runtime-write question left open; **2048
does neither** - it authors four entirely separate mesh nodes, each a
hand-modelled glyph, each toggled visible by sliding its own `Anim Transform`
node on and off screen, with a `scale` channel layered on top as an entrance
flourish rather than a second selector. A shared texture atlas, an offset
track and a static parameter were the only three shapes considered across
three titles; 2048 is a fourth shape a reader should not have assumed was
exhaustive. **Also per-title**: how many files answer slot 8 (one, zero,
four-plus-`fx350`, eight-plus-`fx350`); whether a manifest reaches every file
it ships (2048 is the first title where it does not - six of eight
`321Go_*`/`321Fight_*` glyph files sit on the disc unreached by any of
twenty-six circuits' own manifests across all three archives,
`321Fight_2048.vex`'s swapped-in `FIGHT` node and the empty `321Go_HD.vex`
stub both included); and whether the state-selecting mechanism is an
**authored, played-back track** (Pulse, confidence 55 on the "played" half),
has to be **written by the engine because the format has no room for a track
at all** (HD, confidence 40), or needs **no offset write at all because each
state is its own node** (2048, confidence 90 on the mechanism itself, since a
node's own translation track is on-disk and self-contained - nothing here
needed to observe an engine write to explain how 2048's countdown advances,
which is a real, structural difference from HD's open question rather than a
higher-confidence answer to the same one).

**Placement is the one constant nobody has broken, and the one question no
pass has answered.** Four titles, at least three distinct geometry formats,
and not one of them has been found to read a position, rotation or scale for
the gantry anywhere between a manifest attribute and a constructor - Pulse and
HD by two independent routes reaching the same "nothing here places it"
result, 2048 simply untried. A fifth title, or a return pass on any of these
four, should expect the same absence and treat *recovering* it - not
re-confirming its absence again - as the actual remaining prize this whole
sequence has been circling.

**What a reader picking this up next should carry forward is the shape of
the check, not any row of the table above.** Read the manifest schema first
(does a slot 8 exist, and does anything else reach it, the way 2048's own
slot 7 turned out to be empty where HD's is not); confirm every candidate
file through a real archive listing before reading any of them, the way this
pass closed the previous one's "six or seven, string-search only" gap; then
look for the state-selection mechanism with no assumption about its shape at
all - a shared atlas, an offset track, a static parameter and a set of
separate sliding nodes are the four this sequence has found, and a fifth
title is not obliged to pick from that list.

## Where the placement actually comes from: HD/Fury answers it for Pulse

2026-09-06. Six passes on Pulse had failed to find what writes slot 8's world
transform, and the last one concluded that nothing static-only remained untried
and that a live PPSSPP breakpoint during a track load was required. **That
conclusion rested on a wrong premise: that the billboard system places the
gantry at all.** Reading the same subsystem on a *different title* - where
Ghidra's call xrefs actually work - shows it does not, on either title.

### What HD does, and why it settles the shape of the question

Full account in
[`ghidra/functions/ps3-hdfury-eu/billboards.md`](../ghidra/functions/ps3-hdfury-eu/billboards.md).
The short form:

- The billboard object's transform is written **once, to a literal 4x4
  identity**, confirmed from raw disassembly with every `vsldoi` shift worked
  through (confidence 88). Nothing in the subsystem ever writes anything else
  into it.
- The loader (`0x003a4da0`) touches the *track* in exactly one place: it builds
  the string `"billboard" + num` from the manifest's own `Num` and rebinds the
  matching material's texture (confidence 84).
- The disc corroborates the convention independently: `billboard7.gtf` and
  `billboard8.gtf` are present in **all 17** HD environments' own texture sets,
  bound through `billboarddiffuse.rcsmaterial` (confidence 90).

So on HD the artist models the billboard's mounting surface into the track, at
the right place, per circuit, and the engine swaps its texture. **The placement
lives in the track model, not in billboard-system state.**

### Pulse authors the identical convention

`oag-view --nodes` against `pulse-psp-usa.chd`'s `Data.wad` finds the same
placeholder family inside Pulse's own track scenes, under
`Data\Environments\GenericTrackTextures\`:

| circuit | `billboard8.tga` | `321backplate.tga` |
| --- | --- | --- |
| `01_Track` | yes | yes |
| `02_Track` | yes | yes |
| `03_Track` | yes | yes |
| `05_Track` | yes (with `billboard1`-`billboard7` too) | yes |
| `16_Track` | yes | yes |

**This overturns a standing rejection.**
[`docs/formats/README.md`](../formats/README.md)'s "Track startup" row currently
dismisses this route: "a tempting shortcut - bind a slot to the like-numbered
`billboardN.tga` placeholder quad already baked into the track mesh - fails its
own disc: `16_Track` authors all eight slots but only five numbers have a
matching placeholder texture." **That argument does not hold, and HD's
executable is the counter-example.** `amphiseum` authors eight slots and ships
only `billboard3/7/8.gtf`, and `0x003a4da0` performs the name lookup anyway - a
slot with no matching placeholder simply binds nothing. The count mismatch
refutes "every slot binds"; it does not refute "binding is by name." That row's
stronger claim is untouched and is the load-bearing one: slot 8 is
`321Go_StartFinish.vex` on every circuit of both titles that authors a slot 8.
*(Flagged rather than edited - `docs/formats/` was outside this pass's lane.)*

### The placement, recovered: `16_Track`'s mesh node 74

`oag-view --draws` does run headlessly - the earlier "it needs a GPU" reading was
wrong, it is simply slow on a 603-mesh track and two runs were killed by their
own timeout. Given long enough it resolves the whole chain, and the answer is one
node:

```
$ oag-view "data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad" \
      --mesh 'Data\Environments\16_Track\track.vex' --draws
  ...
  opaque node   74   290 tri  tex 41: factory_floor_01_rp.tga 64x64  centre 34,-42,-185
  opaque node   74   176 tri  tex 41: factory_floor_01_rp.tga 64x64  centre 34,-42,-185
  opaque node   74    36 tri  tex 40: billboard8.tga         8x8     centre 34,-36,-185
  blend  node   74    57 tri  tex 42: 321backplate.tga      64x64    centre 35,-36,-185
```

**One mesh node carries all three materials**: a factory floor, a 36-triangle
`billboard8.tga` quad and a 57-triangle alpha-blended `321backplate.tga` panel,
the last two at the same centre. So both textures are *bound by real draws*, not
unused entries in the file's texture palette - which is what the previous
revision of this section could not yet say.

**`billboard8.tga` is 8x8.** Every other texture on this node is 64x64. An 8x8
texture is a stub, which is exactly what a surface whose texture is replaced at
runtime looks like - the same role HD's `billboard8.gtf` plays there.

### The coordinates, and why they are usable as they stand

Node 74's parent is node 34, a `Transform` with a **zero-byte payload** - a pure
grouping node with no matrix - whose own parent is the `World`. No `Anim
Transform` is anywhere in the chain, so [`mesh`](../../crates/mesh/src/mesh.rs)'s
"anchored, not absolute" caveat does not apply here and the composed centres are
in the track's own space. That the numbers are track-space and not node-local is
confirmed by their range across the file: **x `-1273..952`, z `-1817..683`** over
2,079 draws.

Against that, three positions that agree with each other:

| what | position |
| --- | --- |
| **slot 8's mount surface** (node 74, `billboard8` + `321backplate`) | `(34, -36, -185)` |
| the `Start Position` node (`0x3bc`, the file's only one) | `(-131.8, -50.9, -173.6)` |
| all 55 `startline_*`-textured draws | two clusters, x ~ `-147` and x ~ `22..24`, z `-160..-211` |

The mount sits at the `+x` end of the same start-line structure the grid sits
inside, **about 15 units above the road plane** (`y = -36` against the
surrounding road's `y ~ -51`) - over the track, not on it. That is where a gantry
stands.

**Confidence 85** that this is the gantry's placement on `16_Track`. It is read
straight off the disc by this project's own parser, so it is not capped by the
static-Ghidra ceiling; it is held below 90 because `centre` is a per-draw
centroid rather than the node's transform origin, and because nothing has yet
drawn `321Go_StartFinish.vex` at this position to confirm it looks right.

### The same signature on a second circuit

Repeated on `05_Track`, which authors all eight slots rather than `16_Track`'s
subset, and the shape is identical - **one node, both textures, an 8x8
`billboard8` stub**:

```
  opaque node   19    57 tri  tex 22: billboard8.tga    8x8    centre -37,-2,-219
  blend  node   19    36 tri  tex 21: 321backplate.tga 64x64   centre -38,-2,-219
```

Against `05_Track`'s own reference points:

| what | `16_Track` | `05_Track` |
| --- | --- | --- |
| slot 8's mount | `(34, -36, -185)` | `(-37, -2, -219)` |
| `Start Position` | `(-131.8, -50.9, -173.6)` | `(21.3, -16.4, -66.3)` |
| `startline_*` draws, z | `-160 .. -211` | `-217 .. -218` |
| mount height above the road plane | ~15 | ~10 |
| distance from the grid to the mount | ~166 | ~164 |

Four independent agreements across two circuits: the two textures always share
one node, `billboard8` is always the 8x8 stub, the mount always sits ~10-15
units above the road, and it always stands ~165 units from the grid. **That last
number repeating to within 2 units on two unrelated circuits is the strongest
single signal here** - it is what a fixed authoring convention looks like, and it
is not something a coincidence of unrelated geometry would produce.

### The basis, the scale and the whole disc: `gantry_mount_ground_truth`

The two circuits above were done by hand with `--draws`. The sweep in
[`crates/render/tests/gantry_mount_ground_truth.rs`](../../crates/render/tests/gantry_mount_ground_truth.rs)
does every circuit on the PSP disc through `oag_render::gantry`, which fits the
mount's **plane** - principal axes of its own vertices, so the
smallest-variance axis is the surface normal and the other two span the panel -
rather than only its centroid. **Twelve circuits author a mount**, not the two
this was found on - and that is the count off the base disc alone, over
`track.vex` only:

| | measured |
| --- | --- |
| circuits with a mount | **12** (`01`-`07`, `09`, `10`, `13`, `14`, `16`) |
| panel size | 45.13-45.50 wide x 8.61-12.96 tall, except `13_Track` at **36.30 x 8.61** |
| panel thickness | 0.000-0.100 - flat, so the normal means something |
| angle between the panel's normal and the `Start Position`'s own forward | **0.0-3.0 degrees**, on all twelve |
| the mount's direction from the grid, dotted with that forward | **+0.99 to +1.00** - dead ahead, on all twelve |
| distance from the `Start Position` | 163.5-173.9 |

Two of those close the questions this section used to leave open.

**The basis.** The panel's normal lands within three degrees of the start
line's own authored forward on every circuit, and the panel is flat to a
tenth of a unit. That is not something a centroid could have said and it is not
something unrelated geometry does twelve times: the mounting surface faces
straight back down the track at the grid. The eigenvector's *sign* is
still ambiguous - a principal axis is a line - so `Mount::matrix` takes the
race direction from the caller and points the board at the grid, which the
`+0.99` column is what justifies.

**Which surface.** `321backplate.tga`, and the argument that settles it is not
the texture list - it is `14_Track`. On eleven circuits the `billboard8` stub
sits 0.48-1.63 units from the backplate on the same plane, so the choice would
be noise; on `14_Track` the two are **354 units and 54 degrees apart**. So they
are not interchangeable, the stub is not a safe fallback, and
`oag_render::gantry::mount` prefers the backplate - which the sweep asserts,
per circuit, rather than leaving to a comment.

**And the scale is 1.0, measured.** The gantry's own countdown board
(`start_light_321goShape`) is 34.02 units across against a 36.30-45.50 panel,
so it fits its mount at 1:1 on every circuit including the narrow one. The
model's *widest* piece is the 43.25-unit chequered board, which would overhang
`13_Track` - but that is one of the pieces clipped away below, so it never
reaches the screen.

### What it took to actually draw it

Three things the coordinates alone did not say, found by rendering:

1. **The gantry cannot stand exactly on the panel.** Its board is then coplanar
   with the track's own and loses a `Less` depth test at every distance - the
   whole gantry invisible, with the 8x8 placeholder showing through where it
   should be. `oag_raceplay::gantry::CLEARANCE` stands it one unit in front.
   That value is **chosen, not measured, and carries no confidence score**; what
   is measured is only its order of magnitude, from the 0.48-1.63 units the
   artists themselves put between the two co-located surfaces.
2. **The parked states have to be clipped, or the race opens with `FINAL LAP`.**
   One asset carries every state as windows on one timeline (above), and a state
   that is not its turn is *parked to the side* - 33 units left for `FINAL LAP`,
   43 right for the chequered board - not hidden. The original parks them behind
   the track's own structure; this project draws the model in open air, so
   without a cull the first frame of lap one shows a full-size, legible
   `FINAL LAP`. `oag_render::gantry::clip_to_panel` drops the draws whose own
   extent sits outside the **mount's authored width**, which is 6 or 7 of the
   model's 15 on every circuit - the two states whose trigger is unrecovered,
   and nothing else.
3. **The gantry's clock starts at tick 92 on Pulse (other titles: [Pulse's rule on their own edge](#every-other-title-pulses-rule-on-its-own-go-edge)) and is held at frame 559** - the last
   frame before `Final_Lap`'s own first key. The 92 is measured, not chosen: it
   puts the asset's `GO` step on the thrust release, as the original does - see
   [the timing section](#timing-against-the-measured-countdown).

Rendered on `16_Track`, `05_Track`, `13_Track`, `14_Track` and
`16_Track`'s reversed variant at 1.00 s, 1.83 s, 2.60 s, 4.00 s and 5.90 s, the
board plays the sequence this page predicts from the texel grid: `3`, then `2`,
then `1` white with `3` and `2` gone dark red, then a green `GO`. **The `2.60 s`
frame is the one that was predicted before it was drawn**, and it is the one
that matches.

### The count through the loader is larger than the count off the base disc

The twelve above is what the sweep measures, and the sweep opens the base disc
alone and reads `track.vex` alone. The game's own loader mounts the downloadable
packs too and races the reversed variants, and driving `oag-game --race
--track ...` over every circuit and both variants places a gantry on **32 track
files - sixteen circuits, forward and reversed - with no circuit reporting an
absence.** The four circuits the sweep does not see (`08`, `11`, `12`, `15`) are
the ones that arrive with a pack.

That is the number that describes what a player gets. The twelve is the number
that describes what one disc image proves without any pack mounted, which is why
both are here.

### What is still open

- **What starts the timeline at tick 92 in the original.** The fit places frame 0
  there to about 3 ticks; nothing has read the code that does it. See [the timing
  section](#timing-against-the-measured-countdown).
- **The `FINAL LAP` and chequered states are not shown at all**, because their
  triggers are unrecovered. Recovering them turns the clip above from a
  necessity into a per-state cull.
- **Pulse's own binding path.** `search_strings billboard` on
  `/pulse/BOOT-psp-pulse-usa.BIN` returns only `Billboard` (the XML element),
  `PI_BILLBOARD` and `PI_Billboard` - **no lowercase `billboard` base string for
  a `billboard%d` build**, so Pulse does not obviously use HD's exact name
  concatenation. The geometry is now located either way; how the engine finds it
  is not.
- **The Zone negative control did not run.** `Data\Environments\26_Track\track.vex`
  does not exist in `Data.wad` under that name, so the check that a
  manifest-less circuit also lacks this surface is **untested**, not passed.

**No live emulator work was needed for any of this.** That is the substantive
change this pass makes to the blocker: the six-pass conclusion that unblocking
required a PPSSPP breakpoint on `func_0x00140bd4` during a track load rested on
the premise that the billboard system places the gantry, and it does not.

## Implemented on HD: the same mechanism, on a mount that is not flat

> **Superseded for drawing, 2026-10-08:** HD no longer draws the placed model; slot 8 is a card
> (see ["the gantry is a card"](#wipeout-hd-the-gantry-is-a-card-and-the-mode-picks-the-file-2026-10-08-hd-gantry)).
> The measurement of the mount below still feeds the panel cull and Pulse's route.

2026-09-13. The Pulse mechanism above generalises to HD with one extension and
one genuine title difference, both measured rather than assumed. No Ghidra and
no emulator - everything below is `oag_render`/`oag_game` reading the real
disc through this project's own parser, the same standard the sections above
hold themselves to.

### The mount: matching HD's own label shape, confidence 90

`oag_render::gantry::mount`'s texture match used to compare a draw's whole
texture label against `321backplate.tga`/`billboard8.tga`. That is exactly
right for Pulse, where [`mesh.rs`](../../crates/mesh/src/mesh.rs) reduces an
embedded `Texture` node to its bare file name before it ever reaches
[`Model::textures`](../../crates/mesh/src/mesh.rs) - but an HD (or Wipeout
2048) `.rcsmodel` material names its `.gtf` by the **full archive path** the
sampler table carries (`crates/mesh/src/mesh/rcs/skin.rs`'s `path`
argument, passed to `ModelTexture::from_gtf` unchanged):
`data/environments/talons_junction/textures/dds/billboard8.gtf`, not
`billboard8.gtf`. Matching the whole label never found an HD surface by name
at all. `mount` now matches on the label's own file name
(`oag_render::gantry::basename`) and tries a third candidate,
[`HD_SLOT8_TEXTURE`](../../crates/render/src/gantry.rs) (`billboard8.gtf`),
after Pulse's two - HD ships no backplate-shaped second surface next to its
own stub, so there is only one name to try there.
`oag_render::gantry::strip_slot_placeholders` gained the same eight `.gtf`
spellings, so HD's own `billboard1-8` stubs are finally suppressed the way
Pulse's were.

**Measured on four circuits** (Talon's Junction, Amphiseum, `01_Vineta_K`,
`Tech_De_Ra`), each through exactly one draw bound to `billboard8.gtf`, no
unioning of unrelated surfaces:

| circuit | centre | width | height | thickness | vertices |
| --- | --- | --- | --- | --- | --- |
| Talon's Junction | `(75.2, -34.6, -185.8)` | 46.3 | 9.8 | **3.00** | 228 |
| Amphiseum | `(-516.5, 18.2, 45.7)` | 46.3 | 9.9 | **2.97** | 228 |
| `Tech_De_Ra` | `(-120.3, 3.7, 70.3)` | 46.6 | 9.8 | **2.99** | 228 |
| `01_Vineta_K` | `(-24.1, 52.2, -106.3)` | 44.7 | 10.8 | **2.02** | 276 |
| `12_Sol_2` | `(275.9, 9.1, 411.4)` | 42.4 | 10.3 | **1.91** | 276 |

**The load-bearing number is thickness, and it is not the Pulse story.**
Pulse's own `billboard8.tga`-bound stub measures 0.000-0.100 across all
twelve circuits that author one - flat, which is what makes reading a
"panel" off it meaningful. HD's own `billboard8.gtf`-bound chunk measures
**1.9-3.0** on every circuit checked: it is not a thin placeholder quad, it
is part of a real 3D structure - on Talon's Junction, visibly the same
boost-gate frame arching over the track that every capture in this section
shows. `oag_render::gantry::mount`'s principal-axis fit still returns a
usable plane (width and height both land in the same order Pulse's own
panels do, and thickness stays well under height on every circuit, so it is
not fitting a blob), but the *centroid* it returns sits inside that
structure's own volume rather than on a single flat backing surface the way
Pulse's does. `crates/game/tests/start_gantry_hd_mount_ground_truth.rs`
pins Talon's Junction's own numbers, thickness included, specifically so a
future change that makes this read back near zero is treated as a
regression to check by hand, not a fix.

### Loading the model: the sibling `.rcsmodel`, confidence 90

`race::gantry::load` bailed on every HD gantry with "a PS3 .vex carries no
render geometry" - correct, and the reason is the same one
["The geometry moved out of the `.vex`"](#the-geometry-moved-out-of-the-vex-the-mechanism-has-to-be-looked-for-beside-it)
gives above: `321Go_StartFinish.vex`'s own `Mesh` payloads are a bounding box
and a hash, and the real vertices are in `321go_startfinish.rcsmodel` beside
it. `load` now detects an external PS3 `.vex`
(`oag_mesh::mesh::geometry_is_external`) and builds through
`oag_mesh::mesh::rcs::build` with that sibling - the same function a craft's
own livery loads through, not `build_scene`: the gantry has no world-baked
second pass of its own to justify `build_scene`'s extra skip, the identical
reasoning that function's own doc comment gives for every non-track caller.
Confirmed decoding cleanly: 19 of 19 mesh nodes, 1,530 triangles, radius
23.25 - exactly the count `docs/ghidra/functions/ps3-hdfury-eu/billboards.md`'s
rendering pass already recorded for this file.

### A second, independent bug the load exposed: stale bounds under a placement matrix

Getting the model to load found a bug in an entirely different place, latent
since the mechanism first landed on Pulse. `oag_render::pvs::visible`'s
frustum test reads a draw's own `bounds` and trusts them to already be in
world space - true for the track (baked at build time) and true for any draw
under an `Anim Transform`, which is skipped from the test outright
(`DrawCall::moving`) because its bounds are stale the moment the node
animates. Neither covers a standalone prop carrying its own **external**
placement matrix, the shape `race::gantry::Placed` is: its bounds describe it
centred near its own origin, before the mount's matrix ever moves it.

**Pulse never hit this.** Every one of `321Go_StartFinish.vex`'s nine `Mesh`
nodes sits under its own `Anim Transform` (["The asset"](#the-asset)'s node
tree above), so every Pulse gantry draw is already `moving` and the frustum
test never ran on one. HD's own `321go_startfinish.vex` mixes animated and
plain `Mesh` nodes - the digit board and the background panel both teleport
at 6.000 s and are `moving`, but several smaller pieces are not - so this is
the first gantry pass with a draw left for the bug to reach. Left unfixed,
every non-`moving` draw's local-space sphere is tested against a world-space
frustum and fails everywhere except the world's own origin, which reads
exactly like "the gantry never draws" and is easy to mistake for a placement
error. `race::gantry::place_bounds` now rewrites every draw's bounds into the
placed matrix's own space once, at load.

### The digit board's own teleport plays correctly, confirmed through this project's pipeline

`oag_mesh::mesh::Model::write_node_anims`/`CLOCK_LIMIT` already replay an
`Anim Transform` track off the race clock - nothing new was needed for the
mechanism itself, only for a HD model to reach it. Sampling the loaded
model's own digit-board node at `t = 5.9` and `t = 6.0` through
`Model::sample_anim_nodes` gives world Y `-0.719` and `9.286` - a **+10.004**
move, matching
["The 6.000-second loop close generalises"](#the-6000-second-loop-close-generalises---the-best-result-of-the-three-title-sequence)'s
own figure exactly, now reproduced through this project's own model builder
rather than only the raw `.vex` track that section and
`crates/hd/tests/start_gantry_hd_ground_truth.rs` already read directly.
`crates/game/tests/start_gantry_hd_mount_ground_truth.rs`'s own third test
pins this.

### Placement is confirmed correct; what was actually in the wrong place is borrowed slot-7 art

2026-09-13. A player reported two defects together: "the HD gantry doesn't
show the numbers, and has a Wipeout symbol drawn in the middle of the track."
The first is the glyph-walk gap above; the second reads, before measuring
anything, like a placement bug in [`Mount::matrix`](../../crates/render/src/gantry.rs) -
and it is not one.

**The board itself is placed right.** `data/reference/hd-capture/talons-ships/00.png`
(the original, default chase camera, grid, `0.00.0`/`0 KM/H`) and this
project's own render at the matching pose and tick (`--ticks 5`, same
default chase framing) put the gate structure - the blue speed-pad loop, the
green pipes either side, the grey gate frame - at the same screen position
in both, pixel for pixel, and the countdown board's own backing panel sits
inside that gate at the same height and horizontal centre in both. No change
to `mount()` or `matrix()` was needed or made; changing either to chase this
report would have been a regression against a board that already lines up.

**What actually floats over the open road is seven mesh nodes carrying
slot 7's own art, pasted into slot 8's shared model.** `321go_startfinish.vex`
authors `polySurface151` through `polySurface157` bound to
`fx350_nomip.gtf` - the "FX-350 Official A-G Racing League" checkered
banner [`billboards.md`](../ghidra/functions/ps3-hdfury-eu/billboards.md#the-four-321go_vex-shapes-are-confirmed-as-genuinely-different-content-by-rendering-them)'s
own rendering pass already found, not slot 8's own `321_go_64.gtf`
staircase. Three of the seven, `polySurface155/156/157`, sit at local `z` ~=
33 - a full panel-width off the digit board's own display plane (`z` = 0,
the same plane [`clip_to_panel`](../../crates/render/src/gantry.rs)'s own
doc comment measures this node's normal against), not beside it in `x` the
way `Final_Lap`/chequered are, so the existing width-only clip never touched
them. Once [`Mount::matrix`](../../crates/render/src/gantry.rs) faces the
model at the grid, that trio lands roughly 33 units towards the camera, over
the open road: three overlapping ring/ellipse shapes (`oag-view --only
polySurface15 --screenshot`, autocontrasted to see past the standalone
viewer's own unresolved shading) that render as a stray logo - a
byte-for-byte match to "a Wipeout symbol drawn in the middle of the track."
None of the seven nodes appears anywhere in `talons-ships/00.png`, at the
gate or over the road, so the fix is not "move this trio" but "never draw
any of the seven": `oag_render::gantry::strip_fx350_art` drops every draw
bound to `fx350_nomip.gtf`, the same by-texture pattern
[`strip_slot_placeholders`](../../crates/render/src/gantry.rs) already uses
for the track's own billboard stubs, rather than a per-node coordinate that
happens to explain one report. **Confidence 90** - read straight off the
model's own node/texture bindings and confirmed by a matching-pose render,
not decompiled or inferred.

Rendered before and after, at `--ticks 5/120/180/240/300/400` on Talon's
Junction: the ring cluster is gone at every tick and the gate/board region is
pixel-identical to before the change (the fix only removes draws; it moves
nothing). `crates/game/tests/start_gantry_hd_mount_ground_truth.rs`'s own
second test now also asserts the report names `fx350_nomip.gtf` as stripped.

### The glyph walk stays unwired on HD, on purpose

Pulse's countdown plays because its material's own authored `TEXOFFSET` track
is replayed - the mechanism this page opens with. HD's own equivalent
material carries no such track at all
(["What is not proven: whether anything ever changes it"](#what-is-not-proven-whether-anything-ever-changes-it---confidence-40-and-this-is-where-hds-answer-genuinely-differs-from-pulses)),
confidence 40, deliberately not raised by this pass: nothing here traced a
runtime write, and the render pipeline that would carry one
(`crates/mesh/src/mesh/rcs.rs`'s own material/shader-variant resolution) is
outside this lane's own boundary while `lane-hd-material-curve` is active.
Writing a synthetic UV offset here to make a digit appear would be inventing
the mechanism `CLAUDE.md`'s own "never invent what the assets already
author" rule exists to stop - HD's file has nowhere on disk for such a track
to live, so at rest the board plays exactly what it is authored to: nothing
readable, per that section's own texel-sampling result. This is left as an
honest absence, the same as slots 1-7's own unrecovered hoarding mounts.

**New evidence on the direction of the confidence-40 question, not a
resolution of it.** `data/reference/hd-capture/talons-ships/00.png` - a real
capture at the grid, HUD timer `0.00.0`, `0 KM/H` - shows a lit white `3` on
the gantry board. The static parameter alone cannot produce that: sampled at
rest, all four digit cells land at texel row ~55, below every staircase band
the section above measures. A capture showing a lit digit is independent
evidence, from neither of the two readings that section already weighs
against each other, that **something writes this parameter at runtime** -
strengthening the case for a runtime write without settling where it comes
from or raising the static confidence score itself, which is about the
*format* having no on-disk track, not about whether the engine compensates
for that at runtime.

**2026-09-13, a live capture through an actual countdown makes this a
progression, not a single frame - still not a resolution.** `scripts/rpcs3-drive.py
capture --shots 5 --interval 1 --keep-dumps`, RPCS3 on a virtual display
(`docs/reverse-engineering/rpcs3-capture.md`), Talon's Junction, five poses
roughly a second of real thrust apart from the grid. The gantry board across
the five: nothing, nothing, a faint red sliver at the panel's left edge, a
clear white-on-red `3`, then `3` and `2` visible together (the fourth and
fifth frames, cropped and read directly - `/tmp/oag-drive/glyphs/countdown-crop-{2,3,4}.png`
in this pass's own scratch output). A static rest cell cannot produce a
sequence; this is the board changing *while a race plays*, which is
stronger than the single lit frame above and is new evidence a runtime write
exists, not new evidence of what writes it.

**The write itself was not isolated.** `Billboard_LoadModelAndBind`
(`docs/ghidra/functions/ps3-hdfury-eu/billboards.md`) already names the
carrier a write would take: a per-instance `uvOffset`/`uvScale` pair,
initialised to identity `(0,0,0,0)`/`(1,1,1,1)` and bound as named fragment-program
constants patched **straight into the shader microcode** (the `fslot` patch
chain `scripts/ps3-microcode.py` documents), not a separate register - so a
per-frame write would show up as a change to the uploaded fragment-program
bytes in the RSX pushbuffer, not a constant-buffer upload. A raw byte diff
between consecutive captures' pushbuffer dumps was tried first and is too
noisy to read this way: one region (the RSX FIFO ring proper) changes by only
a handful of bytes between any two frames, but the two larger regions carry
every other draw's own per-frame data (camera, ship, scenery animation) and
change in hundreds to thousands of places a frame apart, with no reliable
way from raw bytes alone to tell "the gantry material's own patched constant"
from "an unrelated draw's constant that happens to live nearby in the ring."
A step-pattern scan (stable across the frames before the digit appears, then
a clean jump) found four candidates, all of which reverted on the very next
frame rather than continuing to step - consistent with ring-buffer command
placement drifting frame to frame rather than with one stable, patched value.
**Confidence 40 is unchanged**: this is new evidence a runtime write exists,
not a location for it. The nearest concrete next step already on the page is
[`billboards.md`](../ghidra/functions/ps3-hdfury-eu/billboards.md#the-slot-8-asymmetry-and-what-is-still-not-established)'s
own named lead - eleven of the fourteen `lwz 0x834(` sites that read
`table_base + 0x834`, the cached slot-8 texture-bind slot the loader stores
and nothing was found reading, are unexamined. That is a *candidate* carrier,
not a confirmed one: `table_base + 0x834` is the texture binding, not the
`uvOffset`/`uvScale` pair above, and nothing on this page or `billboards.md`
has traced either to the other. Either a semantic, packet-aware read of the
pushbuffer against the fragment program's own patch-slot table (the raw diff
above's own missing half), or opening those eleven call sites in Ghidra, is
the next step - not another raw byte diff, which this pass already showed
does not converge on its own.

### 2026-09-17: the runtime write is located, with a patched RPCS3 GDB watchpoint

**`lane/hd-gantry-glyph-walk`.** `just build-rpcs3-watchpoints`'s patched
RPCS3 (working `Z2` write watchpoints - `rpcs3-debugger.md`) attacked the
open question directly instead of another pushbuffer diff: read
`Billboard_LoadModelAndBind`'s own decompile for the *runtime* addresses of
slot 8's 19 per-submesh instance blocks (heap-allocated, not knowable
statically), armed a `Z2` on all 19 at once, then let an actual countdown
play. **Result: a real write fires**, at instance index 2's own `uvOffset.x`,
same PC (`0x005f9c9c`) on two independent boots with two different heap
layouts. The writer is `AnimCurve_EvaluateChannels` (`0x005f9bd0`), called
from the per-frame `Billboard_UpdateInstanceUvs` (`0x003e4f18`) off
`Billboard_UpdateAndRender` (`0x003a5f68`) - three newly named functions, full
write-up, evidence and addresses in
[`billboards.md`](../ghidra/functions/ps3-hdfury-eu/billboards.md#2026-09-17-the-runtime-write-is-located-live-with-a-patched-rpcs3-gdb-watchpoint).

**This closes the location and mechanism, not the authored data.** The
curve's own content (`target+0x20+0xc`) was not decoded, and the resource it
hangs off is the loaded `.rcsmodel`'s own in-memory object - very likely
authored in that file, in a section `oag-rcs` does not parse yet. Playing it
back needs that decode first, then a replay hook in
`crates/mesh/src/mesh/rcs.rs` (owned by `lane-hd-material-curve` while that
lane runs) - see `billboards.md`'s own closing paragraph for the exact two
steps. **The confidence-40 score for "HD's material carries no `TEXOFFSET`
track" is unchanged and still correct** - it is a finding about the
*material's own format*, and this pass's write comes from a different
resource-level mechanism entirely, not from that track.

**The `lwz 0x834(` lead this page pointed at is dead - do not re-run it.**
`billboards.md`'s own 2026-09-15 section (dated two days before this one, in
the same file) already re-examined all 14 matches after the lvlx reimport and
ruled out every one: 3 in `FUN_000654e0` (a `RaceManager` cursor, different
base entirely), 2 more decompiled and ruled unrelated, and the remaining 9
are `-0x834(r2)` - ordinary TOC-relative globals off the TOC pointer, a
different addressing shape than `table_base + 0x834` by construction, caught
by the search pattern's own substring match rather than being real
candidates. The "eleven unexamined" figure the previous version of this
bullet named was already stale when it was written; this is
`handover-threads-lag-docs` in miniature, inside a single doc file rather
than between a thread and the code.

### What is still open

- **RESOLVED, 2026-09-17 (`lane/hd-gantry-wire`): the countdown mechanism is
  material 2's own curve, alone - the same "one shared offset on one
  material" shape as Pulse's gantry, not a distributed four-curve one.**
  `lane/hd-edgeanim` decoded Edge Animation Tools' own byte layout
  ([`docs/formats/edge-animation.md`](../formats/edge-animation.md)) and,
  measuring one curve at a time, read each of the four as "a per-material
  wipe/reveal ramp... not four discrete glyph states" - true of any one curve
  in isolation. This lane wired the replay generically
  (`oag_mesh::mesh::rcs::curve_track`, every `.rcsmodel` material with a
  curve, the same per-model race clock every other animation already rides,
  no new time base), actually played it, and then checked *which* of the
  four curved materials' own geometry was actually on screen: only material
  2's - `Go_HD_start_light_backgroundShape` (the backdrop) and
  `pasted__Go_HD_start_light_321goShape` (the digit glyph mesh with its five
  UV cells) both bind it. Materials 1, 3 and 4's own geometry
  (`polySurface7Shape`, the chequered-flag state; `polySurface151Shape`
  through `polySurface157Shape`, slot 7's embedded `fx350` art; and
  `pasted__Final_Lap*Shape`, the `FINAL LAP` state) is removed from every
  frame by mechanisms this project already had before this lane -
  `oag_render::gantry::clip_to_panel` and `::strip_fx350_art` - so those
  three curves never reach the screen regardless of what they sample.

  Played for real: `oag-game --race --track
  /data/environments/talons_junction/track.vex --no-audio --screenshot`,
  `hdfury-ps3-eu-dec.iso`, ticks 0/90/180/240/360. The board is blank at
  tick 0; `3` and `2` are both legible by tick 90 (1.5 s); `3`/`2`/`1`
  together by tick 180 (3.0 s); `GO` alone, on a banner that has gone from
  red to green, by tick 240 (4.0 s); and the whole board is gone by tick 360
  (6.0 s) - the pre-existing `Anim Transform` teleport-out this page's own
  master-timeline section already documents, untouched by this lane. **A
  control render with every material's curve disabled** (`anim` forced to
  `0`) shows the backdrop and the digits vanish together at every tick, not
  just the digits - ruling out a curve-independent node swap as the source
  of the red-to-green change, and pointing at the shared texture
  (`321_go_64.gtf`) instead. **A plausible but unmeasured reading**: this
  page's own "authored, unused" red/green marker columns are what the
  backdrop samples as the same offset that reveals the digits crosses them -
  the timing and colours fit, but nothing has sampled the backdrop's own UV
  cell against the texture to confirm it, the way this page's own digit
  windows are confirmed. Frames
  from both renders in `scratch/lane-wire-report.md`.

  **The per-node `+0xe4` static UV override table this page used to name as
  a competing glyph-selection candidate is chased and ruled out, live and
  separately from the above**: `scratch/gantry_dump_e4.py` reads the pointer
  at `+0xe4` for all 19 of slot 8's own instances on Talon's Junction, at
  load, +3 s and +6 s into a real countdown, and it is `NULL` at every point
  checked, including the digit board's own submesh - see
  [`billboards.md`](../ghidra/functions/ps3-hdfury-eu/billboards.md)'s own
  2026-09-17 section for the full account, the precise pointer-indirection
  layout the table actually has, and why this also explains the previous
  pass's two unexplained rest-state values as ordinary static material
  parameters rather than this table's doing. This table plays no role in the
  countdown; the curves above do the whole job on their own.

  **What is not settled**: `3` and `2` reveal together rather than strictly
  one after the other, and the full sequence takes about 4 s of the asset's
  own clock rather than the ~6 s the measured thrust gate does - both read
  exactly as the disc authors them, not smoothed into a tidier story. Only
  one circuit and one boot were checked.
- **The backing pieces sit inside the mount's own thick structure.**
  `CLEARANCE` (1.0, unchanged, still Pulse's own measured order) clears the
  digit board's near-zero local depth but not `Honey_Board`'s HD analogue,
  authored several units further back in the model's own local space - a
  larger clearance was tried experimentally and rejected, both because it
  is fitted to an HD rendering question this lane cannot close (whether the
  board is ever lit at rest) and because it visually buried the boost-gate
  structure the reference frame shows the board sitting *inside*, not in
  front of. Left as Pulse's own measured value; a future pass with the
  material-curve lane's own shader-variant work landed may find this stops
  mattering once the board itself paints correctly.
- **Only four circuits are measured by hand**; the ground-truth test covers
  Talon's Junction alone, per `just check-test-budget`'s own ratchet. The
  other twelve HD circuits (and their reversed variants) are unswept.
- **Mode variants** (`321Go_Zone`, `321Go_HD_Zone_Battle`,
  `321go_hd_detonator`) are untouched by this pass, per the standing
  instruction to implement the plain race first - see
  ["Which model a mode selects"](#which-model-a-mode-selects-a-substitution-site-exists-at-slot-7---not-slot-8)
  above.

## Implemented on PS2: the identical asset, under an external skin

**Reported from play by the user, 2026-09-17**: "the gantry is not
implemented/rendered/animated" on PS2 Pulse, while PSP Pulse and HD/Fury have
it. Read on `pulse-ps2-eu.chd` (`SCES-54748`), against `pulse-psp-eu.chd` at
the same ticks with the same team, this session (2026-09-25).

### Authored identically: same name, same manifest slot, same mount

**Confidence 92.** `oag-wad cat` against `WADS2.WAD` finds
`Data\Environments\321_Go\321Go_StartFinish.vex` at the same path PSP uses -
74,944 bytes, against PSP's 54,032, a different file rather than a missing
one - and `TrackStartup.xml` for `16_Track` (Talon's Junction) authors the
identical `<Billboard num="8">` line naming it, byte-for-byte the same
manifest schema. `oag_render::gantry::mount` finds the identical
`321backplate.tga`/`billboard8.tga` surface `crate::race::gantry` already
measures for PSP - node 76 on this disc against PSP's node 74 (the node
numbering shifts because the PS2 track file's own node list differs
elsewhere; the surface itself is the same shape, measured the same way).
**Nothing about placement is PS2-specific**, so `oag_raceplay::gantry`
already stood the model on its mount correctly, before any fix in this
section - confirmed with `--dry-run`, which reported `start gantry ... on
node Some(76)` and no `no start gantry` line even on the unmodified tree.

### What was actually missing: the palette, not the placement

**Confidence 95, measured directly.** A `--race --ticks 170 --screenshot`
capture on the unmodified tree shows the gantry board present, correctly
placed, correctly clipped to its panel - and blank: a near-uniform pale cream
slab with, at most, a faint double-exposure ghost of the glyph geometry
underneath it, at every tick checked. That is the same picture a PS2
`.vex`'s empty texture block always produces when nothing resolves it - see
`docs/formats/ps2-texture.md` and the identical symptom already fixed once
for the hull (`livery::one`), the plume (`livery::plume`) and the shield
(`livery::shield_model`) - and `crate::race::gantry::build` was the one
loader in this family that had not taken the same branch: it built with
`mesh::build_with_textures(name, blob, None)` unconditionally, so
`321Go_StartFinish.vex`'s six `Texture` nodes always bound nothing.

The fix is the same directory-position rule applied a fourth time:
`crate::race::gantry::ps2_skin`, called when every texture slot the file
declares comes back `None` (the PS2 signature; a PSP or HD gantry has its
textures embedded or `.rcsmodel`-resolved and never enters the branch),
resolves `mesh::Ps2TextureSet::parse(&archives.read_preceding(candidate))`
and rebuilds with it. The load report now reads `6 of 6 texture(s) from the
preceding archive entry, into 6 slot(s)` - every one of the model's declared
textures, not a partial match.

### The countdown reads pixel-identical to PSP's, phase for phase

**Confidence 90.** `--race --track Data\Environments\16_Track\track.vex
--team Assegai --ticks N --no-audio --screenshot`, both discs, `N` in `{30,
66, 111, 156, 260}` (before `3` lights; on `3`; on `2`; on `1`; mid-`GO`).
Every pair is the same picture: the same glyph lit white against the same
dark-red neighbours at the same tick, the same green `GO` banner at tick 260.
Nothing was tuned to make this true - the fix only supplies the pixels the
file already authors; the timing was never in question, since the asset's
own `TEXOFFSET` track is read once by `oag_render::TexAnims` regardless of
where its texture came from. `ps2_start_gantry_ground_truth.rs` pins the
three digit phases (1.10 s / 1.85 s / 2.60 s) directly against the decoded
external palette, the same assertions
`start_gantry_ground_truth.rs::one_glyph_is_lit_at_each_phase_in_order` makes
against PSP's embedded one.

### A real content difference from PSP, found while writing the test

**Confidence 90, off `oag-view --nodes` on both discs directly.** PSP's
glyph node, `Jons321go:Jons321go:start_light_321goShape`, is one `Mesh`
carrying all four UV cells this page's own mechanism section documents. **The
PS2 disc splits that one node into two**: `start_light_321Shape` (three UV
cells, `3`/`2`/`1` only) and a separate `Anim Transform`/`Mesh` pair named
`Go`/`GoShape` (8,000 bytes) - ten `Mesh` nodes on this disc against PSP's
nine. Both still bind the same six-texture material set, and the `GO`
capture above shows the split node plays correctly: the renderer walks every
node's own authored `Anim Transform` and texture-transform track generically
(`oag_render`'s `TexAnims`/node-anim tables), so a second node carrying its
own track is not a special case - it is what the generic path was already
built for. `GoShape`'s own track is not separately pinned by the ground-truth
test, which is scoped to the three-digit node; the matched screenshot pair at
tick 260 is the evidence for the word itself.

**Also different: the palette's own resolution.** PSP's `321go_NOMIP.tga` is
16x32; the PS2 disc's copy (same path,
`Data\Environments\GenericTrackTextures\startlights\321go_NOMIP.tga`) is
32x32 - the same three-row staircase, re-authored at double the column width
(four texture columns per digit instead of two), not a scaled copy of the
same bitmap. Read with `examples/ps2_gantry_palette_probe.rs`, kept in the
tree for the next texture that needs eyeballing rather than asserting on
faith. Neither the shader nor `oag_render::TexAnims` cares about a texture's
absolute size - sampling wraps against whatever `width`/`height` the decoded
texture reports - so this needed recording rather than fixing.

### What is still open

- **Only Talon's Junction is checked.** The mount, the manifest and the skin
  are all measured off one circuit; the placement mechanism is shared with
  PSP's own eleven-circuit path so a per-circuit regression is unlikely, but
  no PS2 circuit past `16_Track` has had its own screenshot taken.
- **`GoShape`'s own UV cell and track are not asserted**, only observed to
  render correctly in a capture - see above.
- **Modes other than the plain race are untouched**, per the standing
  instruction the HD section above already states for its own mode variants.

## The board's background is black on the original and transparent here

**Reported from play by the maintainer, 2026-09-07, on `pulse-psp-eu` - not
fixed, deliberately, and recorded so it is not rediscovered.**

> *"the gantry does have a black background, while ours is transparent"*

Visible in the side-by-side capture taken the same day
(`scratch/gantry-compare/side-by-side-countdown.png`, ours left, original
right): the original's countdown board is an opaque dark panel that the
digits sit on, so the arch reads as a screen. Ours lets the environment
through behind the glyphs, which is why the board picks up the red-brown of
the structure behind it and reads as a tint rather than a display.

**This is not the placeholder bug and must not be confused with it.** The
8x8 `billboard1..8.tga` stubs were being drawn as a white blur and are now
suppressed (`oag_render::gantry::strip_slot_placeholders`); that was
confirmed correct against the original, which shows nothing in those slots
either. The background is a separate question about the board's *own*
surface.

**Do not paint it black to make it match.** The rule that governed the
placeholder fix governs this one: suppressing an unbound slot is faithful,
choosing a colour is an invention. Two candidates are worth checking before
anything is authored:

1. **`321backplate.tga`'s own alpha.** The arch attaches to this surface, and
   if the texture authors an opaque dark backing that we are drawing with the
   wrong blend mode - or sampling alpha from the palette byte rather than the
   separate alpha channel - the black is already in the file and we are
   discarding it. `oag_fx::psys` had exactly this class of bug in August,
   where a whitening term overrode an emitter's own colour table.
2. **The `<Mode3D>` widget's blend state.** The countdown glyphs are geometry
   drawn through the perspective dialect; if the backing quad is drawn
   `AlphaOver` where the original uses an opaque pass, the difference is in
   the blend class rather than the asset.

Neither has been checked. Whoever picks this up should parse the backplate
first and only then look at the blend state - the cheaper answer is that the
asset already says what to draw.
