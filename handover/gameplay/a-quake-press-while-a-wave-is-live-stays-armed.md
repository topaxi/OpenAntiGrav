---
status: ready-to-wire
titles: [pulse, hd, 2048, omega]
updated: 2026-10-31
summary: "A Quake press while a wave is live stays armed on all four titles (ours drops it); the LeachBeam half landed 2026-10-31"
size: S (50)
difficulty: medium (50)
---

# A Quake press while a wave is live stays armed

Evidence: `docs/ghidra/functions/ps3-hdfury-eu/weapon-instance-limits.md` (the table for all four
binaries) and the 2026-10-31 section of `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.

The LeachBeam half of this thread landed 2026-10-31 (`leach-beam-pool`): a press at the limit spends
the pickup on every title, and `Weapons::leach_beams` is eight everywhere (2048, Omega measured; Pulse,
HD chosen against a measured one), see `docs/gameplay/weapon-lineage.md`. What is left:

## Open

1. **The Quake press stays armed (measure first, or wire at 65).** Ours drops a press made while a
   wave is up. The originals leave the request bit set, so the next wave fires the update after
   the first wave's last span ends (confidence 65, static, two binaries). Wiring it means a per-slot
   "Quake requested" flag that the fire retries every tick (one bool per craft, also a snapshot
   change). The live check that raises it to 85 or more is in the HD page's "Open" (RPCS3; on Pulse,
   PPSSPP: re-press with a wave live, read `craft+0x1b8 & 8` and the wave's age). Doing the live
   check first is recommended.
2. **Unread**: `FUN_006778d8(..., 0x1d)` after the HD call in `WeaponManager_DispatchFire` runs on every
   press, the spent LeachBeam one included.

## Next Steps

1. The RPCS3 check, 1 hour, then 1 hour to wire.
