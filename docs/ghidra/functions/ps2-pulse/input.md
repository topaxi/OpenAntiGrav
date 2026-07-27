# Input (PS2)

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`.

The PSP counterpart is [psp-pulse/input.md](../psp-pulse/input.md). The
abstract button layer survives the port essentially unchanged, which makes this
the second-strongest corroboration in this directory after the handling loader
- and it also contains the one place where the two builds genuinely disagree.

**The names below are applied**, from [names.tsv](names.tsv). Nothing scores
below 70.

## The per-player pad block

The PS2 build is two-player: `Input_BuildState` (`0x00202100`) loops
`for (pad = 0; pad < 2; ++pad)` over a stride of **`0x138` bytes** per pad, and
every accessor below takes a pad index. The PSP has a single block.

Within a pad's block:

| Offset | Meaning |
| --- | --- |
| `+0x3c` | Connected |
| `+0x40` | Connected last frame |
| `+0x44` | Currently held |
| `+0x48` | Held last frame |
| `+0x4c` | Released this frame |
| `+0x50` | **Pressed** this frame, the rising edge |
| `+0x154`, `+0x158`, `+0x160`, `+0x164` | The four analog axes, stored raw |

The four masks are computed exactly as the PSP page describes:

```c
pressed  = held & ~heldLastFrame;    // -> +0x50
released = heldLastFrame & ~held;    // -> +0x4c
```

**The four-mask block is the PSP's, shifted by 8 bytes** - `+0x3c/+0x40/+0x44/
+0x48` on PSP against `+0x44/+0x48/+0x4c/+0x50` here, with the two extra words
being the connected/connected-last pair the PSP has no use for. Same four
masks, same order, same derivation.

## The hardware-to-abstract mapping is identical

`Input_BuildState` translates raw pad bits into the game's own layout. Every
button the two machines share lands on the same abstract bit:

| Raw | Abstract bit | Index | Name in XML |
| --- | --- | ---: | --- |
| `0x0010` up | `0x0001` | 0 | `up` |
| `0x0040` down | `0x0002` | 1 | `down` |
| `0x0080` left | `0x0004` | 2 | `left` |
| `0x0020` right | `0x0008` | 3 | `right` |
| `0x2000` circle | `0x0010` | 4 | `circle` |
| `0x4000` cross | `0x0020` | 5 | `cross`, `activate`, `forward` |
| `0x1000` triangle | `0x0040` | 6 | `triangle`, `cancel`, `backward` |
| `0x8000` square | `0x0080` | 7 | `square` |
| `0x0100` | `0x0100` | 8 | `l1` |
| `0x0200` | `0x0200` | 9 | `r1` |
| `0x40000` | `0x0400` | 10 | `l2` |
| `0x80000` | `0x0800` | 11 | `r2` |
| `0x100000` | `0x1000` | 12 | `l3` |
| `0x200000` | `0x2000` | 13 | `r3` |
| `0x0008` START | `0x4000` | **14** | `start` |
| `0x0001` SELECT | `0x8000` | 15 | `select` |
| - | - | `0x14` | **any button** |
| - | - | `-1` | `none` |

Indices 0 through 7, 14 and 15 are the PSP table, unchanged, raw mask for raw
mask. Indices 10 to 13 are new and come from bits above `0x10000` that the PSP
pad does not produce - the DualShock's second shoulder pair and the two stick
clicks. `Input_ParseButtonName` (`0x002025d8`) is where the XML names bind,
and its full list was read out of `.rodata` at `0x002c0d50`..`0x002c0e10`.

The cross bit is tested against the *unmasked* pad word while everything else
is tested against a word that a global at `0x002849a0` can suppress, so
whatever that global gates, it never gates thrust off.

Confidence **90** for the mapping, from the translation and the XML name parser
agreeing, as on PSP.

### The divergence: `cancel` and `backward` are triangle, not circle

The PSP page lists `cancel` and `backward` alongside `circle` on index 4. In
this binary they are on index **6**, which the same function calls `triangle`.
Verified in disassembly rather than only in the decompiler, because a shared
return block is exactly the thing that could mislead here:

```
002026c0  jal strcasecmp        ; "triangle"
002026c4  beq  v0,zero,0x002027f8
...
002027ec  jal strcasecmp        ; "backward"
002027f0  bne  v0,zero,0x00202800
002027f4  ...
002027f8  b    0x00202838
002027fc  li   v0,0x6           ; <- both land here
...
00202818  jal  strcasecmp       ; "cancel"
00202820  li   a0,0x6
00202824  li   v1,-0x1
00202828  movz v1,a0,v0         ; equal -> 6, else -1
```

`activate` and `forward` land on `0x5` (cross) in the same function, which is
what the PSP page says, so this is not a wholesale remap - it is two names
moving one button. The obvious reading is that the PS2 release binds "back" to
triangle where the PSP binds it to circle, but **nothing here establishes what
the player actually presses**; it establishes only what the two binaries do
with the same XML string. Recorded as a difference rather than reconciled.
Confidence **90** for the PS2 side, read off the disassembly; the PSP side is
not re-verified here, since this session did not open `BOOT.BIN`.

## Accessors

| Address | Name | Conf |
| --- | --- | ---: |
| `0x00202100` | `Input_BuildState` | 90 |
| `0x00202500` | `Input_IsPressed` | 90 |
| `0x00202560` | `Input_ConsumePress` | 80 |
| `0x00202448` | `Input_IsHeld` | 85 |
| `0x00202580` | `Input_IsReleased` | 85 |
| `0x002025a8` | `Input_ConsumeRelease` | 82 |
| `0x00202480` | `Input_GetAxis` | 80 |
| `0x002025d8` | `Input_ParseButtonName` | 92 |
| `0x00302ec0` | `g_input` | 82 |

`Input_IsPressed(this, pad, index)` is
`*(u32 *)(this + pad * 0x138 + 0x50) & (1 << (index & 0x1f))`, with `index ==
0x14` short-circuiting to "any bit set" and `index == -1` to false - the same
two special indices the PSP page documents.

**`Input_ConsumePress` clears the whole mask, not one bit.** The PSP page has
it "clears a bit so one press cannot be handled twice", at confidence 70,
"inferred from call position rather than decompiled". The PS2 body is three
instructions and unambiguous:

```c
*(u32 *)(this + 0x50 + pad * 0x138) = 0;
return 0x14;
```

Call sites pass a button index as a third argument and the function ignores it.
Its sibling `Input_ConsumeRelease` (`0x002025a8`) *does* clear a single bit, of
the released mask, so the whole-mask clear is a deliberate asymmetry rather
than a compiler artefact. Whether the PSP build does the same is worth a
minute of somebody's time: if it does, the PSP page's description is wrong, and
if it does not, a reimplementation cannot share one routine between the two.

`Input_GetAxis(this, pad, axis)` selects among the four raw axis words by an
index 0..3. Unlike the PSP's `Input_ReadAnalog`, it applies **no deadzone and
no gain** - the PSP page's 0.25 deadzone and 1.25 gain are not here, and were
not found anywhere on the PS2 path. That is a weak negative: the polling
wrapper at `0x00201f30` is a thin shim over a library call and the scaling could
live further down. Named at 80 for that reason.

## Not determined

- **The pad read itself.** `0x00201f30` copies four words out of a library call
  at `0x0010aec8`; neither is named, and no `sceCtrl`-equivalent import table
  was resolved for this binary.
- **Where the analog deadzone and gain are, if they exist.**
- **The global at `0x002849a0`** (per-pad, 4 bytes apart) that suppresses every
  button except cross and, downstream, all but bit `0x20` of the edge masks.
  It reads like a "menu lockout" or "demo mode" flag; that is a guess at
  confidence 40, so it is described here and not named.

## Cross-platform

| Function | PS2 (`SCES_547.48`) | PSP (`BOOT.BIN`) |
| --- | --- | --- |
| `Input_BuildState` | `0x00202100` | `0x0894eec4` |
| `Input_IsPressed` | `0x00202500` | `0x0894f1cc` |
| `Input_ConsumePress` | `0x00202560` | `0x0894f22c` |
| `Input_ParseButtonName` | `0x002025d8` | `0x0894f2a0` |
| `Input_GetAxis` | `0x00202480` | closest is `Input_ReadAnalog` `0x0894ebd0`, which also polls |
| `Input_IsHeld` | `0x00202448` | not located |
| `Input_IsReleased` | `0x00202580` | not located |
| `Input_ConsumeRelease` | `0x002025a8` | not located |
| `g_input` | `0x00302ec0` | `0x08b31780` |

## History

- 2026-07-27: first pass. Mapping 90 from an exact raw-mask-for-raw-mask match
  with the PSP table on every shared button; `cancel`/`backward` divergence and
  the whole-mask `Input_ConsumePress` recorded as findings rather than
  reconciled.
