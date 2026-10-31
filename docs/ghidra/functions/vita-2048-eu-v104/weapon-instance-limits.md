# 2048: one Quake, eight LeachBeams, and the LeachBeam press spends the pickup

**Binary:** `eboot.elf` (`/2048/eboot-vita-2048-eu-v104.elf`, patch v1.04), ARM Thumb, image base
`0x81000000`. Read 2026-10-31, lane `hd-quake-leach-instances`, statically with the Ghidra bridge.
The question and the HD reading:
[`../ps3-hdfury-eu/weapon-instance-limits.md`](../ps3-hdfury-eu/weapon-instance-limits.md);
Omega, which agrees with this page:
[`../ps4-omega-eu/weapon-instance-limits.md`](../ps4-omega-eu/weapon-instance-limits.md).

**Result: differs from HD on the LeachBeam** (8 live at once against HD's 1). It matches HD on the
Quake and on the spend. **Omega: checked, applies, not wired**: the same pool of eight, the same
`< 8` guard after the spend, the same single Quake; this project runs HD's one beam on both, and
the wiring is open in the handover thread on the LeachBeam press at the limit.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x81202568` | `QuakeManager_Construct` | 85 | Writes `"Backend/Weapons/QuakeManager.cpp"` (`0x814a220c`, its only reference) as its tag; allocates one `0x54`-byte object into `+0x68` (`param_1[0x1a]`). |
| `0x812028f8` | `QuakeManager_Fire` | 80 | `q = mgr + 0x68; if (q->busy (+0x48) == 0) { slot+500 &= ~8; slot+0x1f8 = -1; ... }`: HD's guard, slot untouched while busy. |
| `0x811ed368` | `LeachBeamManager_Construct` | 85 | Writes `"Backend/Weapons/LeachBeamManager.cpp"` (`0x8149cdf8`, its only reference); builds **eight** `0xb7a0`-byte beams in `do { ... } while (uVar5 < 8)` into `+0x80 + 4 i`; zeroes `+0xa0` (live count) and `+0xa4`. |
| `0x811ee2c4` | `LeachBeamManager_Fire` | 85 | `mgr+0x30 |= 2; slot+500 &= ~0x8000; slot+0x1f8 = -1;` **then** `if (mgr->live (+0xa0) < 8) { b = mgr+0x80[live]; ...arm, locked 0x811e955a / unlocked 0x811e97aa...; live += 1; }`. |

`0x8120205c` (Quake age against lifetime; on expiry clears the busy byte `+0x48`) is the Quake
update. `0x811edc8e` walks the beams and swap-removes a retired one, decrementing `+0xa0`: HD's
`LeachBeamManager_Update`. Both are read in outline, not renamed (no second check).

## The two laws

- **Quake (80):** one object; a press while it is busy writes nothing, so the pickup is kept and
  slot bit `8` stays set (HD's latch, the same open live check).
- **LeachBeam (85):** up to eight live in a race, from any craft. A press is spent before the
  limit check, so a ninth press while eight are up spends the pickup and fires nothing.
