# Pulse PS2 does bloom: five passes per field, read off a GS dump

**Yes, the PS2 build runs a bloom post-process every race frame. Confidence
90** (rubric: measured directly, four independent frame instances, one
circuit, one image; not 95 because no second circuit and no static
corroboration in the executable). It is the same shape as the PSP's
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

## The mask: the destination alpha channel

Pass 1 multiplies colour by the frame's stored alpha, the PS2 analogue of the
PSP's stencil-masked bright pass. In the dump 535 of 595 groups draw with
`FBMSK = 0xff000000` (alpha untouched) and nine consecutive groups (race
groups 253-261: alpha-tested `PSMT8` textured fans and lists, 3,156 primitives,
after the opaque scene) draw with `FBMSK = 0`, which is how alpha gets
written. That these nine are the glow stamps is read from the register state,
**not proven**: the alpha channel was not read back out of the frame. Its
layout is therefore unmeasured, so the rules in [`glow-mask.md`](glow-mask.md)
do not yet transfer, and the PS2 engine path still has no measured mask.

The glow is visible in the dump's own screenshot, around the thruster plume.

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

- Read the frame's alpha channel out (a GS dump's VRAM, `FBP 0`) to measure the
  mask and its ramp, as `glow-mask.md` did on the PSP.
- A second circuit (Talon's Junction) and a boost or weapon frame.
- The routine in the ELF, and whether any mode (menus, replay) skips it.
