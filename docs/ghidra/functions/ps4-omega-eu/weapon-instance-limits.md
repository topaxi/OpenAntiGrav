# Omega: one Quake, eight LeachBeams, and the LeachBeam press spends the pickup

**Binary:** `eboot.bin` (`/omega/eboot-ps4-omega-eu.bin`), x86-64, image base `0x01000000`. Read
2026-10-31, lane `hd-quake-leach-instances`, statically with the Ghidra bridge. The question and
the HD reading it is checked against:
[`../ps3-hdfury-eu/weapon-instance-limits.md`](../ps3-hdfury-eu/weapon-instance-limits.md).
2048's reading, which this one agrees with:
[`../vita-2048-eu-v104/weapon-instance-limits.md`](../vita-2048-eu-v104/weapon-instance-limits.md).

**Result: checked, differs from HD on the LeachBeam** (8 live at once against HD's 1). It matches
HD on the Quake (1, same guard) and on what a LeachBeam press does at the limit (spends the
pickup). **Against 2048: checked, applies, not wired**: the same pool of eight and the same guard
order; this project runs HD's one beam on both, and the wiring is open in the handover thread on
the LeachBeam press at the limit.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x013674e0` | `QuakeManager_Fire` | 80 | HD's `QuakeManager_Fire` line for line: `q = mgr + 0xc8; if (q->busy (+0x80) == 0) { slot+0x218 = -1; slot+0x214 &= ~8; ...; Quake_Init(...) }`. Busy: nothing written. Only code caller `0x0137641c` in `WeaponManager_DispatchFire`, on slot bit `8`. |
| `0x01366150` | `Quake_Init` | 70 | Called by both Quake arms (`0x013675c7`, `0x01366b90`) with the Quake object, the craft index, the craft and its matrix, HD's `Quake_Init` argument list. |
| `0x01366760` | `QuakeManager_Update` | 65 | QuakeManager vtable `0x01916b58` slot 3. Returns at once unless `q->busy`; advances the age global `0x020e4398` by `dt`; past the lifetime `0x020e4490` it clears flag bits and `q + 0x80` (busy) to 0. |
| `0x01372ee0` | `WeaponManager_DispatchFire` | 75 | HD's dispatcher shape: per slot, bit `0x4000`, `0x100`, `0x40`, `8` (Quake, `mgr + 0xd0`), `4`, `2`, then **bit `0x8000` with the LeachBeam fire inlined** (`mgr + 0xd8`), `0x10000`, `0x20000`, `0x80000`. |

`0x01366ae0` (QuakeManager vtable slot 10 through `0x01366ad0`) arms the Quake behind the same
busy guard from a message record, no slot: HD's `0x001485a0` counterpart, below 50, not renamed.

## The Quake: one, guarded as on HD (80)

`QuakeManager_Construct` (`0x01366f70`, already named on [weapons.md](weapons.md)) allocates one
`0x88`-byte object into `mgr + 0xc8` (`param_1[0x19]`). `QuakeManager_Fire` returns with the slot
untouched when its `+0x80` is set, so the pickup is kept and slot bit `8` stays set for the
dispatcher's next pass: HD's latch, same reading, same open live check (Omega has no memory
readout in shadPS4).

## The LeachBeam: a pool of eight (85)

The composite race member `0x013781e0` ([weapons.md](weapons.md), "LeachBeamManager.cpp and
MineManager.cpp share one composite function") allocates the `LeachBeamManager.cpp`-tagged object
at `0x130` bytes. It then builds **eight** `0xcb00`-byte beams in a loop (`while (lVar19 != 8)`),
storing them at `mgr + 0xe8 + 8 i`. The live count is `mgr + 0x128`, zeroed at the top of the
constructor.

The fire, inlined into `WeaponManager_DispatchFire` (the `(short)flags < 0` test, that is bit
`0x8000`):

```c
mgr = dispatcher->leach;                        // + 0xd8
mgr->flags |= 2;                                // + 0x60
slot->state = -1;                               // spent, unconditionally
slot->flags &= ~0x8000;
if (mgr->live < 8) {                            // + 0x128
    b = mgr->beams[mgr->live];                  // + 0xe8 + 8 * live
    ...arm b; locked 0x0133ff60 / unlocked 0x01340200 by slot's lock index == -1...
    mgr->live += 1;
    ...
}
```

So up to eight LeachBeams can be up in a race at once, each from any craft, and a press with
eight up spends the pickup and fires nothing.

One extra branch after the arm has no HD counterpart. When the global `0x01f999e4` is above
`0x16` (the value `Rocket_Construct` compares against `0x17` to choose `Data/Weapons2048` over
`Data/Weapons`) and the new beam's `+0x80 == 1`, the shooter's slot gets bit `0x10` and a value
from the active stats `+0x1d0` at `+0x2e8`/`+0x2ec`. What that is was not read: below 50.

## Not checked here

- Whether `LeachBeamManager_Update`'s retire path on Omega swap-removes as HD's does (the count's
  decrement); the fire's `live < 8` and the eight constructed objects settle the limit without it.
- Omega's `Ship_FireHeldWeapon` was not located, so "no upstream busy gate" is HD's and 2048's
  reading, inherited here.
