---
status: re-first
titles: [hd, 2048, omega]
updated: 2026-11-26
summary: "HD builds no ShurikenManager but our HD Eliminator hands out the Shuriken; read the pickup grant, then drop it"
size: S (45)
difficulty: medium (50)
---

# HD throws no Shuriken, and our HD Eliminator does

Evidence: [open-air-arms.md](../../docs/ghidra/functions/ps3-hdfury-eu/open-air-arms.md)
("HD builds no Shuriken", confidence 88).

## Open

- HD's `weaponstats_Elimination.xml` gives the Shuriken nonzero odds, and `oag_weapons` hands it out
  from them. Where HD's pickup grant drops it is unread, so what a pad gives instead is unknown.
- The Repulser's manager slot (`WeaponManager + 0x7c`) is zeroed by the same store and read `0` live.
  Whether HD reaches the Repulser another way was not examined.
- 2048: no code reference to `ShurikenManager.cpp` or `RepulserManager.cpp` (55, a census). Omega:
  neither manager tag exists (60, a census). Neither title's weapon manager was read.

## Next Steps

1. Read HD's pickup grant for how it treats the `Shuriken` and `Repulser` odds rows
   (`ps3-hdfury-eu`, static, about 2 hours).
2. Then make the pool per title (`Title` data), so an HD Eliminator pad draws what HD's does.
