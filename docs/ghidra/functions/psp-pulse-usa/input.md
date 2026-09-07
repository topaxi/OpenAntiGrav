# Input

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`.

Small, self-contained, and fully mapped. This is the first subsystem that can be
reimplemented outright.

**The names here are applied**, from [names.tsv](names.tsv). See
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md).

## Polling

There is exactly one pad read, in `Input_ReadAnalog` (`0x0894ebd0`), calling
`sceCtrlPeekBufferPositive` (stub `0x08a7706c`) into `0x08b059f0`. Buttons land
at `0x08b059f4`; analog X and Y at `0x08b059f8` and `0x08b059f9`.

The analog stick gets a **0.25 deadzone and a 1.25 gain**. Both numbers matter
for feel and should be reproduced exactly.

Only three `sceCtrl` functions are imported: `SetSamplingMode` (`0x08a77064`),
`PeekBufferPositive` (`0x08a7706c`) and `SetSamplingCycle` (`0x08a77074`).

## The abstract button layer

The game does not use PSP button masks directly. `Input_BuildState`
(`0x0894eec4`) translates hardware bits into its own layout, and everything
downstream refers to **bit indices**, not masks.

| PSP raw | Abstract bit | Index | Name in XML |
| --- | --- | ---: | --- |
| `0x0010` (up) | `0x0001` | 0 | `up` |
| `0x0040` (down) | `0x0002` | 1 | `down` |
| `0x0080` (left) | `0x0004` | 2 | `left` |
| `0x0020` (right) | `0x0008` | 3 | `right` |
| `0x2000` circle | `0x0010` | 4 | `circle`, `cancel`, `backward` |
| `0x4000` cross | `0x0020` | 5 | `cross`, `activate`, `forward` |
| `0x1000` triangle | `0x0040` | 6 | `triangle` |
| `0x8000` square | `0x0080` | 7 | `square` |
| `0x0100` L | - | 8 | `l` |
| `0x0200` R | - | 9 | `r` |
| `0x0008` START | `0x4000` | **14** | `start` |
| `0x0001` SELECT | `0x8000` | 15 | `select` |
| - | - | `0x14` | **any button** |

Note the deliberate indirection: `activate` maps to cross and `cancel` to
circle. That is the Western convention, and a Japanese release would very likely
swap them, so this layer is where a region difference would live.

State lives in four masks on the input object:

| Offset | Meaning |
| --- | --- |
| `+0x3c` | Currently held |
| `+0x40` | Held last frame |
| `+0x44` | Released this frame |
| `+0x48` | **Pressed** this frame, the rising edge |

`Input_IsPressed` (`0x0894f1cc`) tests
`*(u32 *)(this + 0x48) & (1 << (index & 0x1f))`, and `Input_ConsumePress`
(`0x0894f22c`) clears a bit so one press cannot be handled twice.

`Input_IsHeld` (`0x0894f158`) is the same test against `+0x3c` - **the held
mask, not the rising edge** - and `Input_GetAxis` (`0x0894f198`) returns
`this+0xec` for axis `0` and `this+0xf0` for axis `1`, zero for anything else.
Both were read 2026-09-07 while resolving what gates the Cannon; the pair is
what `PlayerInput_Update` (`0x0883c870`) calls to build the per-craft control
record, and the *held* half of it is the Cannon's whole fire mechanism. See
[cannon-quake-leachbeam.md](cannon-quake-leachbeam.md#the-gate-is-the-fire-button-held-not-a-track-pad-flag)
for the record's layout and the six legs behind it. Confidence **90** for
`Input_IsHeld` (a three-line test against a mask this page already identifies)
and **85** for `Input_GetAxis` (direct decompile; what `+0xec`/`+0xf0` hold is
`Input_ReadAnalog`'s deadzoned output, which is read but not separately
verified against a capture).

`Input_ParseButtonName` (`0x0894f2a0`) maps the XML strings above to indices,
which is how the data-driven front end binds buttons.

Confidence: **92** for the mapping, verified in three independent places (the
translation, the test, and the XML name parser). `Input_ConsumePress` is **70**,
inferred from call position rather than decompiled.

## Applied renames

| Address | Name | Conf |
| --- | --- | ---: |
| `0x0894ebd0` | `Input_ReadAnalog` | 88 |
| `0x0894eec4` | `Input_BuildState` | 90 |
| `0x0894f1cc` | `Input_IsPressed` | 92 |
| `0x0894f158` | `Input_IsHeld` | 90 |
| `0x0894f198` | `Input_GetAxis` | 85 |
| `0x0894f22c` | `Input_ConsumePress` | 70 |
| `0x0894f2a0` | `Input_ParseButtonName` | 88 |
| `0x08b059f0` | `g_pad_buffer` | 90 |
| `0x08b31780` | `g_input` | 85 |

## Cross-platform

The PS2 build has been read against this page; see
[ps2-pulse-eu/input.md](../ps2-pulse-eu/input.md).

| Function | PSP | PS2 (`SCES_547.48`) |
| --- | --- | --- |
| `Input_BuildState` | `0x0894eec4` | `0x00202100` |
| `Input_IsPressed` | `0x0894f1cc` | `0x00202500` |
| `Input_ConsumePress` | `0x0894f22c` | `0x00202560` |
| `Input_ParseButtonName` | `0x0894f2a0` | `0x002025d8` |
| `g_input` | `0x08b31780` | `0x00302ec0` |

**Confirmed by the second binary:** every raw-mask-to-abstract-bit row in the
table above, for all ten buttons the two machines share, plus the four-mask
block (held, held-last, released, pressed) and its
`pressed = held & ~heldLastFrame` derivation - shifted by 8 bytes on PS2 to
make room for a connected/connected-last pair. Index `0x14` still means "any
button" and `-1` still means "none". `activate` and `forward` are still `cross`.

**Contradicted by the second binary, twice:**

- **`cancel` and `backward` are `triangle` (index 6) on PS2**, not `circle`
  (index 4). Read off the disassembly, not the decompiler, because a shared
  return block was the obvious way to be fooled. The PSP side is not re-checked
  here and stands as written.
- **PS2's `Input_ConsumePress` clears the entire pressed mask** and ignores the
  button index its callers pass. This page's one-bit description is scored 70
  and "inferred from call position rather than decompiled", so the cheapest
  thing that would settle it is re-reading `0x0894f22c` properly. If the PSP
  does the same, the description above is wrong; if it does not, a
  reimplementation cannot share one routine across the two.

PS2 also adds indices 10 to 13 (`l2`, `r2`, `l3`, `r3`) from pad bits the PSP
does not produce, and its `Input_GetAxis` applies **no deadzone and no gain** -
this page's 0.25 and 1.25 were not found anywhere on the PS2 path.

## For reimplementation

The abstract layer is worth copying rather than flattening. Keeping
`activate`/`cancel` as names rather than as buttons is what will make
region-specific button swaps and remapping straightforward later, and it is what
the original's own XML expects.
