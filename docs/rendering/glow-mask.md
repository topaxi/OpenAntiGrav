# Pulse's glow mask: what the bloom actually reads, measured

The bloom's four passes are recovered on
[`bloom.md`](../ghidra/functions/psp-pulse-usa/bloom.md) and ported in
`oag_render::post::bloom`, every constant read. What was not known until
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
(`Gfx_BuildBatchStateList(0x282)`). The LeachBeam ribbon stamps `0x28` on
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
  `oag_render::mesh::glow`. Its blended pipeline writes colour only, which is
  rule 3 and takes the plume and the shield out of the mask.
- **The `_GLOW` decals draw.** They are cutouts on the `0x10` reference,
  coplanar with the wall they light, and `Gfx_BuildBatchStateList` gives
  that reference a `GEQUAL` depth test. Ours tested `Less`, so every one tied
  with its wall and was discarded: the neon strips, the lit panels and the
  banner letters drew neither their colour nor their mask. The cutout
  pipeline for that reference now tests `LessEqual`
  (`mesh_render::cutout::depth_compare`). On a Pure and a PS2 start frame the
  change moves no pixel.
- **The bloom is on by default** (`Graphics::bloom`) and runs only over a
  stamped mask, so a Pure or PS2 race draws none.
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
  `4` under it. The race draws the bloom before it composites the HUD, and a
  crop of the countdown widget shows no halo with the bloom on.
- **Resolved 2026-09-23: the absorb overlay's mask was patchy because we
  drew both `LodGroup` tiers.** Read out of EDRAM 0.42 s into a live absorb,
  the original stamps `255` over the whole overlaid hull, with holes only at
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
| `0xff` | 265 | the opaque `_GLOW` decals and the shine passes' REPLACE draws, as before |

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
colour factors read the texel's own, so `oag_render::mesh_render::stamp` builds
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
| mask pixels `>= 100`, rows 45 to 195 | 286 | 5,739 | 6,066 |
| mean abs mask difference, rows 45 to 195 | 17.9 | 9.25 | - |
| mean abs RGB difference over the original's mask `>= 100` | 65.7 | 44.1 | - |

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
and reads `4` where the original is set on 119. So the laser is about 1.5 x too wide
here: the colour test as placed is not the whole of the original's edge. (The
`0xaf`-on-20-pixels row of the Talon table above was a frame after the laser had
mostly gone; the laser is animated, so one tick of ours and one of the original agree
on neither width nor timing.)

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
so there was no glow around them, and the strip itself is **animated**
(`07_Tunnel_Light_Glow`'s texture transform, a period of about 300 ticks on
`col_display7_GLOW`). With the stamp, a frame of ours at a bright phase has
1,584 pixels with green and blue over 225 in the strip's box (rows 40 to 135,
columns 190 to 290) against the original's 1,581, where it had 186 at every
phase before. The first pose's
frame is at a bright phase of the original's clock, which is why one frame of
ours read dull: time, not state.
