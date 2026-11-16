---
status: re-first
titles: [hd, 2048, omega]
updated: 2026-11-02
summary: "Quake press stays armed, wired on all four titles; left: FUN_006778d8 after a press, HD live latch"
size: S (30)
difficulty: medium (45)
---

# After a press: `FUN_006778d8(..., 0x1d)`, and HD's live Quake latch

The Quake half landed 2026-11-02 (`quake-armed`): a press made under a live wave sets
`Held::quake_armed` and `Race::fire_armed_quakes` launches it the tick after the wave ends, on every
title. Pulse PSP was seen live (confidence 90, `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
"the armed press, seen live"); HD, 2048 and Omega are the same static reading (65). The LeachBeam half
landed 2026-10-31.

## Open

1. **Unread**: `FUN_006778d8(..., 0x1d)` after the HD call in `WeaponManager_DispatchFire` runs on every
   press, the spent LeachBeam one included.
2. **HD's own latch, live (65 -> 85)**: the RPCS3 recipe under "Open" in
   `docs/ghidra/functions/ps3-hdfury-eu/weapon-instance-limits.md`.
3. **An absorb with a Quake request armed** is unread on every title; the original probably fires from an
   empty slot, ours cancels it (`Held::take`, chosen).

## Next Steps

1. Read `FUN_006778d8` (Ghidra, `/hdfury/EBOOT-ps3-hdfury-eu.elf`), 1 hour.
