---
status: ready-to-wire
titles: [pulse]
updated: 2026-10-30
summary: "A Plasma craft hit calls Plasma_SpawnDetonation twice in the original (struck craft, then bolt); ours spawns one model blast"
---

# The Pulse Plasma's craft hit spawns two detonations

2026-10-30 (`pulse-plasma-fidelity`). The law is read at confidence 88 in
`docs/ghidra/functions/psp-pulse-usa/plasma.md`, "A craft hit spawns two detonation objects":
`Plasma_SweepCraftHit` (`0x0886afb8`) calls `Plasma_SpawnDetonation` (`0x0886ac88`) at the struck
craft's body position, and `Plasmas_Update`'s teardown pass (`0x0886b490`) calls it again at the
bolt's own last position, in the same tick. Ours: `Race::blast_for` returns one `(Trigger, Vec3)`
and `ignite_blast` spawns one model blast (`crates/raceplay/src/weapons/visuals.rs`).

Not captured on the original (Time Trial has no opponent, the bolt is not aimed), so there is no
picture of the pair yet.

## Open

- Wire a second blast (model blast and `WO_PLASMA_FLASH`) for a Plasma impact with `struck: Some(_)`, at the struck craft's body position, with a test; about 1 hour.
- Capture it on the original: a race with a craft ahead on the bolt's line, `psp-weapon-pair.py plasma`.

## Next Steps

- Spawn an `oag-wire` member on the first item; check Omega/HD (`WeaponExplosions_Start`) for the same double call before generalising.
