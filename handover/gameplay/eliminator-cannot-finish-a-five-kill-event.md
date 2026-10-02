# A parked-player Eliminator finishes at five, slowly; the remaining gap is the original's 85 s

2026-10-02, from the `pulse-eliminator` lane. Evidence: `docs/gameplay/race-modes.md` ("Who is
credited with a kill, and how this build's Eliminator reaches five"). Guarded by
`crates/game/tests/eliminator_finish_ground_truth.rs` (target 5, seeds 5, 9, 13, 16, one test each,
plus the respawn-keeps-its-place test).

## Open

- **Time to five is about 2.7 times the original's**: about 100 to 345 s, median 230 s, over 24
  seeds on `16_Track`, player parked, against 85 s. 22 of 24 finish within six game-minutes.
- **The leader tether is catch-up by slowing the front** (`eliminator_pack_scale`, chosen, not
  measured). It is the lever that makes the mode finish (off: 0 of 8) and it is a throttle
  reduction only. **Accepted by the maintainer 2026-10-02** under the AI-obeys-player-physics
  rule (catch-up by slowing the front is fine; extra speed or thrust for the back is not). The respawn fix alone gives 11 of 24 (it finishes the four pinned seeds); the tether lifts it to 22.
  Wider forward-weapon gates were tried and dropped. A chase on equal speed cannot close; **a
  held-Turbo chase (Turbo is lawful speed) was not tried**.
- **Uncredited deaths**: 11 of 30 in a 233 s run had no landed weapon hit. A Leech Beam's damage
  never sets `last_weapon_hit`, so a kill by beam credits nobody; the original's behaviour for a
  beam kill is not read. Wall deaths are not investigated (why opponents scrape lethally here).
- **Held weapons**: Mine, Bomb, Cannon and Leech Beam together were held for about a quarter of all
  craft-ticks and are rarely usable by a leader or a tail; Eliminator refuses absorbing so those
  craft stop collecting pickups. A discard or a drop rule would free the slot (chosen, would need a label).
- **A backward wrap costs a lap in every mode** (`Standing::update`: lap lowered, gate reset, the
  forward re-crossing earns nothing), so any craft shoved back over the line ends a lap short.
  Not fixed (Single Race must not move); sits against `an_immediate_re_crossing_after_a_backward_wrap_earns_no_lap`.
  The player's Eliminator respawn (`last_on_track`) may lose a lap the same way; unchecked.
- **`WeaponAi_DecideFireOrAbsorb` is not ported.** Its `+0x52` predicted-path test and the fields `+0x44`, `+0x4c`, `+0x5c` are unread.
- **Ghidra data rename rejected**: `g_eliminator_kill_target` for `0x08b30fb0` (Hungarian-prefix
  check). The docs and `names.tsv` are authoritative for the data name.

## Next Steps

1. Credit beam, Cannon and Quake kills if the original does (read `Ship_Damage`'s callers for the
   beam path), then re-measure uncredited deaths.
2. Wall deaths: log where opponents die with no weapon hit and whether the shield was already low.
3. A drop/discard rule for held Mine, Bomb, Cannon and Beam in Eliminator, labelled chosen.
4. A held-Turbo chase: keep Turbo until a craft is 100 to 400 units ahead, then fire it.
5. Port the WeaponAi decision only if the above leaves the finish time far from 85 s.
