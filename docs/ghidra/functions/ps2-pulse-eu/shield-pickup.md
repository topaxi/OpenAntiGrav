# The PS2 Shield pickup's visual object: the same law as the PSP, a different model, and a GS dump of the raised shell

**Binary:** `SCES_547.48` (Pulse PS2, EU), image base `0x00100000`.
**Status:** the object's constructor, activate, deactivate and per-frame update
are decompiled and match the PSP's (`../psp-pulse-usa/shield-pickup.md`) field for
field, offset by `0x80`. A raised shell was **captured live on PCSX2** by writing
the object's fields through PINE (2026-10-01), and a **GS dump** of it
(2026-10-01, second pass) settled what the first pass left open: the PS2 draws
**`<Team>\extrashield.vex`, not `shipshield.vex`**, with its own texture, and
**never tints the shell's vertices**. Both are fixed in `oag-game` and pinned by
`crates/game/tests/ps2_shield_ground_truth.rs`.

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
| `+0x104` | `+0x80` | the shell (`<Team>\extrashield.vex` here, `shipshield.vex` on the PSP) instance |

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

## The model is `extrashield.vex`, not `shipshield.vex` (GS dump, 2026-10-01)

`ShipShield_Construct` (`0x00169168`) formats the shell's name with
`FUN_0025b920(buf, 0x80, 0x2a7928, *(*(craft + 0x400) + 0x98), 0x2a7920)`. Read
from EE RAM at those addresses: `0x2a7928` is `%s\%sshield.vex`, `0x2a7920` is the
literal **`extra`**, and `*(*(craft+0x400)+0x98)` is the team directory. The PSP
build passes the `FE_TeamModel` config key's `"ship"` there, so its shell is
`shipshield.vex`; the PS2 build's is `<Team>\extrashield.vex`. (The cockpit
sphere's `%s\vr_shield_cockpit.vex` at `0x2a78f0` is the same on both.)
Confidence **92**: the call's arguments read off the executable, the assembled
`Data\Ships\Assegai\extrashield.vex` and `Data\Ships\AG_Systems\extrashield.vex`
seen in EE RAM with no `shipshield` string anywhere, and the GS draw below
matching this file vertex for vertex. It is a literal in the PS2 binary, so it
holds for every team and every mode. (`Construct` also looks `FE_TeamModel` up,
but the decompile drops the result; the PSP is where that key is the prefix.)

Everything this page and `batch-draw-state.md` said about `shipshield.vex` on
PS2 (its `0x1031`/`0x18b1`/`0x10b2` class-less batches, the 256x256 `PSMT4`
`grid_GLOW` lattice taken from the preceding texture set, eleven rings with
`u` up to 7.6) described a model the PS2 never draws for the shell.
`shipshield.vex` is on the disc, unreferenced by the executable (no string of
it exists in the binary or in RAM). That is the "lattice against rings" gap:
ours drew the wrong file.

`extrashield.vex` (node `shield_temp1:polySurfaceShape6`, `AG_Systems`, `Assegai`,
`Auricom`, `EGX`, `Goteki`, `Harimau`, ... one per team, about 7 KB) has:

| | |
| --- | --- |
| geometry | 167 vertices, 163 triangles in two batches (115 and 48), the first with 68 distinct positions |
| batch class | `pass_mask` `0x1232`: **`0x200`, additive, in its own words** |
| texture | one node, `pulse_shield_extra_ADD.tga`, a 128x64 `PSMT8` in the preceding set (directory index 4081 in `WADS2.WAD` for Assegai; the entry itself is index 7104), 15 colours, alpha `160` (`0x50` on the GS scale) |
| `u` | 0 and 0.5 |
| `v` | 0, 0.756, 0.928, 1 |
| vertex colour | `(127,127,127,127)` and `(13,0,104,0)` on the GS scale, exactly |
| texture track | `u` offset `0 -> 251/256` over keys 1..59 of 60 per second, loop 0.9833 s (`vex::mesh_tex_transforms`) |

## What the GS dump shows (PCSX2, eight dumps of the raised shell and one control)

Method: [`../../../reverse-engineering/pcsx2-debugger.md`](../../../reverse-engineering/pcsx2-debugger.md)
"GS dumps". Savestate on the grid (Moa Therma, Assegai, craft stationary at
`(-385.617, 4.004, 127.095)`), shell raised by writing `Activate`'s fields, a
verified frame advance, the `GSDumpSingleFrame` hotkey, six more frames so the
file closes. A control from the same savestate and frame counts has no shell.
The shell is the only draw group the two dumps do not share in sequence
(`602` and `1239`, the two fields of the frame), ignoring the texture-pool
addresses, which differ between runs.

One draw, a 159-primitive triangle strip of 161 vertices at 68 distinct
positions, GS state read from the packets:

| Register | Value | Reads as |
| --- | --- | --- |
| `PRIM` | `0x7c` | triangle strip, Gouraud, **textured**, fog on, **alpha blend on**, STQ |
| `ALPHA_1` | `0x48` | `A=Cs, B=0, C=As, D=Cd`: **`Cs * As + Cd`**, source-alpha additive, FIX unused |
| `TEST_1` | `0x70003` | alpha test on but `ALWAYS`, **depth test on (`GEQUAL`)** |
| `ZBUF_1` | bit 32 set | **no depth writes** |
| `TEX0_1` | `PSMT8`, 128x64, `TFX` modulate, `CLUT` 256 entries `CSM1`, `TCC` on | the texture below |
| `TEX1_1` | `0x260` | bilinear, **no mip** (`MXL` 0) |
| `CLAMP_1` | `0x1fc001fc000` | repeat |
| fog | `FOGCOL 0xffe3f1`, vertex `F` 250-251 | effectively none (mix 0.98 to 0.02) |

So the PS2 shell is **additive, source-alpha weighted** (`blend_additively`'s
inference is gone because the model names its class), unmipped and bilinear, and
the GS-side candidates the previous pass listed (a coarser mip level, a second
texture, texel alpha combining oddly) were all wrong: the model and texture were.
Confidence **92**: every register read straight off the packets, identical in
eight dumps across clocks 0.16 to 2.9 s and a fade-out. Single emulator, single
savestate.

It also settles the first pass's question on whether the factor is `src x alpha`
or `src` alone (`As`, with the texel alpha `0x50` and vertex alpha 127), and agrees
with that pass's finding that the shell never darkens a pixel.

**The texture is the disc's, byte for byte.** The 8,192 texel bytes the game uploads
(`PSMT8` as `PSMCT32` 64x32) sit in EE RAM at `0x0184c6c0` with the name
`Data\Weapons\Textures\pulse_shield_extra_ADD.tga` just before them. They equal
bytes `0xcd..` of the `WADS2.WAD` entry `6bb25a5b` (directory index 7104, 128x64,
8 bpp, 9,485 bytes), and the 1,024 palette bytes equal that entry's palette
(+8,461). Confidence **95**. Decoded through the palette's `CSM1` swizzle (entry
`i` at `(i & ~0x18) | ((i & 8) << 1) | ((i & 0x10) >> 1)`, what
`unswizzle_clut` undoes) it has 15 colours, alpha `0x50`.

**The scroll is the file's.** `u` offset advanced `0.039` per game frame (two
fields) and `0.399` over `0.393 s` of the shield's clock: `1.014 /s`, the
authored track's `251/256` over `0.9667 s`. Which clock the phase hangs off was
not separated (the shield's own or the global one advance together).

**The vertex colour is never tinted.** Every white vertex read `(127,127,127,127)`
and every rim vertex `(13,0,104,0)` - the file's own values - in all of:

| State | Object colour (`+0xc0`) | Clock | Flicker factor `0.75 + 0.25 sin` |
| --- | --- | --- | --- |
| fade-up | 0.73, 0.86 | 0.16, 0.24 s | 0.76, 0.81 |
| settled | 0.997 .. 1.0 | 0.71, 1.61, 2.48, 2.87 s | 0.91, 1.0, 0.90, 0.81 |
| fade-out (`Deactivate`'s fields written) | 0.197 | 0.98 s | n/a |

If `ShipShield_Update`'s packed-colour push (`FUN_001df718(model, argb)`, alpha
`rgba.a * (0.25 sin + 0.75)`) were multiplied into the vertices the alpha would
have read 76 to 127 instead of 127. It is not: the shell is drawn at its
authored colours, scaled by the swell, until it switches off at `alpha <= 0.1`.
Confidence **88**: seven states, one savestate. The fade-out was driven by
writing `Deactivate`'s fields, not by a pickup running out. Whether the push
reaches something else (a lit pass, a model that sets a colour-override flag) is
unread; `FUN_001df718`'s body was not followed. The cockpit sphere (not drawn
from the chase camera) was not measured and keeps the tint in `oag-game`.

The **swell** (`+0xe4`, 0.95 at 0.157 s, 0.98 at 0.236 s, 1.18 at the fade-out)
is applied, through the model matrix `ShipShield_Update` builds, and was not
separated from the dump (the vertices are post-transform).

## Which instance is drawn

`ShipShield_Update` sets bit 2 of `+0x2c` on the instance it draws. After a write
of `Activate`'s fields and ten frames, `*(obj+0x104)` (the shell) went from
`0x0005b022` to `0x0105e026` and `*(obj+0x100)` (the cockpit sphere) stayed
`0x0005b022`, as the chase camera needs. A write of `Deactivate`'s fields and
four more frames left `0x0105e026`; ten frames cleared it.

## Not yet compared

- **A pixel-matched frame, and whether the brightness agrees.** The craft is at the
  savestate's position in `oag-game --pose`, but the chase camera differs: the
  original's hull fills about 38 % of the frame's width, ours 23 %, and neither
  published eye (`craft+0x850`, `+0x860`) with the look-at at `craft+0x840`
  reproduces the original (the camera lane's gap). The comparison made is of the
  draw: the same 68 positions, the same `uv` and colour sets, the same state, and
  both draw both faces (the dump's strip has 72 counter-clockwise and 44 clockwise
  non-degenerate triangles; `oag-render`'s pipelines do not cull).

  **The shell's contribution to the picture is not matched, and why is open.** Each
  side's frame minus its own no-shield frame (the PCSX2 control is bit-identical
  but for the shell; ours is rendered at the same craft position), inside the
  dome, over three clocks (0.9, 1.8, 2.7 s of the shield):

  | | changed pixels | median added B | 90th pct B | median added G |
  | --- | ---: | ---: | ---: | ---: |
  | original | 38,121 / 40,284 / 40,807 | 20 / 80 / 101 (mean 67) | mean 121 | mean 45 |
  | ours | 23,168 / 24,484 / 24,138 | 42 / 44 / 47 (mean 44) | mean 96 | mean 27 |

  The pixel counts differ by the camera (1.66x, close to the hull-width ratio
  squared). The brightness does not obviously: the original's per-frame median
  swings from 20 to 101 with the texture's scroll phase, and ours swings from 21 to 69
  across `--anim-seconds` 0.1 to 0.9 on one tick (51, 69, 21, 50, 50). Ours' sweep
  contains the original's low frame and not its two high ones, and its mean is
  about two thirds of the original's. Camera and phase do not obviously account for
  that; a real term may remain. Not found by reading the registers: `ALPHA`,
  vertex colour, texture alpha, fog and winding all agree with ours. Untested:
  that the original's own phase at those frames lands on the texture's bright
  band (the scroll clock is not recovered), the dump's `TEXA`, and any difference
  in how the two blend in sRGB against linear.
- The cockpit sphere on PS2, the hit flash (`ShipShield_Hit`), the shell on a
  team other than Assegai, and a shield running out by its timer.
- `FUN_001df718` (the model colour setter): where its argument goes.

## Raising a shell, and the harness

Own display, own PCSX2 `-datapath`, own PINE slot; `GSDumpSingleFrame = Keyboard/F9`
added to `[Hotkeys]`. See `scripts/pcsx2-gsdump.py` for the reader. The savestate
is a grid state made by walking the front end once.
