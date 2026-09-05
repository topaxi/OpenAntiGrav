# The start gantry: how a `3`, `2`, `1`, `GO` is drawn

The object standing over the start line, not the HUD graphic drawn on the
screen. Two different things have both been called "the countdown display" in
this tree; [`ui/hud.md`](../ui/hud.md) covers the on-screen `<Mode3D>` overlay
(`Pulse_Ready_Go` / `Cockpit_321GO`, identical across every mode on every title
checked), and this page covers the physical gantry, which arrives through
`TrackStartup.xml`'s **billboard slot 8** ([`formats/README.md`](../formats/README.md)'s
Track startup row).

**Read on Wipeout Pulse (`pulse-psp-usa.chd`, `UCUS-98712`), Wipeout Pure
(`pure-psp-usa.chd`, `UCUS-98612`), Wipeout HD/Fury (`hdfury-ps3-eu-dec.iso`)
and Wipeout 2048 (`data/extracted/vita/PCSF00007`, the decrypted EU Vita
package).** The closing section says what looks title-wide across all four and
what is per-title. Pure ships no track-side gantry at all; HD ships four
files, and its own section is where the packaging question the Pulse pass
left open gets an answer; 2048 ships those same four plus four of its own
(three real glyph files and one empty stub), and turns out to author a third
countdown mechanism entirely - a manifest reaches only two of the eight.

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
`crates/formats/tests/start_gantry_pure_ground_truth.rs` reproduces the count:
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
- `oag_formats::rcsmodel` is where the real vertices, indices, texture
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

`321_go_64.gtf` decodes (`oag_formats::gtf`, format `Dxt45`) to **64x128** -
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
file.** `oag_formats::rcsmodel::material::parameters` decodes a material
record's parameter table as a name-hashed array of `{value: [f32; 4], quads:
u32}` entries with no time axis at all - a `quads` above one would be a longer
vector the shader indexes, not a sequence of keys, and every parameter this
material carries reports `quads == 1`. Feeding this exact node's payload
through `oag_formats::vex::mesh_tex_transforms` - the same parser that
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
own HD section places the gantry either; the loader report has nothing to
name yet because nothing here loads `321go_startfinish.vex` at all.

## Wipeout 2048: eight files plus `fx350`, two reached by any of 26 manifests, a third countdown mechanism

**Read on `data/extracted/vita/PCSF00007`** (the decrypted EU package: base,
the 1.04 patch, and both DLC packages), with
`crates/formats/tests/start_gantry_2048_ground_truth.rs` (9 tests) and
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
`oag_formats::rcsmodel::psp2`'s container (`0 unpaired_pointers` on all eight)
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
all** - `oag_formats::rcsmodel::psp2`'s own module doc states this plainly:
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
