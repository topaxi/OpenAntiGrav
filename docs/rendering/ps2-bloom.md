# Pulse PS2 does bloom: five passes per field, read off a GS dump, and ported

**Yes, the PS2 build runs a bloom post-process every race frame. Confidence
90** (rubric: measured directly, four independent frame instances, one
circuit, one image; not 95 because no second circuit and no static
corroboration in the executable). **Ported 2026-10-02**: `oag_render::post::ps2_bloom`
runs the chain and `mesh_render::GlowMask::StampedByTexel` writes the mask, see
"What ours does" below. It is the same shape as the PSP's
([`bloom.md`](../ghidra/functions/psp-pulse-usa/bloom.md)) with different
constants. The maintainer's hypothesis, that the PS2 build left it out to hold
frame rate, is **falsified**: the passes are drawn on the grid and while
racing. Whether it costs frame rate is not judged here.

## Method

PCSX2 v2.9.39 (the harness's `scripts/pcsx2-drive.py`, `GSDumpSingleFrame`
on F9, see [`pcsx2-debugger.md`](../reverse-engineering/pcsx2-debugger.md)),
`pulse-ps2-eu.chd`, an own datapath and display, muted. Moa Therma, Assegai,
from a savestate on the grid (Talon's Junction was not reached). Two dumps:
the grid after 30 verified frames, and racing after 240 frames holding thrust.
Each dump holds two fields, so four instances of the pass sequence. Listed
with `scripts/pcsx2-gsdump.py`'s decoder, grouped by `FRAME_1`'s `FBP`.

Falsifier written first: no draw targets an off-screen buffer that is later
sampled back into the frame with an additive blend. It came out the other way.

## The passes (race field 0, groups 290-298 of 595; identical on the grid)

Buffers are `FBP` pages of 2048 words; a texture's `TBP0` is 32 times the page
(`tbp 8960` = page 280, `tbp 11008` = page 344).

| # | Group | Target | What it draws | Blend |
| --- | --- | --- | --- | --- |
| 1 | 290-291 | page 280, 320x224 of 512 wide | a clear sprite, then 2 triangles sampling the **finished frame** (`tbp 0`, UV 0..512) onto 320x224 | `ALPHA 0x88`: `Cs * As`, `ATE` off. The source alpha is the frame's own alpha channel |
| 2 | 292-293 | page 344 | clear, then 896 triangles sampling page 280: 7 horizontal taps (dest x offset -3..+3), each a 16 x 4 grid of 20x56 strips | `ALPHA 0x68`, `FIX` 128: `Cs * FIX + Cd`; the weight is the vertex colour |
| 3 | 294-295 | page 280 | clear, then the same 896 triangles sampling page 344, vertical offsets -3..+3 | same |
| 4 | 296 | the frame (page 0, page 140 the other field) | 2 triangles, page 280 stretched 320x224 to 512x512 at -5,-5 (UV 5120 x 3584 in 1/16), bilinear | `ALPHA 0x68`, `FIX` **64**: `Cs * 0.5 + Cd`, additive, `FBMSK 0xff000000` so alpha is kept |
| 5 | 297-298 | both frames | a 512x512 sprite that fails its alpha test with `AFAIL` = ZB only | rewrites Z. `ZBUF`'s `ZBP` is 0x118 = page 280, the bloom buffer, so the Z buffer aliases it; chosen reading: that is why Z is cleared here. Not measured further |

Tap weights (vertex colour, over 128): **16, 32, 32, 64, 32, 32, 16**, the
same in both axes, 224/128 = 1.75 per axis. Compare the PSP's eleven taps
`20,30,40,50,64,64,64,50,40,30,20` at 240x136. The PS2's are seven at 320x224
and the add is half strength; the PSP adds with `GU_FIX 0xaf`.

**Cost as counted from the dump, not timed:** about 1.6 million pixels of GS
fill per field (71,680 downsample, 2 x 7 x 71,680 for the blurs, 2 x 131,072
clears, 262,144 composite), 1,800 triangles. Nothing here is a PS2 timing.

## The mask, measured: the frame's alpha channel (2026-10-02, confidence 88)

**How it was read.** A GS dump holds the VRAM as it was when the dump began and
the packets that follow, so the alpha of the frame *being drawn* is in neither:
the snapshot's page 0 reads `0x80` over 99.99 % of 512 x 512, which is not a
mask (the VRAM is the state's last 4 MiB, at `state_size - 4 MiB`). It is read by **replaying**: the dump is
cut at the end of the last group before the downsample and one full-frame sprite
is appended that blends white onto the frame buffer with `ALPHA = Cs * Ad`
(`A = Cs`, `B = 0`, `C = Ad`, `D = 0`), colour writes on and `FBMSK` `0xff000000`.
The replay's own screenshot is then a picture of the alpha: grey = `2 * alpha`.
`scripts/pcsx2-gsdump-alpha.py` builds that dump. Replayed on PCSX2's **OpenGL
hardware renderer** (the software renderer needs a Vulkan present queue Xvfb does
not give, lavapipe included), so it is PCSX2's emulation of the GS, not the GS:
the same caveat as every other PCSX2 figure here.

**What it reads** (race frame on Moa Therma, 03_Track, 240 frames into the race;
the grid frame at slot 8 of the same circuit):

| | Race, `race.gs` | Grid, `grid.gs` |
| --- | ---: | ---: |
| pixels at alpha 0 | 98.66 % | 96.66 % |
| distinct levels | 249 | 222 |
| highest | `0xfd` of `0x80` (grey 253) | 253 |
| pixels at 4 (the PSP's floor) | 16 | 77 |

So **the PS2's mask is not the PSP's stencil**: the frame clears to alpha `0`
(group 1, `FBMSK` 0, a full-frame sprite of colour `(0,0,0,0)`), opaque batches
leave alpha alone (`FBMSK` `0xff000000`, 535 of 595 groups), and the nine to
forty-one groups that write it (race groups 253-261, grid 521-561: alpha-tested
`PSMT8` textured fans and lists, after the opaque scene) are what stamp it, with
**graded** values.

**The value is the fragment's own alpha.** Every one of those groups has
`TEX0` `TCC` 1 and `TFX` 0 (`MODULATE`), vertex colours of `127` alpha for a
static batch and `0..127` for a fading one, and a `PSMT8` texture whose `CT32`
palette carries the alpha ramp: the `64 x 64` texture in grid group 531 has a
palette alpha running `11, 12, 13 ... 128`, and the blue track-edge strip that
samples its low end reads grey 21 (alpha 10) in the readout. So the written alpha
is `At * Av >> 7`, the PS2 analogue of the PSP's per-texture `+0x1e` byte but
**per texel**, which is why the chevrons on the pads read as graded shapes and
the oval and the arches as flat full-brightness blocks.

**What does and does not stamp**, from the readout:

- Stamps: the `_GLOW` surfaces (the gantry oval and its arch, the banners, the
  pad chevrons, the track-edge strips, the lit panels), the craft's engine and
  blink lights and the small lit block at the nozzle (the hull's, not the plume's).
- Does not: the sky, the opaque scene, **the exhaust ribbon and the boost plume
  (nothing of their length is in the readout)**, and
  **nothing at all outside a glow batch** - the PS2 has no floor of `4`.
- A blended batch with the glow bits stamps too (`ABE` groups 258, 546, 548): the
  blend never touches alpha, so what lands is the same `Ct * Cv`.

`TEST` on those groups is `0x5006d` / `0x7006d`: alpha test on, `ATST`
`NOTEQUAL` against `AREF` 6, depth test `GEQUAL` (or `GREATER`). Read from the
register only, not exercised.

**Second instance.** The grid frame agrees with the race frame on every rule
above; the glow geometry differs because the circuit position does. **Talon's
Junction was not reached**, so one circuit.

## What ours does (2026-10-02)

- **Mask:** a model marked `stamps_glow` and `glow_by_texel` is built with
  `GlowMask::StampedByTexel`. A batch with the glow bits (`pass_mask & 0xc0`,
  carried per vertex as `slots::GLOW_BATCH`) writes `texel alpha * vertex alpha`
  into the target's alpha - opaque and cutout pipelines directly, a blended batch
  through the same second stamp draw the PSP's blended glow batches use - and
  every other batch writes `0`, the clear's value. The loader marks the PS2's
  models through `race::load::pulse_ps2`; the PSP's keep `GlowMask::Stamped` and
  its constants (`glow-mask.md`).
- **Chain:** `post::ps2_bloom`, beside `post::bloom` and not a parameterisation of
  it, because every constant and three pieces of arithmetic differ (its module
  doc has the table). Downsample `Cs * As >> 7` into a 320 x 224 buffer (the
  scene's alpha `1.0` is the PS2's `0x80`), seven taps `texel * w >> 7` per axis
  with **zero past the buffer's edge** (the strips are drawn shifted over a clear
  buffer, so past the edge is absent, not clamped), and the composite
  `(Ct * 127 >> 7) * 64 >> 7` added to the frame, the buffer stretched bilinearly
  and drawn **five pixels of 512 up and left**, leaving the last five rows and
  columns without it.
- **Read off the dump this pass, not in the first reading above:** the downsample
  is `TFX` decal (the frame's colour and alpha straight through, the vertex
  colour 127 ignored); the taps are `MODULATE` with the weight as vertex colour
  (`texel * 16 >> 7` and so on) and a point read, 1:1, so there is no filtering in
  the blur; the composite's vertex colour is 127, so its strength is `127/128 *
  64/128`, not `0.5`.
- **Chosen, not measured:** the buffer maps the *whole viewport*, because the
  original reads all of its 512 x 512 frame; and the five-pixel offset is `5/512`
  of the viewport on each axis, so at anything but 512 x 512 it is a scaling of
  the original's offset. The ghost craft stamps nothing on PS2 (its stamp
  pipeline follows the PSP bloom). Neither is a measurement.
- **The plume, the shield shell and the exhaust ribbon write no mask on PS2**
  (`Model::glow_by_texel` without `stamps_glow` builds `GlowMask::Protected`;
  `exhaust::Pipeline::new`'s `trail_stamps_mask` is `false`). The first build of
  this port let the PSP-recovered ribbon stamp its ramp on PS2 and the craft box of
  a boosting frame read 7.7 added luma against the original's 0.40; the original's
  readout has no ribbon in it.

## Is ours as strong as the original? (matched grid pose)

Moa Therma's grid, slot 8, the Assegai, `--opponents` (the full grid), 682 x 512
at the PS2 aspect (`display.aspect = ps2`, 4:3, as the PCSX2 frame is), against
the dump's own frame replayed to just before and just after the composite (grid
groups 646 and 647). Mean luma the bloom adds, the same method as
`docs/ghidra/functions/psp-pulse-usa/bloom.md`'s "Is ours stronger than the
original?":

| | whole frame | craft box (x 240-460, y 290-512) | everywhere else |
| --- | ---: | ---: | ---: |
| the original, replayed | **1.527** | 0.716 | 1.659 |
| ours | **1.623** | 0.788 | 1.759 |

**Ours is 1.06 times the original over the frame**, 1.10 around the craft. The
shapes agree (the oval, the orange arch, the banners, the cyan strips, the
engine), and the frame-to-frame differences left are the scrolling glow strip's
phase (`col_display7_GLOW` animates; the dump's frame is an arbitrary phase, ours
is tick 1), our eight craft against the dump's, and the HUD in both. One
instance, one circuit, a **grid frame at rest**: there is no racing-frame
comparison because the racing dump's pose is not recoverable (no position read
out of the emulator), so the 1.2-1.6x residual the PSP chain carries on a straight
is not known for the PS2. Confidence **70**: one matched frame, the same
renderer on the original side as everywhere else.

**Racing, craft box only, pose not matched** (the chase camera puts the craft in
the same place whatever the pose): the racing dump's craft box (x 240-460, y
290-512 at 682 x 512) gains 0.404 luma from the bloom, ours with the flare forced
hot at about 133 km/h (`--pose-boost 10 --pose-intensity 1.0 --pose-speed 38.3`)
gains **0.84**, 2.1 times, and the mask in that box agrees with the original's
readout nearly pixel for pixel (hull lights, nozzle block). A shield frame
(`--force-shield 1:50`) does not white out (craft box 0.94). Whole-frame racing
numbers are not comparable: our pose faces the gantry. The excess is
unexplained, as the PSP's is.

`crates/render/tests/ps2_bloom_gain.rs` (a GPU, `#[ignore]`d) runs the chain on a
known frame against the GS arithmetic in integers: it agrees to a level. The
mask's rule is pinned on the disc by
`crates/game/tests/ps2_glow_mask_ground_truth.rs`.

## A pass that is not bloom

Race groups 223-236: a 512x512 `PSMCT16S` target (page 350) is cleared and
filled with a ship-shaped fan from a 64x64 texture, then composited with
`(Cd - Cs) * FIX` (a subtraction) at 236. It reads as the ship's shadow.
Unnamed, not investigated.

## Static corroboration

Not done. An EE RAM search found no byte, word or float image of the 7-tap
weights, and the only `Blur` and `Glow` strings are front-end XML attributes
(`CalcBlur`, `RealGlow`), unrelated. The weights are almost certainly
immediates in code. A clean miss on a data table says nothing; finding the
routine needs Ghidra on the PS2 ELF, starting from the GIF packet it builds
(`ALPHA` `0x4000000068`, UV `5120`/`3584`).

## Still needed

- ~~Read the frame's alpha channel out~~ Done, above.
- A second circuit (Talon's Junction) and a boost or weapon frame: the mask and
  the strength are one circuit, a grid frame.
- A racing frame at a recoverable pose, for the racing-straight strength.
- What the `ATST` `NOTEQUAL` 6 alpha test discards (the register is read, no texel
  alpha 6 was looked for), and whether the HUD groups before the downsample
  (race groups 262-289) take the composite. **Ours now composites over the
  HUD (2026-10-04), inherited from Pulse PSP's measured queue order
  (`docs/ghidra/functions/psp-pulse-usa/bloom.md`, "The bloom draws over the
  HUD"), not measured on the PS2.** The register reading above, with the HUD
  groups before the downsample, points the same way; a GS-dump check of
  groups 262-289 against the composite is what would measure it.
- The routine in the ELF, and whether any mode (menus, replay) skips it.
