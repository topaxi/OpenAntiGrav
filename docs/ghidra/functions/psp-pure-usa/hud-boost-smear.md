# Pure's `FUN_088c9ce8` is `Mode3D_DrawQueued`: no copy loop

**Binary:** `psp-pure-usa` `BOOT.BIN`, image base `0x08804000`, Ghidra
`/pure/BOOT-psp-pure-usa.BIN`. Read 2026-11-02 (lane `pure-boost-reread`) by full
`disassemble_function` of `0x088c9ce8`-`0x088c9e80`. `jal` operands are image-base-relative
(`jal 0x000eb90c` is `0x088ef90c`). Companion to Pulse's
[hud-boost-smear.md](../psp-pulse-usa/hud-boost-smear.md), "`Mode3D_DrawQueued` has no copy loop".

## Verdict

**The earlier reading "Pure's `FUN_088c9ce8` carries the same `lui 0x44e2` copy loop at `0x088c9dec`" is false**
(confidence 90). `0x44e20000` and `0x44ef0000` are `1808.0` and `1912.0`, the GE screen-offset base
(`2048 - 240`, `2048 - 136`). They feed one `Gu_SetOffset`:

| Address | Instruction | Meaning |
| --- | --- | --- |
| `0x088c9db0`-`0x088c9db4` | `lw a0,0x9c(s1)`; `beq a0,s4(=1),0x088c9e2c` | branch on the widget's projection kind (`== 1` is the orthographic branch) |
| `0x088c9dbc`-`0x088c9de8` | perspective setup, constants `1.1344640` (65 deg), `1.7647059` (aspect), `1.0`, args `widget+0xac/+0xb0` (near/far), `jal 0x0022d148` = `0x08a31148` | projection matrix |
| `0x088c9dec`-`0x088c9e18` | `f12 = widget+0xa4 + 1808.0`, `f14 = widget+0xa8 + 1912.0`, `trunc.w.s`, `jal 0x000eb90c` | `Gu_SetOffset(trunc(OriginX + 1808), trunc(OriginY + 1912))`; no loop |
| `0x088c9e20` | `b 0x088c9e5c` | forward jump to the epilogue; no branch back anywhere in the function |
| `0x088c9e2c`-`0x088c9e50` | ortho branch: `0, 480.0, 0, 272.0, -100.0, 100.0`, `jal 0x0022ca98` = `0x08a30a98` | orthographic matrix |
| `0x088c9e58`-`0x088c9e60` | `f12 = 0x42820000 (65.0)`, `jal 0x0005d8dc` = `0x088618dc` | view-projection build |

`0x088ef90c` is `Gu_SetOffset`: it appends two GE words, `(x << 4) & 0xffffff | 0x4c000000` and
`(y << 4) & 0xffffff | 0x4d000000` (OFFSETX, OFFSETY) to the list at `*(0x00013258 + 0x10000)+4`, confidence 88.

The function copies the two 4x4 view and projection matrices at `hud + 0x1410` and `hud + 0x1190`
(indexed by `hud + 0x1694`/`0x1690`, `<< 6`), the `Gu_SetOffset`, one matrix call each branch, and one tail call. There
is no load at `hud + 0x274` or of a `+0xa8` count off a HUD pointer (`0xa8`/`0xb0` here are the widget's own
`+0xac`/`+0xb0` near/far; `+0xa8` is `OriginY`), no counter and no branch back. It draws nothing, so it cannot draw
anything several times.

It is the structural twin of Pulse's `Mode3D_DrawQueued` (`0x088b8548`): same two-branch projection, the same
`Gu_SetOffset(+1808, +1912)`, the same tail call; the widget's `OriginX/OriginY` sit at `+0xa4/+0xa8` here and
`+0x9c/+0xa0` on Pulse (eight bytes later). The function has no caller by xref (reached through a vtable, as on
Pulse).

## Functions

| Address | Name | Confidence | What it is |
| --- | --- | ---: | --- |
| `0x088c9ce8` | `Mode3D_DrawQueued` | 72 | Twin of Pulse's `0x088b8548` (78): sets the projection, `Gu_SetOffset`, view-projection; draws nothing. Not a boost copy loop. |
| `0x088ef90c` | `Gu_SetOffset` | 88 | Emits GE OFFSETX/OFFSETY words (`0x4c`/`0x4d`) from `(x, y) << 4`; Pulse's is `0x08811220`. |

## What this leaves for Pure

Pure's real boost copy loop (the 2D `Image`/`Text` widget draws that read the count at `*(hud+0x274)+0xa8` on Pulse)
was not searched for here. Pure's `boost_smear` stays `None`; the lead is the Pulse recipe: find the Pure
`Image_Draw`/`Text_Draw` twins (`lui 0x44e2` inside a loop with a branch back) and read the count's source. Cross-title:
this is a Pure/Pulse PSP comparison only; HD/Omega differ (`FunkLayerZoom`), 2048 not checked.
