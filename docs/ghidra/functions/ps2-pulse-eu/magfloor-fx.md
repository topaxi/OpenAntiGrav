# Mag-floor effect on PS2: the same anchor, spelled out without `vmmul`

| | |
| --- | --- |
| **Binary** | `SCES_547.48` (Pulse, PS2, EU) |
| **Subsystem** | ship scene-graph effects |
| **Related** | [PSP `magfloor-fx.md`](../psp-pulse-usa/magfloor-fx.md), the reference this page corroborates |

The PSP page is the record; this page exists to hold the three PS2 addresses
and what each one corroborates. Found 2026-10-05 from the two strings
`Data\visual_effects\MagEffect1.vex` (`0x002a71e0`) and `MagEffect2.vex`
(`0x002a7208`), whose only reader is the constructor below.

| Address | Name | Conf. | What it does |
| --- | --- | ---: | --- |
| `0x00165688` | `MagFloorFx_Construct` | 88 | Loads MagEffect1 under the craft (`obj+0xc0`) and MagEffect2 under the scene root `[0x002e0280]`, stamps both with the tint, hides both (`&= ~4` on `+0x2c`), builds identity with the translation row's `y` set to `[0x0027e950] = 0xc0200000 = -2.5` (`vaddx.y vf2,vf0,vf1` at `0x00165948`, y only), and calls `Node_SetLocalMatrix` (`0x001fe888`) with mode `1` then `0`. Clears the active word `obj+0x134`. |
| `0x001659a0` | `MagFloorFx_Update` | 88 | vtable `0x00293b50` slot `+0x24`. Gated on `obj+0x134`. Up = `normalize(-craft+0xbd0)` (`vsub` from a zero vector, `vrsqrt`); `O * W` row by row with `vmulax/vmadday/vmaddaz/vmaddw` against the craft's four world rows, so translation = `W.row3 - 2.5 * W.row1`; nose = `normalize(W.row2 - up * dot(up, W.row2))`; side = `up x nose` (`vopmula/vopmsub`); `Node_SetLocalMatrix(MagEffect2, rows, 0)`. |
| `0x00165ce0` | `MagFloorFx_InitTint` | 85 | GCC static-init shape (`a1 == 0xffff && a0 != 0`): writes `1.0` to all four floats `0x002e35e0..0x002e35ec`, the tint the constructor packs. |

**Why it matters for the PSP reading:** the PSP update composes `O` and `W`
with one `vmmul.q`, whose operand order has to be decoded. The PS2 has no
equivalent instruction and writes the product out as row-vector arithmetic,
which fixes the order: `O * W`, the translation carried down the craft's own
up axis. Agreement in structure and constants with the PSP is what moves the
PSP anchor's score past the decompile-only ceiling. Not runtime-verified on
either platform.
