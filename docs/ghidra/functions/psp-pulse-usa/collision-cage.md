# The collision-cage effect pool (`cage_collision_curved.vex`)

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`. Names are applied from [names.tsv](names.tsv). Written 2026-10-02
from static reading of `boot.dis` plus a live PPSSPP v1.20.4 session on
`03_Track` (a `collisionCageEnabled` circuit). The decode of VFPU code came from
PPSSPP's own `memory.disasm`, not `objdump` (which reads `lv.q`/`sv.q` as Octeon
`bbit`).

## What is established

| Claim | Evidence | Confidence |
| --- | --- | --- |
| `0x0883170c` (`CageEffect_Construct`) is the only code that references the vex path: `lui 0x27 / addiu 0x6b98` at `0x0883170c+0x64` builds the string `Data\visual_effects\cage_collision_curved.vex`, passed to the model loader `jal 0x08943330` with flags `1, 0`; it then sets bit 2 of model+0x14 and builds four 0x90-byte scene objects in a ring at `this+0x734..0x740`, cursor at `this+0x744` | `boot.dis` `2d70c..2d888`; live: singleton at `[0x08ab0c88]` has four slot pointers at `+0x734`, each a scene object with vtable `0x08a6ccfc`, the vex name pointer `0x08a7abe0` and an identity 4x4 at `+0x50` (parked at the origin, nothing placed) | 75 |
| It is built once per race load: one caller, `0x08829640` inside `RaceManager_Construct`, and the singleton pointer moved from `0x09a0a3d0` to `0x09a0c9a0` across a `psp-drive.py restart` | caller `jal 0x2d70c` at `0x25640`; two live reads | 80 |
| `g_cage_effect` (`0x08ab0c88`) holds that singleton; `0x08ab0c84` the loaded model (`0x08cf69f0`); `0x08ab0c8c` a byte "cage usable" flag; `0x08ab0c80` a float set to the tick `dt` (0.016689) by the setter `0x08831980`, called from `0x08829938` | live read, flag 1 and the float on `03_Track` | 80 |
| `0x08832b0c` (`CageEffect_Spawn`) is the spawn: for each of the four slots it calls `0x0883318c` (slot age) and reuses a slot whose age is in (0.1, 0.25) and which is within 7.0 of the new contact point, otherwise takes the ring slot `[this+0x744]`; either way it calls `0x08832fb0(slot, contact frame, a2, a3)` to pose and start it, and advances the cursor modulo 4 | static, `boot.dis` plus `memory.disasm` of `0x08832b0c..0x08832d40`. **Never observed to fire live** (breakpoint at its entry silent through a wall scrape and four off-track `place` teleports) | 58 |
| `0x08831aa4` is a per-craft query, reached through the wrapper `0x0883191c` from `0x08842384` (inside the per-craft update, 5.9 KB past `Craft_Construct`) every tick while the flag `0x08ab0c8c` is set; it casts a 500-unit query (`AiTrack_UpdateCursor`, `0x0887e464`) from craft+0xb0, returns **2 when the hit is further than 200.0 (`0x4348`)**, 127 for "nothing", and on a nearer hit tests the segment against the track data and, within 15.0 (`0x8a7abd8`), calls `CageEffect_Spawn` then `Body_ResolveContact` | static; live: returns 127 on every one of ~200 sampled ticks on `03_Track`, including a wall scrape | 55 |
| The caller treats a return of 2 as event 3: `jal 0x08844100(craft, 3)` at `0x08842390`. This is the likely home of the 200-unit airborne rule the leaving-the-track page lists as measured but unread | static only | 50 |
| The sphere in `yaw.ppdmp` (prims 627/628, world translation about `(-124.6, 2.3, 480.4)`, a non-uniform scale of about 40/40/81) is a **posed instance of this model**, not the placed-at-origin pool state | prim census of `yaw.ppdmp`; the pool instances here sit at the identity | 65 |

`0x0883198c` (`Collision_SweepSegment`, named elsewhere) calls `0x08832134` through
the wrapper `0x08831948` as its fallback; `0x08832134` also fires every frame, from
`Camera_UpdatePlayerView`'s sweep (`ra=0x08831974`). It was not decoded.

## What is not established

- **What puts the sphere in the player's frame.** `CageEffect_Spawn` is the only
  path seen that poses an instance, and it never fired. The trigger as read is a
  segment hit on the cage geometry inside 15 units; no pose reached it.
- Whether the spawn is ever reached on a circuit without the attribute. The
  attribute is read at `0x088c40a8` into the definition record at `+0x16f`; no
  reader of that byte was found, and `CageEffect_Construct` is unconditional (the
  singleton exists on `03_Track`, which has the attribute; a circuit without it
  was not booted).
- The pose and fade written by `0x08832fb0`, and the scale seen in the dump.

## Next addresses

`0x08832fb0` (slot pose and start), `0x0883318c` (age), `0x08832134` (the sweep's
fallback), the consumer of definition `+0x16f`, and a PPSSPP execution breakpoint
on `0x08832b0c` while a craft is driven into the track's leaky edge on a circuit
in the yaw dump (`07_Track`).
