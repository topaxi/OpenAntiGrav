# A parked-player Eliminator cannot finish a five-kill event

2026-09-30, from the `eliminator-finish` lane. Evidence: `docs/gameplay/race-modes.md`
("Who is credited with a kill, and why this build's Eliminator does not finish") and
`docs/ghidra/functions/psp-pulse-usa/eliminator-kill-target.md`. Guarded by
`crates/game/tests/eliminator_finish_ground_truth.rs`, which finishes at a target of 2 only.

## Open

- **A 5-kill finish is not reached** with the player parked on `16_Track`: about 11 kills in
  five game-minutes over four seeds (5, 9, 16, 13), spread over seven craft. The original
  reached 5 in 85 s.
- **The field strings out**, 118 units at the start to 3,900 by minute six. Identical craft on
  one racing line; the original's weapon AI fires readily only at a craft inside 100 units ahead.
- **Unfired weapons.** Every opponent ended a run holding a pickup, and Eliminator refuses
  absorbing, so pads stop paying out. 79 % of fire consults found nobody ahead inside 120.
- **Wall deaths**: 9 of 20 deaths in one run had no weapon hit in the last second, and credit
  nobody, as in the original.
- **Why the pack knob works is not understood.** The leader easing (`eliminator_pack_scale`,
  chosen, not measured) doubled kills in the sample, but its parameters changed nothing.
- **`WeaponAi_DecideFireOrAbsorb` is not ported.** A probe showed the port alone would add no
  shots: its Eliminator skill index is 3 only with a craft ahead within 100, else 0, so it fires
  about once in 45 s. The original's shots come from a dense field, which its AI gets by cheating.
  Its `+0x52` predicted-path test and the fields `+0x44`, `+0x4c`, `+0x5c` are also unread.
- **Ghidra data rename rejected**: the bridge refused `g_eliminator_kill_target` for
  `0x08b30fb0` (Hungarian-prefix check). The function rename applied; the docs and `names.tsv`
  are authoritative for the data name.

## Next Steps

1. Find why the easing is insensitive to its parameters: log the scale per tick per slot.
2. Try a real hunting behaviour (chase the nearest craft around the ring) instead of a leader wait.
3. Look at wall deaths: why do opponents scrape lethally in Eliminator when a lone craft does not?
4. Only then port the WeaponAi decision, if density alone does not reach 5 kills.
