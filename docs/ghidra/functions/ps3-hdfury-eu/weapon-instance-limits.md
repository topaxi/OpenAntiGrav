# How many Quakes and LeachBeams can be live at once, and what a press does when that limit is reached

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`), PowerPC, big endian. Read 2026-10-31,
lane `hd-quake-leach-instances`, statically, with the Ghidra bridge plus raw disassembly (capstone,
PPC64 BE) where Ghidra marks `_opd_FUN_00126c38` non-returning and cuts a body short. The same
question was read on Pulse PSP, 2048 (Vita v1.04) and Omega (PS4) in the same pass; their own
pages are
[`../ps4-omega-eu/weapon-instance-limits.md`](../ps4-omega-eu/weapon-instance-limits.md) and
[`../vita-2048-eu-v104/weapon-instance-limits.md`](../vita-2048-eu-v104/weapon-instance-limits.md).
Pulse's fire functions are on
[`../psp-pulse-usa/cannon-quake-leachbeam.md`](../psp-pulse-usa/cannon-quake-leachbeam.md).

## The answer

| | Pulse PSP | HD / Fury | 2048 | Omega |
| --- | --- | --- | --- | --- |
| Quakes live at once | 1 per race | **1 per race** (90) | 1 per race (80) | 1 per race (80) |
| Quake press while one is live | pickup kept, request left armed | **pickup kept, request left armed** (85); fires on its own once the old wave ends (65) | same shape (80) | same shape (80) |
| LeachBeams live at once | 1 per race | **1 per race** (88) | **8 per race** (85) | **8 per race** (85) |
| LeachBeam press while the limit is reached | **pickup spent, nothing fires** (88) | **pickup spent, nothing fires** (85) | pickup spent, nothing fires (85) | pickup spent, nothing fires (85) |

"Per race" means one manager per race shared by every craft, not a per-craft limit: the slot
loop in `WeaponManager_DispatchFire` below passes each craft's slot to the same manager pointer.

**Verdict: HD matches Pulse** on both counts. 2048 and Omega are more permissive on the LeachBeam
only (a pool of eight). And on every title the LeachBeam press that finds the limit reached
**spends the pickup**. This project's player path keeps it, which contradicts Pulse's own law as
well (see "What this project does" below).

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x0012b2c0` | `WeaponManager_DispatchFire` | 75 | Walks the per-craft slot records (stride `0x230` from `this + 0x90`) and, per set bit of slot `+0x200`, calls that weapon's manager: bit `0x8` -> `QuakeManager_Fire(this->+0x74, slot, craft)` at `0x0012bb7c`, bit `0x8000` -> `LeachBeamManager_Fire(this->+0x78, slot, craft)` at `0x0012ba8c`. The only code caller of either. Pulse's `Weapons_DispatchFire` (the call to `Weapon_FireQuake` at `0x08861b60`) has the same role. |
| `0x000ea0c8` | `Ship_FireHeldWeapon` | 75 | Pulse's `Ship_FireHeldWeapon` (`0x08844ae8`) shape: needs the slot `+0x204 != -1`, refuses only state `6` with bit `0x800`, then calls `Weapon_FireFromSlot` (`0x0012d970`) with the lock target when craft `+0x6a60 != -1` and `+0x6a64 & 1`. **No check of any weapon manager's busy state.** |
| `0x0012d970` | `Weapon_FireFromSlot` | 85 | Already cited by [quake.md](quake.md) and [leach-beam-strips.md](leach-beam-strips.md): the sixteen-way jump on slot `+0x204`. State 2's handler (`0x0012daa0`) only ORs `0x8` into `+0x200`; state 10's (`0x0012dba8`) only ORs `0x8000` and stores the lock (`+0x1a8`, `+0x1ac`). Neither tests a manager. |
| `0x00147ca0` | `QuakeManager_Update` | 75 | QuakeManager vtable `0x008647a8` slot 3 (OPD `0x00877498`): `if (quake->busy) Quake_Tick(quake)`, returns 1. |
| `0x00155e70` | `Quake_Tick` | 75 | Raw read (Ghidra truncates it): walks the span table of the singleton at TOC `-0x21f0` (records `0x80` bytes, live byte `+0x7e`), calls `Quake_UpdateSpan` (`0x002d3760`) on each live one; when none is left, clears flag bits and `stb 0, 0x4c(quake)` at `0x00155f6c`. The only writer that frees the busy byte. |

`0x00147bb0` and `0x001485a0` (QuakeManager vtable slot 14, OPD `0x008774f8`) arm the same Quake
object behind the same `+0x4c` guard from a two-word record (`craft index`, `id`) without touching
any slot: a remote-player or replay path is the likely reading, below 50, not renamed.

## The Quake: one object, a guard that leaves the request armed

**Storage (90).** `QuakeManager_Construct` (`0x001483a0`) allocates one `0x58`-byte object and
stores it at `manager + 0x6c`; there is no array and no count. (`manager + 0x4c..` holds the
per-craft `WEAPON SYSTEM %d` lookups, not Quakes.)

**The guard (90).** `QuakeManager_Fire` (`0x00147d18`) is one `if`:

```c
q = manager->quake;                     // + 0x6c
if (q->busy == 0) {                     // + 0x4c, set by Quake_Init
    slot->flags &= ~0x8;                // + 0x200
    q->active = 1; q->owner = craft;    // + 0x40, + 0x44
    slot->state = -1;                   // + 0x204: the pickup is spent here, and only here
    ...Quake_Init(q, craft, ...); cue(2);
}
return;                                 // busy: nothing written at all
```

**A press while a wave is live keeps the pickup (85).** With `busy != 0` the function writes
nothing: slot `+0x204` still holds state 2 (the HUD keeps showing the Quake) and bit `0x8` of
`+0x200` stays set.

**And the request stays armed (65).** `WeaponManager_DispatchFire` calls `QuakeManager_Fire`
whenever that bit is set, every update. Nothing in the dispatcher clears bit `0x8`. A sweep of
the binary for the `rlwinm rX, rY, 0x1c, 0x1, 0x1f` form that clears bit 3 (40 hits) finds
`QuakeManager_Fire` as the only one on a slot `+0x200` in the weapon code. Two hits on generic
pointers (`0x001118cc`, `0x0014c658`) were not resolved. So **the second press fires a new wave on
its own, the first update after `Quake_Tick` frees the busy byte**, which is when the last span ends
(`g_quake +0x70` lifetime, 5.0 s). Static only: not seen live. Read recipe for a live check is under
"Open" below.

Pulse has the same shape: `Weapon_FireQuake` (`0x0886c600`) returns on `q->active != 0` before it
writes `held = -1` or clears `fire_flags & 8`, and Pulse's only other writer of `+0x1b8` that
could clear bit 3 is the whole-slot reset `FUN_088629f8`. That is a store search on `sw ... 0x1b8`,
with the weapon fire handlers, `Weapon_RequestFire` and that reset as the hits.

## The LeachBeam: one beam, and the pickup goes before the check

**Storage (88).** `LeachBeamManager_Construct` (`0x0013d520`) allocates one `0xc9c0`-byte beam
(`LeachBeam_Construct`) into `manager + 0x84` and zeroes `+0x88` (the live count) and `+0x8c`;
the manager is `0x90` bytes, so `+0x84` is a one-entry array. `LeachBeamManager_Update`
(`0x0013d7a8`) walks it as a pool: it loops `i < +0x88` over `+0x84 + 4 i`, and on a retired
beam (`+0x40 & 4`) decrements `+0x88` and swaps the last live entry into place. That is a pool's
shape over capacity one. **Do not read the loop as a pool of N**: only one beam is ever built,
and the fire arms only at count 0.

**The press (85).** `LeachBeamManager_Fire` (`0x0013ccd0`):

```c
slot->state = -1;                       // + 0x204: spent, unconditionally
manager->flags |= 2;                    // + 0x34; Update clears it when + 0x88 == 0
slot->flags &= ~0x8000;                 // + 0x200
if (manager->live == 0) {               // + 0x88
    b = manager->beams[0];              // + 0x84
    ...arm b, LeachBeam_InitLocked / InitUnlocked (0x00154558 / 0x00154360)...
    manager->live += 1;
}
```

**So a press while any craft's beam is up spends the pickup and fires nothing.** Nothing upstream
refuses the press first: `Ship_FireHeldWeapon` (`0x000ea0c8`) and the state-10 handler
(`0x0012dba8`) test no manager state. The manager's `+0x34 & 2` flag is set here and cleared by
`LeachBeamManager_Update`; neither of the two paths above reads it. Nothing sounds on such a
press: HD's press-time switch case 10 (`0x000ea6f4`, table at TOC `-0x441c`) and Pulse's
`Ship_FireHeldWeapon` case 10 only bump stats counters, and HD's LeachBeam cue `0x0012d820(2)` is
inside the armed branch. The dispatcher's `FUN_006778d8(..., 0x1d)` after the call runs on every
press; what it drives is unread.

Pulse's `Weapon_FireLeachBeam` (`0x08866658`) is the same order, re-read this pass in the
disassembly: `sw a1(-1), 0x1bc(s1)` at `0x08866680` and the masked `sw` to `0x1b8` at
`0x08866688` come before `lw s2, 0x68(a0)` and the `beq` on the live count at `0x088666a8`.
`Ship_FireHeldWeapon` (`0x08844ae8`) has no busy gate either. The confidence is 88 (Pulse) and 85
(HD), two binaries agreeing on a direct decompile.

## What this project does

- **Quake**: one instance (`World::quake: Option<Wave>`). A press while busy keeps the pickup,
  which matches. The press is then **dropped** (`return`, nothing re-armed), where the original
  keeps it armed and fires when the wave ends (the 65 above).
- **LeachBeam**: one instance (`World::leach_beam: Option<Beam>`), which matches Pulse and HD. A
  press while busy **keeps** the pickup (`crates/raceplay/src/weapons.rs`, the `LeachBeam` arm).
  **That contradicts both originals**: they spend it. The opponents' hold-until-free
  (`fire_opponent_leach_beam`, labelled chosen) is a separate choice, written down as such.
- 2048 and Omega inherit the one-beam law from HD. Their own executables build eight.

## Open

- **The Quake latch, live (65 -> 85+).** Recipe on RPCS3 (`scripts/rpcs3-hd-weapon.py --state 2`,
  then the poll of `scripts/rpcs3-hd-quake-poll.py`): fire a Quake, write state 2 into the player's
  slot `+0x204` again while `g_quake + 0x140` (age) is under 5, press triangle, and read slot
  `+0x200 & 0x8` (expect it to stay set) and the age (expect a reset to near 0 when the first wave
  ends, with no further press).
- **The LeachBeam spend, live (85 -> 90).** After a beam fires and while manager `+0x88 == 1`,
  write state 10 into a clean slot and press: expect `+0x204 -> -1` with `+0x88` still 1. The
  `leach-beam-strips.md` warning applies: stale `0x800`/`0x2000` bits from earlier writes block
  state 10, so start from a fresh boot's slot.
