# An AI craft cannot fire a Cannon here, and in the original it can

2026-09-08, reported from play by the maintainer: **AI craft do fire the
Cannon, at the player and at each other.** This build reproduces the opposite,
deliberately, off a reading that has now been falsified.

The reading is on
[`cannon-quake-leachbeam.md`](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md)
and it carries a correction banner. In short: `Cannon_UpdateReload`
(`0x0883f424`) gates on `record+0x16`, the "fire button held" flag;
`WeaponAi_Update` (`0x08851550`) and `WeaponAi_DecideFireOrAbsorb`
(`0x088518b4`) write `record+0x15` (press) and `record+0x17` (absorb) but were
found never to write `+0x16`; and an exhaustive `scripts/psp-relocate.py field
0x16` over the relocated image reported **zero byte stores**. On that basis
`oag_game::race::weapons` lets only the player's Cannon fire.

## Open

- **A producer of `record+0x16` exists and the search did not find it.** The
  play report is unambiguous and is about the ordinary case (opponents shooting
  each other, not a rare mode), so it is not a marginal counter-example.
- **The prime suspect is the search's own shape, and it is a known trap.**
  `field 0x16` looks for **byte** accesses. `+0x15`, `+0x16` and `+0x17` are
  three *adjacent* bytes, so a single `sh` at `+0x16`, an `sw` at `+0x14`, or a
  `memcpy`/struct assignment of the whole control record would set the held
  flag and never appear in a byte-store search. **"Zero byte stores" was
  reported as "zero stores"**, and those are different claims. This is a
  hypothesis about why the search missed it, raised from the search's own
  parameters; it carries **no confidence score** and has not been checked.
- **What is not in doubt**: `+0x16` is the held flag (confidence 90, six legs),
  the countdown is gated on it, and `PlayerInput_Update` (`0x0883ccd0`) writes
  the human's copy. Only "the human pad is the only producer" is refuted.
- **This build's behaviour is now a known deviation, not a faithful copy.**
  Anyone reading `race::weapons`'s Cannon arm should know it encodes a wrong
  conclusion until this lands.

## Next Steps

- **Re-run the field search for wider stores.** Halfword and word stores whose
  range covers `+0x16`, plus bulk copies into the control record. That is the
  cheapest thing that would explain both the exhaustive byte search and the
  play report, and it needs the Ghidra bridge on `psp-pulse-usa`.
- Look inside `WeaponAi_Update` and `WeaponAi_DecideFireOrAbsorb` first: they
  already hold a pointer to the record through `*(ai+0x10)` and already write
  two of its three adjacent flag bytes, so a wider store there is the obvious
  candidate.
- **Only once a producer is found**, let an AI-held Cannon fire in
  `oag_game::race::weapons`. Do not simply ungate it to match the play report -
  that would be implementing from a report rather than a reading, and the
  *mechanism* is what decides whether an opponent's Cannon obeys the same
  reload rate the player's does.
- Worth checking in the same pass, since it is the same record and the same
  kind of claim: whether `+0x15`/`+0x17` are genuinely byte stores or were
  simply the ones a byte search could see.
