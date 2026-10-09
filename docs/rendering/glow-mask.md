# Pulse's glow mask: what the bloom actually reads, measured

The bloom's four passes are recovered on
[`bloom.md`](../ghidra/functions/psp-pulse-usa/bloom.md) and ported in
`oag_post::bloom`, every constant read. What was not known until
2026-09-23 is what the framebuffer's alpha channel holds when the bright pass
reads it, and whether the pass runs at all. This page is that measurement,
and the rules it gives for writing the mask.

## The bloom runs every race frame

A logged, non-halting read watchpoint on `g_bloom_scratch_a` (`0x08a84cf8`),
whose only per-frame reader is `Bloom_Draw` (its list builders read it once,
at construction), counted **121** reads at each of `0x08907630` and
`0x08907724` in two seconds of a single race. The positive control, a write
watchpoint on the player's clock `craft+0x830`, counted **121**. So
`Bloom_Draw` runs once per race frame, and there is no switch: the
constructor is called unconditionally from `FUN_0888adcc`, on
`Game_MainLoop`'s path. Confidence 90.

## Alpha is the stencil, and blending never writes it

The GE keeps the stencil in the framebuffer's alpha. PPSSPP's software
rasteriser (`GPU/Software/DrawPixel.cpp`, `DrawSinglePixel`) writes
`new_color |= stencil << 24` on both the blended and the unblended path: the
blend equation produces RGB only, and the alpha byte is the stencil after the
stencil op. With the stencil test off, the stencil is the old value, so
alpha is kept. The hardware backends say the same in words
(`GPU/Common/GPUStateUtils.cpp`, `ConvertBlendState`): "we shouldn't do any
blending in the alpha channel as that doesn't seem to happen on PSP", and
"Retain the existing value when stencil testing is off."

So on the original a surface reaches the mask **only through a stencil op**,
and what it writes is the stencil reference, not its own alpha. That settles
the question `Graphics::bloom`'s doc comment left open: an additive
`SrcAlpha`/`One` alpha blend accumulates a mask the original never
accumulates.

## The mask, read out of EDRAM

PPSSPP's software renderer writes emulated EDRAM, so `memory.read` at
`0x04000000` returns the real framebuffers. Talon's Junction, single race,
Assegai stationary on the grid after GO, the displayed buffer (`fb1`, 480 x
272 at stride 512):

| Alpha | Pixels | Where |
| ---: | ---: | --- |
| 4 | 125,008 | everything else, sky included |
| 92 | 2,028 | the lit panels overhead, `walls002_sb_GLOW` |
| 254 | 1,680 | the `nuricom` banner, `hub_banner_GLOW` |
| 255 | 852 | the neon strips along the track edges and wall lights |
| 0 | 830 | behind the nozzle |
| 247 | 126 | the hull's blink lights, `colours_flashing_GLOW` |
| 175 | 20 | the start-line laser, `startline_laser_ADD_GLOW` |
| 63 | 16 | an advert, `Harimau2_Glow` |

The frames are `~/.cache/oag/drive/reports/pulse-bloom/vram-grid-fb1-*.png`.
The buffer being drawn (`fb0`) had only the track in it and read **4** on
every one of its 130,560 pixels, so the frame clear leaves 4 too.

## The three rules that reproduce it

Read in `Gfx_BuildBatchStateList` (`0x0891f890`, the per-batch state list)
and `FUN_089307b4` (the batch set's recorder), and each checked against the
table above:

1. **An opaque or alpha-tested batch stamps `g_display+0x1178`.** For
   `pass_mask & 0x700 == 0` without `& 0xc0` the state list sets
   `StencilFunc(ALWAYS, *(g_display + 0x1178), 0xff)`,
   `StencilOp(KEEP, KEEP, REPLACE)` and enables the test. The byte read live
   is **4**. Its writer is not read. Confidence 85.
2. **A batch with `pass_mask & 0xc0` stamps its texture's glow byte.** The
   state list sets ref `0xff` and REPLACE for it whatever its blend class,
   and `FUN_089307b4` then sets the ref to `*(texture + 0x9c) + 0x1e`, which
   is **byte `+0x1e` of the `Texture` node's own payload**. Every value in the
   table matches that byte: `walls002_sb_GLOW` `0x5c`, `hub_banner_GLOW`
   `0xfe`, `rp_tunnel_lights2_GLOW` `0xff`, `colours_flashing_GLOW` `0xf7`,
   `startline_laser_ADD_GLOW` `0xaf`, `Harimau2_Glow` `0x3f`. The laser's two
   batches are transparent, so a transparent `0x80` batch stamps too.
   Confidence 88.
3. **A transparent batch without `0xc0` leaves the mask alone.** The state
   list disables the stencil test for it, and a disabled test keeps alpha.
   Confidence 85.

On `16_Track`, `pass_mask & 0x80` is set on 214 of 2,079 batches over 21
textures, and every one of the 21 is named `_GLOW`; no batch sets `0x40`
(`crates/vex/examples/light_rig_probe.rs`). On Assegai's hull it is one of
14 batches, the blink lights. The 2026-08-10 census on `bloom.md` counted
`0x40` alone and found none, which is why the mask looked unused.

Three effect writers sit outside the mesh path. The exhaust ribbon stamps its
ramp (`Trail_BuildStateList`, REPLACE), which is the 0 behind an idle
nozzle. Both hull overlays, absorb and LeachBeam, stamp `0xff`
(`Gfx_BuildBatchStateList(0x282)`) - **and the shadow pass then wipes all of it
but the glow batch's own; see "The hull overlay's mask is wiped" below**. The LeachBeam ribbon stamps `0x28` on
every fragment, transparent texels included (`LeachBeam_SubmitStrip`, REPLACE
with the alpha test off). That last one is read statically, not measured out
of EDRAM - see
[cannon-quake-leachbeam.md](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md).
The PSP boost plume's batches are `0x1232`: transparent, no `0xc0`, so the
plume **does not** write the mask.

## What this changes in the port

2026-09-23, on Pulse off a PSP disc only (`race::load::pulse_psp`):

- **`mesh_render::GlowMask::Stamped`.** Every `.vex` model the race draws -
  the track, the sky, the pads, the hulls, the plumes and the shields - is
  `Model::stamps_glow`, and its opaque and cutout pipelines write
  `GpuVertex::glow` into alpha: rules 1 and 2 above, per batch, through
  `oag_mesh::mesh::glow`. Its blended pipeline writes colour only, which is
  rule 3 and takes the plume and the shield out of the mask.
- **The `_GLOW` decals draw.** They are cutouts on the `0x10` reference,
  coplanar with the wall they light, and `Gfx_BuildBatchStateList` gives
  that reference a `GEQUAL` depth test. Ours tested `Less`, so every one tied
  with its wall and was discarded: the neon strips, the lit panels and the
  banner letters drew neither their colour nor their mask. The cutout
  pipeline for that reference now tests `LessEqual`
  (`mesh_render::cutout::depth_compare`). On a Pure and a PS2 start frame the
  change moves no pixel.
- **The bloom always runs** (no setting since 2026-10-02: the original
  offers none) and only over a stamped mask, so a Pure or PS2 race draws none.
- `OAG_DUMP_GLOW_MASK=<png>` writes a `--screenshot`'s mask for this
  comparison. Talon's Junction on the grid: ours reads `4` over 107,644 of
  130,560 pixels at 480 x 272, with the strips, the banner, the panels and
  the blink lights where the original has them
  (`~/.cache/oag/drive/reports/pulse-bloom/mask-orig-vs-ours.png`).

What is not reproduced:

- ~~**Transparent batches with the glow bits do not stamp**~~ **Closed
  2026-10-01** - the frame measured on Talon's Junction had 20 such pixels, but
  on Outpost 7's grid they are the whole tunnel; see "Transparent batches
  stamp" below.
- **The HUD writes alpha into our target**, where the original's mask reads
  `4` under it. It does not reach the bloom: our bright pass reads the scene
  before the HUD is drawn. ~~A crop of the countdown widget shows no halo~~
  **Corrected 2026-10-04**: the original's composite runs after the HUD, and
  its haze lands on the HUD's glyphs. Ours now adds it after the HUD too. See
  [bloom.md](../ghidra/functions/psp-pulse-usa/bloom.md), "The bloom draws
  over the HUD".
- **Resolved 2026-09-23, and then superseded 2026-10-01 (see "The hull
  overlay's mask is wiped"): the absorb overlay's mask was patchy because we
  drew both `LodGroup` tiers.** Read out of EDRAM 0.42 s into a live absorb
  **by halting the emulator at an arbitrary moment, which caught the buffer
  mid-draw**, the original stamps `255` over the whole overlaid hull, with holes only at
  the canopy and the rear, and its colour at that moment is still almost the
  plain hull: the white blob a player sees is the bloom of that mask. Under
  the then-default `lod = "both"` ours also drew the hull's `lodShape`,
  which the overlay leaves out - likely, not measured, its coarser triangles
  sat in front of `shipShape`, failed the overlay's `LessEqual` and left
  whole triangles unstamped - and it drew a dark ring round the rear hull
  the original has no trace of. Drawing tier 0 alone made the mask solid,
  took the ring away and gave the original's white blob
  (`~/.cache/oag/drive/reports/pulse-bloom/lod-both-vs-single-absorb.png`,
  `compare-absorb-bloom-lod-single.png`). The original draws only tier 0 up
  close (`docs/formats/vex.md`, "the running original does not draw tier 1
  up close"), and since the same day the tiers switch per frame the way
  `LodGroup_SelectChild` does, which keeps the player's hull on tier 0 at
  every chase distance - "implemented - the switch runs every frame", same
  page. The old setting and its both-tiers view are gone.

## The hull overlay's mask is wiped (2026-10-01, `pulse-hull-bloom`)

The question: ours bloomed into a white blob around a craft in its absorb window
and PPSSPP's software renderer does not. The bloom arithmetic is right (above),
so the suspect was the overlay's `0xff` stamp.

**Method.** Own PPSSPP on the **software renderer** (EDRAM is real), Talon's
Junction, Venom, Assegai, Single Race, the craft stationary on the grid after GO
(`(-132.30, -49.58, -175.32)`, `psp-drive.py restart`). `scripts/psp-absorb-frames.py`
grants the pickup (`weapon record +0x1bc = 0`, written at a `Weapons_DispatchFire`
breakpoint) and holds circle, then at **every fourth frame boundary** of the
one-second window reads `0x04000000`, `0x04088000` (both framebuffers, alpha =
the stencil) and `0x04110000` (the bloom's final layer), and once records the GE
list of a frame inside the window (`gpu.record.dump`, breakpoint removed first).
Two boots; the second reproduced every structural number below.

**What the original's completed frames read.** The hull holds the neutral `4`
through the whole window, in both buffers, all 16 frames of both boots, though
its RGB brightens (hull mean `87/95/92` at age 0 to `128/156/165` at 0.53 s). The
mask pixels that change between age 0 and the peak, inside the box rows 110 to
255, columns 150 to 290: **95 and 123** on two boots, a few small patches at the
hull's lights, against **about 4,000** if the whole overlay stamped (what ours
did). Inside the hull's neighbourhood the bloom layer's mean is `5.1/5.7/4.8/6.0/5.3`
(original, boot 1) and `5.4/5.2/4.6/5.9/4.8` (boot 2) against `4.4/5.7/5.2/5.7/4.7`
(ours after the fix) at ages 0.13/0.33/0.53/0.73/0.93, and `35.0/33.6/28.8/33.5/33.1` before it.

**Why: the draw order, read off the GE list** (the same frame, both boots; prim
numbers of boot 1, boot 2 differs by a constant):

| Prims | What | Stencil state (`0xDC` func/ref/mask, `0xDD` ops) |
| --- | --- | --- |
| 48 to 62 | the hull's ordinary batches | `ALWAYS`, ref `4`, `REPLACE` on pass |
| **63 to 72** | **the absorb overlay**, ten batches (425, 168, 29, 29, 150, 47, 123, 5, 5, 133 vertices): `blend 0xa2`, depth `EQUAL` | `ALWAYS`, ref **`0xff`**, `REPLACE` on pass |
| 326 to 329 | the craft's drop-shadow volume, colour masked off | ref `0`, `INCR`/`DECR` on **z-fail** |
| **330** | **a full-screen quad**, 4 vertices, depth test off | `GREATER`, ref `4`, mask `0xff`, **`REPLACE` on stencil-fail, z-fail and pass** |
| 331 onward | the track's glow batches, the weapon bodies, the blink-light batch | their own texture bytes (`0xf7`, ...) |
| **387 to 389** | **`colours_flashing_GLOW` (75 vertices), its ordinary pass, and its overlay (75 vertices)** | ref `0xf7`, ref `4`, ref **`0xff`** |

The pixel mask (`0xE8`/`0xE9`) is open on prim 330, so its `REPLACE` reaches the
framebuffer's alpha. **The quad rewrites every pixel's mask to `4`**, and it
runs after the overlay's ten ordinary batches. It exists in a frame with no absorb
(a control dump has the same shadow volumes and the same quad), so it is the
frame's own structure and not something the absorb causes. What is drawn after
it keeps its stamp: the track's glow batches, which is why the neon strips
survive, and the one hull batch with the glow bits and **its own overlay** (prim
389, the only overlay draw after the reset), which is why a few patches of the
hull do change. Confidence **88**: the GE words, the pixel counts and the
bloom layer agree on two boots; one hull (Assegai) only.

(Decoding: `0xDC` is `func | ref << 8 | mask << 16`, `0xDD` is `sfail | zfail << 8
| zpass << 16`, with the enum `KEEP 0, ZERO 1, REPLACE 2, INVERT 3, INCR 4, DECR
5` and `NEVER 0, ALWAYS 1, EQUAL 2, NOTEQUAL 3, LESS 4, LEQUAL 5, GREATER 6,
GEQUAL 7`; a texture address differs between boots, so the overlay is found by
its state, not by its texture.)

**Why the 2026-09-23 reading said otherwise.** That probe halted the emulator
at an arbitrary time and read the buffer. A halt between the overlay's draws and
prim 330 leaves exactly the overlaid hull at `255` in the buffer being drawn. The
same read in this lane's boot 1 caught it: halts at 0.58 s and 0.68 s showed
2,822 and 3,054 hull pixels at `255` in one buffer and 4 in the other, while
every frame-boundary read showed 4 in both. The rule for any later read of
EDRAM: **read at a frame boundary (a breakpoint in a once-a-frame function) and in
both buffers.**

**The hardware backend does not do this.** The same absorb on PPSSPP's OpenGL
backend (**one boot, seen once**, same pose, 16 frames) draws the hull white-hot, a blob wider than the
silhouette, as the 2026-09-23 screenshots did and as ours did before the fix
(`compare-sw-hw-ours-before-after.png`, in that order: software PPSSPP, OpenGL PPSSPP, ours before, ours after). A plausible cause,
**not verified**: an alpha-as-stencil emulation can write the stencil only where a
fragment passes the test, and prim 330's reset is a `REPLACE` on stencil *fail*. The
software renderer (`DrawPixel.cpp`) runs every stencil outcome on every pixel,
which is what the PSP's raster does. This page treats the software renderer as the
reference, as it does for every EDRAM figure above, and the white blob a player sees
on PPSSPP's OpenGL backend as that backend's approximation. No PSP hardware was available to arbitrate; that is the one thing that would. **The maintainer confirmed the software renderer as the reference on 2026-10-01**: PPSSPP comparisons of anything that depends on the stencil-held glow mask use the software renderer, and the OpenGL backend's look is not a target.

**What ours does now.** `hull_overlay::stamps_mask`: the overlay writes `0xff`
only over a batch with a glow of its own (`GpuVertex::glow` above the neutral
`4`), and leaves the mask alone elsewhere; its colour is unchanged (a rendered
no-bloom frame moves by at most one level). That is the rule's second half,
"what is drawn after the reset keeps its stamp", read off one hull. **Chosen, not
measured:** it is keyed on the batch having the glow bits, which on Assegai
singles out the one batch the dump shows after the reset; whether that holds for
the other seven teams, and what orders the batches around the shadow pass, is
not read. At the original's grid pose (ours via `--pose-from`, `--camera-fov 60`),
at ages 0.13 to 0.93 s, the bloom mean in the hull's neighbourhood is `4.4 / 5.7 /
5.2 / 5.7 / 4.7` against the original's `5.1 / 5.7 / 4.8 / 6.0 / 5.3` (boot 1) and `5.4 / 5.2 / 4.6 / 5.9 / 4.8` (boot 2), the mask
pixels at `255` match to within about 8 %, and the frames read alike. `crates/game/tests/absorb_mask_ground_truth.rs` pins it
on the disc: the full-glow pixels in the hull's box grow by **232 - 119 = 113** at the
peak against 4,189 - 119 without the fix, and the original's 95 and 123.

**Not done:** the LeachBeam overlay is the same routine (`HullOverlay_Submit`) and
takes the same rule here, but no frame of it was read; the shadow pass's reset
quad itself is not modelled (ours has no stencil shadow volumes, so nothing else
that draws before it is wiped either); the HUD's white energy-bar flash stamps
the original's mask (the bar block in the diff), which ours does not.

## Transparent batches stamp (2026-10-01, Outpost 7)

A PPSSPP running the original, **software renderer** (so EDRAM is the real
framebuffer; the hardware backend leaves it zero), on Outpost 7's Black start
grid (`track_reversed.vex`, craft at rest at `(-102.6, -0.90, 330.9)`), with a
GE dump of the same frame. `0x04000000` is the displayed buffer, alpha being the
stencil, and `0x04110000` (240 x 136, stride 256) is the bloom's final blurred
layer. Counts over the 480 x 272 frame:

| Mask value | Pixels | What stamps it |
| ---: | ---: | --- |
| `0xfa` | 4,076 | the tunnel's arch lights: `07_Pulse_light_BLEND_GLOW`, **alpha-over batches**, `Gu_BlendFunc` `0x32` (`07`'s own `track.vex` authors these as `pass_mask` `0x1192`) |
| `0xaf` | 1,469 | the start-line laser: `startline_laser_ADD_GLOW`, an **additive** batch, blend `0xa2` with the colour test on |
| `0x8b` | 221 | a second additive light batch (`blend 0xa2`) |
| `0xff` | 265 | the opaque `_GLOW` decals, as before |

The GE state of those blended draws, read off the dump (prims 574 to 601):
`STENCILTEST` on, `ALWAYS`, **ref = the texture's glow byte**, `STENCILOP` zpass
`REPLACE`, depth test `GREATER` (the PSP's `Less`) with depth **write off**, the
alpha test `GREATER 0`, and the blend equation untouched. That is rule 2 above
holding for a blended batch exactly as for an opaque one - the state list sets
the stencil from `pass_mask & 0xc0` whatever the blend class - and rule 3
(a blended batch **without** the bits leaves the mask) is the same dump's other
half: the transparent-class batches that carry no glow bits - the light shafts,
`pass_mask` `0x1222`, grid prims 705 to 737 - read `STENCILTEST` off. (The
chrome-map extra pass and its EQUAL ordinary pass are blended too, with the
stencil **on** and ref `4`; they are not transparent-class batches, and the
second re-stamps the base over the first's ref.) Confidence **90**: two
batch classes, both reads agreeing with the texture bytes `texture_bytes`
already extracts (`0xfa`, `0xaf`; `col_arrows1_GLOW_ADD`'s `0x8d` is stamped by a draw in the dump that no pixel of this frame shows).

The reason the frame matters: the bloom reads `rgb * alpha`, so a surface that
does not reach the mask does not glow at all. Before this change ours held `4`
there and the arches, the laser and the tunnel's rim light drew with no glow.

**How ours stamps it - chosen, not measured.** The original writes the stencil
inside the blended draw. A blend state cannot write a constant alpha while its
colour factors read the texel's own, so `oag_mesh::mesh_render::stamp` builds
a second pipeline - colour write mask off, alpha only, no blend, the batch's own
alpha test and the blended pipelines' depth state - and
`race::Drawable::draw_stamps` submits every visible transparent draw through it
after their colour. `mesh.wgsl`'s `fs_main_stamp` discards a fragment whose
vertex glow is `0` (a batch without the bits) or that fails the alpha test. It
reuses `GpuVertex::glow`, which already carried the byte for transparent
batches. **The additive class also runs the GE's colour test** (`NOTEQUAL`
against black; `Gfx_BuildBatchStateList`'s `0x200` branch, and on in every
additive glow draw of the dump, off in the alpha-over ones), so the additive
pipeline pair sets `stamp_colour_test` and a black texel stamps nothing. Where
the GE places that test relative to the fog is unread; ours tests the lit texel
before the fog. That test took the over-stamped pixels around the laser
(`0xaf`, original `4`) from 567 to 61 on the frame below.

Measured against the original's own frame at the same pose (`track_reversed.vex`
at `pose-from`, 480 x 272, the original's mask read out of EDRAM):

| | before | after | original |
| --- | ---: | ---: | ---: |
| mask pixels `>= 100`, rows 45 to 195 | 286 | 5,080 | 6,066 |
| mean abs mask difference, rows 45 to 195 | 17.9 | 8.12 | - |
| mean abs RGB difference over the original's mask `>= 100` | 65.7 | 43.8 | - |
| mean abs RGB difference, all of rows 45 to 195 | 19.73 | 17.48 | - |

Per pixel, rows 45 to 195 at that pose: ours is set where the original reads `4` on
721 pixels (649 of them `0xfa` inside the arches, which animate - ours at ticks 1, 60,
120, 200, 300 and 400 holds 3,153 to 368 pixels at `0xfa` against the original's 4,076,
so a fixed tick cannot be compared pixel for pixel) and reads `4` where the original is
set on 1,707.

**A cross-check on a different circuit**, Talon's Junction's second grid (craft at rest
at `(-130.05, -49.59, -175.34)`, `track_reversed.vex` - mean abs RGB difference to the
original 14.6 against 21.5 on `track.vex`, which is how the file was told). Rows 45 to
195, original / ours / ours without the stamp:

| Mask value | original | ours | without the pass |
| ---: | ---: | ---: | ---: |
| `0xaf` (the laser) | 710 | 1,061 | 0 |
| `0x8b` | 109 | 175 | 0 |
| `0xff` | 1,860 | 1,676 | - |

Ours is set where the original reads `4` on 388 pixels (244 of them the laser's `0xaf`)
and reads `4` where the original is set on 119. **That is a phase difference, not a
width.** Closed 2026-10-01 (`pulse-hull-pass`): the laser's stamped area cycles with the
race clock - ours at ticks 5 to 230 of that pose holds 588 to 1,390 pixels at `0xaf`, a
period of about 40 ticks (minima 606 and 599 at ticks 15 and 55) - and the single tick
the table above compared (tick 1, 1,061) sits mid-cycle. The original's own three
frames at that pose hold 592, 592 and 710 pixels (`shots6` ticks 0, 1, 2). At the ticks
of ours that match them, rows 45 to 195, pixel overlap (intersection over union):

| Original frame | Ours at tick | `0xaf`: original / ours / overlap | `0x8b` | `0xff` |
| --- | ---: | --- | --- | --- |
| tick 0 and 1 (592) | 16 | 592 / 588 / 0.94 | 173 / 209 / 0.79 | 1,822 / 1,784 / 0.96 |
| tick 0 and 1 (592) | 17 | 592 / 617 / 0.94 | 173 / 154 / 0.86 | 1,822 / 1,784 / 0.96 |
| tick 2 (710) | 19 | 710 / 744 / 0.94 | 109 / 123 / 0.86 | 1,860 / 1,783 / 0.94 |

So the stamp, the colour test as placed (on the lit texel) and the alpha test
reproduce the original's laser edge to about one pixel, and **no term widens it**. Two
things worth keeping: (1) the fog cannot move these pixels at this pose - the laser's
draws carry fog on, but `FOG1` is 1600 and `FOG2` `1/1540` in the dump and the laser is
tens of units from the eye, so the factor there is about zero; where the GE places the
colour test relative to the fog stays unread, and nothing here depends on it. (2) Three
experiments that did not move the Talon number to the original's and were reverted: the
colour test's black threshold swept to 4, 12 and 30 of 255 (it thins Outpost 7's
laser, which already agrees at 1,468 against 1,469, in proportion - a fit, not a
measurement), a stamp depth test of `Always` (1,053 against 1,061: the laser is not
depth-limited), and rounding the alpha test to the 8-bit result (no change). Outpost 7's
White grid at tick 1 reads 1,375 against the original's 1,264 (not phase-matched). The lesson for any comparison of the laser, the arch lights or the neon strip:
compare at a matched tick, never at one fixed tick of ours.

`crates/game/tests/glow_stamp_ground_truth.rs` pins it on the disc (arch lights
at `0xfa` and the laser at `0xaf`, zero of each without the pass); the
`oag-game` loader report says so.

## The bloom's own arithmetic is right (2026-10-01)

Checked because a bloom that is merely dim reads as a plausible glow. On the
Outpost 7 White grid (`(-122.4, -0.88, 188.3)`, craft at rest) a numpy model of
`Bloom_Draw` - 2 x 2 box downsample of `rgb * a`, the 11-tap kernel along y then
x with each pass clamped to `[0, 1]` like the GE's 8-bit target, a bilinear 2 x
upscale, `+ 0xaf / 255` of it - run on our scene reproduces the original's
bloom layer at the neon strip to about 6 % (`0.686 x layer`: model 16 / 11 / 9,
original 23 / 16 / 10 in red, the one channel that does not saturate there).
`crates/render/tests/bloom_gain.rs` runs the real shader against the same
arithmetic on a synthetic strip and agrees to a level or two. **A comparison
through `result - base` of a saturated pixel reads 0.37 of the truth** - the
first measurement here did exactly that on green and blue and briefly pointed
at a missing 2.7 x of gain; red, which stays under 255, is the honest channel.

## The tunnel rim's neon strip (2026-10-01)

The strip on `07_Track` that the original draws at `(134, 246, 248)` and ours
drew at `(50, 109, 115)` was this: the arch-light and rim batches did not stamp,
so there was no glow around them. **The strip's brightness also changes with the
race clock** - measured, not attributed: ours at ticks 1, 150, 300, 450, 600 and
900 holds 1,736, 510, 1,584, 583, 1,583 and 1,584 pixels with green and blue over 225
in the strip's box (rows 40 to 135, columns 190 to 290), a period of about 300
ticks. Which batch or texture animates is **unread**; on the Outpost 7 White
grid the animated surface was `col_display7_GLOW`'s own texture transform, and
nothing here shows the rim's is the same one. With the stamp, a bright-phase
frame of ours has 1,584 such pixels against the original's 1,581; before it had 186 at
every phase. The original frame sits at a bright phase of its own clock, which is
why one frame of ours read dull: the missing stamp, and a dim phase.

## The weapon bodies and the Bomb's dome stamp too (2026-10-01)

`pulse_psp::finish` listed what stamps the mask - the track, sky, pads, hulls, plumes and
shields - and the weapon bodies were not on it. They go through the same
`Gfx_BuildBatchStateList`, so they stamp, and a PPSSPP on the **software renderer** shows
it (EDRAM read at fire+3 to +7 of a Bomb dropped at speed 106 on Talon's Junction, beside a
no-fire control; `psp-weapon-pair.py --edram`):

| What | Original | Ours before | Ours now |
| --- | --- | --- | --- |
| mask `0xba` (186), the canister's lamp batch (`mine_flash_GLOW`), pixels at fire+5 to +7 | 1,006 to 1,326 (boot 1), 758 to 1,406 (boot 2) | 0 | 1,361 (one frame) |
| road pixels `255` -> `4` under the opaque canister, fire+5 / fire+6 | 756 / 2,965 (boot 1), 2,218 / 4,581 (boot 2) | 400 at the same frame | 3,357 |

So the canister's body writes the neutral `4` over the road's `255`, and its lamp writes its
own glow byte. Ours kept the road's `255` under the body, and the body bloomed as a pale
wash - the grey-green sheen on the canister that the original's saturated olive does not
have. `crates/game/tests/weapon_stamp_ground_truth.rs` pins both numbers (it reads 0 and
400 with the bodies off the list). Confidence **88** for the Bomb's body and lamp, from the
EDRAM read on two boots alone (the two values are what its batches' glow bytes and the neutral `4` would
write; the Bomb's own GE batches were not separated out of the dump); the
Rocket, Mine and Cannon bodies are on the list by the same generic state list, with the
Rocket's rockets seen turning 7 to 87 road pixels to `4` per frame and no frame of the
other two compared.

**The Bomb's blast dome stamps as well.** `explosion_hemisphere.vex`'s two batches are
`pass_mask 0x12b2` - transparent with the glow bit, like the tunnel's arch lights - and a GE
dump of a Bomb detonated 120 units ahead (one dump, one boot, so seen once) shows them (prims 527 and 528: 21 and 87
vertices, 37 units across) drawn additive, colour test `NOTEQUAL` black, depth write off,
stencil `REPLACE` reference `80`. Ours stamps through the second alpha-only pipeline this
page's "Transparent batches stamp" section built. The shockwave beside it
(`Bomb_Shockwave.vex`, `0x1232`) has no glow bit and stamps nothing. **The Rocket's flare
and its other particles do not**: the same EDRAM comparison with a Rocket launched shows
7 to 33 pixels of alpha differing from the control across the whole frame, the rockets'
own hulls, so a particle draw leaves the mask alone as `particle-system.md` reads.

### The ship explosion does not stamp either (2026-10-01)

`scripts/psp-wreck-capture.py --edram` (software renderer, a grid opponent put into `Ship_SetState(entity, 4)`,
one boot) wrote both EDRAM framebuffers at 100, 118, 121 to 124, 126, 130, 140, 150 and 160 frames after the
call. Against the frame at 100 (no explosion, the wreck smoking), the alpha bytes that differ inside the
explosion's screen region are `4` (the opaque wreck hull stamping its `g_display+0x1178` over the banner it
covers), `254` and `255` (that banner, `hub_banner_GLOW`, uncovered or scrolled), two or three pixels of `175`
(the start laser) and nothing else - no new value, and no blob of the size the fire covers (7,000 to 12,000
pixels). So `WO_SHIP_EXPLOSION`'s particles, like the Rocket's, leave the mask alone, and the orange band along the
horizon is the `Bomb_Shockwave.vex` ring ([ship-shockwave.md](../ghidra/functions/psp-pulse-usa/ship-shockwave.md)),
not a bloom of the fire. One boot, four frames read in detail: seen once.

## Pulse PS2 writes a different mask (2026-10-02)

Everything above is the PSP's: a stencil reference, a constant per batch, `4`
under every opaque batch. **The PS2 has none of those.** Read off replayed GS
dumps of Moa Therma, the PS2's frame alpha clears to `0`, opaque batches leave
it alone, and the groups that write it write `texel alpha * vertex colour alpha
>> 7`, a ramp, over `_GLOW` batches only - 96.7 to 98.7 % of a frame reads `0`
and 222 to 249 levels are in use. The measurement, the readout method and the
port are on [`ps2-bloom.md`](ps2-bloom.md): `GlowMask::StampedByTexel`, with the
loader's `pulse_ps2` marking the models, and the PSP's `Stamped` rule unchanged.
