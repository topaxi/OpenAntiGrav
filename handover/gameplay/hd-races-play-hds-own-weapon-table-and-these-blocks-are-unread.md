# HD races play HD's own weapon table; these blocks are authored and unread

2026-10-06. The brief's premise ("an HD race most likely fires Pulse's table") was false: `oag_hd::TITLE.weapons` has named HD's own `Data\XML\WeaponStats_Race.xml` and `_Elimination.xml` since 2026-10-03, and they decode with the Pulse roster. `crates/tables/tests/hd_weapons_ground_truth.rs` now pins every shipped copy (HD `DATA00`/`DATA02`/`DATA05`, Omega `data00`) and that a race is served `DATA00`'s. See [weapon-stats.md](../../docs/formats/weapon-stats.md#wipeout-hd-and-fury-their-own-tables-and-which-copy-a-race-reads).

## Open

- Three race copies disagree on the LeachBeam and the Bomb's shot count; a race reads `DATA00`'s because that is our mount order. What a PS3 loads is unmeasured (same question as `skin.xml`'s copies).
- Authored and unread: the Cannon's `recharge_time` and the Detonator Cannon's `round_recharge_time`/`recharge_pause`/`num_rebounds`; the Bomb's `number_of_shots_to_destroy`; the `LightBarrier` block (dropped silently by `Weapon::from_type`, never in `skipped`); the `EMP` block; the whole `weaponstats_detonator.xml`.
- Omega's HD-named tables add a fifth `SuperPhantom` pickup class nothing reads.
- The HD weapon behaviour is Pulse's law, inherited, not measured on HD.

## Next Steps

- Decide whether `Weapon::from_type` should report an unknown `type` (LightBarrier, EMP) to the loader so the absence is visible.
- Read HD's Cannon recharge law (`CannonManager_Construct`) and the Bomb's shot count if a Detonator or Zone-Battle mode is wired.
