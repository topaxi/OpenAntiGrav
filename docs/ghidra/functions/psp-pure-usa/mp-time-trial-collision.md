# Pure: the `MPTimeTrial` race manager turns craft-to-craft collision off (2026-10-14, `mp-collision-re`)

**Binary:** `PSP_GAME/SYSDIR/BOOT.BIN` (Pure, PSP, USA), image base `0x08804000`. The EU pressing
is the same code with different addresses, see
[../psp-pure-eu/mp-time-trial-collision.md](../psp-pure-eu/mp-time-trial-collision.md). Static
only, and the PSP relocations are not applied in this database (workflow.md): a `jal` or a
`lui`/`addiu` constant reads as an offset from the image base, so `jal 0x00020fbc` is
`0x08824fbc`. Nothing here is above 84.

## Answer

Yes, the same law as Pulse. `MPTimeTrial_Construct` (`0x08815e1c`) stores `0` to the collision
world byte `world + 0x3664`. The world's constructor (`0x08822490`) stores `1` there, and
`Collision_DispatchPair` (`0x08824e34`) runs `Collision_BoxAgainstBox` (`0x08824fbc`) for a pair of
type-3 bodies only when the byte is set.

| Claim | Evidence | Conf |
| --- | --- | ---: |
| The constructor at `0x08815e1c` is Pure's `MPTimeTrial` race manager | it passes `0x24135c` to its tag call, which is `0x08a4535c`, the string `c:/Work/Wipeout/Code/Backend/General/MPTimeTrial_RaceManager.cpp` (read from memory); it ORs `1, 2, 4, 8, 0x10, 0x40, 0x100` into the HUD flag word, which is `0x15f`, Pulse's mode 17 set; it allocates `0x1e8`, `0x6c` and `0x12f0` bytes around `Hud_Construct`-shaped calls; one caller, `0x0880dbe0` in `0x0880d778` | 82 |
| It clears `world + 0x3664` | `sb zero,0x3664(a0)` at `0x08815e70`, on the object `_DAT_00288a90` points to | 82 |
| The world constructor sets it | `0x08822490`: `li s1,1` at `0x088224f8`, `sb s1,0x3664(s0)` at `0x08822518`; the same function stores `this` to the global at `0x08822510` and the object count field `+0x3668` to zero | 80 |
| Those are the only two writers | `search_instructions` `sb` with `0x3664`: two hits in the program | 82 |
| One reader, the type-3 pair | `lbu a0,0x3664(s2)` at `0x08824f88` (the only `lbu` of that offset) in `0x08824e34`, reached when both bodies' shape type, read through each body's vtable (`lw 0x78` then the slot), equals `3` (`bne a0,s3` at `0x08824f80`, `s3 = 3`); a zero byte branches to the epilogue, otherwise `jal 0x00020fbc` = `Collision_BoxAgainstBox` | 82 |
| Type 3 is the craft collider | [wreck-collision.md](wreck-collision.md): `Collision_BoxAgainstBox` refuses a pair when either collider's `+0x68` byte is zero, and `Ship_SetState` clears that byte in the destroyed states; the dispatch has the same instruction shape as Pulse's (`0x08816eac`) | 78 |

Pulse's offsets for the same three sites: `world + 0x5464`, `FUN_088151bc`, `0x08822e34`
([../psp-pulse-usa/mp-time-trial.md](../psp-pulse-usa/mp-time-trial.md)). Both title builds put
the byte at a different offset because the world structure differs; the roles line up
one for one.

## Names

| Address | Name | Conf |
| --- | --- | ---: |
| `0x08815e1c` | `MPTimeTrial_Construct` | 82 |
| `0x08824e34` | `Collision_DispatchPair` | 80 |

The name `Collision_DispatchPair` is Pulse's (`0x08816eac`, 85); Pure's twin is the same 96
instructions with the call targets relocated by the file layout (EU diff: 96 of 96 equal).
Not named: the world constructor `0x08822490` (its identity rests on the byte and the global
store only).

## Port consequence

None. Pure's local split Time Trial already takes Pulse's rule (chosen), and Pure's own online
mode agrees with it.
