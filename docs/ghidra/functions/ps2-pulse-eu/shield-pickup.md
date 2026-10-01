# The PS2 Shield pickup's visual object: the same law as the PSP, and a raised shell captured live

**Binary:** `SCES_547.48` (Pulse PS2, EU), image base `0x00100000`.
**Status:** the object's constructor, activate, deactivate and per-frame update
are decompiled and match the PSP's (`../psp-pulse-usa/shield-pickup.md`) field for
field, offset by `0x80`. A raised shell was **captured live on PCSX2** by writing
the object's fields through PINE (2026-10-01). Read there: the shell adds and
never darkens, and it looks nothing like ours. The cause of the second is open.

## The object

`FUN_00152dc8` (the craft's model set-up) allocates `0x140` bytes, calls
`ShipShield_Construct` (`0x00169168`) and stores the result at **`craft+0x958`**.
The object, with the PSP's offset beside each (the PSP's are `0x80` lower, the
two model pointers aside):

| PS2 | PSP | Field |
| --- | --- | --- |
| `+0xc0` | `+0x40` | current colour, `rgba`, four floats |
| `+0xd0` | `+0x50` | target colour |
| `+0xe0` | `+0x60` | colour rate, `0.15` (`0x3e19999a`) |
| `+0xe4` | `+0x64` | swell |
| `+0xe8` | `+0x68` | swell target |
| `+0xec` | `+0x6c` | swell rate, `0.2` (`0x3e4ccccd`) |
| `+0xf0` | - | the craft (`param_2` of the constructor) |
| `+0xf4` | `+0x74` | active |
| `+0xf8` | `+0x75` | fading |
| `+0xfc` | `+0x78` | clock, seconds |
| `+0x100` | `+0x7c` | `vr_shield_cockpit.vex` instance |
| `+0x104` | `+0x80` | `shipshield.vex` instance |

## Three functions, named

| Address | Name | What it does | Confidence |
| --- | --- | --- | --- |
| `0x00169418` | `ShipShield_Activate` | colour `<-` the `.bss` vec4 at `0x002e35f0` (`(0,0,0,0)` read live), target `<-` `0x002e3600` (`(1,1,1,1)`), swell `<-` `0x0027e97c` (`0.7`), swell target `1.0`, rates `0.15` and `0.2`, `active = 1`, clock `0`, and the colour is pushed to both models | 85 |
| `0x001695d8` | `ShipShield_Deactivate` | target colour zero, swell target `0x0027e980` (`1.2`), `fading = 1`, both models' draw bit (bit 2 of `+0x2c`) cleared | 80 |
| `0x00169658` | `ShipShield_Update` | the whole animation: `(int)(dt * 59.999996)` substeps of the colour and swell lerps, `sin(clock)` into both the alpha `x (0.25 sin + 0.75)` and the scale `swell + 0.012 + 0.012 sin`, the cockpit sphere at `x 1.8` when the camera is internal, the draw bit set on the model drawn, clock `+= dt`, and the fade-out test `alpha <= 0.1` | 88 |

Update has no direct caller (it is a virtual slot). Every constant is the PSP's:
`0.15`, `0.2`, `0.7`, `1.0`, `1.2`, `0.012`, `0.25`, `0.75`, `1.8`, `0.1`, and the
`.bss` colours are the same `(0,0,0,0)` start and `(1,1,1,1)` target.

## Raising a shell without a gameplay path (the 2026-09-25 blocker)

The PS2 disc has no `--give`, and reaching a Weapon Pad failed. The object makes
it unnecessary: `Update` runs every frame and returns at once while `+0xf4` is 0,
so **writing what `Activate` writes over PINE raises the shell**. From the grid
savestate (Moa Therma, Assegai; `scripts/pcsx2-camera-eye.py find` gives the
craft, `0x1b374c0` in that state; the object is `0x00721860` there):

```
+0xc0..0xcc = 0,0,0,0   +0xd0..0xdc = 1,1,1,1
+0xe0 = 0x3e19999a  +0xe4 = 0x3f333333  +0xe8 = 0x3f800000  +0xec = 0x3e4ccccd
+0xf8 = 0  +0xfc = 0  +0xf4 = 1
```

then step verified frames (`pcsx2-drive.py`'s `advance_frames`). Read back at 30,
60 and 120 frames: colour `0.992`, `0.99994`, `1.0` (the `0.15` lerp), swell
`0.9996`, `0.9999994`, clock `0.59`, `1.18`, `2.36` s (PAL, 50 Hz). The law runs
as read. A control from the same savestate and frame counts is **bit-identical
except for the shell** (the HUD clock reads the same), so the difference of the
two frames is the shell's contribution exactly.

## What the original draws (read, three frames, 640x448)

- **It adds and never darkens.** Over 42,000 to 48,000 changed pixels per frame,
  not one pixel is darker with the shell than without (`min` of the difference 0,
  the negative sum 0 on all three channels). An alpha-over shell would darken
  every pixel it covered with a shell colour below the background. This raises
  `livery::shield::blend_additively`'s reading from an inference on the plume
  (confidence 70) to a measured one: the PS2 shell is additive. Confidence
  **88** for "additive in the sense of never darkening"; whether the factor is
  `src x alpha` or `src` alone is not separated.
- **Look.** A translucent cyan-blue dome with soft horizontal ring bands, brighter
  on the rim bands, nothing resembling hexagon cells.
- **It breathes and moves**: the same three frames differ in band brightness (the
  `sin` alpha) and in band position.

## Ours, same disc, same craft class (not pixel-matched)

`oag-game data/images/pulse-ps2-eu.chd --race --give shield` (the craft's slot and
the camera differ from the savestate's, so no per-pixel comparison was made): a
bright hexagon lattice over the whole craft, crisp cells, nearly opaque. The shell
here is 395 vertices in three additive ranges, **UVs up to `u 7.58`, `v 6.91`
(eleven rings spaced `0.652` in `v`)**, a 256x256 `PSMT4` lattice (grey cells,
bright edges, texel alpha 0 to 128-scale), vertex colours `(103,177,253)` at
alpha 253, and `(115,199,253)` at alpha 0 and 131.

**The difference is named and not fixed.** The original shows the vertex-colour
ring pattern with no lattice; ours shows the lattice. Candidates, none tested:
the GS samples a coarser mip level of a tiled texture (the lattice averages out
at that minification, the rings stay), the texture used is not this one, or the
texture's own alpha and the vertex alpha combine differently on the GS's 128 =
1.0 scale. Whether the 66-triangle band draws twice (the open question on the
handover thread) is untouched.

## Method and what remains

Own display `:93`, own PCSX2 `-datapath`, PINE slot 45093 (`data/scratch/pulse-shield-look/scripts/ps2lib.py`,
`ps2_cap.py`: `pcsx2-drive.py`'s globals patched, the render window picked by
title because a PPSSPP window shared the display). Frames: `ps2a` (shield),
`ps2c` (control).

Open: a PCSX2 GS dump of the shell's draw (texture level, `TEX0`/`TEX1`, `ALPHA`
register, `TEST`) would turn every candidate above into a read; no GS capture
was taken. The cockpit sphere is not captured.
