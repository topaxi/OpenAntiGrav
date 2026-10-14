# HD/Fury: the `MPTimeTrial` race manager turns craft-to-craft collision off (2026-10-14, `mp-collision-re`)

**Binary:** `EBOOT.elf` (HD/Fury, PS3, EU), `PowerPC:BE:64`, image base `0`. Static only: HD's
online Time Trial cannot be started here, so nothing below is above 84.

## Answer

Yes. `MPTimeTrial_RaceManager`'s constructor stores `0` to a byte of the collision world,
`collision + 0x17778`. `Collision_Construct` stores `1` there, no other function writes it, and
`Collision_ProcessPairs` reads it in exactly one place: the pair case for two type-3 (oriented
box) objects, which is the craft collider. Craft against the track mesh does not read it. This is
Pulse PSP's law ([mp-time-trial.md](../psp-pulse-usa/mp-time-trial.md)), the same shape in a
different binary with different offsets.

| Claim | Evidence | Conf |
| --- | --- | ---: |
| The gate is the byte at `collision + 0x17778` (`addis rX, world, 1` then offset `0x7778`) | `Collision_Construct` (`0x00034aa0`): `addis r29,r25,0x1` at `0x00034ad4`, `li r0,1` at `0x00034b20`, `stb r0,0x7778(r29)` at `0x00034b30`. The older [collision.md](collision.md) text writes `+0x7778`; the `addis +1` makes it `+0x17778` (correction note appended there) | 84 |
| `MPTimeTrial` clears it | `0x00041248` (`stb r0,0x7778(r9)` at `0x000412d0`, `r9 = *slot + 0x10000`) and its twin `0x00041780` (`stb` at `0x00041808`). Both chain to `MPRaceManager_Construct` (`0x00045178`), store `MPTimeTrial_RaceManager.cpp` at `+0x30`, and load `Arcade_HUD.xml` with HUD flags `0x15f`, the same set as Pulse's mode 17 | 84 |
| The cleared object is the collision world | the slot the constructor reads is TOC `0x008a6250`, holding `0x009389e0`; `Collision_Construct` writes `this` to `0x009389e0` (`stw r31,0x28(r30)` at `0x00034b40`) and to `0x009389e4` (`g_CollisionWorld`, [physics.md](physics.md)) | 84 |
| Nothing else writes it | `search_instructions` `stb` with `0x7778`: four hits in the whole program: `Collision_Construct`, `0x00034438` (its byte-identical sibling), and the two `MPTimeTrial` constructors. `MPArcade` (`0x0003d620`), `MPElimination`, `MPNitro`, `MPTournament` and every `SP*` manager leave it at `1` | 84 |
| One reader: the type-3 against type-3 case | `Collision_ProcessPairs` (`0x00038428`) read raw end to end (2,016 bytes): the only `7778` displacement is `88097778` at `0x00038b04`, `lbz r0,0x7778(r9)` with `r9 = world + 0x10000` (`3d3e0001`), then `cmpwi r0,0 ; beq` to the skip-this-pair label. It sits in the type-3 landing block after `cmpwi r10,3 ; bne`. Ghidra's own `lbz` search finds none because that block was never disassembled; the raw bytes are the evidence | 84 |
| Type 3 pairs are craft against craft | the gated call is `0x00035760(collision, index1, index2)`: a 15-axis separating-axis test (three axes of each box and the nine cross products) over each object's half-extents at `+0x190`, `+0x194`, `+0x198`, with a contact written on overlap; both objects need their `+0x70` byte set. `Ship_Construct` (`0x000ddd58`) allocates a `0x1a0` collider, runs `0x0003a320(obj, 5)` (sets the vtable, `+0x78 = 5` and **`+0xc = 3`**, the shape type the dispatch reads) and `0x0003a1a8` (stores the three half-extents), then `Collision_AddObject` (`0x000dece4`). Those two setters have exactly two callers, `Ship_Construct` and `Ship_ConstructAlt_q` | 84 |

Craft against track is untouched: `(1, 3)` and `(3, 1)` go to `Collision_TestMeshObb` with no
test of the byte. In HD, spheres (type 5) are tested against the mesh only, so the byte
switches off **only** craft against craft.

## Names

| Address | Name | Conf | Why |
| --- | --- | ---: | --- |
| `0x00035760` | `Collision_TestObbObb` | 82 | the 15-axis box-box test the gate leads to, read from the decompile; the older page left it unnamed as "not a geometry test" |
| `0x0003a320` | `Collision_ConstructObb` | 80 | sets `+0xc = 3`, the type the `(3, 3)` case dispatches on; called with `5` by both ship constructors |
| `0x0003a1a8` | `Collision_SetObbHalfExtents` | 78 | writes `+0x190`, `+0x194`, `+0x198`, the three values `0x00035760` multiplies by its axes |

Not named: the `MPTimeTrial` constructors `0x00041248` and `0x00041780`.
[race-manager.md](race-manager.md) leaves the leaf constructor pairs unnamed because the two
members of a pair cannot be told apart (its "constructor pair" section); the attribution is
recorded here and nowhere else changes.

## What this means for the port

HD agrees with Pulse: the online Time Trial has craft pass through each other and the track
still stops them, which is what `Mode::craft_collide` already does on every title. No port
change. HD's online mode needs the `MPRaceManager` family and a network layer this project does
not have, so the local split Time Trial remains "Pulse's rule, **chosen**".

Not read: whether the `0x40f00` mode manager sets anything else, and what `0x00045178` does
that `MPArcade` shares (both construct the same base). A live check needs an online HD session.
