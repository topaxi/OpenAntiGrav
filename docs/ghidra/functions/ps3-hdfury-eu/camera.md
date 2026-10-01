# HD/Fury's chase-camera rig carries the same 0.75 craft scale (0.7 in Detonator)

Whether HD scales its external chase eye the way Pulse PSP does (measured there:
[`psp-pulse-usa/camera.md`](../psp-pulse-usa/camera.md), "The 3/4 factor is
`g_craft_scale`"). Read headless on 2026-10-01, `EBOOT.elf`, decompiler plus the
instruction stream, with the TOC resolved per function by
[`scripts/ps3-toc.py`](../../../../scripts/ps3-toc.py) (see [memory.md](memory.md):
Ghidra's own TOC reads are wrong for most of this binary, so no TOC-relative
address below is taken from a Ghidra cross-reference). **Static only**: nothing
here was read from a running HD. Names went into [`names.tsv`](names.tsv) in the
same change.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x000d80e0` | `Ship_UpdateCameraRigs` | 80 |

## The answer: 0.75, applied to the eye and the look-at, outside Detonator

**The write.** The two craft constructors (`FUN_000ddd58` and `FUN_000dfd90`, the
ones that build `"%s external close tripod"` and `"%s external far tripod"`) each
write the same four floats into one global object, the one `ps3-toc.py` resolves
`lwz r5,-0x486c(r2)` to: `0x008c15e0`. In `FUN_000ddd58`:

```text
000de3c8  lwz  r8, -0x48d0(r2)   ; a byte at 0x009384e1, meaning not read
000de3cc  lbz  r0, 0(r8)
000de3d0  cmpwi r0, 0
000de3d4  bne  0x000de3e8
000de3d8  ld   r9, 0x248(r1)     ; the game state; +0xe0 is the mode id
000de3dc  lwz  r0, 0xe0(r9)
000de3e0  cmpwi r0, 0xe
000de3e4  beq  0x000dfc94       ; byte clear and mode 14: the alternative
000de3e8  lwz  r5, -0x486c(r2)  ; 0x008c15e0
000de3ec  lis  r9, 0x3f80
000de3f0  lis  r0, 0x3f40       ; 0.75
000de3f4  stw  r0, 0x44(r5)     ; scale  = 0.75
000de3f8  stw  r9, 0xb0(r5)     ;  three more words = 1.0
000de3fc  stw  r9, 0xa8(r5)
000de400  stw  r9, 0xac(r5)
...
000dfc94  lwz  r11, -0x486c(r2)
000dfc98  lfs  f0, -0x455c(r2)   ; -> 0x008a8f7c = 0.9
000dfc9c  lfs  f13, -0x4560(r2)  ; -> 0x008a8f78 = 0.7
000dfca0  stfs f13, 0x44(r11)    ; scale  = 0.7
000dfca4  stfs f0, 0xb0(r11)     ;  the three words = 0.9
```

`FUN_000dfd90` has the same pair (`0x000e0430` and `0x000e1d38`). Mode `14` is
**Detonator** ([mode-manager.md](mode-manager.md), settled 2026-08-30), so the
scale is `0.75` in every mode but Detonator, where it is `0.7` (and the three
neighbouring words are `0.9` rather than `1.0`; what they scale is not read).

**The consumer.** `FUN_000d80e0` is Pulse's `Ship_UpdateCameraRigs`
(`0x08845ed0`), written for AltiVec: it reads the same params block layout, off a
pointer at craft `+0x78` (`lfs +0x5c`, `+0x58`, `+0x54`, `+0x50` for the close
block, `+0x40` and its neighbours for the far block, the springs at
`+0x68`/`+0x64` and `+0x4c`/`+0x48`), and its two scale sites are

```text
000d844c  lfs   f11, 0x44(r21)    ; r21 = 0x008c15e0, the scale
000d84a8  stfs  f11, 0x90(r1)
000d84b0  lvewx v13, r1, r0 ; vspltw v13, v13, 0
000d84ac  vsubfp v1, v1, v30      ; eye  - bodyPos     (v30 = *(*(craft+0x6944)+0x200))
000d84c0  vmaddfp v1, v1, v13, v30 ; (eye  - bodyPos) * scale + bodyPos
000d84cc  vsubfp v0, v0, v30      ; look - bodyPos
000d84d0  vmaddfp v0, v0, v13, v30 ; (look - bodyPos) * scale + bodyPos
...
000d87b0  lfs   f0, 0x44(r21)     ; the same again for the other block
```

so each external block scales its eye and its look-at about the body position by
the global's `+0x44`, which is `ship + (p - ship) * scale`, Pulse's form. The
spring state is stored before the scale, as in Pulse.

Confidence **80** that HD's external eye and look-at carry `0.75` in every mode
but Detonator: the literal and its store are read directly, the consumer matches
Pulse's rig structurally on two scale sites and the params layout, and 2048's own
rig does the same ([vita-2048-eu-v104/camera.md](../vita-2048-eu-v104/camera.md)).
Not higher: decompilation plus disassembly only, the vector code was read for its
shape rather than traced through every branch of the function, and nothing here
ran.

## What this retires, and what it leaves

It retires the hypothesis in the 2026-10-01 thread that HD might run at scale
`1.0` with its distance authored directly: its authored close distance (11.07
to 12.86) is **not** what the player sees, it is that times `0.75`.

Not read: what `+0xa8`, `+0xac` and `+0xb0` of the same object scale (`1.0`, and
`0.9` in Detonator); the byte at `0x009384e1` that gates the Detonator values;
whether the internal rig reads the scale (Pulse's does not).
